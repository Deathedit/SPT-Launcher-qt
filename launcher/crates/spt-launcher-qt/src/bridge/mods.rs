//! Mods page: sp-mod.com search, install / update / remove (logic in spt_core::forge / mods).

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use cxx_qt::{CxxQtThread, CxxQtType, Threading};
use cxx_qt_lib::QString;
use serde_json::json;
use spt_core::bundles::format_bytes;
use spt_core::config::SptInstall;
use spt_core::forge::{self, Forge, ForgeMod, ForgeVersion};
use spt_core::mods::{self, InstalledMod, Manifest, Part};

use super::launcher::qobject::Launcher;

type L = Launcher;
type Qt = CxxQtThread<L>;

#[derive(Default)]
pub struct ModsState {
    search_gen: u64,
    busy: bool,
    results: Vec<serde_json::Value>,
    /// mod id -> newer compatible version
    updates: BTreeMap<u64, ForgeVersion>,
}

pub type Shared = Arc<Mutex<ModsState>>;

fn q(s: &str) -> QString {
    QString::from(s)
}

fn log(qt: &Qt, level: &str, text: impl Into<String>) {
    let (level, text) = (level.to_string(), text.into());
    let _ = qt.queue(move |o| o.log_line(q("mods"), q(&level), q(&text)));
}

fn toast(o: Pin<&mut L>, kind: &str, text: &str) {
    o.toast(q(kind), q(text));
}

fn install_of(o: &L) -> SptInstall {
    SptInstall::new(PathBuf::from(o.spt_path().to_string()))
}

fn cache_dir() -> PathBuf {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".cache"))
        .join("spt-launcher-qt/mods")
}

fn result_json(m: &ForgeMod) -> serde_json::Value {
    json!({
        "id": m.id,
        "name": m.name,
        "author": m.author(),
        "teaser": m.teaser,
        "thumbnail": m.thumbnail,
        "downloads": m.downloads,
        "url": m.page_url(),
        "fika": m.fika_compatibility,
    })
}

/// Installed (managed) and other mods, from disk
pub fn reload_local(mut o: Pin<&mut L>) {
    let install = install_of(&o);
    let manifest = Manifest::load(&install);
    let updates = o.rust().mods.lock().unwrap().updates.clone();
    let name_of = |id: &u64| manifest.get(*id).map(|m| m.name.clone()).unwrap_or_else(|| format!("#{id}"));
    let installed: Vec<_> = manifest
        .mods
        .iter()
        .map(|m| {
            json!({
                "id": m.id,
                "name": m.name,
                "version": m.version,
                "url": m.page_url,
                "files": m.files.len(),
                "serverSkipped": m.server_skipped,
                "requiredBy": m.required_by.iter().map(name_of).collect::<Vec<_>>(),
                "update": updates.get(&m.id).map(|v| v.version.clone()).unwrap_or_default(),
            })
        })
        .collect();
    let other = if install.is_installed() { mods::unmanaged(&install, &manifest) } else { Vec::new() };
    o.as_mut().set_installed_mods(q(&serde_json::to_string(&installed).unwrap_or_default()));
    o.as_mut().set_other_mods(q(&serde_json::to_string(&other).unwrap_or_default()));
}

/// Reloads the local lists and asks sp-mod.com for updates
pub fn reload(mut o: Pin<&mut L>) {
    reload_local(o.as_mut());
    let install = install_of(&o);
    let spt = o.spt_version().to_string();
    let installed: Vec<(u64, String)> = Manifest::load(&install).mods.iter().map(|m| (m.id, m.version.clone())).collect();
    if installed.is_empty() || spt.is_empty() {
        return;
    }
    let qt = o.qt_thread();
    std::thread::spawn(move || match Forge::new().updates(&installed, &spt) {
        Ok(list) => {
            let n = list.len();
            let _ = qt.queue(move |mut o| {
                o.rust().mods.lock().unwrap().updates = list.into_iter().map(|u| (u.mod_id, u.version)).collect();
                reload_local(o.as_mut());
            });
            if n > 0 {
                log(&qt, "ok", format!("{n} mod update(s) available"));
            }
        }
        Err(e) => log(&qt, "warn", format!("Update check failed: {e:#}")),
    });
}

pub fn search(mut o: Pin<&mut L>, query: String, page: i32) {
    let spt = o.spt_version().to_string();
    let page = page.max(1) as u32;
    let gen = {
        let shared = o.rust().mods.clone();
        let mut st = shared.lock().unwrap();
        st.search_gen += 1;
        if page == 1 {
            st.results.clear();
        }
        st.search_gen
    };
    o.as_mut().set_forge_state(q("searching"));
    o.as_mut().set_forge_error(q(""));
    let qt = o.qt_thread();
    std::thread::spawn(move || {
        let forge = Forge::new();
        let result = match forge::parse_mod_ref(&query) {
            Some(id) => forge.get_mod(id).map(|m| (vec![m], false)),
            None => forge.search(&query, &spt, page).map(|p| (p.mods, p.has_more)),
        };
        let _ = qt.queue(move |mut o| {
            let shared = o.rust().mods.clone();
            let mut st = shared.lock().unwrap();
            if st.search_gen != gen {
                return; // a newer search is running
            }
            match result {
                Ok((list, more)) => {
                    st.results.extend(list.iter().map(result_json));
                    let json = serde_json::to_string(&st.results).unwrap_or_default();
                    drop(st);
                    o.as_mut().set_forge_results(q(&json));
                    o.as_mut().set_forge_has_more(more);
                    o.as_mut().set_forge_state(q("idle"));
                }
                Err(e) => {
                    drop(st);
                    o.as_mut().set_forge_error(q(&format!("{e:#}")));
                    o.as_mut().set_forge_state(q("error"));
                }
            }
        });
    });
}

/// Common guards for install / remove
fn begin(o: &mut Pin<&mut L>, task: &str) -> bool {
    if !*o.installed() {
        toast(o.as_mut(), "warn", "Install SPT first");
        return false;
    }
    if o.game_state().to_string() != "idle" {
        toast(o.as_mut(), "warn", "Close the game before changing mods");
        return false;
    }
    {
        let shared = o.rust().mods.clone();
        let mut st = shared.lock().unwrap();
        if st.busy {
            drop(st);
            toast(o.as_mut(), "warn", "Another mod operation is running");
            return false;
        }
        st.busy = true;
    }
    o.as_mut().set_mod_task(q(task));
    o.as_mut().set_mod_pct(-1.0);
    true
}

fn finish(qt: &Qt) {
    let _ = qt.queue(|mut o| {
        o.rust().mods.lock().unwrap().busy = false;
        o.as_mut().set_mod_task(q(""));
        o.as_mut().set_mod_pct(-1.0);
        reload_local(o.as_mut());
    });
}

struct Job {
    id: u64,
    guid: String,
    name: String,
    page_url: String,
    version: ForgeVersion,
    required_by: Option<u64>,
}

pub fn install(mut o: Pin<&mut L>, id: u64) {
    if !begin(&mut o, "Resolving mod…") {
        return;
    }
    let install = install_of(&o);
    let spt = o.spt_version().to_string();
    let with_server = *o.local_enabled();
    let qt = o.qt_thread();
    std::thread::spawn(move || {
        if let Err(e) = run_install(&qt, &install, &spt, with_server, id) {
            log(&qt, "error", format!("{e:#}"));
            let msg = format!("{e:#}");
            let _ = qt.queue(move |o| toast(o, "error", &msg));
        }
        finish(&qt);
    });
}

fn task(qt: &Qt, text: String, pct: f64) {
    let _ = qt.queue(move |mut o| {
        o.as_mut().set_mod_task(q(&text));
        o.as_mut().set_mod_pct(pct);
    });
}

fn run_install(qt: &Qt, install: &SptInstall, spt: &str, with_server: bool, id: u64) -> anyhow::Result<()> {
    let forge = Forge::new();
    let m = forge.get_mod(id)?;
    let v = forge.latest_version(id, spt)?.ok_or_else(|| {
        anyhow::anyhow!("{} has no version for SPT {}", m.name, if spt.is_empty() { "?" } else { spt })
    })?;
    log(qt, "cmd", format!("Install {} {} by {} ({})", m.name, v.version, m.author(), m.page_url()));
    for l in &v.virus_total_links {
        log(qt, "info", format!("VirusTotal ({}): {}", if l.label.is_empty() { "scan" } else { &l.label }, l.url));
    }

    let mut manifest = Manifest::load(install);
    let mut jobs = Vec::new();
    if !spt.is_empty() {
        match forge.dependencies(id, &v.version, spt) {
            Ok(deps) => {
                for d in deps {
                    if d.conflict {
                        log(qt, "warn", format!("Dependency {} conflicts with another requirement", d.name));
                    }
                    let Some(dv) = d.latest_compatible_version else {
                        log(qt, "warn", format!("Dependency {} has no version for SPT {spt}; install it by hand", d.name));
                        continue;
                    };
                    if manifest.get(d.id).is_some() {
                        log(qt, "info", format!("Dependency {} already installed", d.name));
                        continue;
                    }
                    log(qt, "info", format!("Needs {} {}", d.name, dv.version));
                    let page_url = format!("{}/mod/{}/{}", forge::SITE, d.id, d.slug);
                    jobs.push(Job { id: d.id, guid: d.guid, name: d.name, page_url, version: dv, required_by: Some(id) });
                }
            }
            Err(e) => log(qt, "warn", format!("Could not resolve dependencies: {e:#}")),
        }
    }
    jobs.push(Job { id, guid: m.guid.clone(), name: m.name.clone(), page_url: m.page_url(), version: v, required_by: None });

    for job in jobs {
        install_one(qt, install, &forge, &mut manifest, with_server, &job)?;
    }
    Ok(())
}

fn install_one(
    qt: &Qt,
    install: &SptInstall,
    forge: &Forge,
    manifest: &mut Manifest,
    with_server: bool,
    job: &Job,
) -> anyhow::Result<()> {
    let label = format!("{} {}", job.name, job.version.version);
    let work = cache_dir().join(format!("{}-{}", job.id, job.version.id));
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work)?;
    let result = (|| -> anyhow::Result<()> {
        task(qt, format!("Downloading {label}"), 0.0);
        let partial = work.join("download");
        let mut last = -1i64;
        let suggested = forge.download(&job.version.link, &partial, |done, total| {
            let total = if total > 0 { total } else { job.version.content_length };
            if total > 0 {
                let pct = (done as f64 * 100.0 / total as f64).min(100.0);
                if pct as i64 != last {
                    last = pct as i64;
                    task(qt, format!("Downloading {label} · {} / {}", format_bytes(done as f64), format_bytes(total as f64)), pct);
                }
            }
        })?;
        // Keep the host's file name: 7-Zip picks the format from the extension first
        let name = std::path::Path::new(&suggested).file_name().map(|n| n.to_string_lossy().into_owned());
        let archive = work.join(name.filter(|n| !n.is_empty() && !n.starts_with('.')).unwrap_or_else(|| "mod.zip".into()));
        std::fs::rename(&partial, &archive)?;

        task(qt, format!("Extracting {label}"), -1.0);
        let extracted = work.join("x");
        mods::extract(&archive, &extracted)?;
        let plan = mods::plan(&extracted)?;
        let (client, server) = (plan.count(Part::Client), plan.count(Part::Server));
        if client == 0 && !with_server {
            if job.required_by.is_some() {
                log(qt, "warn", format!("{} is server-only; skipped (the server host needs it)", job.name));
                return Ok(());
            }
            anyhow::bail!(
                "{} is a server mod. Install it on the server host, or enable the local server in Settings",
                job.name
            );
        }

        task(qt, format!("Installing {label}"), -1.0);
        if manifest.get(job.id).is_some() {
            mods::remove_files(&install.root, manifest, job.id)?;
        }
        let files = mods::apply(&install.root, &plan, with_server)?;
        let n = files.len();
        manifest.upsert(InstalledMod {
            id: job.id,
            guid: job.guid.clone(),
            name: job.name.clone(),
            version: job.version.version.clone(),
            page_url: job.page_url.clone(),
            files,
            server_skipped: server > 0 && !with_server,
            required_by: job.required_by.into_iter().collect(),
            installed_at: 0,
        });
        manifest.save(install)?;
        log(qt, "ok", format!("Installed {label} ({n} files)"));
        if server > 0 && !with_server {
            log(qt, "warn", format!("{}: server part skipped (client-only). The server host needs it too.", job.name));
        }
        if !plan.ignored.is_empty() {
            log(qt, "info", format!("Not copied: {}", plan.ignored.join(", ")));
        }
        let id = job.id;
        let _ = qt.queue(move |o| {
            o.rust().mods.lock().unwrap().updates.remove(&id);
        });
        Ok(())
    })();
    let _ = std::fs::remove_dir_all(&work);
    result.map_err(|e| e.context(label))
}

pub fn remove(mut o: Pin<&mut L>, id: u64) {
    if !begin(&mut o, "Removing mod…") {
        return;
    }
    let install = install_of(&o);
    let mut manifest = Manifest::load(&install);
    let name = manifest.get(id).map(|m| m.name.clone()).unwrap_or_else(|| format!("#{id}"));
    let dependents: Vec<String> = manifest.dependents(id).iter().map(|m| m.name.clone()).collect();
    let result = mods::remove_files(&install.root, &mut manifest, id).and_then(|n| manifest.save(&install).map(|_| n));
    match result {
        Ok(n) => {
            o.as_mut().log_line(q("mods"), q("ok"), q(&format!("Removed {name} ({n} files)")));
            if !dependents.is_empty() {
                o.as_mut().log_line(q("mods"), q("warn"), q(&format!("{name} was required by {}", dependents.join(", "))));
            }
        }
        Err(e) => {
            o.as_mut().log_line(q("mods"), q("error"), q(&format!("Removing {name} failed: {e:#}")));
            toast(o.as_mut(), "error", &format!("Removing {name} failed"));
        }
    }
    o.rust().mods.lock().unwrap().updates.remove(&id);
    o.rust().mods.lock().unwrap().busy = false;
    o.as_mut().set_mod_task(q(""));
    reload_local(o);
}
