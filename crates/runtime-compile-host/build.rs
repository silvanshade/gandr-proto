//! Binding the compilation host at link time, under the `full` feature.
//!
//! # Contract
//! - requires: with `full` enabled, a workspace whose `compile-host:build` task
//!   can run; without it, nothing at all.
//! - ensures: with `full` enabled, the crate links the host's C boundary and
//!   every entry it declares is resolved by the linker; without it, the crate
//!   acquires no MLIR, no C++ toolchain, and no link flags.
//! - provides: the one place the bridge's link posture is decided.
//! - fails: by returning the reason, which cargo prints and stops on.
//! - panics: none.

use core::error::Error;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

/// The workspace root, from this crate's manifest.
fn workspace_root() -> Result<PathBuf, Box<dyn Error>>
{
    let manifest =
        std::env::var_os("CARGO_MANIFEST_DIR").ok_or("cargo set no CARGO_MANIFEST_DIR")?;
    let manifest = PathBuf::from(manifest);
    let root = manifest
        .parent()
        .and_then(Path::parent)
        .ok_or("this crate does not sit under crates/<name>")?;
    Ok(root.to_path_buf())
}

/// Rebuilds when the host's own sources or its pin move.
fn watch(root: &Path)
{
    for relative in [
        "runtime/compile-host/src",
        "runtime/compile-host/include",
        "runtime/compile-host/CMakeLists.txt",
        "runtime/compile-host/cmake/mlir-pin.cmake",
    ] {
        println!("cargo:rerun-if-changed={}", root.join(relative).display());
    }
}

/// One library, as the host's own build named it in its link line.
#[repr(transparent)]
struct LinkEntry<'line>(&'line str);

/// Emits one library from the link line the host's own build wrote.
fn emit(
    entry: &LinkEntry<'_>,
    rpaths: &mut Vec<PathBuf>,
) -> Result<(), Box<dyn Error>>
{
    let entry = entry.0;
    let path = Path::new(entry);
    let directory = path
        .parent()
        .ok_or_else(|| format!("the link line entry {entry} names no directory"))?;
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .and_then(|stem| stem.strip_prefix("lib"))
        .ok_or_else(|| format!("the link line entry {entry} is not a library file"))?;
    let kind = match path.extension().and_then(|extension| extension.to_str()) {
        | Some("a") => "static",
        | Some("dylib" | "so") => "dylib",
        | _ => {
            return Err(format!(
                "the link line entry {entry} has an extension this build does not know"
            )
            .into());
        },
    };
    println!("cargo:rustc-link-search=native={}", directory.display());
    println!("cargo:rustc-link-lib={kind}={stem}");

    // Both aggregate libraries sit in the keg's one directory, so the rpath is
    // emitted once rather than per library.
    if kind == "dylib" && !rpaths.iter().any(|seen| seen == directory) {
        rpaths.push(directory.to_path_buf());
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", directory.display());
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>>
{
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_FULL");

    // Without `full` this build script does nothing, which is the property the
    // default dependency graph rests on: no MLIR, no C++ toolchain, no link
    // flags, nothing to find. `mise run compile-host:default-graph` is the
    // witness that it stays that way.
    if std::env::var_os("CARGO_FEATURE_FULL").is_none() {
        return Ok(());
    }

    let root = workspace_root()?;
    watch(&root);

    // The host is built through its own task rather than by re-implementing
    // the toolchain resolution here. The task owns the pin, the keg match, and
    // the exception and RTTI posture; a second implementation of any of them
    // would be a second thing to keep true.
    let status = Command::new("mise")
        .arg("run")
        .arg("compile-host:build")
        .current_dir(&root)
        .status()
        .map_err(|error| {
            format!(
                "could not run `mise run compile-host:build` ({error}); the `full` feature links \
                 the compilation host, which that task owns"
            )
        })?;
    if !status.success() {
        return Err(format!(
            "the compilation host did not build ({status}); the `full` feature links it, so run \
             `mise run compile-host:build` to see the failure"
        )
        .into());
    }

    // The link line is WRITTEN BY THE BUILD THAT OWNS THE DISCIPLINE, never
    // reconstructed here. `runtime/compile-host/CMakeLists.txt` carries the
    // rule that the aggregate shared libraries are linked and the component
    // archives never are — mixing them gives the process two MLIR type-storage
    // registries and the first dialect `Type::get` crashes inside the uniquer
    // with no diagnostic. Restating that rule in this file would be a copy that
    // can drift from the one that matters.
    let link_file = root.join("runtime/compile-host/build/gandr-compile-host-link.txt");
    let line = std::fs::read_to_string(&link_file).map_err(|error| {
        format!(
            "the host built but wrote no link line at {} ({error})",
            link_file.display()
        )
    })?;

    let mut rpaths: Vec<PathBuf> = Vec::new();
    for entry in line.lines().filter(|entry| !entry.trim().is_empty()) {
        emit(&LinkEntry(entry), &mut rpaths)?;
    }

    // The host is C++ and the archives above carry no standard library of
    // their own.
    println!("cargo:rustc-link-lib=dylib=c++");
    Ok(())
}
