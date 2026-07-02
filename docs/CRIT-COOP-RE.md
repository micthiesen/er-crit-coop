# Crit-coop RE: why clearing the flag doesn't work, and where the real gate is

Working notes for finding the actual mechanism that blocks a third party from damaging a
throw victim (riposte/backstab) in Elden Ring, so this personal mod can neutralize it.
Addresses are for `eldenring.exe` **2.6.2.0** (WW), preferred base `0x140000000`. Personal
use only; this is our own behavioral analysis of a legitimately-owned binary.

## Settled: the action-modifier bit is NOT the gate

`CSChrActionFlagModule::action_modifiers_flags` bit 3
(`invincible_excluding_throw_attacks_defender`, TAE 67) is a dead end. The instrumented
DLL (`src/patch.rs`, v2/v3) proved it live:

- v2 fixed a real bug — the loop only iterated `open_field_chr_set`, missing legacy-dungeon
  / boss-arena enemies (they live in other `WorldChrMan::chr_sets` containers). Now walks
  all 196 sets (player set skipped).
- With full coverage, backstabs logged: bit3 cleared **every frame**, `re-set by dmg pass
  0f`, and `is_invincible(1c5)/perfect_inv/immortality` all `0f` — yet summons visibly
  passed through the victim. (`CRIT window end ...` lines.)
- v3 also clears at `ChrIns_BehaviorSafe` (right after the TAE setter) in case the state is
  latched early: `set@behavior Nf, still set@postphysics 0f`, still whiffed.

Conclusion: whatever rejects third-party damage on a throw victim does not consult that
bit. The one constant across every crit window is `CSChrThrowNode::throw_state ==
InThrowTarget (4)`, so the gate is keyed on the throw pairing, in the damage path.

## Tooling

- **Frida host-attach does NOT work** on this Wine-hosted PE (bootstrapper SIGSEGVs — see
  `scripts/re/frida-probe.py`). The workable Frida path is **frida-gadget** injected into
  the game (a Windows DLL in `mods/` exposing a localhost server); see unseamless-coop
  `docs/RUNTIME-RE.md`. Not yet set up here.
- **Static:** `scripts/re/static.py` (borrowed from unseamless-coop via sys.path) —
  capstone/numpy PE reader: `find_ascii/find_utf16/find_riprefs/find_calls_to/func_bounds/
  vtables_for_rtti/vtable_slots/disasm`. Instant; no Ghidra needed for the hunt.
- **Ghidra headless:** unseamless-coop `scripts/re/ghidra-decompile.sh <bin> <addr>` for
  readable C. First analysis of the 87MB exe is slow (~20-40 min) but the project caches
  under `/tmp/unseamless-ghidra-projects/`, so later decompiles are fast.
- Hunt scripts that produced the map below: `scripts/re/hunt*.py`. Disasm artifacts:
  `scripts/re/artifacts/`.

## Address map (v2.6.2.0)

RTTI type descriptor -> vtable:

| Class | RTTI name | vtable VA |
|---|---|---|
| `CS::CSChrThrowModule`   | `.?AVCSChrThrowModule@CS@@`   | `0x142a3b488` |
| `CS::CSChrDamageModule`  | `.?AVCSChrDamageModule@CS@@`  | `0x142a36a60` |
| `CS::CSEnemyDamageModule`| `.?AVCSEnemyDamageModule@CS@@`| `0x142a370a8` |
| `CS::CSPlayerDamageModule`| `.?AVCSPlayerDamageModule@CS@@`| `0x142a373b0` |

Source dir (from a retained assert string at `0x142a37208`):
`..\..\Source\Game\Chr\ChrModule\Damage\CSEnemyDamageModule.cpp`.

Enemy damage functions (`.pdata`, region `0x14044a000..0x14044d000`):

- **`0x14044a910` (size 0x65b) — enemy damage-apply / accept-hit.** Returns `bool`:
  `al=1` at `0x14044af37` = **hit accepted** (sets `word[this+0x25e]=0x101`, calls
  vtable+0x38, `or [.. +0x214], 0x10`); `al=0` at `0x14044af3b` = **hit rejected**. This
  is the top of the accept/reject decision for enemies (matches "bosses AND enemies").
- Reject gates jumping to `0x14044af3b` worth checking against a live crit:
  - `0x14044a9bf: cmp dword [this+0x54], 3; je reject` — a damage-module state == 3.
  - `0x14044aa79: call 0x14044b060; test al,al; jne reject` — a predicate (below).
  - plus `0x14044ad62 / 0x14044adb4 / 0x14044ae52 / 0x14044ae67`.
- `0x14044b9d0` (size 0x6b) — small setter: validates a FieldIns handle (`0xffffffff`
  sentinel) and stores it at `[this+0xc0]` (candidate: attacker / throw-partner handle).
- `0x14044b060` (size 0x268) — predicate called before a reject. Reads global manager
  `0x143d76060` -> `+0x98`, iterates a collection (`0x140c73d10/0x140c75ac0/0x140c67060`)
  testing bit 5 of `byte[obj+0x3b]`. Reads more like an area/hitlist query than the throw
  gate; **needs the decompiler to type it** before trusting either way.

## Next steps (resume here)

1. **Decompile in C** (Ghidra cache is now warm): `0x14044a910` (apply) and its reject
   predicates. Identify which gate reads the throw module / a throw-partner handle vs. the
   attacker. The throw module ptr sits at container `+0x88` (`data 0x0, action_flag 0x8, …,
   action_request 0x80, throw 0x88`); `throw_state` is inside `CSThrowNode`.
2. **Runtime-confirm which gate fires during a real crit.** Either (a) a hardware
   *read/access* watchpoint (extend unseamless-coop `scripts/re/watch-write.py` from
   DR7 write to read-write) on the victim's throw-partner handle / throw_state, filtered to
   the damage pass; or (b) frida-gadget hooking `0x14044a910` to log the reject path taken.
3. **Patch** the confirmed gate. Likely an inline byte-patch (force the reject branch not
   taken, or make the partner-handle compare always succeed) rather than a field write —
   use the SDK/injected code-patch path. Must not break legitimate throw behavior (the
   riposte itself, death transitions).

## Deployed state

`Game/mods/er_crit_coop.dll` is the **instrumented v3** (uncommitted `src/patch.rs`). It
still functions as the original mod (the flag-clear is a harmless no-op for this purpose)
plus logs crit windows. Before shipping any real fix, strip the instrumentation back to a
clean patch.
