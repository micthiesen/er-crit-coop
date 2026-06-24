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

**Phase 1 — diagnostic (current).** Nobody has published *where* the throw-invuln state
lives on `ChrIns`, so this build only observes: it snapshots every open-field character's
active SpEffects and a raw `ChrIns` byte window, logging on change. Riposting a lone enemy
reveals the flag/speffect that toggles during the invuln window. See `docs/session.md`.

**Phase 2 — patch (next).** Clear the identified flag/speffect each frame for enemies (or
neutralize the TAE-67 setter), so the enemy stays damageable during the crit.

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
