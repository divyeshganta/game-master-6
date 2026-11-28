#![windows_subsystem = "windows"]

mod ab_bench;
mod builder;
mod config;
mod diagnostics;
mod guardian;
mod hud;
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
mod update_check;
mod vram_guard;
mod win32;

fn main() {
    ui::run();
}


#[cfg(test)]
mod mojibake_lock {
    #[test]
    fn no_utf8_mojibake_in_src_rs_files() {
        let src_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let needles: [&str; 6] = [
            "\u{00C2}\u{00B7}",
            "\u{00E2}\u{0153}",
            "\u{00E2}\u{2013}",
            "\u{00E2}\u{20AC}",
            "\u{00E2}\u{02DC}",
            "\u{00C3}\u{2014}",
        ];
        let mut offenders = Vec::new();
        for entry in std::fs::read_dir(&src_dir).expect("read src dir").flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("rs") { continue; }
            let bytes = std::fs::read(&path).expect("read rs file");
            assert!(!bytes.starts_with(&[0xEF, 0xBB, 0xBF]), "BOM present in {}", path.display());
            let text = std::str::from_utf8(&bytes).unwrap_or_else(|_| panic!("not valid UTF-8: {}", path.display()));
            for needle in needles {
                if text.contains(needle) { offenders.push(format!("{} contains mojibake {:?}", path.display(), needle)); }
            }
        }
        assert!(offenders.is_empty(), "UTF-8 mojibake found:\n{}", offenders.join("\n"));
    }
}
