use std::env;
use std::error::Error;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn Error>>
{
    if env::var_os("CARGO_FEATURE_NATIVE_FIXTURE").is_none() {
        return Ok(());
    }
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap_or_default());
    let source = manifest.join("testlib").join("testlib.c");
    println!("cargo:rerun-if-changed={}", source.display());

    let target = env::var("TARGET").unwrap_or_default();
    let extension = if target.contains("windows") {
        "dll"
    }
    else if target.contains("apple") {
        "dylib"
    }
    else {
        "so"
    };
    let compiler = cc::Build::new().get_compiler();
    let msvc = compiler.is_like_msvc();
    let prefix = if msvc { "" } else { "lib" };
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap_or_default());
    let output = out_dir.join(format!("{prefix}gandr_testlib.{extension}"));
    let mut command = compiler.to_command();
    if msvc {
        let object = out_dir.join("gandr_testlib.obj");
        let import = out_dir.join("gandr_testlib.lib");
        let pdb = out_dir.join("gandr_testlib.pdb");
        command
            .arg("/LD")
            .arg(&source)
            // typos:ignore
            .arg(format!("/Fo{}", object.display()))
            .arg("/link")
            .arg(format!("/OUT:{}", output.display()))
            .arg(format!("/IMPLIB:{}", import.display()))
            .arg(format!("/PDB:{}", pdb.display()));
    }
    else {
        command.arg("-o").arg(&output).arg(&source);
        if target.contains("apple") {
            command.arg("-dynamiclib");
        }
        else {
            command.arg("-shared");
            if !target.contains("windows") {
                command.arg("-fPIC");
            }
        }
    }
    let status = command.status()?;
    if !status.success() {
        return Err(format!("native testlib compiler exited with {status}").into());
    }
    println!("cargo:rustc-env=GANDR_TESTLIB={}", output.display());
    Ok(())
}
