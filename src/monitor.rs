use crate::win32::{CloseHandle, OpenProcess, BOOL, DWORD, HANDLE};

const PROCESS_QUERY_LIMITED_INFORMATION: DWORD = 0x1000;

#[repr(C)]
#[derive(Default)]
struct ProcessMemoryCountersEx {
    cb: DWORD,
    page_fault_count: DWORD,
    peak_working_set_size: usize,
    working_set_size: usize,
    quota_peak_paged_pool_usage: usize,
    quota_paged_pool_usage: usize,
    quota_peak_non_paged_pool_usage: usize,
    quota_non_paged_pool_usage: usize,
    pagefile_usage: usize,
    peak_pagefile_usage: usize,
    private_usage: usize,
}

#[link(name = "kernel32")]
extern "system" {
    fn K32GetProcessMemoryInfo(
        process: HANDLE,
        ppsmem_counters: *mut ProcessMemoryCountersEx,
        cb: DWORD,
    ) -> BOOL;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ProcessStats {
    pub working_set_gb: f64,
    pub private_commit_gb: f64,
    pub peak_ws_gb: f64,
}

/// Samples live Working Set RAM and Private Commit of the running GameRunner child process.
pub fn sample_process(pid: u32) -> Option<ProcessStats> {
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if h.is_null() {
            return None;
        }
        let mut pmc = ProcessMemoryCountersEx {
            cb: std::mem::size_of::<ProcessMemoryCountersEx>() as DWORD,
            ..Default::default()
        };
        let ok = K32GetProcessMemoryInfo(
            h,
            &mut pmc as *mut ProcessMemoryCountersEx,
            pmc.cb,
        );
        CloseHandle(h);
        if ok == 0 {
            return None;
        }
        const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
        Some(ProcessStats {
            working_set_gb: pmc.working_set_size as f64 / GIB,
            private_commit_gb: pmc.private_usage as f64 / GIB,
            peak_ws_gb: pmc.peak_working_set_size as f64 / GIB,
        })
    }
}
