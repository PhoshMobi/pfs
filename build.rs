use std::env;
use std::path::Path;
use std::process::Command;

fn build_resources() {
    if env::var("PFS_RESOURCE_DIR").is_ok() {
        println!("Skipping resources building as resources have been already built");
        return;
    }

    let target = Path::new(&env::var("OUT_DIR").unwrap()).join("pfs.gresource");
    let output = Command::new("glib-compile-resources")
        .arg("src/pfs.gresource.xml")
        .arg("--sourcedir")
        .arg("src")
        .arg("--target")
        .arg(&target)
        .output()
        .unwrap();

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        for line in error.lines() {
            println!("cargo::error={line}");
        }
        println!("cargo::error={}", "Failed to bundle resources",);
        return;
    }

    println!(
        "cargo::rustc-env=PFS_RESOURCE_DIR={}",
        env::var("OUT_DIR").unwrap()
    );
}

fn main() {
    build_resources();
}
