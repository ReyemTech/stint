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

/// Build the StintIntents.framework via xcodebuild and place a stable copy
/// into `crates/stint-app/Frameworks/StintIntents.framework`, which
/// `tauri.conf.json`'s `bundle.macOS.frameworks` references at app bundle
/// time.
///
/// Set `STINT_SKIP_SWIFT_BUILD=1` to skip (useful for stint-core-only
/// development cycles, CI runs that don't need the bundle, etc).
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

    // Rerun-if-changed on every Swift source (cheap glob — depth 3 covers
    // Sources/StintIntents/<area>/<file>.swift).
    println!("cargo:rerun-if-changed={}", package_swift.display());
    let sources_dir = swift_dir.join("Sources/StintIntents");
    if let Ok(entries) = fs::read_dir(&sources_dir) {
        for entry in entries.flatten() {
            print_rerun_if_changed_recursive(&entry.path());
        }
    }
    println!("cargo:rerun-if-env-changed=STINT_SKIP_SWIFT_BUILD");

    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".into());
    // Swift "Release" config maps to cargo "release"; we still build Swift
    // release for both because Tauri's bundle step only consumes release
    // artifacts and the framework is tiny.
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

    // Stable destination for Tauri to consume.
    let dest = Path::new(&manifest_dir).join("Frameworks/StintIntents.framework");
    let _ = fs::remove_dir_all(&dest);
    copy_dir(&built_framework, &dest).map_err(|e| format!("copy framework: {e}"))?;

    // Inject Metadata.appintents into Resources/.
    let dest_meta = dest.join("Versions/A/Resources/Metadata.appintents");
    let _ = fs::remove_dir_all(&dest_meta);
    copy_dir(&metadata_bundle, &dest_meta).map_err(|e| format!("copy metadata: {e}"))?;

    // Patch Info.plist with NSAppIntentsPackage=YES so macOS auto-discovers
    // the embedded intents when the framework loads.
    let info_plist = dest.join("Versions/A/Resources/Info.plist");
    patch_info_plist(&info_plist).map_err(|e| format!("patch Info.plist: {e}"))?;

    // The framework was signed by xcodebuild. Our copy + Metadata injection +
    // Info.plist patch invalidated that signature. Re-sign ad-hoc so the
    // framework loads at runtime (Gatekeeper rejects modified-but-signed
    // bundles with "code has no resources but signature indicates they must
    // be present"). Release builds get re-signed by the Tauri bundle step
    // with a real identity; the ad-hoc signature here is just a stable base.
    codesign_adhoc(&dest).map_err(|e| format!("codesign framework: {e}"))?;

    // Tell cargo to link the stint-app binary against StintIntents.framework
    // so the framework loads at app launch (and its @_cdecl symbols become
    // resolvable via dlsym(RTLD_DEFAULT)). Without this the framework would
    // sit unused in Contents/Frameworks/ — nothing pulls it into the process.
    //
    // -F <dir>:       framework search path
    // -framework:     link directive
    // -rpath @exec/.. /Frameworks: where dyld looks at launch time when the
    //                 binary is run from inside an .app bundle. Matches what
    //                 Tauri's bundle step copies to.
    let frameworks_dir = Path::new(&manifest_dir).join("Frameworks");
    println!(
        "cargo:rustc-link-search=framework={}",
        frameworks_dir.display()
    );
    // Use -needed_framework rather than -framework so ld doesn't dead-strip
    // the LC_LOAD_DYLIB record when no Rust code references the framework's
    // symbols at link time (all our calls go through dlsym).
    println!("cargo:rustc-link-arg=-Wl,-F,{}", frameworks_dir.display());
    println!("cargo:rustc-link-arg=-Wl,-needed_framework,StintIntents");
    println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path/../Frameworks");
    // Export the stint_verb_* / stint_settings_* / stint_log_warn /
    // stint_current_focus_id / stint_free_string symbols (statically linked
    // from libstint_core) so the dynamically-loaded StintIntents framework
    // can resolve them at app launch — the framework was built with
    // `-undefined dynamic_lookup`, expecting the host to provide them.
    println!("cargo:rustc-link-arg=-Wl,-export_dynamic");

    println!(
        "cargo:warning=StintIntents framework rebuilt at {} (profile={})",
        dest.display(),
        profile
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
            // best-effort symlink; ignore if it already exists
            let _ = std::os::unix::fs::symlink(target, &dst_path);
        } else if typ.is_dir() {
            copy_dir(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// Ad-hoc re-sign the framework. After build.rs injects Metadata.appintents
/// and patches Info.plist, the original xcodebuild signature no longer matches
/// the on-disk state. `codesign --force --sign -` overwrites with an ad-hoc
/// signature, which is enough for dev/local runs. CI release builds re-sign
/// the framework with the real Apple identity in release-artifacts.yml.
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

/// Inject `NSAppIntentsPackage=true` into the framework Info.plist via
/// `plutil`. The plist is generated by xcodebuild with the standard
/// framework keys; we just need to add the App Intents marker.
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
        // -insert errors if the key already exists; try -replace as a fallback.
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
