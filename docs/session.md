# Diagnostic session procedure (phase 1)

Goal: find the `ChrIns` flag/speffect that grants invulnerability during a critical hit,
by riposting a lone enemy while the DLL logs every open-field character's state on change.

## Setup
1. Build + deploy: `cargo build --release --target x86_64-pc-windows-gnu && ./scripts/deploy.sh`
   (copies `er_crit_coop.dll` into the game's Elden Mod Loader `mods/` folder).
2. Launch Elden Ring the normal way (Steam → Seamless Co-op). Solo is fine for discovery.
3. Confirm load: `er_crit_coop.log` should appear in `ELDEN RING/Game/` with an
   `er-crit-coop loaded` line. If it doesn't, Elden Mod Loader didn't load it alongside
   ERSC and we switch to loading via ModEngine2/me3.

## The test
1. Find a **single, isolated weak humanoid** enemy (e.g. a lone Godrick Soldier / wandering
   noble) so the log isn't crowded. Note the area is calm.
2. Get it staggered and perform a **critical hit (riposte)**. A backstab also works.
3. Do it 2–3 times, pausing a couple seconds between, then quit to desktop.

## Reading the result
- Each log line: `cid=<charId> type=<ChrType> ptr=<chrIns> b@0x1c0=<hexbytes> spfx=[..]`.
- Filter to the riposted enemy's `cid`/`ptr`. Diff consecutive lines around the riposte:
  - a **new SpEffect id** appearing only during the crit → that's the lever (erase it), or
  - a **byte/bit flipping** in the `b@0x1c0` window during the crit → that's the flag (clear it).
- That offset/id gets hardcoded (or AOB-anchored) into phase 2's patch.

If neither the byte window nor speffects move during the riposte, widen `DUMP_START`/`DUMP_LEN`
in `src/lib.rs` or add a dump of the `ChrInsModuleContainer` (the TGA cheat table located a
per-instance no-damage flag at `module + 0x10EF8`).
