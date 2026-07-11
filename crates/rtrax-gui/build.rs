//! Embed the icon + version resource into the Windows executable.
//!
//! Guarded by `cfg(windows)` (host), which is fine because Windows builds are
//! native in CI and for users; a Linux→Windows cross-compile would just skip
//! the icon, not fail.

fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("ProductName", "rtrax");
        res.set("FileDescription", "rtrax — MOD/XM/IT/S3M module player");
        if let Err(err) = res.compile() {
            // Missing rc.exe etc. shouldn't fail the build over an icon.
            println!("cargo:warning=failed to embed Windows resources: {err}");
        }
    }
}
