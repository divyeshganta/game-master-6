<div align="center">

<img src="assets/banner.png" alt="Game Master 6 Banner" width="100%" />

# Game Master 6 — v1.6.2

**Native Windows launcher and NVIDIA RTX auto-tuner for Xbox Series compatibility runners.**

[![Platform](https://img.shields.io/badge/Platform-Windows%2010%2F11%20x64-00e5ff?style=for-the-badge&logo=windows)](https://www.microsoft.com/windows)
[![GPU](https://img.shields.io/badge/GPU-NVIDIA%20RTX-76b900?style=for-the-badge)](https://www.nvidia.com/)
[![Built with](https://img.shields.io/badge/Built%20with-Rust%20%2B%20Win32%20GDI-ff2e93?style=for-the-badge&logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-GPL--3.0-22c55e?style=for-the-badge)](LICENSE)
[![Release](https://img.shields.io/badge/Release-v1.6.2-f59e0b?style=for-the-badge)](https://github.com/<owner>/game-master-6/releases)

</div>

Game Master 6 is a small, native Rust + Win32 GDI Windows launcher for Xbox Series compatibility runners. It detects your GPU, applies a safe NVIDIA RTX preset, prepares the shader cache, and launches the game. Release builds are self-contained and **contain no game files** — bring your own game directory and runner. Licensed under **GPL-3.0**.

---

## Quick Start

1. **Download** the latest release ZIP from **Releases** and **extract** it to a clean folder.
2. **Run `GameMaster6.exe`** — no install and no extra dependencies.
3. Click **`AUTO (THIS PC)`**, then **`PREPARE CACHE`** once, then **`LAUNCH`**.

---

## `AUTO (THIS PC)` VRAM tiers

The launcher reads your dedicated VRAM and applies the matching baseline automatically:

| VRAM | Console | Res | DLSS | FG | Texture Room | Levels on Demand |
|---|---|---|---|---|---|---|
| 6 GB (<7.0) | Series-S | 1920x1080 | Performance | 2x | 60% | 1 |
| 8 GB (7.0-9.5) | Series-S | 1920x1080 | Performance | 2x | 68% | 1 |
| 10 GB (9.5-11.0) | Series-X | 2560x1440 | Performance | 2x | 80% | 0 |
| 12 GB+ (>=11.0) | Series-X | 2560x1440 | Balanced | 2x | 85% | 0 |

RTX 20 / 30 series cards use the `VICTORY_MENU_FG_TYPE=auto` Frame Generation fallback. RTX 50 series cards support selectable **3x Frame Generation**.

---

## The five tabs

- **PLAY** — hardware summary, `AUTO (THIS PC)` and preset selection, cache preparation, and the launch control.
- **RUNNERS** — pick, add, build from source, or update a `victory.exe` runner and its profile.
- **SAVES** — one-click save snapshots, restore, and quick benchmarks.
- **DIAG** — game-file checks, live runner logs, and a shareable debug bundle.
- **SETTINGS** — display, visuals, HUD, and advanced runner environment options.

### Save Manager

`EXPORT SAVES (.ZIP)` writes a timestamped snapshot to `backups\saves\` and never modifies your originals. Restore any snapshot in one click, and saves are backed up automatically before a runner update.

### 1-Click Updates

A non-blocking background check watches for launcher and runner updates, pauses while the game is running, and applies updates in one click — with saves backed up first.

---

## Build from source

```powershell
git clone https://github.com/<owner>/game-master-6.git
cd game-master-6
cargo build --release
```

The binary is written to `target\release\GameMaster6.exe`.

---

## License

GPL-3.0 — see [LICENSE](LICENSE). This project contains no game files and does not bundle any compatibility runner.

<details>
<summary>Previous releases (v1.0-v1.5)</summary>

- **v1.5** — one-click setup for runners, NVIDIA DLLs, and shader cache; process cleanup and diagnostics polish.
- **v1.4** — multi-runner manager, VRAM-tier presets, live telemetry, and save snapshots.
- **v1.0–v1.3** — initial releases: GDI launcher, runner profiles, and basic GPU-tier tuning.

</details>
