//! Diagnostic mode — observe, don't patch.
//!
//! A fallback for investigating an enemy that uses a different invuln flag than the
//! patch targets. The throw-invuln state is *transient*: it rises when the crit
//! animation starts and clears when it ends. So instead of dumping raw bytes to diff
//! by hand, this watches every candidate bit per enemy and logs **rising edges**
//! (0->1), suppressing bits that flip constantly (per-frame churn like `force_update`).
//! Riposte a lone enemy and the invuln bit shows up as a rare RISE naming the exact
//! region/offset/bit; map that to a typed SDK field and clear it in `patch.rs` the
//! same way action 67 is.
//!
//! Candidate memory, read via the SDK's typed pointers (assumed valid for a live ChrIns):
//!   * `ChrIns` window at 0x1c0 (named flag bytes incl. `is_invincible` + nearby), and
//!   * heads of the combat-relevant modules: `data`, `action_flag`, `throw`, `super_armor`.
//! Active SpEffects are tracked separately (a crit-only speffect is an alternative lever).

use std::collections::HashMap;
use std::time::Duration;

use eldenring::cs::{ChrIns, WorldChrMan};
use fromsoftware_shared::FromStatic;

const SCAN_INTERVAL: Duration = Duration::from_millis(60);

/// `ChrIns` window.
const CHRINS_START: usize = 0x1c0;
const CHRINS_LEN: usize = 0x60;
/// Bytes read from each module head.
const MODULE_LEN: usize = 0x40;

/// A bit that has flipped more than this many times is per-frame churn, not the
/// once-per-riposte invuln bit — stop reporting it.
const NOISE_FLIPS: u32 = 24;

/// Regions concatenated into the per-enemy snapshot buffer, in order.
/// `(label, len)`; the ChrIns region's real offset is `CHRINS_START + local`.
const REGIONS: [(&str, usize); 5] = [
    ("chrins", CHRINS_LEN),
    ("data", MODULE_LEN),
    ("aflag", MODULE_LEN),
    ("throw", MODULE_LEN),
    ("sa", MODULE_LEN),
];

struct Watch {
    bytes: Vec<u8>,
    /// Flip count per bit (index = byte*8 + bit).
    flips: Vec<u32>,
    speffects: Vec<i32>,
}

unsafe fn read_into(dst: &mut Vec<u8>, ptr: *const u8, len: usize) {
    let start = dst.len();
    dst.resize(start + len, 0);
    unsafe { std::ptr::copy_nonoverlapping(ptr, dst.as_mut_ptr().add(start), len) };
}

fn sample_bytes(chr: &ChrIns) -> Vec<u8> {
    let chr_base = chr as *const ChrIns as *const u8;
    let m = &*chr.modules;
    let mut buf = Vec::with_capacity(CHRINS_LEN + 4 * MODULE_LEN);
    unsafe {
        read_into(&mut buf, chr_base.add(CHRINS_START), CHRINS_LEN);
        read_into(&mut buf, (&*m.data) as *const _ as *const u8, MODULE_LEN);
        read_into(&mut buf, (&*m.action_flag) as *const _ as *const u8, MODULE_LEN);
        read_into(&mut buf, (&*m.throw) as *const _ as *const u8, MODULE_LEN);
        read_into(&mut buf, (&*m.super_armor) as *const _ as *const u8, MODULE_LEN);
    }
    buf
}

/// Map a global byte index in the snapshot buffer to `(region_label, offset_repr)`.
fn locate(byte_idx: usize) -> (&'static str, String) {
    let mut base = 0;
    for (label, len) in REGIONS {
        if byte_idx < base + len {
            let local = byte_idx - base;
            let repr = if label == "chrins" {
                format!("{:#x}", CHRINS_START + local) // real ChrIns offset
            } else {
                format!("+{:#x}", local) // offset into module head
            };
            return (label, repr);
        }
        base += len;
    }
    ("?", format!("{byte_idx:#x}"))
}

pub fn diagnostic_loop() {
    log::info!("diagnostic (edge-detector) alive; waiting for WorldChrMan...");

    loop {
        if unsafe { WorldChrMan::instance() }.is_err() {
            std::thread::sleep(Duration::from_secs(1));
            continue;
        }

        log::info!("WorldChrMan up; watching open-field characters for invuln rising edges");
        let mut watches: HashMap<usize, Watch> = HashMap::new();

        loop {
            let Ok(wcm) = (unsafe { WorldChrMan::instance() }) else {
                log::info!("WorldChrMan gone; re-acquiring");
                break;
            };

            for chr in wcm.open_field_chr_set.base.characters() {
                let key = chr as *const ChrIns as usize;
                let bytes = sample_bytes(chr);
                let speffects: Vec<i32> = chr.special_effect.entries().map(|e| e.param_id).collect();

                let Some(prev) = watches.get_mut(&key) else {
                    let bits = bytes.len() * 8;
                    watches.insert(
                        key,
                        Watch { bytes, flips: vec![0; bits], speffects },
                    );
                    continue;
                };

                // SpEffect changes (additions are the interesting signal).
                if prev.speffects != speffects {
                    let added: Vec<i32> =
                        speffects.iter().copied().filter(|s| !prev.speffects.contains(s)).collect();
                    if !added.is_empty() {
                        log::info!("SPFX+ cid={} ptr={:#x} added={:?}", chr.character_id, key, added);
                    }
                    prev.speffects = speffects;
                }

                // Bit edges across the whole snapshot.
                if prev.bytes.len() == bytes.len() {
                    for i in 0..bytes.len() {
                        let diff = prev.bytes[i] ^ bytes[i];
                        if diff == 0 {
                            continue;
                        }
                        for bit in 0..8u8 {
                            let mask = 1u8 << bit;
                            if diff & mask == 0 {
                                continue;
                            }
                            let idx = i * 8 + bit as usize;
                            prev.flips[idx] += 1;
                            let rising = bytes[i] & mask != 0;
                            if rising && prev.flips[idx] <= NOISE_FLIPS {
                                let (region, off) = locate(i);
                                log::info!(
                                    "RISE cid={} ptr={:#x} region={} off={} bit={} (rises~{})",
                                    chr.character_id, key, region, off, bit, prev.flips[idx]
                                );
                            }
                        }
                    }
                    prev.bytes = bytes;
                }
            }

            std::thread::sleep(SCAN_INTERVAL);
        }
    }
}
