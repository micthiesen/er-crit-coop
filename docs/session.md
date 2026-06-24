# Diagnostic mode

The mod ships in `Patch` mode (the actual fix). `Diagnostic` mode is a fallback for
investigating an enemy that turns out to use a different invulnerability flag than
TAE action 67 — it patches nothing, just logs each open-field enemy's state on change.

## Enable it
In `src/lib.rs`, set `const MODE: Mode = Mode::Diagnostic;`, then build + deploy:
```bash
cargo build --release --target x86_64-pc-windows-gnu && ./scripts/deploy.sh
```

## Use it
1. Launch the game; `er_crit_coop.log` is written to `ELDEN RING/Game/` (cwd is logged on
   startup in case Proton differs).
2. Find a lone enemy and riposte/backstab it a few times, then quit.
3. Read the log. It reports, per enemy:
   - `RISE region=<...> off=<...> bit=<n>` — a bit that turned on (suppressing per-frame
     churn), across the `ChrIns` flag window and the `data`/`action_flag`/`throw`/
     `super_armor` module heads, and
   - `SPFX+ added=[...]` — SpEffects gained.
   A rising bit (or new speffect) that lines up with the riposte is the invuln state; map
   it to a typed field in the SDK and clear it in `patch.rs` the same way action 67 is.

Reads are confined to memory the SDK guarantees valid (the `ChrIns` window and module heads
via typed pointers), so it won't fault on a deep raw offset.
