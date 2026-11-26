//! Native CGBJ Live A/B Benchmark Controller (v15.2)
//! Hooks directly into CGBJ's built-in `VICTORY_AB=<seconds>,<switch>` engine
//! (verified in `crates\victory-gpu\src\ab.rs:26`) to scientifically measure whether
//! a specific optimization switch gains or loses FPS in the EXACT same live scene.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AbExperiment {
    Off,
    AsyncSubmit,
    ThinLists,
    WideRefresh,
    UploadAhead,
}

impl AbExperiment {
    pub fn id(self) -> &'static str {
        match self {
            AbExperiment::Off => "off",
            AbExperiment::AsyncSubmit => "async-submit",
            AbExperiment::ThinLists => "thin",
            AbExperiment::WideRefresh => "wide-refresh",
            AbExperiment::UploadAhead => "upload-ahead",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            AbExperiment::Off => "A/B TEST: OFF",
            AbExperiment::AsyncSubmit => "A/B: ASYNC_SUBMIT (15s)",
            AbExperiment::ThinLists => "A/B: THIN LISTS (15s)",
            AbExperiment::WideRefresh => "A/B: WIDE_REFRESH (15s)",
            AbExperiment::UploadAhead => "A/B: UPLOAD_AHEAD (15s)",
        }
    }

    pub fn env_value(self) -> Option<&'static str> {
        match self {
            AbExperiment::Off => None,
            AbExperiment::AsyncSubmit => Some("15,VICTORY_ASYNC_SUBMIT"),
            AbExperiment::ThinLists => Some("15,VICTORY_THIN"),
            AbExperiment::WideRefresh => Some("15,VICTORY_WIDE_REFRESH"),
            AbExperiment::UploadAhead => Some("15,VICTORY_UPLOAD_AHEAD"),
        }
    }

    pub fn from_id(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "async-submit" => AbExperiment::AsyncSubmit,
            "thin" => AbExperiment::ThinLists,
            "wide-refresh" => AbExperiment::WideRefresh,
            "upload-ahead" => AbExperiment::UploadAhead,
            _ => AbExperiment::Off,
        }
    }

    pub fn cycle_next(self) -> Self {
        match self {
            AbExperiment::Off => AbExperiment::AsyncSubmit,
            AbExperiment::AsyncSubmit => AbExperiment::ThinLists,
            AbExperiment::ThinLists => AbExperiment::WideRefresh,
            AbExperiment::WideRefresh => AbExperiment::UploadAhead,
            AbExperiment::UploadAhead => AbExperiment::Off,
        }
    }
}
