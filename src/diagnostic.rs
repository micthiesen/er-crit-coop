//! Phase 1 — observe, don't patch.
//!
//! Periodically snapshots every open-field character and logs only on change, so a
//! riposte on a lone enemy stands out. We capture three things that could carry the
//! throw-invuln state, each read from memory the SDK guarantees is valid:
//!   * a raw `ChrIns` byte window (named flag bytes at 0x1c4 + surrounding state),
//!   * the heads of the combat-relevant modules (`data`, `action_flag`, `throw`,
//!     `super_armor`) reached via the container's typed pointers, and
//!   * the list of active SpEffect param ids.

use std::collections::HashMap;
use std::time::Duration;

use eldenring::cs::{ChrIns, WorldChrMan};
use fromsoftware_shared::FromStatic;

const SCAN_INTERVAL: Duration = Duration::from_millis(120);

/// Raw `ChrIns` window: covers the named flag bytes at 0x1c4..0x1cb plus nearby state.
const CHRINS_DUMP_START: usize = 0x1c0;
const CHRINS_DUMP_LEN: usize = 0x60;

/// Bytes to read from the head of each sampled module.
const MODULE_HEAD_LEN: usize = 0x40;

#[derive(PartialEq)]
struct Snapshot {
    chrins: Vec<u8>,
    data_mod: Vec<u8>,
    action_flag_mod: Vec<u8>,
    throw_mod: Vec<u8>,
    super_armor_mod: Vec<u8>,
    speffects: Vec<i32>,
}

/// Read `len` bytes starting at `ptr`. Caller guarantees the range is mapped.
unsafe fn read_bytes(ptr: *const u8, len: usize) -> Vec<u8> {
    let mut buf = vec![0u8; len];
    unsafe { std::ptr::copy_nonoverlapping(ptr, buf.as_mut_ptr(), len) };
    buf
}

fn snapshot(chr: &ChrIns) -> Snapshot {
    let chr_base = chr as *const ChrIns as *const u8;
    let modules = &*chr.modules;
    Snapshot {
        chrins: unsafe { read_bytes(chr_base.add(CHRINS_DUMP_START), CHRINS_DUMP_LEN) },
        data_mod: unsafe {
            read_bytes((&*modules.data) as *const _ as *const u8, MODULE_HEAD_LEN)
        },
        action_flag_mod: unsafe {
            read_bytes((&*modules.action_flag) as *const _ as *const u8, MODULE_HEAD_LEN)
        },
        throw_mod: unsafe {
            read_bytes((&*modules.throw) as *const _ as *const u8, MODULE_HEAD_LEN)
        },
        super_armor_mod: unsafe {
            read_bytes((&*modules.super_armor) as *const _ as *const u8, MODULE_HEAD_LEN)
        },
        speffects: chr.special_effect.entries().map(|e| e.param_id).collect(),
    }
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

pub fn diagnostic_loop() {
    log::info!("diagnostic thread alive; waiting for WorldChrMan...");

    loop {
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
                        "cid={} type={:?} ptr={:#x} | chrins@{:#x}={} | data={} aflag={} throw={} sa={} | spfx={:?}",
                        chr.character_id,
                        chr.chr_type,
                        key,
                        CHRINS_DUMP_START,
                        hex(&snap.chrins),
                        hex(&snap.data_mod),
                        hex(&snap.action_flag_mod),
                        hex(&snap.throw_mod),
                        hex(&snap.super_armor_mod),
                        snap.speffects,
                    );
                    last.insert(key, snap);
                }
            }

            std::thread::sleep(SCAN_INTERVAL);
        }
    }
}
