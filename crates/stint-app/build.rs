use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    if let Err(e) = build_stint_intents() {
        println!("cargo:warning=StintIntents build skipped: {e}");
    }
    tauri_build::build()
}

/// Build the StintIntents Swift package as a static library, link its
/// merged .o file into stint-app, and copy the App Intents metadata
/// stencil to a path Tauri's bundle stage can consume.
///
/// Why static: macOS's App Intents indexer only scans the main app binary's
/// Swift module for type metadata. When intents lived in an embedded
/// .framework, siriactionsd / Shortcuts.app silently skipped them. Linking
/// the Swift `.o` into stint-app puts the types directly in the main
/// binary's Mach-O where the indexer can find them.
///
/// Set `STINT_SKIP_SWIFT_BUILD=1` to skip (useful when iterating on
/// stint-core only).
fn build_stint_intents() -> Result<(), String> {
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

    // xcodebuild (not plain `swift build`) so appintentsmetadataprocessor
    // runs as a build phase and emits the Metadata.appintents stencil.
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

    let release_dir = derived_data.join("Build/Products/Release");
    let static_obj = release_dir.join("StintIntents.o");
    let stencil_dir = release_dir.join("StintIntents.appintents/Metadata.appintents");
    if !static_obj.exists() {
        return Err(format!("missing {}", static_obj.display()));
    }
    if !stencil_dir.exists() {
        return Err(format!("missing {}", stencil_dir.display()));
    }

    // Stable copy of the merged .o so the cargo link arg points at a path
    // that survives `swift build` rebuilds and doesn't get pruned.
    let stable_obj_dir = Path::new(&manifest_dir).join("build-deps");
    fs::create_dir_all(&stable_obj_dir).map_err(|e| e.to_string())?;
    let stable_obj = stable_obj_dir.join("StintIntents.o");
    fs::copy(&static_obj, &stable_obj).map_err(|e| format!("copy .o: {e}"))?;

    // Stable copy of the metadata stencil. tauri.conf.json references this
    // path under bundle.resources so the stencil ends up at
    // <Stint.app>/Contents/Resources/Metadata.appintents/ where macOS's
    // intent indexer expects to find it.
    let stable_stencil = Path::new(&manifest_dir).join("Metadata.appintents");
    let _ = fs::remove_dir_all(&stable_stencil);
    copy_dir(&stencil_dir, &stable_stencil).map_err(|e| format!("copy stencil: {e}"))?;

    // Link the Swift static .o into stint-app + pull in everything Swift
    // needs at runtime.
    //
    // -force_load (instead of bare path): release LTO would otherwise strip
    // the Swift type metadata records (`_$s12StintIntents...`) because no
    // Rust code references them at link time. Apple's App Intents indexer
    // needs those records present in the main binary's Mach-O to discover
    // the intent types via reflection.
    println!(
        "cargo:rustc-link-arg=-Wl,-force_load,{}",
        stable_obj.display()
    );

    // Swift runtime — macOS ships these in /usr/lib/swift; the linker also
    // needs the toolchain's runtime stub.
    println!("cargo:rustc-link-search=native=/usr/lib/swift");
    let xcode_swift = "/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/lib/swift/macosx";
    println!("cargo:rustc-link-search=native={xcode_swift}");
    println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");

    // Apple frameworks the Swift code references.
    for fw in [
        "AppIntents",
        "CoreSpotlight",
        "UniformTypeIdentifiers",
        "Foundation",
        "CoreFoundation",
    ] {
        println!("cargo:rustc-link-lib=framework={fw}");
    }

    // Export Rust FFI symbols (stint_verb_*, etc) so Swift code statically
    // linked in this same binary can resolve them — they were marked
    // #[no_mangle] but cargo's default visibility doesn't put them in the
    // dynamic symbol table.
    println!("cargo:rustc-link-arg=-Wl,-export_dynamic");

    println!(
        "cargo:warning=StintIntents static linked into stint-app; stencil at {}",
        stable_stencil.display()
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

#[allow(dead_code)]
fn _unused(p: PathBuf) {}
