//! er-crit-coop — let Seamless Co-op partners damage enemies during critical
//! (riposte/backstab) animations.
//!
//! Vanilla makes the enemy invulnerable for the crit window via TAE "Event Type 0,
//! flag 67 (Invincible excluding Throw Attacks)", which sets runtime state on the
//! enemy's `ChrIns`. That state isn't reachable from `regulation.bin` params and
//! the location isn't publicly documented, so the work splits in two:
//!
//!   * [`MODE`] `Diagnostic` — observe where the invuln state lives (see `diagnostic`),
//!   * [`MODE`] `Patch` — clear it each frame so the enemy stays damageable (see `patch`).
//!
//! Default is `Diagnostic`. Once the in-game session pins the bit (set in `patch::TARGET`),
//! flip `MODE` to `Patch` and rebuild — that's the whole switch to the working mod.

use std::ffi::c_void;

use windows::Win32::Foundation::HINSTANCE;
use windows::Win32::System::SystemServices::DLL_PROCESS_ATTACH;
use windows::core::BOOL;

mod diagnostic;
mod logger;
mod patch;

#[allow(dead_code)] // Patch is selected by flipping MODE after the diagnostic session.
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
