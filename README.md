# er-crit-coop

An Elden Ring DLL mod that lets **Seamless Co-op** partners deal damage to an enemy
while it's locked in a **critical-hit animation** (riposte / backstab / guard counter).

In vanilla, when one player lands a riposte the enemy becomes invulnerable to everyone
*except* the player performing the crit, so your partners' hits whiff for the whole
animation. This mod removes that lockout, so anyone can keep damaging the enemy through a
crit — the way Nightreign handles it.

## Install

1. Grab `er_crit_coop.dll` from the [latest release](../../releases/latest).
2. Drop it in your Elden Ring `Game/mods/` folder (the
   [Elden Mod Loader](https://github.com/techiew/EldenRingModLoader) folder — the same one
   you already use for DLL mods alongside Seamless Co-op).
3. Launch as usual. That's it — no ModEngine or me3 required.

**For co-op:** the enemy is simulated on the **host**, so the host must have the mod for
its damage to register; simplest is for **everyone in the session to install it**. It
doesn't touch `regulation.bin`, so it won't block anyone from connecting.

> Seamless Co-op runs outside EAC, so this is safe to use in co-op. Don't take a modded
> session onto the official servers.

## How it works

A riposte/backstab is a *throw*: TAE Event 0, action **67**
(`INVINCIBLE_EXCLUDING_THROW_ATTACKS_DEFENDER`) sets a flag on the victim's `ChrIns` that
blocks all damage except the throwing player's. This mod clears that one flag every frame
on every open-field enemy, via the SDK's typed field — so the riposte itself still lands,
but everyone else can hit the enemy too. Nothing else is touched.

This is why it's a DLL rather than an animation pack: the flag is runtime combat state, not
something `regulation.bin` params can express, and clearing it in memory takes effect where
the enemy is actually simulated (the host) — which is what makes it hold up in co-op.

An existing approach, the *Critical Attack IFrame Remover* mod, instead edits ~425
`chr/*.anibnd.dcx` animation files to strip the invulnerability event. That needs
ModEngine/me3 to load and is unreliable in co-op (animation overrides can't merge and are
evaluated per-client), which is what this mod is meant to avoid.

## Build from source

Cross-compiles to a Windows DLL from Linux (no Windows host needed):

```bash
# needs mingw-w64 (Arch: pacman -S mingw-w64-gcc). The Rust target is pinned in
# rust-toolchain.toml and installed automatically.
cargo build --release --target x86_64-pc-windows-gnu
# -> target/x86_64-pc-windows-gnu/release/er_crit_coop.dll
./scripts/deploy.sh   # copies it into the local game's mods/ folder
```

Built on the [`fromsoftware-rs`](https://github.com/vswarte/fromsoftware-rs) SDK
(`eldenring` crate), pinned by commit in `Cargo.toml`.

### Diagnostic mode

`src/lib.rs` has a `MODE` switch. `Patch` (default) is the mod; `Diagnostic` instead logs
each enemy's flag/SpEffect changes to `er_crit_coop.log` — useful if some enemy turns out
to use a different invulnerability flag than action 67.

## Releases

Pushing a `vX.Y.Z` tag triggers CI (`.github/workflows/release.yml`), which cross-compiles
the DLL and publishes a GitHub release with the binary attached, using the tag's annotated
message as the release notes. Use the `/release` skill to mint one.

## License

MIT OR Apache-2.0.
