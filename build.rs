//! Tell rustc where libopenmpt lives.
//!
//! `openmpt-sys`'s build.rs only emits `-lopenmpt` — it relies on the system
//! linker to find the library, which fails on macOS/Homebrew where libs live
//! outside the default search path. We ask pkg-config and emit the right
//! `rustc-link-search`. Falls back to common locations if pkg-config is absent.

use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=PKG_CONFIG_PATH");
    println!("cargo:rerun-if-env-changed=RTRAX_OPENMPT_LIB_DIR");

    if let Ok(dir) = std::env::var("RTRAX_OPENMPT_LIB_DIR") {
        println!("cargo:rustc-link-search=native={dir}");
        return;
    }

    if let Some(libs) = pkg_config_libs() {
        for path in libs {
            println!("cargo:rustc-link-search=native={path}");
        }
        return;
    }

    // Last-ditch fallbacks for common install locations.
    for path in [
        "/opt/homebrew/lib",       // macOS / Apple Silicon brew
        "/usr/local/lib",          // macOS / Intel brew, FreeBSD
        "/usr/lib/x86_64-linux-gnu",
        "/usr/lib/aarch64-linux-gnu",
    ] {
        if std::path::Path::new(path).join("libopenmpt.so").exists()
            || std::path::Path::new(path).join("libopenmpt.dylib").exists()
        {
            println!("cargo:rustc-link-search=native={path}");
        }
    }
}

fn pkg_config_libs() -> Option<Vec<String>> {
    let out = Command::new("pkg-config")
        .args(["--libs-only-L", "libopenmpt"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?;
    let dirs: Vec<String> = s
        .split_whitespace()
        .filter_map(|tok| tok.strip_prefix("-L").map(|s| s.to_string()))
        .collect();
    if dirs.is_empty() { None } else { Some(dirs) }
}
