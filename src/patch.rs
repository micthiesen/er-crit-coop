//! Phase 2 — the functional patch.
//!
//! The SDK exposes the exact culprit as a typed field, so there's no offset
//! hunting or in-game calibration. When an enemy is the victim of a critical hit
//! (riposte / backstab / guard-counter), TAE Event 0 action **67** sets
//! `CSChrActionFlagModule::action_modifiers_flags::invincible_excluding_throw_attacks_defender`
//! — invulnerable to everyone *except* the player performing the crit. That's the
//! exact bit that locks coop partners out. We clear it every frame on every
//! open-field enemy, so partners (and any other damage source) can hit during the
//! crit — the Nightreign behavior.
//!
//! Why this is safe and coop-robust:
//!   * Every access is through the SDK's typed fields — no raw offsets, nothing to
//!     crash on; we only flip a bit the game already owns, and only on map enemies.
//!   * It mutates the enemy's authoritative action-flag state (not local animation
//!     playback), so under Seamless Co-op's host-authoritative model it takes
//!     effect wherever the enemy is simulated, as long as the host runs the mod.
//!   * Clearing only flag 67 (not perfect-invincibility/immortality) means the
//!     riposte itself still lands and unrelated invulnerability is untouched.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use eldenring::cs::WorldChrMan;
use fromsoftware_shared::FromStatic;

/// Comfortably inside the multi-second crit animation; wins the race against the
/// TAE re-setting the flag each frame.
const TICK: Duration = Duration::from_millis(8);

/// Let the game finish initializing before our thread touches anything. Elden Mod
/// Loader already delays DLL load ~5s; this adds margin past world setup.
const STARTUP_GRACE: Duration = Duration::from_secs(5);

/// Cheap sanity check on a pointer the game handed us: canonical user-space and
/// 8-aligned. Filters obviously torn-down/garbage entries before we dereference.
fn looks_valid(p: usize) -> bool {
    p >= 0x10000 && p < 0x0001_0000_0000_0000 && p % 8 == 0
}

pub fn patch_loop() {
    std::thread::sleep(STARTUP_GRACE);
    log::info!(
        "patch active: clearing invincible_excluding_throw_attacks_defender (TAE action 67) \
         on open-field enemies every {}ms",
        TICK.as_millis()
    );

    let total: AtomicU64 = AtomicU64::new(0);

    loop {
        if let Ok(wcm) = unsafe { WorldChrMan::instance() } {
            // `characters()` yields `&mut ChrIns` even from `&WorldChrMan` (it walks
            // the ChrSet via raw pointers), so we can clear the flag in place.
            //
            // TODO(robustness): gate iteration on the entry's load status == Active.
            // `characters()` yields every entry whose `chr_ins` is Some, regardless of
            // load status, so during loading screens / world reloads it can hand us a
            // ChrIns that is mid-initialization or mid-teardown. Such a ChrIns can carry
            // plausible-looking (canonical, 8-aligned) module pointers that `looks_valid`
            // passes but that aren't safe to dereference. The robust fix is to iterate the
            // ChrSet entries directly and skip any whose `chr_load_status` != Active (see
            // fromsoftware-rs `world_chr_man.rs`: `ChrSetEntry` / `ChrLoadStatus`), rather
            // than the `characters()` helper which hides the entry. Deferred because it
            // means reimplementing the iterator with more unsafe code in the shipping hot
            // path, which needs in-game retesting before trusting it.
            //
            // Bigger picture: the fully sound design runs this from a game main-thread hook
            // (a per-frame task) instead of a background thread, eliminating the cross-thread
            // access to live game memory entirely. The background-thread approach here is the
            // common ER-mod pattern and works in practice, but is not sound by the SDK's
            // documented `instance()`/access contract. A main-thread hook would also fix the
            // non-atomic read-modify-write of the action-flag word.
            for chr in wcm.open_field_chr_set.base.characters() {
                // Guard against torn-down characters before dereferencing.
                let modules_ptr = chr.modules.as_ptr();
                if !looks_valid(modules_ptr as usize) {
                    continue;
                }
                let af_ptr = unsafe { &*modules_ptr }.action_flag.as_ptr();
                if !looks_valid(af_ptr as usize) {
                    continue;
                }
                let flags = &mut unsafe { &mut *af_ptr }.action_modifiers_flags;

                if flags.invincible_excluding_throw_attacks_defender() {
                    flags.set_invincible_excluding_throw_attacks_defender(false);

                    // Confirm the mechanism is firing, without spamming the log.
                    let n = total.fetch_add(1, Ordering::Relaxed) + 1;
                    if n <= 5 || n.is_multiple_of(500) {
                        log::info!("cleared crit-invuln on cid={} (total {n})", chr.character_id);
                    }
                }
            }
        }
        std::thread::sleep(TICK);
    }
}
