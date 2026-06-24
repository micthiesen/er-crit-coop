//! The functional patch, run as a recurring game-frame task.
//!
//! Clears the riposte-victim invulnerability (`CSChrActionFlagModule`
//! `action_modifiers_flags::invincible_excluding_throw_attacks_defender`, TAE action 67)
//! on every open-field enemy each frame, so co-op partners can damage an enemy during a
//! crit (riposte/backstab/guard counter) instead of it being immune to everyone but the
//! player landing the crit.
//!
//! It registers a recurring task in the `WorldChrMan_PostPhysics` phase of the game's frame
//! pipeline. The safety here is **frame-ordering**, not thread exclusivity: the phase runs
//! after the character behavior update has (re)set the flag for the frame and before
//! `DmgMan` reads it later in the same frame, so clearing it there makes the enemy
//! damageable for that frame's damage pass. Running inside the game's own scheduled phase
//! (rather than a free-running background thread, as an earlier version did) means we touch
//! `ChrIns` in step with the frame instead of racing the behavior/damage phases that own
//! those writes. Access goes through the SDK's typed field, no raw pointers.
//!
//! Caveat: the flag shares a `u64` word with other action-modifier bits, and the SDK setter
//! is a read-modify-write of that word. That's fine as long as nothing else writes the word
//! during this phase (the behavior phase that sets these bits has already run for the frame).

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use eldenring::cs::{CSTaskGroupIndex, CSTaskImp, WorldChrMan};
use eldenring::fd4::FD4TaskData;
use fromsoftware_shared::{FromStatic, SharedTaskImpExt};

/// How long the init thread waits for the game's task system before giving up.
const INIT_TIMEOUT: Duration = Duration::from_secs(60);

/// Run after the behavior update (which sets the flag) and before damage is read.
/// `DmgMan_Pre` runs later in the same frame.
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

    // The task is registered into the game's task pool for the rest of the process'
    // lifetime; the SDK never unregisters it (its `cancel()` is a no-op stub and the task
    // keeps an internal self-reference). Forget the handle so its `Drop` can't flip the
    // cancel flag, and so this never gets "tidied up" into a dangling task. Do not replace
    // this with a stored/dropped handle.
    let handle = cs_task.run_recurring(|_: &FD4TaskData| on_frame(), PHASE);
    std::mem::forget(handle);

    log::info!("patch installed: clearing crit-invuln in {PHASE:?} each frame");
}

/// Per-frame, in the `PostPhysics` phase. Clears the crit-invuln flag on every open-field
/// enemy.
///
/// TODO(robustness): `characters()` yields every entry whose `chr_ins` is `Some` regardless
/// of `ChrSetEntry::chr_load_status`, so across a loading/fast-travel transition this could
/// touch a mid-init/teardown `ChrIns` whose module pointers aren't wired up. Running only in
/// `PostPhysics` (vs the old every-8ms background thread) makes that window small, but the
/// fully robust version would iterate the ChrSet entries directly and skip any whose status
/// isn't `Active` before dereferencing `modules`. Left out for now because it needs an
/// in-game retest to confirm it doesn't gate out live enemies.
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
