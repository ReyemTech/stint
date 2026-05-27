use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    if let Err(e) = build_stint_intents_framework() {
        println!("cargo:warning=StintIntents framework build skipped: {e}");
    }
    tauri_build::build()
}

/// Build the StintIntents Swift package as a dynamic framework, ad-hoc
/// codesign it, inject the App Intents metadata stencil, and place the
/// wrapped framework into `crates/stint-app/Frameworks/StintIntents.framework`
/// where `tauri.conf.json`'s `bundle.macOS.frameworks` picks it up for
/// inclusion in the final app bundle.
///
/// Why framework (not static): static-linking the Swift package's `.o` into
/// stint-app's main Rust binary crashes WebKit's Swift Concurrency on
/// startup (executor lookup SIGSEGV — Tauri's webview and our Swift code
/// disagree on the Swift runtime's executor state). The framework loads
/// in its own dyld image with isolated Swift runtime state, which
/// coexists cleanly with WebKit.
///
/// Trade-off: Apple's App Intents indexer doesn't fully discover intents
/// when they live in a sub-framework rather than the main binary — Siri
/// and Shortcuts.app stay silent. Core Spotlight indexing does work via
/// the framework path. See spec §1.5 for the deferred-scope framing.
///
/// Set `STINT_SKIP_SWIFT_BUILD=1` to skip (useful when iterating on
/// stint-core only).
fn build_stint_intents_framework() -> Result<(), String> {
    if env::var_os("STINT_SKIP_SWIFT_BUILD").is_some_and(|v| !v.is_empty()) {
        return Err("STINT_SKIP_SWIFT_BUILD is set".into());
    }
    if env::var_os("CARGO_CFG_TARGET_OS").is_some_and(|v| v != "macos") {
        return Err("non-macOS target".into());
    }

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").map_err(|e| e.to_string())?;
    let swift_dir = Path::new(&manifest_dir).join("swift/StintIntents");
    let package_swift = swift_dir.join("Package.swift");
    if !package_swift.exists() {
        return Err(format!("missing {}", package_swift.display()));
    }

    println!("cargo:rerun-if-changed={}", package_swift.display());
    let sources_dir = swift_dir.join("Sources/StintIntents");
    if let Ok(entries) = fs::read_dir(&sources_dir) {
        for entry in entries.flatten() {
            print_rerun_if_changed_recursive(&entry.path());
        }
    }
    println!("cargo:rerun-if-env-changed=STINT_SKIP_SWIFT_BUILD");

    let derived_data = swift_dir.join("build/derived");

    let status = Command::new("xcodebuild")
        .current_dir(&swift_dir)
        .args([
            "-scheme",
            "StintIntents",
            "-configuration",
            "Release",
            "-destination",
            "platform=macOS",
            "-derivedDataPath",
            derived_data.to_str().ok_or("derived path not utf8")?,
            "build",
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::inherit())
        .status()
        .map_err(|e| format!("xcodebuild spawn: {e}"))?;
    if !status.success() {
        return Err(format!("xcodebuild exit {status}"));
    }

    let built_framework =
        derived_data.join("Build/Products/Release/PackageFrameworks/StintIntents.framework");
    let metadata_bundle =
        derived_data.join("Build/Products/Release/StintIntents.appintents/Metadata.appintents");
    if !built_framework.exists() {
        return Err(format!("missing {}", built_framework.display()));
    }
    if !metadata_bundle.exists() {
        return Err(format!("missing {}", metadata_bundle.display()));
    }

    let dest = Path::new(&manifest_dir).join("Frameworks/StintIntents.framework");
    let _ = fs::remove_dir_all(&dest);
    copy_dir(&built_framework, &dest).map_err(|e| format!("copy framework: {e}"))?;

    let dest_meta = dest.join("Versions/A/Resources/Metadata.appintents");
    let _ = fs::remove_dir_all(&dest_meta);
    copy_dir(&metadata_bundle, &dest_meta).map_err(|e| format!("copy metadata: {e}"))?;

    let info_plist = dest.join("Versions/A/Resources/Info.plist");
    patch_info_plist(&info_plist).map_err(|e| format!("patch Info.plist: {e}"))?;

    codesign_adhoc(&dest).map_err(|e| format!("codesign framework: {e}"))?;

    // Link the framework into stint-app at build time. Without
    // -needed_framework the linker would dead-strip the LC_LOAD_DYLIB
    // record because no Rust code references its symbols at link time
    // (everything goes through dlsym). @executable_path/../Frameworks
    // matches Tauri's bundle.macOS.frameworks copy destination.
    let frameworks_dir = Path::new(&manifest_dir).join("Frameworks");
    println!("cargo:rustc-link-arg=-Wl,-F,{}", frameworks_dir.display());
    println!("cargo:rustc-link-arg=-Wl,-needed_framework,StintIntents");
    println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path/../Frameworks");
    // The framework was built with -undefined dynamic_lookup; its calls to
    // stint_verb_*, stint_settings_*, etc. need to resolve against this
    // binary's flat namespace at load time. -export_dynamic exposes the
    // Rust #[no_mangle] symbols so dyld finds them; without this, dyld
    // aborts launch with "symbol not found in flat namespace".
    println!("cargo:rustc-link-arg=-Wl,-export_dynamic");

    println!(
        "cargo:warning=StintIntents framework rebuilt at {}",
        dest.display()
    );

    Ok(())
}

fn print_rerun_if_changed_recursive(path: &Path) {
    if let Ok(meta) = fs::metadata(path) {
        if meta.is_dir() {
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.flatten() {
                    print_rerun_if_changed_recursive(&entry.path());
                }
            }
        } else {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }
}

fn copy_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let typ = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if typ.is_symlink() {
            let target = fs::read_link(&src_path)?;
            let _ = std::os::unix::fs::symlink(target, &dst_path);
        } else if typ.is_dir() {
            copy_dir(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

fn codesign_adhoc(framework: &Path) -> Result<(), String> {
    let status = Command::new("codesign")
        .args([
            "--force",
            "--sign",
            "-",
            framework.to_str().ok_or("path not utf8")?,
        ])
        .status()
        .map_err(|e| format!("codesign spawn: {e}"))?;
    if !status.success() {
        return Err(format!("codesign exit {status}"));
    }
    Ok(())
}

fn patch_info_plist(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("missing {}", path.display()));
    }
    let status = Command::new("plutil")
        .args([
            "-insert",
            "NSAppIntentsPackage",
            "-bool",
            "YES",
            path.to_str().ok_or("path not utf8")?,
        ])
        .status()
        .map_err(|e| format!("plutil spawn: {e}"))?;
    if !status.success() {
        let replace = Command::new("plutil")
            .args([
                "-replace",
                "NSAppIntentsPackage",
                "-bool",
                "YES",
                path.to_str().ok_or("path not utf8")?,
            ])
            .status()
            .map_err(|e| format!("plutil replace spawn: {e}"))?;
        if !replace.success() {
            return Err(format!("plutil failed: {replace}"));
        }
    }
    Ok(())
}
