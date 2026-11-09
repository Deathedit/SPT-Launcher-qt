//! Pre-launch steps and the game launch, following the official launcher:
//! check client (spt-core.dll vs server) -> remove files that must not exist ->
//! (clear cache) -> restore + apply patches -> bundles -> umu-run.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, bail, Context, Result};

use crate::config::{LauncherSettings, SptInstall};

// ---------------------------------------------------------------------------
// Client version check
// ---------------------------------------------------------------------------

/// FileVersion (major, minor, build) of a PE file, read from VS_FIXEDFILEINFO
pub fn pe_file_version(path: &Path) -> Result<(u16, u16, u16)> {
    let data = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    const SIG: [u8; 4] = 0xFEEF04BDu32.to_le_bytes();
    let pos = data
        .windows(4)
        .position(|w| w == SIG)
        .ok_or_else(|| anyhow!("{} has no version resource", path.display()))?;
    let field = |off: usize| -> Result<u32> {
        let b = data.get(pos + off..pos + off + 4).ok_or_else(|| anyhow!("truncated version resource"))?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };
    // dwSignature, dwStrucVersion, dwFileVersionMS, dwFileVersionLS
    let ms = field(8)?;
    let ls = field(12)?;
    Ok(((ms >> 16) as u16, (ms & 0xffff) as u16, (ls >> 16) as u16))
}

/// `"4.1.6"` or `"4.1.6 - bleeding edge"` -> (4, 1, 6)
pub fn parse_server_version(v: &str) -> Option<(u16, u16, u16)> {
    let core = v.split('-').next()?.trim();
    let mut it = core.split('.').map(|p| p.trim().parse::<u16>().ok());
    Some((it.next()??, it.next()??, it.next().flatten().unwrap_or(0)))
}

#[derive(Debug, Clone, PartialEq)]
pub enum ClientCheck {
    Match(String),
    /// (client, server)
    Mismatch(String, String),
    /// Locally built modules (major 1): the official launcher skips the check
    Skipped(String),
}

pub fn check_client(install: &SptInstall, server_version: &str) -> Result<ClientCheck> {
    let server = parse_server_version(server_version).ok_or_else(|| anyhow!("unknown server version \"{server_version}\""))?;
    let dll = install.core_dll();
    if !dll.is_file() {
        bail!("spt-core.dll is missing ({}). Is SPT installed?", dll.display());
    }
    let (ma, mi, pa) = pe_file_version(&dll)?;
    let client = format!("{ma}.{mi}.{pa}");
    if ma == 1 {
        return Ok(ClientCheck::Skipped(client));
    }
    if (ma, mi, pa) == server {
        Ok(ClientCheck::Match(client))
    } else {
        Ok(ClientCheck::Mismatch(client, format!("{}.{}.{}", server.0, server.1, server.2)))
    }
}

// ---------------------------------------------------------------------------
// Game files
// ---------------------------------------------------------------------------

/// Deletes live-game files SPT must not have (BattlEye etc.). Returns what was removed.
pub fn setup_game_files(install: &SptInstall, settings: &LauncherSettings) -> Vec<String> {
    let exclude = settings.exclude_from_cleanup();
    let mut targets: Vec<PathBuf> = [
        "BattlEye",
        "Logs",
        "ConsistencyInfo",
        "EscapeFromTarkov_BE.exe",
        "Uninstall.exe",
        "UnityCrashHandler64.exe",
        "WinPixEventRuntime.dll",
    ]
    .iter()
    .filter(|n| !exclude.iter().any(|e| e == *n))
    .map(|n| install.root.join(n))
    .collect();
    // Never excludable
    targets.push(install.root.join("EscapeFromTarkov_Data/Plugins/x86_64/hwecho.dll"));

    let mut removed = Vec::new();
    for t in targets {
        let ok = if t.is_dir() {
            fs::remove_dir_all(&t).is_ok()
        } else if t.is_file() || t.is_symlink() {
            fs::remove_file(&t).is_ok()
        } else {
            false
        };
        if ok {
            removed.push(t.strip_prefix(&install.root).unwrap_or(&t).display().to_string());
        }
    }
    removed
}

pub fn clear_cache(install: &SptInstall) -> Result<()> {
    let dir = install.app_data_dir();
    if dir.exists() {
        fs::remove_dir_all(&dir).with_context(|| format!("removing {}", dir.display()))?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Patching (SPT_Data/Launcher/Patches/<name>/**/*.delta, HDiffPatch)
// ---------------------------------------------------------------------------

/// Restores every `*.spt-bak` under the game directory (symlinked dirs are skipped)
pub fn restore_backups(root: &Path) -> Result<usize> {
    let mut restored = 0;
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            let Ok(ft) = e.file_type() else { continue };
            let p = e.path();
            if ft.is_symlink() {
                continue;
            }
            if ft.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "spt-bak") {
                let target = p.with_extension("");
                let _ = fs::remove_file(&target);
                fs::copy(&p, &target).with_context(|| format!("restoring {}", target.display()))?;
                restored += 1;
            }
        }
    }
    Ok(restored)
}

fn collect_deltas(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect_deltas(&p, out);
        } else if p.extension().is_some_and(|x| x == "delta") {
            out.push(p);
        }
    }
}

/// Applies all patches. `progress(done, total, file)`.
pub fn patch_game(install: &SptInstall, hpatchz: &str, mut progress: impl FnMut(usize, usize, &str)) -> Result<usize> {
    restore_backups(&install.root)?;

    let patches_dir = install.patches_dir();
    let mut patch_dirs: Vec<PathBuf> =
        fs::read_dir(&patches_dir).map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect()).unwrap_or_default();
    patch_dirs.sort();

    let mut jobs = Vec::new();
    for pd in &patch_dirs {
        let mut deltas = Vec::new();
        collect_deltas(pd, &mut deltas);
        deltas.sort();
        for d in deltas {
            let rel = d.strip_prefix(pd).unwrap().with_extension("");
            jobs.push((d, install.root.join(rel)));
        }
    }

    let total = jobs.len();
    for (i, (delta, target)) in jobs.iter().enumerate() {
        let rel = target.strip_prefix(&install.root).unwrap_or(target).display().to_string();
        progress(i, total, &rel);
        let backup = PathBuf::from(format!("{}.spt-bak", target.display()));
        if !backup.exists() {
            fs::copy(target, &backup).with_context(|| format!("backing up {rel}"))?;
        }
        let out = Command::new(hpatchz)
            .arg("-f")
            .arg(&backup)
            .arg(delta)
            .arg(target)
            .output()
            .with_context(|| format!("running {hpatchz}"))?;
        if !out.status.success() {
            bail!("patching {rel} failed: {}", String::from_utf8_lossy(&out.stderr).trim());
        }
    }
    progress(total, total, "");
    Ok(total)
}

// ---------------------------------------------------------------------------
// Launch
// ---------------------------------------------------------------------------

pub struct LaunchOptions {
    pub profile_id: String,
    pub backend_url: String,
    pub umu_run: String,
}

/// `[gamemoderun] umu-run EscapeFromTarkov.exe <args>` with WINEPREFIX / PROTONPATH and
/// the DefaultEnv + LaunchSettings tokens (`NAME=VALUE` -> env, anything else -> argument)
pub fn launch_command(install: &SptInstall, settings: &LauncherSettings, opts: &LaunchOptions) -> Result<Command> {
    let prefix = settings.prefix_path();
    let proton = settings.proton_path();
    if prefix.is_empty() {
        bail!("No wine prefix set (Settings, or run the installer)");
    }
    if proton.is_empty() {
        bail!("No Proton build selected (Proton page)");
    }
    if !install.root.join("EscapeFromTarkov.exe").is_file() {
        bail!("EscapeFromTarkov.exe not found in {}", install.root.display());
    }

    let mut cmd = if settings.game_mode() {
        let mut c = Command::new("gamemoderun");
        c.arg(&opts.umu_run);
        c
    } else {
        Command::new(&opts.umu_run)
    };
    cmd.current_dir(&install.root)
        .arg("EscapeFromTarkov.exe")
        .arg("-force-gfx-jobs")
        .arg("native")
        .arg(format!("-token={}", opts.profile_id))
        .arg(format!("-config={{'BackendUrl':'{}','Version':'live','MatchingVersion':'live'}}", opts.backend_url))
        .env("WINEPREFIX", &prefix)
        .env("PROTONPATH", &proton)
        // Steam-injected libraries break umu; nix-shell's private TMPDIR is not visible in its container
        .env_remove("LD_PRELOAD")
        .env_remove("LD_LIBRARY_PATH")
        .env_remove("TMP")
        .env_remove("TEMP")
        .env_remove("TEMPDIR")
        .env("TMPDIR", "/tmp");

    let mut tokens = Vec::new();
    let default_env = settings.default_env();
    tokens.extend(tokenize_launch_settings(&default_env));
    tokens.extend(tokenize_launch_settings(&settings.launch_settings()));
    for t in tokens {
        match t.find('=') {
            Some(i) if i > 0 && !t.starts_with('-') => {
                cmd.env(&t[..i], t[i + 1..].trim_matches('"'));
            }
            _ => {
                cmd.arg(t);
            }
        }
    }
    Ok(cmd)
}

/// Steam launch-options style splitting; quotes group (and are stripped), `\"` is literal
pub fn tokenize_launch_settings(s: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut cur = String::new();
    let mut quote: Option<char> = None;
    let mut started = false;
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\\' && i + 1 < chars.len() && (chars[i + 1] == '"' || chars[i + 1] == '\'') {
            cur.push(chars[i + 1]);
            started = true;
            i += 2;
            continue;
        }
        if let Some(q) = quote {
            if c == q {
                quote = None;
            } else {
                cur.push(c);
            }
        } else if c == '"' || c == '\'' {
            quote = Some(c);
            started = true;
        } else if c.is_whitespace() {
            if started {
                tokens.push(std::mem::take(&mut cur));
                started = false;
            }
        } else {
            cur.push(c);
            started = true;
        }
        i += 1;
    }
    if started {
        tokens.push(cur);
    }
    tokens
}

/// PIDs of running game processes (Linux truncates comm to 15 chars)
pub fn game_pids() -> Vec<i32> {
    let mut pids = Vec::new();
    let Ok(rd) = fs::read_dir("/proc") else { return pids };
    for e in rd.flatten() {
        let Some(pid) = e.file_name().to_str().and_then(|s| s.parse::<i32>().ok()) else { continue };
        if let Ok(comm) = fs::read_to_string(e.path().join("comm")) {
            let comm = comm.trim();
            if comm == "EscapeFromTarko" || comm == "EscapeFromTarkov.exe" {
                pids.push(pid);
            }
        }
    }
    pids
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes() {
        assert_eq!(
            tokenize_launch_settings(r#"MANGOHUD=1 -arg="/path with spaces" WINEDLLOVERRIDES="d3d11=n,b" "" x"#),
            vec!["MANGOHUD=1", "-arg=/path with spaces", "WINEDLLOVERRIDES=d3d11=n,b", "", "x"]
        );
        assert_eq!(tokenize_launch_settings(r#"WINEDLLOVERRIDES="winhttp=n,b""#), vec!["WINEDLLOVERRIDES=winhttp=n,b"]);
        assert!(tokenize_launch_settings("   ").is_empty());
    }

    #[test]
    fn server_versions() {
        assert_eq!(parse_server_version("4.1.6"), Some((4, 1, 6)));
        assert_eq!(parse_server_version("4.1.6 - BE build"), Some((4, 1, 6)));
        assert_eq!(parse_server_version("x"), None);
    }

    #[test]
    fn reads_pe_version() {
        let dir = std::env::temp_dir().join(format!("spt-core-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let f = dir.join("a.dll");
        let mut data = vec![0u8; 64];
        data.extend_from_slice(&0xFEEF04BDu32.to_le_bytes());
        data.extend_from_slice(&0x0001_0000u32.to_le_bytes());
        data.extend_from_slice(&((4u32 << 16) | 1).to_le_bytes());
        data.extend_from_slice(&(6u32 << 16).to_le_bytes());
        fs::write(&f, &data).unwrap();
        assert_eq!(pe_file_version(&f).unwrap(), (4, 1, 6));
        fs::remove_dir_all(&dir).unwrap();
    }
}
