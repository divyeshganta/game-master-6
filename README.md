<div align="center">

<img src="assets/banner.png" alt="Game Master 6 Banner" width="100%" />

# Game Master 6 (v1.4.0)

**A lightweight, zero-dependency Windows GUI launcher, hardware-adaptive VRAM preset manager, and live system telemetry frontend for `victory.exe` compatibility runners.**

[![Platform](https://img.shields.io/badge/Platform-Windows%2010%2F11%20x64-00e5ff?style=for-the-badge&logo=windows)](https://github.com)
[![Built with](https://img.shields.io/badge/Built%20with-Rust%20%2B%20Win32%20GDI-ff2e93?style=for-the-badge&logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-GPL--3.0-22c55e?style=for-the-badge)](LICENSE)
[![Latest Release](https://img.shields.io/badge/Release-v1.4.0-f59e0b?style=for-the-badge)](GameMaster6-v1.4-Release.zip)

</div>

> **Disclaimer:** Game Master 6 is a pure frontend and configuration manager. It ships **without** any compatibility runners, third-party DLLs, or game files—users must supply their own title directory and `victory.exe` runner.

---

## What's New in v1.4.0

- **Focused Primary-Runner Play UI**: Play stays laser-focused on your #1 active runner (showing `DLSS + FG READY` and quick-cycle `DLSS` & `FG` pills when Streamline/DLSS DLLs are present), while secondary runner management (`BUILD FROM URL`, `UPDATE`, `ROLLBACK`, `IMPORT`) lives cleanly in the `Runners` tab.
- **True Two-Way Sync with `victory-settings.txt`**: Reads `victory-settings.txt` on startup, runner switch, and game exit so the Play UI always reflects what will launch—and clicking `LOW` / `BALANCED` / `HIGH` / `DISPLAY` / `SERIES-S/X` / `DLSS` / `FG` immediately writes those keys back while preserving unrelated settings.
- **Live Hover Hint Bar & `?` Shortcuts Guide**: Hovering any button, pill, or preset shows a plain-English explanation at the bottom of the window, and the `?` button in the header opens a quick-reference overlay (`F1`, `F3`, `F9`, `Alt+Enter`, `Win+P`, and keyboard-to-pad controls).

- **Shared Save Vault + Isolated Shader Caches**: All runners share one persistent save directory (`VICTORY_DATA_DIR`, defaulting to `%LOCALAPPDATA%\victory\data`) with rolling 5-slot ZIP backups before launch, while shader/pipeline caches stay strictly isolated per runner (`caches\<runner_id>\`) with SHA-256 `.runner_hash` tracking.
- **1-Click Runner `UPDATE` + `ROLLBACK`**: Source-built runners store `runner-meta.json` (git URL, branch, commit) and expose 1-click `UPDATE` (backs up `.exe`, `git pull`, `cargo build --release`, updates `.runner_hash`, runs `shaders` + `warm`) and `ROLLBACK`.
- **Zero-Code-Edit Runner Auto-Detection & DLSS/FG Wiring**: Auto-probes `<runner> --help` for `shaders`, `warm`, `translate`, and `upscaler-check`; automatically chains `shaders --jobs <N>` + `warm` in `PREPARE CACHE`; auto-detects `sl.interposer.dll` / `nvngx_dlss.dll` beside `victory.exe` (`VICTORY_STREAMLINE_DIR`), displays `DLSS / FG READY (F1 MENU · F3 STATS)`, and preserves `F1`-saved `victory-settings.txt`.
- **Opt-In Official NVIDIA SDK Fetcher**: When `BUILD FROM URL...` clones a repo containing `scripts/get-nvidia-sdks.ps1`, an explicit `GET DLSS` button lets you fetch signed NVIDIA Streamline/DLSS DLLs with 1 click.
- **5-Tier All-GPU Presets (Including 6 GB RTX 4050)**: Added `Tight6` (`<= 6.5 GB` VRAM) alongside `Tight8`, `Mid12`, `High16`, and `Ultra20`.
- **Optional Local Skin Overlay (`theme.json`)**: Drop a gitignored `theme.json` + custom `logo.png`/`logo.ico` next to `GameMaster6.exe` to customize accent/background colors locally.

---

## Hardware-Adaptive VRAM Tiers

Game Master 6 probes your GPU's dedicated VRAM on first launch and applies a safe baseline (`VICTORY_RELAY=2`, `VICTORY_CARD_STEPS=2`, `VICTORY_HOST_WAVE=32` on NVIDIA, `VICTORY_LOG=warn`, Ray Tracing OFF):

| VRAM Tier | Example GPUs | Console / Output | `TEXTURE_ROOM_PERCENT` | `CARD_ROOM_FOR_TARGETS_MB` | `CARD_BUFFERS_MB` / `MIN_MB` | Notes |
|---|---|---|---|---|---|---|
| **6 GB (`Tight6`)** | RTX 4050 6 GB, RTX 3050, RTX 2060 | `series-s` / `1280x720` | `78%` | `1024` | `384` / `4` | Aggressive idle trim (`3s` / `64 MB`) keeping ~400 MB VRAM headroom for 2x DLSS FG or Lossless Scaling (`LSFG`) |
| **8 GB (`Tight8`)** | RTX 4060, RTX 3060 Ti / 3070 | `series-s` / `1280x720` | `85%` | `1536` | `512` / `4` | `VICTORY_THIN=0` on `LOW` |
| **12 GB (`Mid12`)** | RTX 5070 Ti 12 GB, RTX 4070, RTX 3060 12 GB | `series-x` or `series-s` / `1920x1080` | `80%` (with DLSS FG) / `90–94%` | `2048–2560` | `1024` / `4` | 1-click `SERIES-S` / `SERIES-X` toggle |
| **16 GB (`High16`)** | RTX 4080, RTX 5080, RX 7800 XT | `series-x` / `1920x1080–2560x1440` | `94%` | `2560` | `1024–2048` / `4` | Full Series X fidelity |
| **20 GB+ (`Ultra20`)** | RTX 4090, RTX 5090, RX 7900 XTX | `series-x` / `auto` | `94%` | `2560–3072` | `1024–4096` / `4` | Uncapped headroom |

---

## Quick Start (Release ZIP)

1. Download **`GameMaster6-v1.4-Release.zip`** from **Releases** and extract it to a clean folder (e.g., `C:\Tools\GameMaster6\`).
2. Run **`GameMaster6.exe`** — it auto-detects your VRAM tier (`6 GB` / `8 GB` / `12 GB` / `16 GB+`).
3. On **PLAY**, click **BROWSE GAME FOLDER...** to select your title directory and executable.
4. Open **RUNNERS**:
   - Select an existing `victory.exe`, drop in a custom `runner-profile.json`, or click **BUILD FROM URL...** to clone and compile a runner from GitHub.
   - If the runner repo includes `scripts/get-nvidia-sdks.ps1`, click **GET DLSS** on the runner card to fetch official signed NVIDIA Streamline/DLSS DLLs.
   - Use **UPDATE** or **ROLLBACK** anytime on source-built runners.
5. Click **PREPARE CACHE** once (runs `shaders --jobs <N>` + `warm` automatically if supported), then click **LAUNCH TITLE**.

### In-Game DLSS & Frame Generation Controls (`F1` / `F3`)
When `sl.interposer.dll` / `nvngx_dlss.dll` are beside `victory.exe`, the runner card displays **`DLSS / FG READY (F1 MENU · F3 STATS)`**:
- Press **`F1`** in-game to open the DLSS / **2x–6x Frame Generation** / FPS cap overlay (arrow keys navigate/change, **Enter** confirms, `SAVE AND RESTART NOW` persists to `victory-settings.txt`).
- Press **`F3`** in-game to toggle the live FPS, frame time, Frame Gen output, draw count, RAM, and VRAM overlay.

---

## Building from Source

```powershell
git clone https://github.com/game-master-6/game-master-6.git
cd game-master-6
cargo test
cargo build --release
```

The compiled binary is output to `target\release\GameMaster6.exe`.
