//! Proton builds: directories with a `compatibilitytool.vdf` in Steam's
//! compatibilitytools.d folders (plus `ProtonPaths` from LauncherSettings).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::config::home;

#[derive(Debug, Clone)]
pub struct ProtonBuild {
    pub name: String,
    pub path: PathBuf,
    /// "GE", "UMU", "CachyOS" or "Other"
    pub kind: &'static str,
}

pub fn default_roots() -> Vec<PathBuf> {
    vec![home().join(".steam/steam/compatibilitytools.d"), PathBuf::from("/usr/share/steam/compatibilitytools.d")]
}

pub fn is_valid(p: &Path) -> bool {
    p.is_dir() && !p.to_string_lossy().contains("LegacyRuntime") && p.join("compatibilitytool.vdf").is_file()
}

fn kind_of(name: &str) -> &'static str {
    let n = name.to_ascii_lowercase();
    if n.starts_with("ge-proton") {
        "GE"
    } else if n.starts_with("umu-proton") {
        "UMU"
    } else if n.contains("cachyos") {
        "CachyOS"
    } else {
        "Other"
    }
}

/// Natural-ish ordering key so GE-Proton10-9 < GE-Proton10-25 < GE-Proton11-1
fn sort_key(name: &str) -> Vec<(u8, u64, String)> {
    let mut key = Vec::new();
    let mut num = String::new();
    let mut txt = String::new();
    for c in name.chars() {
        if c.is_ascii_digit() {
            if !txt.is_empty() {
                key.push((0, 0, std::mem::take(&mut txt)));
            }
            num.push(c);
        } else {
            if !num.is_empty() {
                key.push((1, num.parse().unwrap_or(0), String::new()));
                num.clear();
            }
            txt.push(c);
        }
    }
    if !num.is_empty() {
        key.push((1, num.parse().unwrap_or(0), String::new()));
    }
    if !txt.is_empty() {
        key.push((0, 0, txt));
    }
    key
}

/// Newest first, GE builds before others
pub fn list(extra_roots: &[PathBuf]) -> Vec<ProtonBuild> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for root in default_roots().iter().chain(extra_roots) {
        let Ok(rd) = std::fs::read_dir(root) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if !is_valid(&p) {
                continue;
            }
            let name = e.file_name().to_string_lossy().into_owned();
            if seen.insert(p.clone()) {
                out.push(ProtonBuild { kind: kind_of(&name), name, path: p });
            }
        }
    }
    out.sort_by(|a, b| {
        (a.kind != "GE").cmp(&(b.kind != "GE")).then_with(|| sort_key(&b.name).cmp(&sort_key(&a.name)))
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordering() {
        let mut v = vec!["GE-Proton10-9", "GE-Proton11-1", "GE-Proton10-25"];
        v.sort_by_key(|a| std::cmp::Reverse(sort_key(a)));
        assert_eq!(v, vec!["GE-Proton11-1", "GE-Proton10-25", "GE-Proton10-9"]);
    }
}
