//! er-crit-coop — let Seamless Co-op partners damage enemies during critical
//! (riposte/backstab) animations.
//!
//! Vanilla makes the enemy invulnerable for the crit window via TAE "Event Type 0,
//! action 67 (Invincible excluding Throw Attacks)", a runtime flag on the enemy's
//! `ChrIns`. It isn't reachable from `regulation.bin` params, but the fromsoftware-rs
//! SDK exposes it as a typed field, so the mod just clears it. Two modes:
//!
//!   * [`MODE`] `Patch` (default) — clear the flag each frame so the enemy stays
//!     damageable during crits (see `patch`).
//!   * [`MODE`] `Diagnostic` — log enemy flag/state changes instead of patching, to
//!     investigate an enemy that uses a different invuln flag (see `diagnostic`).

use std::ffi::c_void;

use windows::Win32::Foundation::HINSTANCE;
use windows::Win32::System::SystemServices::DLL_PROCESS_ATTACH;
use windows::core::BOOL;

mod diagnostic;
mod logger;
mod patch;

#[allow(dead_code)] // One variant is always unused: MODE is a compile-time constant.
enum Mode {
    Diagnostic,
    Patch,
}

/// Active behavior. The SDK exposes the crit-invuln flag by name, so the patch is
/// the default; `Diagnostic` is kept for investigating any enemies it misses.
const MODE: Mode = Mode::Patch;

#[unsafe(no_mangle)]
unsafe extern "system" fn DllMain(_: HINSTANCE, reason: u32, _: *mut c_void) -> BOOL {
    if reason == DLL_PROCESS_ATTACH {
        logger::init();
        std::thread::spawn(|| match MODE {
            Mode::Diagnostic => diagnostic::diagnostic_loop(),
            Mode::Patch => patch::patch_loop(),
        });
    }
    true.into()
}
