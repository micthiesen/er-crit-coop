# Development notes

How this mod is built, run, and verified — and the Linux-specific tricks that took a while
to work out. Useful for this repo and for any FromSoftware DLL-mod work on Linux.

## Toolchain: cross-compiling a Windows DLL from Linux

No Windows host needed. The mod is a `cdylib` built for `x86_64-pc-windows-gnu`:

- `rust-toolchain.toml` pins `channel = "stable"` and `targets = ["x86_64-pc-windows-gnu"]`,
  so `rustup` installs the target automatically.
- The GNU target links with **mingw-w64** (`pacman -S mingw-w64-gcc`); cargo finds
  `x86_64-w64-mingw32-gcc` on PATH automatically.
- `cargo build --release --target x86_64-pc-windows-gnu` → `target/.../er_crit_coop.dll`.

Builds are not bit-reproducible across hosts (CI's mingw vs local mingw differ), so the
release `.dll` won't sha-match a local build. They're equivalent; compare the `.text` size
(`x86_64-w64-mingw32-objdump -h`) if you want to sanity-check — a few hundred bytes of
linker-stub difference is normal.

## The SDK and the patch design

Built on [`fromsoftware-rs`](https://github.com/vswarte/fromsoftware-rs) (`eldenring` crate),
pinned by commit in `Cargo.toml`. Key pieces used:

- `WorldChrMan::instance()` → `open_field_chr_set.base.characters()` to iterate map enemies
  as `&mut ChrIns`.
- `chr.modules.action_flag.action_modifiers_flags` (`CSChrActionFlagModule`) — the TAE
  "action flag" bitfield. Bit for action 67 is `invincible_excluding_throw_attacks_defender`.
  The SDK names every TAE Event-0 action flag, so no offset hunting was needed.
- **Task system** (`src/patch.rs`): instead of a background thread, register a per-frame
  task on the game's own scheduler:
  ```rust
  let cs_task = CSTaskImp::wait_for_instance(timeout)?;        // off the main thread
  let handle = cs_task.run_recurring(|_: &FD4TaskData| on_frame(), CSTaskGroupIndex::WorldChrMan_PostPhysics);
  std::mem::forget(handle);                                    // registration is permanent
  ```
  `DllMain` only spawns the short init thread (avoids loader-lock issues; `wait_for_instance`
  must not run on the main thread). **Phase choice matters**: clear the flag in a phase that
  runs *after* the behavior update sets it and *before* `DmgMan` reads it
  (`WorldChrMan_PostPhysics`). The safety is frame-ordering, not thread exclusivity. See the
  module doc in `src/patch.rs`.

## Loading the mod

Single `.dll` dropped in `ELDEN RING/Game/mods/`, loaded by **Elden Mod Loader**
(`DINPUT8.dll`). It coexists with Seamless Co-op in practice (the ERSC docs claim DLL
injectors are unsupported, but it works via the exe-swap launch). No ModEngine/me3.

`scripts/deploy.sh` copies the built DLL into that folder.

## Run + verify loop on Linux (no Windows, no manual clicking)

You can drive the whole launch/observe/kill cycle from the shell:

```bash
# launch (uses the Steam launch options + ersc exe-swap, so the mod loads)
steam -applaunch 1245620

# the DLL logs to ELDEN RING/Game/er_crit_coop.log; watch it
#   "patch installed ..."  -> task registered (wait_for_instance + run_recurring OK)
#   "frame task live (...)" -> the task is actually executing each frame
#   "cleared crit-invuln ..." -> it cleared the flag on a real enemy (needs gameplay)

# kill
pkill -f '[e]ldenring.exe'
```

Gotchas learned the hard way:

- **`pgrep`/`pkill` match their own command line.** `pgrep -f eldenring.exe` reports a false
  positive by matching the very command running it. Use the bracket trick: `'[e]ldenring.exe'`.
- **The log is truncated when the DLL loads** (`File::create`). When re-launching to check a
  new build, `rm` the log first so a `grep 'patch installed'` match means a *fresh* load, not
  stale content from the previous run.
- **`WorldChrMan_PostPhysics` doesn't tick at the title screen** (no world). To prove the task
  *fires* without loading a save, temporarily set the phase to `CSTaskGroupIndex::FrameBegin`
  (ticks every frame, including menus) and watch the heartbeat, then switch back.
- **What you can verify solo vs not:** registration, per-frame firing, and stability are all
  observable from the title screen via the log. The actual "partner damages an enemy during a
  crit" effect needs a loaded save / co-op session and a real backstab, which can't be driven
  from the CLI.
- This rig has a pre-existing tendency to instant-close (ERSC/Proton), unrelated to the mod;
  crash dumps land in `Game/SeamlessCoop/crashdumps/reports/`. Check dump timestamps against
  the DLL-load time before blaming a code change.

## Releasing

Push a `vX.Y.Z` tag (use the `/release` skill, which bumps `Cargo.toml`, writes notes into
the annotated tag, and pushes). `.github/workflows/release.yml` then cross-compiles the DLL
and publishes a GitHub release with the binary, using the tag message as the notes.
