//! er-crit-coop — phase 1: DIAGNOSTIC build.
//!
//! Goal of the mod: in Seamless Co-op, let other players damage an enemy while
//! it's locked in a critical-hit (riposte/backstab) animation. Vanilla makes the
//! enemy invulnerable for that window via TAE "Event Type 0, flag 67 (Invincible
//! excluding Throw Attacks)", which sets some runtime state on the enemy's ChrIns.
//!
//! Nobody has published *where* that state lives, so this build doesn't patch
//! anything yet — it observes. It periodically snapshots every open-field
//! character's active SpEffects and a raw byte window of its ChrIns, and logs
//! only when something changes. Perform a single riposte on a lone enemy and the
//! invuln window will show up as a distinct change against that enemy's pointer.
//! That tells us the exact flag/speffect to clear in phase 2.

use std::collections::HashMap;
use std::ffi::c_void;
use std::time::Duration;

use eldenring::cs::{ChrIns, WorldChrMan};
use fromsoftware_shared::FromStatic;
use windows::Win32::Foundation::HINSTANCE;
use windows::Win32::System::SystemServices::DLL_PROCESS_ATTACH;
use windows::core::BOOL;

mod logger;

/// How often to sample the world.
const SCAN_INTERVAL: Duration = Duration::from_millis(120);

/// Raw ChrIns byte window to snapshot, relative to the ChrIns base. Covers the
/// named flag bytes at 0x1c4..0x1cb (incl. `is_invincible`) plus surrounding
/// combat state, which is the most likely home of the throw-invuln bit.
const DUMP_START: usize = 0x1c0;
const DUMP_LEN: usize = 0x60; // 0x1c0 .. 0x220

#[derive(PartialEq)]
struct Snapshot {
    bytes: Vec<u8>,
    speffects: Vec<i32>,
}

fn snapshot(chr: &ChrIns) -> Snapshot {
    let base = chr as *const ChrIns as *const u8;
    let mut bytes = vec![0u8; DUMP_LEN];
    // SAFETY: ChrIns is far larger than DUMP_START + DUMP_LEN; reading these bytes
    // is in-bounds for a live ChrIns.
    unsafe {
        std::ptr::copy_nonoverlapping(base.add(DUMP_START), bytes.as_mut_ptr(), DUMP_LEN);
    }
    let speffects = chr.special_effect.entries().map(|e| e.param_id).collect();
    Snapshot { bytes, speffects }
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

fn diagnostic_loop() {
    log::info!("diagnostic thread alive; waiting for WorldChrMan...");

    // Outer loop: (re)acquire WorldChrMan across loading screens / world resets.
    loop {
        // SAFETY: called from our own thread; we only read. WorldChrMan may not
        // exist yet (main menu) so this can fail until the world loads.
        if unsafe { WorldChrMan::instance() }.is_err() {
            std::thread::sleep(Duration::from_secs(1));
            continue;
        }

        log::info!("WorldChrMan up; scanning open-field characters");
        let mut last: HashMap<usize, Snapshot> = HashMap::new();

        loop {
            let Ok(wcm) = (unsafe { WorldChrMan::instance() }) else {
                log::info!("WorldChrMan gone; re-acquiring");
                break;
            };

            for chr in wcm.open_field_chr_set.base.characters() {
                let key = chr as *const ChrIns as usize;
                let snap = snapshot(chr);

                if last.get(&key) != Some(&snap) {
                    log::info!(
                        "cid={} type={:?} ptr={:#x} b@{:#x}={} spfx={:?}",
                        chr.character_id,
                        chr.chr_type,
                        key,
                        DUMP_START,
                        hex(&snap.bytes),
                        snap.speffects,
                    );
                    last.insert(key, snap);
                }
            }

            std::thread::sleep(SCAN_INTERVAL);
        }
    }
}

#[unsafe(no_mangle)]
unsafe extern "system" fn DllMain(_: HINSTANCE, reason: u32, _: *mut c_void) -> BOOL {
    if reason == DLL_PROCESS_ATTACH {
        logger::init();
        std::thread::spawn(diagnostic_loop);
    }
    true.into()
}
