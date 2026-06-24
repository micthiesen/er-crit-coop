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

pub fn patch_loop() {
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
            for chr in wcm.open_field_chr_set.base.characters() {
                let action_flag = &mut *(&mut *chr.modules).action_flag;
                let flags = &mut action_flag.action_modifiers_flags;

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
