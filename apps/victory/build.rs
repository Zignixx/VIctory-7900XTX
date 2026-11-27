// LOCAL-ONLY (branch local/win10-agility, never pushed): exports the two
// symbols D3D12 reads from the process executable to select an app-local
// D3D12 Agility SDK runtime. Without them, Windows 10's in-box d3d12.dll
// refuses every translated pipeline's root signature
// ("Unsupported bit-flag set (root signature flags c00)").
fn main() {
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        println!("cargo:rustc-link-arg-bins=/EXPORT:D3D12SDKVersion,DATA");
        println!("cargo:rustc-link-arg-bins=/EXPORT:D3D12SDKPath,DATA");
        // `d3d12.dll` loaded at its first call, not with the executable: it
        // reads the two exports when it loads, and `main` first sets the
        // version to the system runtime's when there is no runtime beside
        // the executable (`agility::choose`).
        println!("cargo:rustc-link-arg-bins=/DELAYLOAD:d3d12.dll");
        println!("cargo:rustc-link-arg-bins=delayimp.lib");
    }
}
