# er-crit-coop

An Elden Ring DLL mod that lets **Seamless Co-op** partners deal damage to an enemy
while it's locked in a **critical-hit animation** (riposte / backstab / guard counter),
instead of the enemy being invulnerable to everyone but the player performing the crit.
This mirrors the behavior in Nightreign.

## Why a DLL instead of an animation pack

The popular [Critical Attack IFrame Remover](https://www.nexusmods.com/eldenring/mods/9624)
edits ~425 `chr/*.anibnd.dcx` files to strip the invulnerability TAE event. That approach:

- needs ModEngine/me3 to load (asset override), and
- is unreliable in Seamless Co-op (animation overrides are evaluated where the enemy is
  simulated — the host — and can't merge, so coverage is partial).

The invulnerability comes from **TAE Event Type 0, flag 67 ("Invincible excluding Throw
Attacks")**, which sets a runtime state on the victim's `ChrIns`. It is **not** expressible
in `regulation.bin` params (`ThrowParam` has no invuln field; `AtkParam` notes TAE
invincibility cannot be overridden by params). So the robust, simple-to-install option is a
single DLL that clears that runtime state, applied on every machine (host included).

- **Install:** one `.dll`, loaded by Elden Mod Loader (or ModEngine2/me3 alongside `ersc.dll`).
- **Coop:** patches the actual simulation state, not local animation playback.
- **Anti-cheat:** Seamless Co-op bypasses EAC (launches `eldenring.exe` directly), so a
  memory-patching DLL is fine in coop. Never take a modded session onto official servers.

## Status

**Functional (`MODE = Patch`), pending in-game verification.** The `fromsoftware-rs` SDK
exposes the exact flag by name — `CSChrActionFlagModule::action_modifiers_flags::`
`invincible_excluding_throw_attacks_defender` (TAE Event 0, action 67) — so no offset
hunting or calibration session was needed. `src/patch.rs` clears that one bit every 8ms on
every open-field enemy, through the SDK's typed setter (crash-safe, no raw offsets). The
riposte still lands; only the "everyone-else-can't-hit-me" bit is dropped.

What's left is to confirm it in-game (the log records `cleared crit-invuln` when the
mechanism fires) and in a coop test. `src/diagnostic.rs` (`MODE = Diagnostic`) remains for
investigating any enemy that uses a different invuln flag.

## Build

Cross-compiles to a Windows DLL from Linux:

```bash
rustup target add x86_64-pc-windows-gnu   # one-time; needs mingw-w64
cargo build --release --target x86_64-pc-windows-gnu
# -> target/x86_64-pc-windows-gnu/release/er_crit_coop.dll
./scripts/deploy.sh                        # copy into the game's Elden Mod Loader mods/ folder
```

Built on the [`fromsoftware-rs`](https://github.com/vswarte/fromsoftware-rs) SDK
(`eldenring` crate), pinned by commit in `Cargo.toml`.
