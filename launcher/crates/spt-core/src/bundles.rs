//! Mod bundles: the server lists them at `/singleplayer/bundles`; anything not
//! already in the cache (or unchanged in the mod folder) is downloaded before
//! the game starts, 8 at a time, 3 attempts each.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, UNIX_EPOCH};

use anyhow::{bail, Result};

use crate::api::Client;
use crate::config::SptInstall;
use crate::models::BundleManifestItem;

const MAX_CONCURRENT: usize = 8;
const MAX_ATTEMPTS: usize = 3;
/// .NET ticks at the Unix epoch
const EPOCH_TICKS: i64 = 621_355_968_000_000_000;

#[derive(Debug, Clone, Default)]
pub struct BundleProgress {
    pub done: usize,
    pub total: usize,
    pub bytes: u64,
    pub total_bytes: u64,
    pub bytes_per_sec: f64,
    pub current: String,
}

impl BundleProgress {
    pub fn percent(&self) -> f64 {
        if self.total_bytes > 0 {
            self.bytes as f64 / self.total_bytes as f64 * 100.0
        } else if self.total > 0 {
            self.done as f64 / self.total as f64 * 100.0
        } else {
            100.0
        }
    }
}

pub fn cache_path(install: &SptInstall, b: &BundleManifestItem) -> PathBuf {
    install.bundle_cache_dir().join(format!("{:08X}", b.crc)).join(&b.file_name)
}

pub fn mod_path(install: &SptInstall, b: &BundleManifestItem) -> PathBuf {
    install.runtime_dir().join(&b.mod_path).join("bundles").join(&b.file_name)
}

fn mtime_ticks(p: &Path) -> Option<i64> {
    let d = fs::metadata(p).ok()?.modified().ok()?.duration_since(UNIX_EPOCH).ok()?;
    Some(EPOCH_TICKS + d.as_secs() as i64 * 10_000_000 + (d.subsec_nanos() / 100) as i64)
}

fn crc32_file(p: &Path) -> Option<u32> {
    let mut f = fs::File::open(p).ok()?;
    let mut h = crc32fast::Hasher::new();
    let mut buf = vec![0u8; 128 * 1024];
    loop {
        let n = f.read(&mut buf).ok()?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Some(h.finalize())
}

pub fn is_available(install: &SptInstall, b: &BundleManifestItem) -> bool {
    let cache = cache_path(install, b);
    if fs::metadata(&cache).map(|m| m.len() == b.size).unwrap_or(false) {
        return true;
    }
    let modp = mod_path(install, b);
    match fs::metadata(&modp) {
        Ok(m) if m.len() == b.size => mtime_ticks(&modp) == Some(b.modified_utc_ticks) || crc32_file(&modp) == Some(b.crc),
        _ => false,
    }
}

/// Downloads every missing bundle. `progress` is called at most every 100 ms.
pub fn acquire(
    client: &Client,
    install: &SptInstall,
    cancel: &AtomicBool,
    progress: impl Fn(&BundleProgress) + Sync,
) -> Result<BundleProgress> {
    let manifest = client.bundle_manifest()?;
    let total = manifest.len();
    let missing: Vec<BundleManifestItem> = manifest.into_iter().filter(|b| !is_available(install, b)).collect();
    let total_bytes: u64 = missing.iter().map(|b| b.size).sum();

    let done = AtomicUsize::new(total - missing.len());
    let bytes = AtomicU64::new(0);
    let failures = Mutex::new(Vec::<String>::new());
    let queue = Mutex::new(missing.into_iter());
    let started = Instant::now();
    let last_report = Mutex::new((Instant::now() - Duration::from_secs(1), 0u64, 0.0f64));

    let report = |current: &str, force: bool| {
        let mut lr = last_report.lock().unwrap();
        let now = Instant::now();
        if !force && now.duration_since(lr.0) < Duration::from_millis(100) {
            return;
        }
        let b = bytes.load(Ordering::Relaxed);
        let window = now.duration_since(lr.0).as_secs_f64();
        if window > 0.0 && window < 5.0 {
            lr.2 = b.saturating_sub(lr.1) as f64 / window;
        }
        *lr = (now, b, lr.2);
        progress(&BundleProgress {
            done: done.load(Ordering::Relaxed),
            total,
            bytes: b,
            total_bytes,
            bytes_per_sec: lr.2,
            current: current.to_string(),
        });
    };

    report("", true);
    std::thread::scope(|s| {
        for _ in 0..MAX_CONCURRENT {
            s.spawn(|| loop {
                if cancel.load(Ordering::Relaxed) {
                    return;
                }
                let Some(b) = queue.lock().unwrap().next() else { return };
                let dest = cache_path(install, &b);
                let mut ok = false;
                for _attempt in 0..MAX_ATTEMPTS {
                    let mut counted = 0u64;
                    let r = client.download_bundle(&b.file_name, &dest, |w| {
                        bytes.fetch_add(w - counted, Ordering::Relaxed);
                        counted = w;
                        report(&b.file_name, false);
                    });
                    if r.is_ok() {
                        ok = true;
                        break;
                    }
                    bytes.fetch_sub(counted, Ordering::Relaxed);
                    if cancel.load(Ordering::Relaxed) {
                        break;
                    }
                }
                if ok {
                    done.fetch_add(1, Ordering::Relaxed);
                } else {
                    failures.lock().unwrap().push(b.file_name.clone());
                }
            });
        }
    });

    if cancel.load(Ordering::Relaxed) {
        bail!("cancelled");
    }
    let failures = failures.into_inner().unwrap();
    if let Some(first) = failures.first() {
        bail!("failed to download {} bundle(s), e.g. {first}", failures.len());
    }
    let elapsed = started.elapsed().as_secs_f64().max(0.001);
    let final_progress = BundleProgress {
        done: total,
        total,
        bytes: bytes.load(Ordering::Relaxed),
        total_bytes,
        bytes_per_sec: bytes.load(Ordering::Relaxed) as f64 / elapsed,
        current: String::new(),
    };
    progress(&final_progress);
    Ok(final_progress)
}

pub fn format_bytes(b: f64) -> String {
    let units = ["B", "KB", "MB", "GB", "TB"];
    let mut v = b;
    let mut u = 0;
    while v >= 1024.0 && u < units.len() - 1 {
        v /= 1024.0;
        u += 1;
    }
    if u == 0 {
        format!("{v:.0} {}", units[u])
    } else {
        format!("{v:.1} {}", units[u])
    }
}

/// Keeps a shareable cancel flag next to the progress (used by the UI layer)
pub type CancelFlag = Arc<AtomicBool>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes() {
        assert_eq!(format_bytes(512.0), "512 B");
        assert_eq!(format_bytes(1536.0), "1.5 KB");
    }
}
