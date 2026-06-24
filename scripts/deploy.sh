#!/usr/bin/env bash
# Build (if needed) and copy the DLL into the game's Elden Mod Loader mods/ folder.
set -euo pipefail

GAME="/mnt/games/SteamLibrary/steamapps/common/ELDEN RING/Game"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DLL="$ROOT/target/x86_64-pc-windows-gnu/release/er_crit_coop.dll"

if [[ ! -f "$DLL" ]]; then
  echo "DLL not built; run: cargo build --release --target x86_64-pc-windows-gnu" >&2
  exit 1
fi

cp -v "$DLL" "$GAME/mods/er_crit_coop.dll"
echo
echo "Deployed to Elden Mod Loader. After launching, the diagnostic log appears at:"
echo "  $GAME/er_crit_coop.log"
echo "(If it's not there, the game's cwd differs — search the Proton prefix drive_c.)"
