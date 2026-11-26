#![windows_subsystem = "windows"]

mod ab_bench;
mod builder;
mod config;
mod diagnostics;
mod guardian;
mod hud;
mod keymap;
mod launcher;
mod monitor;
mod portwatch;
mod preflight;
mod presets;
mod profiles;
mod render;
mod runners;
mod saves;
mod shader_heal;
mod telemetry;
mod ui;
mod vram_guard;
mod win32;

fn main() {
    ui::run();
}
