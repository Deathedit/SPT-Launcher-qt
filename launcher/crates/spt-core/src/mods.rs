//! Installing mod archives into an SPT 4.1 directory.
//!
//! SPT mod archives mirror the SPT folder: client parts live in
//! `BepInEx/plugins|patchers|config/...`, server parts in
//! `SPT_Runtime/user/mods/...` (`SPT/user/mods` or `user/mods` in older
//! archives). Archives are extracted into a scratch directory first, then only
//! files below those folders are copied, so an archive can never write
//! anywhere else in the game directory. Everything the launcher installs is
//! recorded in a manifest so it can be updated or removed again.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::config::{write_atomic, SptInstall};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    Client,
    Server,
}

#[derive(Debug, Clone)]
pub struct PlannedFile {
    pub src: PathBuf,
    /// Destination relative to the SPT root, `/`-separated
    pub dest: String,
    pub part: Part,
}

#[derive(Debug, Default)]
pub struct Plan {
    pub files: Vec<PlannedFile>,
    /// Archive entries outside the known folders (readmes, licenses, ...)
    pub ignored: Vec<String>,
}

impl Plan {
    pub fn has(&self, part: Part) -> bool {
        self.files.iter().any(|f| f.part == part)
    }

    pub fn count(&self, part: Part) -> usize {
        self.files.iter().filter(|f| f.part == part).count()
    }
}

const SERVER_MODS: &str = "SPT_Runtime/user/mods";

fn is(s: &str, name: &str) -> bool {
    s.eq_ignore_ascii_case(name)
}

/// Known top-level folders of an SPT mod archive
fn is_known_root(s: &str) -> bool {
    ["BepInEx", "SPT_Runtime", "SPT", "user", "plugins", "patchers"].iter().any(|r| is(s, r))
}

/// Maps an archive path (relative, already stripped of wrapper folders) to its
/// place in the SPT directory
fn map_path(c: &[String]) -> Option<(String, Part)> {
    let rest = |n: usize| c[n..].join("/");
    match c {
        [b, sub, _, ..] if is(b, "BepInEx") => {
            let sub = ["plugins", "patchers", "config"].into_iter().find(|s| is(sub, s))?;
            Some((format!("BepInEx/{sub}/{}", rest(2)), Part::Client))
        }
        [r, u, m, _, ..] if (is(r, "SPT_Runtime") || is(r, "SPT")) && is(u, "user") && is(m, "mods") => {
            Some((format!("{SERVER_MODS}/{}", rest(3)), Part::Server))
        }
        [u, m, _, ..] if is(u, "user") && is(m, "mods") => Some((format!("{SERVER_MODS}/{}", rest(2)), Part::Server)),
        [p, _, ..] if is(p, "plugins") => Some((format!("BepInEx/plugins/{}", rest(1)), Part::Client)),
        [p, _, ..] if is(p, "patchers") => Some((format!("BepInEx/patchers/{}", rest(1)), Part::Client)),
        [f] if f.to_ascii_lowercase().ends_with(".dll") => Some((format!("BepInEx/plugins/{f}"), Part::Client)),
        _ => None,
    }
}

/// Regular files below `dir` as path components (symlinks are skipped)
fn walk(dir: &Path) -> Result<Vec<(PathBuf, Vec<String>)>> {
    let mut out = Vec::new();
    let mut stack = vec![(dir.to_path_buf(), Vec::<String>::new())];
    while let Some((path, comps)) = stack.pop() {
        for entry in fs::read_dir(&path).with_context(|| format!("read {}", path.display()))? {
            let entry = entry?;
            let meta = fs::symlink_metadata(entry.path())?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let mut c = comps.clone();
            // Zips made on Windows sometimes store `BepInEx\plugins\X.dll` as one name;
            // a backslash can't be part of a Windows file name, so it is a separator
            c.extend(name.split('\\').filter(|p| !p.is_empty()).map(String::from));
            if meta.is_dir() {
                stack.push((entry.path(), c));
            } else if meta.is_file() {
                out.push((entry.path(), c));
            }
        }
    }
    out.sort_by(|a, b| a.1.cmp(&b.1));
    Ok(out)
}

/// Works out where the files of an extracted archive go
pub fn plan(extracted: &Path) -> Result<Plan> {
    let mut files = walk(extracted)?;
    // Strip wrapper folders ("ModName-1.0/BepInEx/...") while every file shares one
    loop {
        let first = match files.first() {
            Some((_, c)) if c.len() > 1 => c[0].clone(),
            _ => break,
        };
        if is_known_root(&first) || !files.iter().all(|(_, c)| c.len() > 1 && c[0] == first) {
            break;
        }
        for (_, c) in files.iter_mut() {
            c.remove(0);
        }
    }
    let mut plan = Plan::default();
    for (src, comps) in files {
        if comps.iter().any(|c| c == ".." || c == "." || c.is_empty()) {
            plan.ignored.push(comps.join("/"));
            continue;
        }
        match map_path(&comps) {
            Some((dest, part)) => plan.files.push(PlannedFile { src, dest, part }),
            None => plan.ignored.push(comps.join("/")),
        }
    }
    plan.files.sort_by(|a, b| a.dest.cmp(&b.dest));
    if plan.files.is_empty() {
        bail!("no BepInEx/plugins or user/mods files in the archive (unusual layout; install it by hand from the mod page)");
    }
    Ok(plan)
}

/// Copies the planned files (optionally without the server part) into the SPT
/// directory. Returns the installed destinations.
pub fn apply(root: &Path, plan: &Plan, with_server: bool) -> Result<Vec<String>> {
    let mut done = Vec::new();
    for f in &plan.files {
        if f.part == Part::Server && !with_server {
            continue;
        }
        let dest = root.join(&f.dest);
        if let Some(dir) = dest.parent() {
            fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
        }
        // Replace instead of writing through: never follow a symlink at the destination
        if fs::symlink_metadata(&dest).map(|m| !m.is_dir()).unwrap_or(false) {
            fs::remove_file(&dest)?;
        }
        fs::copy(&f.src, &dest).with_context(|| format!("copy {}", f.dest))?;
        done.push(f.dest.clone());
    }
    Ok(done)
}

/// Extracts an archive with 7-Zip (zip, 7z, tar.*; RAR needs an unfree 7-Zip)
pub fn extract(archive: &Path, dest: &Path) -> Result<()> {
    if dest.exists() {
        fs::remove_dir_all(dest)?;
    }
    fs::create_dir_all(dest)?;
    let out = Command::new("7zz")
        .arg("x")
        .arg("-y")
        .arg("-bso0")
        .arg("-bsp0")
        .arg(format!("-o{}", dest.display()))
        .arg(archive)
        .output()
        .context("7zz not found (it ships with the spt-launcher-qt package)")?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        let err = err.lines().rfind(|l| !l.trim().is_empty()).unwrap_or("unknown error").trim().to_string();
        if err.to_lowercase().contains("unsupported") && archive.to_string_lossy().to_lowercase().ends_with(".rar") {
            bail!("RAR archives are not supported by the bundled 7-Zip; install this mod by hand");
        }
        bail!("extracting failed: {err}");
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Manifest
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct InstalledMod {
    pub id: u64,
    pub guid: String,
    pub name: String,
    pub version: String,
    pub page_url: String,
    /// Installed files, relative to the SPT root
    pub files: Vec<String>,
    /// The archive had a server part that was not installed (client-only mode)
    pub server_skipped: bool,
    /// Installed automatically as a dependency of these mods
    pub required_by: Vec<u64>,
    pub installed_at: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Manifest {
    pub mods: Vec<InstalledMod>,
}

pub fn manifest_path(install: &SptInstall) -> PathBuf {
    install.runtime_dir().join("user/Launcher/spt-launcher-qt-mods.json")
}

impl Manifest {
    pub fn load(install: &SptInstall) -> Self {
        fs::read_to_string(manifest_path(install)).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
    }

    pub fn save(&self, install: &SptInstall) -> Result<()> {
        let path = manifest_path(install);
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        write_atomic(&path, serde_json::to_string_pretty(self)?.as_bytes())
    }

    pub fn get(&self, id: u64) -> Option<&InstalledMod> {
        self.mods.iter().find(|m| m.id == id)
    }

    /// Adds or replaces an entry (keeps who required an existing dependency)
    pub fn upsert(&mut self, mut m: InstalledMod) {
        m.installed_at = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        if let Some(old) = self.mods.iter_mut().find(|o| o.id == m.id) {
            for r in &old.required_by {
                if !m.required_by.contains(r) {
                    m.required_by.push(*r);
                }
            }
            *old = m;
        } else {
            self.mods.push(m);
        }
        self.mods.sort_by_key(|m| m.name.to_lowercase());
    }

    /// Files of `id` that no other managed mod also installed
    fn exclusive_files(&self, id: u64) -> Vec<String> {
        let others: BTreeSet<&String> = self.mods.iter().filter(|m| m.id != id).flat_map(|m| m.files.iter()).collect();
        self.get(id).map(|m| m.files.iter().filter(|f| !others.contains(f)).cloned().collect()).unwrap_or_default()
    }

    /// Mods that list `id` as a requirement
    pub fn dependents(&self, id: u64) -> Vec<&InstalledMod> {
        self.mods.iter().filter(|m| m.id != id && self.get(id).is_some_and(|d| d.required_by.contains(&m.id))).collect()
    }
}

/// Deletes the files a managed mod installed and drops it from the manifest
pub fn remove_files(root: &Path, manifest: &mut Manifest, id: u64) -> Result<usize> {
    let files = manifest.exclusive_files(id);
    let mut removed = 0;
    for rel in &files {
        let path = root.join(rel);
        if fs::symlink_metadata(&path).map(|m| !m.is_dir()).unwrap_or(false) {
            fs::remove_file(&path).with_context(|| format!("remove {rel}"))?;
            removed += 1;
        }
        prune_dirs(root, rel);
    }
    manifest.mods.retain(|m| m.id != id);
    for m in manifest.mods.iter_mut() {
        m.required_by.retain(|r| *r != id);
    }
    Ok(removed)
}

/// Removes empty parent folders up to (not including) the mod roots
fn prune_dirs(root: &Path, rel: &str) {
    let stops = ["BepInEx/plugins", "BepInEx/patchers", "BepInEx/config", SERVER_MODS];
    let mut cur = Path::new(rel).parent();
    while let Some(dir) = cur {
        let s = dir.to_string_lossy();
        if s.is_empty() || stops.iter().any(|st| s == *st) || !stops.iter().any(|st| s.starts_with(st)) {
            break;
        }
        if fs::remove_dir(root.join(dir)).is_err() {
            break; // not empty
        }
        cur = dir.parent();
    }
}

/// Entries in BepInEx/plugins and user/mods that the launcher did not install
pub fn unmanaged(install: &SptInstall, manifest: &Manifest) -> Vec<String> {
    let owned: BTreeSet<String> = manifest
        .mods
        .iter()
        .flat_map(|m| m.files.iter())
        .filter_map(|f| {
            let c: Vec<&str> = f.split('/').collect();
            match c.as_slice() {
                ["BepInEx", "plugins", name, ..] => Some(format!("BepInEx/plugins/{name}")),
                ["SPT_Runtime", "user", "mods", name, ..] => Some(format!("{SERVER_MODS}/{name}")),
                _ => None,
            }
        })
        .collect();
    let mut out = Vec::new();
    for base in ["BepInEx/plugins", SERVER_MODS] {
        let Ok(rd) = fs::read_dir(install.root.join(base)) else { continue };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            let key = format!("{base}/{name}");
            if name == "spt" || owned.contains(&key) {
                continue;
            }
            out.push(key);
        }
    }
    out.sort_by_key(|s| s.to_lowercase());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("spt-mods-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    fn touch(root: &Path, rel: &str) {
        let p = root.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, rel).unwrap();
    }

    fn dests(p: &Plan) -> Vec<(&str, Part)> {
        p.files.iter().map(|f| (f.dest.as_str(), f.part)).collect()
    }

    #[test]
    fn standard_layout_with_both_parts() {
        let d = tmp("std");
        touch(&d, "BepInEx/plugins/SAIN/SAIN.dll");
        touch(&d, "SPT_Runtime/user/mods/SAIN/SAIN.Server.dll");
        touch(&d, "README.md");
        let p = plan(&d).unwrap();
        assert_eq!(
            dests(&p),
            vec![("BepInEx/plugins/SAIN/SAIN.dll", Part::Client), ("SPT_Runtime/user/mods/SAIN/SAIN.Server.dll", Part::Server)]
        );
        assert_eq!(p.ignored, vec!["README.md"]);
    }

    #[test]
    fn wrapper_folder_and_lowercase() {
        let d = tmp("wrap");
        touch(&d, "MyMod-1.0/bepinex/Plugins/MyMod.dll");
        touch(&d, "MyMod-1.0/SPT/user/mods/mymod/package.json");
        let p = plan(&d).unwrap();
        assert_eq!(
            dests(&p),
            vec![("BepInEx/plugins/MyMod.dll", Part::Client), ("SPT_Runtime/user/mods/mymod/package.json", Part::Server)]
        );
    }

    #[test]
    fn windows_backslash_paths() {
        let d = tmp("backslash");
        touch(&d, "BepInEx\\plugins\\BetterBepInEx.dll");
        assert_eq!(dests(&plan(&d).unwrap()), vec![("BepInEx/plugins/BetterBepInEx.dll", Part::Client)]);
        let w = tmp("backslash-wrap");
        touch(&w, "Mod-1.0\\BepInEx\\config\\x.cfg");
        touch(&w, "Mod-1.0\\BepInEx\\plugins\\X.dll");
        assert_eq!(
            dests(&plan(&w).unwrap()),
            vec![("BepInEx/config/x.cfg", Part::Client), ("BepInEx/plugins/X.dll", Part::Client)]
        );
    }

    #[test]
    fn bare_dll_and_plugins_root() {
        let d = tmp("bare");
        touch(&d, "Fov.dll");
        touch(&d, "plugins/Other/Other.dll");
        let p = plan(&d).unwrap();
        assert_eq!(dests(&p), vec![("BepInEx/plugins/Fov.dll", Part::Client), ("BepInEx/plugins/Other/Other.dll", Part::Client)]);
    }

    #[test]
    fn refuses_unknown_layout_and_game_files() {
        let d = tmp("bad");
        touch(&d, "EscapeFromTarkov_Data/Managed/Assembly-CSharp.dll");
        touch(&d, "BepInEx/core/BepInEx.dll");
        assert!(plan(&d).is_err());
    }

    #[test]
    fn apply_remove_roundtrip() {
        let src = tmp("rt-src");
        let root = tmp("rt-root");
        touch(&src, "BepInEx/plugins/A/A.dll");
        touch(&src, "BepInEx/plugins/A/res/x.png");
        touch(&src, "SPT_Runtime/user/mods/A/mod.dll");
        fs::create_dir_all(root.join("BepInEx/plugins/spt")).unwrap();
        let p = plan(&src).unwrap();
        let files = apply(&root, &p, false).unwrap();
        assert_eq!(files.len(), 2);
        assert!(!root.join("SPT_Runtime/user/mods/A").exists());

        let install = SptInstall::new(&root);
        let mut m = Manifest::default();
        m.upsert(InstalledMod { id: 1, name: "A".into(), files, server_skipped: true, ..Default::default() });
        touch(&root, "BepInEx/plugins/Manual.dll");
        assert_eq!(unmanaged(&install, &m), vec!["BepInEx/plugins/Manual.dll"]);

        assert_eq!(remove_files(&root, &mut m, 1).unwrap(), 2);
        assert!(!root.join("BepInEx/plugins/A").exists());
        assert!(root.join("BepInEx/plugins/spt").is_dir());
        assert!(m.mods.is_empty());
    }

    #[test]
    fn shared_files_survive_removal() {
        let root = tmp("shared");
        touch(&root, "BepInEx/plugins/Lib.dll");
        let mut m = Manifest::default();
        m.upsert(InstalledMod { id: 1, files: vec!["BepInEx/plugins/Lib.dll".into()], ..Default::default() });
        m.upsert(InstalledMod { id: 2, files: vec!["BepInEx/plugins/Lib.dll".into()], ..Default::default() });
        remove_files(&root, &mut m, 1).unwrap();
        assert!(root.join("BepInEx/plugins/Lib.dll").exists());
    }
}
