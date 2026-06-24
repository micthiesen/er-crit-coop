//! The functional patch, run as a game main-thread task.
//!
//! Clears the riposte-victim invulnerability (`CSChrActionFlagModule`
//! `action_modifiers_flags::invincible_excluding_throw_attacks_defender`, TAE action 67)
//! on every open-field enemy each frame, so co-op partners can damage an enemy during a
//! crit (riposte/backstab/guard counter) instead of it being immune to everyone but the
//! player landing the crit.
//!
//! It runs as a recurring task in the `WorldChrMan_PostPhysics` phase: after the character
//! behavior update has (re)set the flag for the frame, and immediately before `DmgMan`
//! applies damage. Because the closure runs on the game's main thread, in step with the
//! game, the character set is stable while we touch it: no cross-thread data race, and no
//! pointer-validity guards, atomics, or raw `&mut` re-derivation are needed (contrast the
//! earlier free-running background-thread version). We only flip a bit the game owns.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use eldenring::cs::{CSTaskGroupIndex, CSTaskImp, WorldChrMan};
use eldenring::fd4::FD4TaskData;
use fromsoftware_shared::{FromStatic, SharedTaskImpExt};

/// How long the init thread waits for the game's task system before giving up.
const INIT_TIMEOUT: Duration = Duration::from_secs(60);

/// Clear after the behavior update (which sets the flag) and before damage is applied:
/// `DmgMan_Pre` runs immediately after this phase.
const PHASE: CSTaskGroupIndex = CSTaskGroupIndex::WorldChrMan_PostPhysics;

static FRAMES: AtomicU64 = AtomicU64::new(0);
static CLEARS: AtomicU64 = AtomicU64::new(0);

/// Runs on a short-lived init thread spawned from `DllMain`: wait for the task system,
/// then register the per-frame task and return. Must not run on the main thread, since
/// [`CSTaskImp::wait_for_instance`] blocks on main-thread initialization.
pub fn install() {
    let cs_task = match CSTaskImp::wait_for_instance(INIT_TIMEOUT) {
        Ok(task) => task,
        Err(e) => {
            log::error!("CSTaskImp unavailable; patch not installed: {e:?}");
            return;
        }
    };

    // Dropping the handle cancels the task, so leak it: we want it for the process'
    // lifetime.
    let handle = cs_task.run_recurring(|_: &FD4TaskData| on_frame(), PHASE);
    std::mem::forget(handle);

    log::info!("patch installed: clearing crit-invuln in {PHASE:?} each frame");
}

/// Per-frame, on the main thread. Clears the throw-invuln flag on every open-field enemy.
fn on_frame() {
    // Heartbeat first, so it confirms the task fired regardless of world state
    // (first tick, then ~every 10s at 60fps).
    let f = FRAMES.fetch_add(1, Ordering::Relaxed);
    if f == 0 || f.is_multiple_of(600) {
        log::info!("frame task live (frame {f})");
    }

    let Ok(wcm) = (unsafe { WorldChrMan::instance() }) else {
        return;
    };

    for chr in wcm.open_field_chr_set.base.characters() {
        let flags = &mut chr.modules.action_flag.action_modifiers_flags;
        if flags.invincible_excluding_throw_attacks_defender() {
            flags.set_invincible_excluding_throw_attacks_defender(false);
            let n = CLEARS.fetch_add(1, Ordering::Relaxed) + 1;
            if n <= 5 || n.is_multiple_of(500) {
                log::info!("cleared crit-invuln on cid={} (total {n})", chr.character_id);
            }
        }
    }
}
