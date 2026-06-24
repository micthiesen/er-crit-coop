//! Phase 2 — the functional patch.
//!
//! Strategy A ("continuous clear"): every frame, for each open-field enemy, clear
//! the runtime invulnerability state that the critical-hit (riposte/backstab) TAE
//! event sets on its `ChrIns`. The TAE re-asserts the flag each frame while the
//! crit animation plays, so we clear it each frame too — the same technique the
//! community "freeze" cheat-table scripts use. Because this mutates the enemy's
//! authoritative combat state (not local animation playback), it holds up under
//! Seamless Co-op's host-authoritative model as long as the host runs the mod.
//!
//! No game functions are hooked or called — we only flip a bit in memory that the
//! game already owns — so there's nothing version-specific to break except the
//! offset of the bit itself, which is captured by [`TARGET`] below.

// Until the diagnostic sets TARGET to Some(..), the addressing variants are unused.
#![allow(dead_code)]

use std::time::Duration;

use eldenring::cs::{ChrIns, WorldChrMan};
use fromsoftware_shared::FromStatic;

/// Tighter than the diagnostic cadence: we want to win the race against the TAE
/// re-setting the flag, comfortably inside the multi-second crit animation.
const TICK: Duration = Duration::from_millis(8);

/// Where the throw/critical invulnerability bit lives, relative to a base.
///
/// **This is the one value the in-game diagnostic session pins down.** Until then
/// it's `None` and the patch loop is a safe no-op (logs that it's unconfigured),
/// so shipping this build can't misbehave. Once the diagnostic identifies the
/// flipping bit, set it here and rebuild — that's the whole change.
const TARGET: Option<InvulnBit> = None;

/// A single invulnerability bit to clear, addressed relative to a base pointer.
#[derive(Clone, Copy)]
struct InvulnBit {
    base: Base,
    /// Byte offset from `base`.
    offset: usize,
    /// Bit index within that byte (0..=7).
    bit: u8,
}

#[derive(Clone, Copy)]
enum Base {
    /// Offset is relative to the `ChrIns` itself.
    ChrIns,
    /// Offset is relative to the `ChrInsModuleContainer` (`ChrIns + 0x190` dereferenced).
    /// The TGA cheat table located a per-instance no-damage flag at module + 0x10EF8.
    ModuleContainer,
}

impl InvulnBit {
    /// Resolve the absolute address of the target byte for a given character.
    ///
    /// # Safety
    /// `chr` must be a live `ChrIns`. For [`Base::ModuleContainer`] the module
    /// pointer must be valid and the container at least `offset + 1` bytes large.
    unsafe fn byte_ptr(&self, chr: &ChrIns) -> *mut u8 {
        let base = match self.base {
            Base::ChrIns => chr as *const ChrIns as *mut u8,
            Base::ModuleContainer => {
                // `chr.modules` is an OwnedPtr to the container; take its address.
                (&*chr.modules) as *const _ as *mut u8
            }
        };
        unsafe { base.add(self.offset) }
    }

    /// Clear the bit on this character if it's set.
    unsafe fn clear_on(&self, chr: &ChrIns) {
        unsafe {
            let p = self.byte_ptr(chr);
            let mask = 1u8 << self.bit;
            if *p & mask != 0 {
                *p &= !mask;
            }
        }
    }
}

pub fn patch_loop() {
    let Some(target) = TARGET else {
        log::warn!(
            "patch mode active but TARGET is unconfigured \u{2014} no-op. \
             Run the diagnostic session to identify the invuln bit, then set TARGET."
        );
        return;
    };

    log::info!(
        "patch loop active: clearing {:?}+{:#x} bit {}",
        target.base_name(),
        target.offset,
        target.bit
    );

    loop {
        if let Ok(wcm) = unsafe { WorldChrMan::instance() } {
            for chr in wcm.open_field_chr_set.base.characters() {
                // SAFETY: characters() yields live ChrIns; target resolved per above.
                unsafe { target.clear_on(chr) };
            }
        }
        std::thread::sleep(TICK);
    }
}

impl InvulnBit {
    fn base_name(&self) -> &'static str {
        match self.base {
            Base::ChrIns => "ChrIns",
            Base::ModuleContainer => "ModuleContainer",
        }
    }
}
