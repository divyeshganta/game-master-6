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

### Prerequisites
- **Windows 10 / 11 (x86_64)**
- **Rust toolchain** via [rustup](https://rustup.rs/) (`stable-x86_64-pc-windows-msvc`)
- **Visual Studio Build Tools** (or Visual Studio) with the *Desktop development with C++* workload (Windows SDK + MSVC linker)

### Build & Test
```powershell
git clone https://github.com/<your-username>/Game-Master-6.git
cd Game-Master-6
cargo test
cargo build --release
```

The compiled binary is output to `target\release\GameMaster6.exe`.

## Project Structure

```text
src/
├── main.rs          # Application entry point
├── ui.rs            # Win32 window loop, event/input handling, background worker polling
├── render.rs        # Custom Win32 GDI renderer for all tabs, cards, sparklines, and controls
├── win32.rs         # Raw Win32 FFI bindings (GDI, WinINet, Registry, XInput, Power/Process APIs)
├── config.rs        # Persistent settings loader/saver and option cyclers
├── presets.rs       # Hardware-adaptive VRAM-tier presets (8 GB, 12 GB, 16 GB, 24 GB+)
├── preflight.rs     # Live 0–100 hardware & system readiness scorer
├── launcher.rs      # Runner environment builder, process launcher, and shader census runner
├── runners.rs       # Local runner discovery, rollback, and cache directory management
├── builder.rs       # Automated runner build helper script generator
├── diagnostics.rs   # System probes, RAM/commit/process tips, log parsing, and debug bundles
├── telemetry.rs     # Log-synced session history, unified FPS parser, and A/B scorecard
├── ab_bench.rs      # A/B experiment flag definitions
├── guardian.rs      # Crash-recovery verdict engine and Last-Known-Good settings snapshot
├── hud.rs           # Click-through always-on-top performance HUD and center reticle overlay
├── keymap.rs        # Keyboard/mouse stand-in keymap presets and conflict checker
├── monitor.rs       # Live process RAM/commit usage sampler
├── portwatch.rs     # Community runner tracker client (WinINet JSON/HTML + offline fallback)
├── profiles.rs      # Portable profile import/export
├── saves.rs         # Save backup/restore and mod toggle helpers
├── shader_heal.rs   # Shader skip-list / healing helpers
└── vram_guard.rs    # Background VRAM-heavy process detector
```
