//! LOCAL-ONLY (branch `local/win10-agility`, never pushed): the D3D12
//! Agility SDK exports.
//!
//! Windows 10's in-box `d3d12.dll` stops at shader model 6.5 and refuses root
//! signatures with the directly indexed heap flags (0x400, 0x800), which every
//! translated pipeline uses: "Unsupported bit-flag set (root signature flags
//! c00)". D3D12 reads these two symbols from the process executable; when they
//! name a newer runtime than the system's, it loads `D3D12Core.dll` from
//! `D3D12SDKPath`, relative to the executable.
//!
//! `build.rs` exports both. `D3D12Core.dll` (package `Microsoft.Direct3D.D3D12`
//! 1.620.2, whose minor version is the number below) must sit in `.\D3D12\`
//! beside `victory.exe`. Windows 11 or a missing folder keeps the in-box one.
#![allow(non_upper_case_globals)]

/// The runtime's SDK version: the minor number of the package (1.620.2).
/// Writable: [`choose`] sets it to the system runtime's own version when
/// there is no `D3D12Core.dll` beside the executable, and D3D12 then keeps
/// the system's runtime. (A version the
/// system's runtime is older than, with no runtime at the path, is not a
/// fallback: device creation fails with `D3D12_ERROR_INVALID_REDIST`,
/// 0x887E0003, as on Windows 11 26200.) D3D12 reads the value when it loads
/// its runtime, at the first D3D12 call, after `main` has begun.
#[unsafe(no_mangle)]
pub static mut D3D12SDKVersion: u32 = 620;

/// Whether `D3D12Core.dll` lies in `.\D3D12\` beside the executable; if not,
/// [`D3D12SDKVersion`] is made 0 (the system's runtime). Called first thing
/// in `main`, before anything of D3D12.
pub fn choose() -> bool {
    let present = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join("D3D12").join("D3D12Core.dll")))
        .is_some_and(|dll| dll.is_file());
    if !present && let Some(system) = system_version() {
        // The system's own runtime's version: D3D12 then loads the runtime
        // it has (0 is no version: 0x887E0003 all the same).
        // SAFETY: one thread so far (the start of `main`); D3D12 has not
        // read the value yet.
        unsafe { std::ptr::addr_of_mut!(D3D12SDKVersion).write_volatile(system) };
    }
    present
}

/// The `D3D12SDKVersion` the system's `D3D12Core.dll` exports (616 on
/// Windows 11 26100), read from the file's export table without loading it;
/// `None` if there is no such file or export.
fn system_version() -> Option<u32> {
    let dir = std::env::var_os("SystemRoot").map(std::path::PathBuf::from)?.join("System32");
    let d = std::fs::read(dir.join("D3D12Core.dll")).ok()?;
    let u16_at = |o: usize| d.get(o..o + 2).map(|b| u16::from_le_bytes([b[0], b[1]]));
    let u32_at = |o: usize| d.get(o..o + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
    let pe = u32_at(0x3c)? as usize;
    let optional = pe + 24;
    let dirs = optional + if u16_at(optional)? == 0x20b { 112 } else { 96 };
    let exports = u32_at(dirs)? as usize;
    let sections = u16_at(pe + 6)? as usize;
    let first = optional + u16_at(pe + 20)? as usize;
    let at = |rva: usize| -> Option<usize> {
        (0..sections).find_map(|i| {
            let s = first + i * 40;
            let (size, va, raw_size, raw) =
                (u32_at(s + 8)? as usize, u32_at(s + 12)? as usize, u32_at(s + 16)? as usize, u32_at(s + 20)? as usize);
            (va <= rva && rva < va + size.max(raw_size)).then_some(raw + rva - va)
        })
    };
    let e = at(exports)?;
    let (functions, names, ordinals) = (u32_at(e + 28)? as usize, u32_at(e + 32)? as usize, u32_at(e + 36)? as usize);
    let count = u32_at(e + 24)? as usize;
    (0..count).find_map(|i| {
        let name = at(u32_at(at(names)? + 4 * i)? as usize)?;
        let end = d[name..].iter().position(|&b| b == 0)?;
        if &d[name..name + end] != b"D3D12SDKVersion" {
            return None;
        }
        let ordinal = u16_at(at(ordinals)? + 2 * i)? as usize;
        u32_at(at(u32_at(at(functions)? + 4 * ordinal)? as usize)?)
    })
}

/// A C string pointer D3D12 reads once at device creation.
#[repr(transparent)]
pub struct SdkPath(*const std::ffi::c_char);

// SAFETY: points at a static, immutable C string.
unsafe impl Sync for SdkPath {}

/// Where `D3D12Core.dll` is, relative to the executable.
#[unsafe(no_mangle)]
pub static D3D12SDKPath: SdkPath = SdkPath(c".\\D3D12\\".as_ptr());
