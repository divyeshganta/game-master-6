# Game Master 6

A lightweight, zero-dependency Windows Win32/GDI GUI frontend, hardware-adaptive preset manager, and live RAM/commit monitor for `victory.exe` compatibility runners.

> **Disclaimer:** Game Master 6 ships **without** any runners, binaries, or game files. Users must supply their own title files and compatibility runner executable.

## Features
- **Hardware-Adaptive Presets (`LOW` / `BALANCED` / `HIGH`)**: Automatically detects GPU VRAM tier (8 GB, 12 GB, 16 GB, 24 GB+) and CPU core count on first launch to configure safe `VICTORY_*` environment variables.
- **Live Hardware & Pre-Flight Breakdown**: Real-time progress bars for GPU VRAM ceiling (`VICTORY_TEXTURE_ROOM_PERCENT`), System RAM, and Windows Commit/Pagefile, plus an itemized Pre-Flight readiness score.
- **Dynamic Process-Aware System Tip**: Inspects live free RAM/commit, top memory-consuming user apps, multi-monitor VRAM overhead, AC/battery power state, and shader cache status.
- **5-Section Engineering Panel**: Organized toggles for System & Host Scheduling, VRAM/Textures/Buffers, GPU Pipeline & Parallelism, Display/Visuals/HUD, and Diagnostics & Debug.
- **Input, Keybinds & Session Telemetry**: Automatic FPS curve, runner head-to-head scorecard, session benchmark leaderboard, and shader translator census.

## Using the Release ZIP
1. Extract `GameMaster6-v1.0-Release.zip` to a clean folder (e.g., `C:\Tools\GameMaster6\`).
2. Run `GameMaster6.exe`.
3. On the **PLAY** tab, select your title folder/executable and import or select your `victory.exe` runner on the **RUNNERS** tab.
4. Choose a preset (`LOW`, `BALANCED`, or `HIGH`) and `SERIES-S` / `SERIES-X` console mode, run **PREPARE CACHE** if supported, and click **LAUNCH TITLE**.

## Building from Source
Requires Rust (`cargo`) on Windows x86_64:

```powershell
cargo build --release
cargo test
```

The compiled binary will be output to `target\release\GameMaster6.exe`.
