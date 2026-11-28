use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::config::Settings;
use crate::runners::runners_dir;

pub fn runners_src_dir() -> PathBuf {
    let dir = Settings::app_dir().join("runners_src");
    let _ = fs::create_dir_all(&dir);
    dir
}

/// Validates a strict `owner/repo` GitHub slug (or full `https://github.com/owner/repo[.git]` URL).
pub fn validate_github_repo(repo: &str) -> Result<(String, String), String> {
    let mut clean = repo.trim().trim_matches('/');
    if let Some(rest) = clean.strip_prefix("https://github.com/") {
        clean = rest;
    } else if let Some(rest) = clean.strip_prefix("http://github.com/") {
        clean = rest;
    }
    if let Some(rest) = clean.strip_suffix(".git") {
        clean = rest;
    }
    clean = clean.trim_matches('/');
    let parts: Vec<&str> = clean.split('/').collect();
    if parts.len() != 2 {
        return Err(format!("Invalid GitHub repository format: '{repo}'"));
    }
    let valid_part = |s: &str| {
        !s.is_empty()
            && s.len() <= 64
            && s.bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
            && !s.contains("..")
    };
    if !valid_part(parts[0]) || !valid_part(parts[1]) {
        return Err(format!("Repository name contains unsafe characters: '{repo}'"));
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

/// Launches a hardened PowerShell build terminal that:
/// 1) Clones or git-pulls the validated GitHub repo
/// 2) Scans build.rs / CMakeLists.txt for suspicious network/shell patterns and pauses if found
/// 3) Backs up any existing runner .exe to `prev\*.bak` before replacing
/// 4) Compiles release (and optional diag) binaries, skipping build helpers like `xtask.exe`
/// 5) Writes `.gm-meta` with repo, short commit SHA, and timestamp so the Hub displays `UP TO DATE`
pub fn launch_clone_and_build(repo: &str, name: &str) -> Result<(), String> {
    let (owner, repo_name) = validate_github_repo(repo)?;
    let slug: String = repo_name
        .to_ascii_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect();
    let slug = if slug.is_empty() { "custom-runner".into() } else { slug };

    let safe_name: String = name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.'))
        .collect();

    let src_root = runners_src_dir();
    let repo_dir = src_root.join(&slug);
    let dest_dir = runners_dir().join(&slug);
    let prev_dir = dest_dir.join("prev");
    let _ = fs::create_dir_all(&dest_dir);

    let clone_url = format!("https://github.com/{owner}/{repo_name}.git");
    let script_path = src_root.join(format!("build-{slug}.ps1"));

    let ps_script = format!(
        "$ErrorActionPreference = 'Stop'\n\
         Write-Host '============================================================' -ForegroundColor Cyan\n\
         Write-Host '  Game GameMaster6 v15.2 — Safe Auto-Builder ({safe_name})' -ForegroundColor Magenta\n\
         Write-Host '  Repo: {clone_url}' -ForegroundColor Gray\n\
         Write-Host '============================================================' -ForegroundColor Cyan\n\
         $repoDir = '{repo_dir}'\n\
         $destDir = '{dest_dir}'\n\
         $prevDir = '{prev_dir}'\n\
         if (Test-Path (Join-Path $repoDir '.git')) {{\n\
             Write-Host '[1/4] Updating existing clone (git pull --ff-only)...' -ForegroundColor Yellow\n\
             Set-Location $repoDir\n\
             git pull --ff-only\n\
         }} else {{\n\
             Write-Host '[1/4] Cloning repository (shallow)...' -ForegroundColor Yellow\n\
             git clone --depth 1 '{clone_url}' $repoDir\n\
             Set-Location $repoDir\n\
         }}\n\
         $commitSha = ''\n\
         try {{ $commitSha = (git rev-parse --short HEAD).Trim() }} catch {{ $commitSha = 'local' }}\n\
         Write-Host ('  Commit: ' + $commitSha) -ForegroundColor Cyan\n\
         Write-Host '[2/4] Scanning build scripts (build.rs / CMakeLists.txt) for suspicious commands...' -ForegroundColor Yellow\n\
         $flags = Get-ChildItem -Recurse -Include 'build.rs','CMakeLists.txt' -ErrorAction SilentlyContinue | Select-String -Pattern 'Invoke-WebRequest|curl\\s|certutil|powershell|AppData\\\\Roaming|wallet|discord\\.com/api/webhooks' -ErrorAction SilentlyContinue\n\
         if ($flags) {{\n\
             Write-Host '[SECURITY WARNING] Suspicious patterns found in build scripts:' -ForegroundColor Red\n\
             $flags | Select-Object -First 10 | ForEach-Object {{ Write-Host ('  ' + $_.Path + ':' + $_.LineNumber + ' -> ' + $_.Line.Trim()) -ForegroundColor Red }}\n\
             $ans = Read-Host 'Type YES to continue compiling anyway, or press Enter to abort'\n\
             if ($ans -ne 'YES') {{ Write-Host 'Aborted safely.' -ForegroundColor Yellow; exit 1 }}\n\
         }} else {{\n\
             Write-Host '  No obvious suspicious build-script patterns flagged.' -ForegroundColor Green\n\
         }}\n\
         if (Test-Path 'apps\\victory\\src\\run.rs') {{\n\
             $runRs = Get-Content 'apps\\victory\\src\\run.rs' -Raw\n\
             $needle = 'let title_dir = exe.parent().unwrap_or(Path::new(\".\"));'\n\
             if ($runRs.Contains($needle)) {{\n\
                 Write-Host '[2b/4] Patching apps/victory/src/run.rs to respect VICTORY_GAME_DIR...' -ForegroundColor Cyan\n\
                 $runRs = $runRs.Replace($needle, 'let title_dir = cfg.game_dir.as_deref().unwrap_or_else(|| exe.parent().unwrap_or(Path::new(\".\")));')\n\
                 Set-Content 'apps\\victory\\src\\run.rs' -Value $runRs -NoNewline\n\
             }}\n\
         }}\n\
         Write-Host '[3/4] Compiling release binary...' -ForegroundColor Yellow\n\
         $agilityNeeded = ($env:GM_AGILITY -eq '1') -or (($env:GM_AGILITY -ne '0') -and ([System.Environment]::OSVersion.Version.Build -lt 22000))\n\
         $agilityWork = Join-Path (Split-Path $repoDir -Parent) 'agility'\n\
         $prevRustFlags = $env:CARGO_ENCODED_RUSTFLAGS\n\
         if ($agilityNeeded -and (Test-Path 'Cargo.toml')) {{\n\
             $agilityObj = Join-Path $agilityWork 'agility-exports-620.obj'\n\
             if (-not (Test-Path $agilityObj)) {{\n\
                 New-Item -ItemType Directory -Force -Path $agilityWork | Out-Null\n\
                 $agilityRs = Join-Path $agilityWork 'agility-exports.rs'\n\
                 @\"\n\
#![no_std]\n\
#![allow(non_upper_case_globals)]\n\
#[no_mangle]\n\
#[used]\n\
pub static D3D12SDKVersion: u32 = 620;\n\
#[no_mangle]\n\
#[used]\n\
pub static D3D12SDKPath: &[u8; 9] = &[0x2E, 0x5C, 0x44, 0x33, 0x44, 0x31, 0x32, 0x5C, 0x00];\n\
\"@ | Set-Content $agilityRs -NoNewline\n\
                 Write-Host '[3a/4] Windows 10 detected: building D3D12 Agility SDK exports...' -ForegroundColor Cyan\n\
                 rustc --edition 2021 --crate-type=lib --emit=obj -C codegen-units=1 -C opt-level=1 -C panic=abort $agilityRs -o $agilityObj\n\
             }}\n\
             if (Test-Path $agilityObj) {{\n\
                 $sep = [char]0x1F\n\
                 $agilityFlags = \"-C$sep`link-arg=$agilityObj$sep-C$sep`link-arg=/EXPORT:D3D12SDKVersion$sep-C$sep`link-arg=/EXPORT:D3D12SDKPath\"\n\
                 $env:CARGO_ENCODED_RUSTFLAGS = if ($prevRustFlags) {{ \"$prevRustFlags$sep$agilityFlags\" }} else {{ $agilityFlags }}\n\
             }}\n\
         }}\n\
         $built = @()\n\
         $excludePattern = '^(xtask|build-script|rustc|cargo|clippy).*\\.exe$'\n\
         if (Test-Path 'Cargo.toml') {{\n\
             cargo build --release\n\
             $built = Get-ChildItem -Path 'target\\release' -Filter '*.exe' -ErrorAction SilentlyContinue | Where-Object {{ $_.Name -notmatch $excludePattern }}\n\
             if (Select-String -Path 'Cargo.toml' -Pattern '\\[profile\\.diag\\]' -Quiet) {{\n\
                 Write-Host '[3b/4] Detected [profile.diag] — building diagnostics runner (victory-diag.exe)...' -ForegroundColor Yellow\n\
                 $diagDir = $destDir + '-diag'\n\
                 New-Item -ItemType Directory -Force -Path $diagDir | Out-Null\n\
                 cargo build --profile diag --features diagnostics\n\
                 Get-ChildItem -Path 'target\\diag' -Filter '*.exe' -ErrorAction SilentlyContinue | Where-Object {{ $_.Name -notmatch $excludePattern }} | ForEach-Object {{\n\
                     $dName = if ($_.Name -eq 'victory.exe') {{ 'victory-diag.exe' }} else {{ $_.Name }}\n\
                     Copy-Item $_.FullName -Destination (Join-Path $diagDir $dName) -Force\n\
                     Write-Host ('  -> Installed Diag: ' + $dName) -ForegroundColor Magenta\n\
                 }}\n\
             }}\n\
         }} elseif (Test-Path 'CMakeLists.txt') {{\n\
             cmake -B build -DCMAKE_BUILD_TYPE=Release\n\
             cmake --build build --config Release --parallel\n\
             $built = Get-ChildItem -Path 'build' -Recurse -Filter '*.exe' -ErrorAction SilentlyContinue | Where-Object {{ $_.Name -notmatch $excludePattern }}\n\
         }} else {{\n\
             Write-Host '[WARN] No Cargo.toml or CMakeLists.txt found. Inspect manually:' $repoDir -ForegroundColor Red\n\
         }}\n\
         $env:CARGO_ENCODED_RUSTFLAGS = $prevRustFlags\n\
         if ($built.Count -gt 0) {{\n\
             Write-Host '[4/4] Backing up previous .exe (if any) & installing...' -ForegroundColor Green\n\
             New-Item -ItemType Directory -Force -Path $prevDir | Out-Null\n\
             foreach ($f in $built) {{\n\
                 $target = Join-Path $destDir $f.Name\n\
                 if (Test-Path $target) {{\n\
                     Copy-Item $target -Destination (Join-Path $prevDir ($f.Name + '.bak')) -Force\n\
                 }}\n\
                 Copy-Item $f.FullName -Destination $target -Force\n\
                 $hash = (Get-FileHash $target -Algorithm SHA256).Hash.Substring(0, 16)\n\
                 Write-Host ('  -> Installed: ' + $f.Name + '  (SHA256: ' + $hash + '...)') -ForegroundColor Cyan\n\
             }}\n\
             if ($agilityNeeded) {{\n\
                 $d3dDir = Join-Path $destDir 'D3D12'\n\
                 if (-not (Test-Path (Join-Path $d3dDir 'D3D12Core.dll'))) {{\n\
                     try {{\n\
                         New-Item -ItemType Directory -Force -Path $agilityWork | Out-Null\n\
                         $nupkg = Join-Path $agilityWork 'microsoft.direct3d.d3d12.1.620.2.nupkg'\n\
                         $expectedHash = '70489669C2BBEAF318B62D72B0AC3E753D521387BFC848C9763791C4E920BFCC'\n\
                         if (-not (Test-Path $nupkg) -or ((Get-FileHash $nupkg -Algorithm SHA256).Hash -ne $expectedHash)) {{\n\
                             Write-Host '  -> Downloading D3D12 Agility SDK 1.620.2 (nuget.org)...' -ForegroundColor Cyan\n\
                             Invoke-WebRequest -Uri 'https://api.nuget.org/v3-flatcontainer/microsoft.direct3d.d3d12/1.620.2/microsoft.direct3d.d3d12.1.620.2.nupkg' -OutFile $nupkg -UseBasicParsing\n\
                         }}\n\
                         if ((Get-FileHash $nupkg -Algorithm SHA256).Hash -eq $expectedHash) {{\n\
                             Add-Type -AssemblyName System.IO.Compression.FileSystem\n\
                             New-Item -ItemType Directory -Force -Path $d3dDir | Out-Null\n\
                             $zip = [System.IO.Compression.ZipFile]::OpenRead($nupkg)\n\
                             foreach ($dll in @('D3D12Core.dll', 'd3d12SDKLayers.dll')) {{\n\
                                 $entry = $zip.GetEntry(\"build/native/bin/x64/$dll\")\n\
                                 if ($entry) {{ [System.IO.Compression.ZipFileExtensions]::ExtractToFile($entry, (Join-Path $d3dDir $dll), $true) }}\n\
                             }}\n\
                             $zip.Dispose()\n\
                             Write-Host ('  -> Installed D3D12 Agility SDK 1.620.2 in ' + $d3dDir) -ForegroundColor Green\n\
                         }} else {{\n\
                             Write-Host '  [WARN] Agility SDK SHA-256 mismatch; skipping D3D12Core.dll.' -ForegroundColor Yellow\n\
                         }}\n\
                     }} catch {{ Write-Host ('  [WARN] Could not install Agility SDK: ' + $_) -ForegroundColor Yellow }}\n\
                 }}\n\
             }}\n\
             $nowStr = (Get-Date).ToString('yyyy-MM-dd HH:mm')\n\
             $branchName = ''\n\
             try {{ $branchName = (git rev-parse --abbrev-ref HEAD).Trim() }} catch {{ $branchName = 'main' }}\n\
             $meta = \"repo={owner}/{repo_name}`ncommit=$commitSha`nbuilt_at=$nowStr`n\"\n\
             Set-Content (Join-Path $destDir '.gm-meta') -Value $meta -NoNewline\n\
             $metaJson = @{{\n\
                 repo = '{owner}/{repo_name}'\n\
                 git_url = '{clone_url}'\n\
                 branch = $branchName\n\
                 commit = $commitSha\n\
                 built_at = $nowStr\n\
             }} | ConvertTo-Json\n\
             Set-Content (Join-Path $destDir 'runner-meta.json') -Value $metaJson -Encoding UTF8\n\
             $mainExe = Join-Path $destDir 'victory.exe'\n\
             if (Test-Path $mainExe) {{\n\
                 $cDir = Join-Path (Split-Path (Split-Path $destDir -Parent) -Parent) 'caches\\{slug}'\n\
                 New-Item -ItemType Directory -Force -Path $cDir | Out-Null\n\
                 $fullHash = (Get-FileHash $mainExe -Algorithm SHA256).Hash.ToLower()\n\
                 Set-Content (Join-Path $cDir '.runner_hash') -Value $fullHash -NoNewline\n\
             }}\n\
             Write-Host ('  -> Wrote runner-meta.json & .gm-meta (' + $commitSha + ')') -ForegroundColor Green\n\
         }}\n\
         Write-Host ''\n\
         Write-Host '[DONE] GameMaster6 will auto-refresh the Hub to UP TO DATE (or click RESCAN).' -ForegroundColor Green\n\
         Read-Host 'Press Enter to close'\n",
        safe_name = safe_name,
        clone_url = clone_url,
        owner = owner,
        repo_name = repo_name,
        slug = slug,
        repo_dir = repo_dir.display(),
        dest_dir = dest_dir.display(),
        prev_dir = prev_dir.display(),
    );

    fs::write(&script_path, ps_script).map_err(|e| format!("Could not write build script: {e}"))?;

    Command::new("cmd")
        .args([
            "/C",
            "start",
            "GameMaster6 Safe Auto-Builder",
            "powershell",
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            &script_path.to_string_lossy(),
        ])
        .spawn()
        .map_err(|e| format!("Failed to launch PowerShell builder: {e}"))?;

    Ok(())
}

/// 1-click UPDATE for source-built runners: backs up `victory.exe` -> `victory.exe.bak` & `prev\victory.exe.bak`,
/// pulls latest commits, rebuilds release, updates `runner-meta.json` & `.runner_hash`, and runs `shaders` + `warm`.
pub fn launch_update_runner(exe_path: &std::path::Path, game_dir: &str) -> Result<(), String> {
    let dest_dir = exe_path.parent().ok_or("Invalid runner folder")?;
    let meta = crate::runners::read_runner_meta(exe_path);
    if meta.repo.trim().is_empty() {
        return Err("No runner-meta.json / .gm-meta repo metadata found for this runner.".into());
    }
    let slug = dest_dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("custom-runner");
    let repo_dir = runners_src_dir().join(slug);
    if !repo_dir.join(".git").is_dir() {
        return launch_clone_and_build(&meta.repo, slug);
    }
    let prev_dir = dest_dir.join("prev");
    let cache_dir = Settings::app_dir().join("caches").join(slug);
    let script_path = runners_src_dir().join(format!("update-{slug}.ps1"));
    let game_dir_esc = game_dir.replace('\'', "''");
    let ps = format!(
        "$ErrorActionPreference = 'Stop'\n\
         $repoDir = '{}'\n\
         $destDir = '{}'\n\
         $prevDir = '{}'\n\
         $cacheDir = '{}'\n\
         $gameDir = '{}'\n\
         Write-Host '=== Updating Runner: {slug} ===' -ForegroundColor Cyan\n\
         Set-Location $repoDir\n\
         git pull --ff-only\n\
         $commitSha = (git rev-parse --short HEAD).Trim()\n\
         $branchName = (git rev-parse --abbrev-ref HEAD).Trim()\n\
         cargo build --release\n\
         New-Item -ItemType Directory -Force -Path $prevDir | Out-Null\n\
         $built = Get-ChildItem -Path 'target\\release' -Filter '*.exe' | Where-Object {{ $_.Name -notmatch '^(xtask|build-script|rustc|cargo|clippy).*\\.exe$' }}\n\
         foreach ($f in $built) {{\n\
             $target = Join-Path $destDir $f.Name\n\
             if (Test-Path $target) {{\n\
                 Copy-Item $target -Destination (Join-Path $prevDir ($f.Name + '.bak')) -Force\n\
                 Copy-Item $target -Destination ($target + '.bak') -Force\n\
             }}\n\
             Copy-Item $f.FullName -Destination $target -Force\n\
         }}\n\
         $nowStr = (Get-Date).ToString('yyyy-MM-dd HH:mm')\n\
         $metaJson = @{{ repo = '{}'; branch = $branchName; commit = $commitSha; built_at = $nowStr }} | ConvertTo-Json\n\
         Set-Content (Join-Path $destDir 'runner-meta.json') -Value $metaJson -Encoding UTF8\n\
         Set-Content (Join-Path $destDir '.gm-meta') -Value \"repo={}`ncommit=$commitSha`nbuilt_at=$nowStr`n\" -NoNewline\n\
         $exe = Join-Path $destDir 'victory.exe'\n\
         if (Test-Path $exe) {{\n\
             New-Item -ItemType Directory -Force -Path $cacheDir | Out-Null\n\
             $hash = (Get-FileHash $exe -Algorithm SHA256).Hash.ToLower()\n\
             Set-Content (Join-Path $cacheDir '.runner_hash') -Value $hash -NoNewline\n\
             $help = (& $exe --help 2>&1) -join \"`n\"\n\
             $env:VICTORY_CACHE_DIR = $cacheDir\n\
             $env:VICTORY_HOST_WAVE = '32'\n\
             if ($gameDir -and (Test-Path $gameDir) -and ($help -match '(?m)^\\s*shaders\\b')) {{\n\
                 Write-Host 'Running shaders --jobs 12...' -ForegroundColor Yellow\n\
                 & $exe shaders --jobs 12 $gameDir\n\
             }}\n\
             if ($gameDir -and (Test-Path $gameDir) -and ($help -match '(?m)^\\s*warm\\b')) {{\n\
                 Write-Host 'Running warm...' -ForegroundColor Yellow\n\
                 $env:VICTORY_GAME_DIR = $gameDir\n\
                 & $exe warm\n\
             }}\n\
         }}\n\
         Write-Host 'Update complete!' -ForegroundColor Green\n\
         Read-Host 'Press Enter to close'\n",
        repo_dir.display(),
        dest_dir.display(),
        prev_dir.display(),
        cache_dir.display(),
        game_dir_esc,
        meta.repo,
        meta.repo,
    );
    fs::write(&script_path, ps).map_err(|e| format!("Could not write update script: {e}"))?;
    Command::new("cmd")
        .args(["/C", "start", "GameMaster6 Runner Updater", "powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", &script_path.to_string_lossy()])
        .spawn()
        .map_err(|e| format!("Failed to launch updater: {e}"))?;
    Ok(())
}

/// Explicit opt-in fetcher for official NVIDIA DLSS/Streamline SDKs when `scripts/get-nvidia-sdks.ps1` exists.
pub fn launch_fetch_nvidia_sdks(exe_path: &std::path::Path) -> Result<(), String> {
    let dest_dir = exe_path.parent().ok_or("Invalid runner folder")?;
    let slug = dest_dir
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("Invalid runner slug")?;
    let repo_dir = runners_src_dir().join(slug);
    let script = repo_dir.join("scripts").join("get-nvidia-sdks.ps1");
    if !script.is_file() {
        return Err(format!("No scripts/get-nvidia-sdks.ps1 found in {}", repo_dir.display()));
    }
    let ps = format!(
        "$ErrorActionPreference = 'Stop'\n\
         Write-Host 'Fetching official NVIDIA DLSS/Streamline SDKs...' -ForegroundColor Cyan\n\
         & '{}' \n\
         $bin = Join-Path '{}' 'nvidia-sdk\\Streamline\\bin\\x64'\n\
         if (Test-Path $bin) {{\n\
             Copy-Item (Join-Path $bin '*') -Destination '{}' -Force\n\
             Write-Host 'Copied signed NVIDIA DLLs into runner folder!' -ForegroundColor Green\n\
         }}\n\
         Read-Host 'Press Enter to close'\n",
        script.display(),
        repo_dir.display(),
        dest_dir.display(),
    );
    let wrapper = runners_src_dir().join(format!("fetch-dlss-{slug}.ps1"));
    fs::write(&wrapper, ps).map_err(|e| format!("Could not write SDK fetcher script: {e}"))?;
    Command::new("cmd")
        .args(["/C", "start", "Fetch Official NVIDIA DLSS/FG SDKs", "powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", &wrapper.to_string_lossy()])
        .spawn()
        .map_err(|e| format!("Failed to launch NVIDIA SDK fetcher: {e}"))?;
    Ok(())
}
