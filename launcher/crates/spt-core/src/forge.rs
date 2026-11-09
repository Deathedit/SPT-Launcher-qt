//! Client for the public sp-mod.com ("The Forge") API: search mods, pick the
//! version that matches the installed SPT, resolve dependencies, check for
//! updates and download archives.
//!
//! The API is read-only and needs no key (https://sp-mod.com/developers).
//! Download links redirect to wherever the author hosts the file (mostly
//! GitHub releases).

use std::io::{Read, Write};
use std::path::Path;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize};
use ureq::tls::{TlsConfig, TlsProvider};
use ureq::Agent;

pub const SITE: &str = "https://sp-mod.com";
pub const API: &str = "https://sp-mod.com/api/v0";
const USER_AGENT: &str =
    concat!("spt-launcher-qt/", env!("CARGO_PKG_VERSION"), " (+https://github.com/Deathedit/SPT-Launcher-qt)");

/// `null` in the API becomes the type's default
fn nd<'de, D: Deserializer<'de>, T: Default + Deserialize<'de>>(d: D) -> Result<T, D::Error> {
    Ok(Option::<T>::deserialize(d)?.unwrap_or_default())
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Owner {
    #[serde(deserialize_with = "nd")]
    pub name: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ForgeMod {
    pub id: u64,
    #[serde(deserialize_with = "nd")]
    pub guid: String,
    #[serde(deserialize_with = "nd")]
    pub name: String,
    #[serde(deserialize_with = "nd")]
    pub slug: String,
    #[serde(deserialize_with = "nd")]
    pub teaser: String,
    #[serde(deserialize_with = "nd")]
    pub thumbnail: String,
    #[serde(deserialize_with = "nd")]
    pub downloads: u64,
    #[serde(deserialize_with = "nd")]
    pub detail_url: String,
    #[serde(deserialize_with = "nd")]
    pub fika_compatibility: bool,
    pub owner: Option<Owner>,
}

impl ForgeMod {
    pub fn author(&self) -> &str {
        self.owner.as_ref().map(|o| o.name.as_str()).unwrap_or("")
    }

    pub fn page_url(&self) -> String {
        if self.detail_url.is_empty() {
            format!("{SITE}/mod/{}/{}", self.id, self.slug)
        } else {
            self.detail_url.clone()
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct VirusTotalLink {
    #[serde(deserialize_with = "nd")]
    pub url: String,
    #[serde(deserialize_with = "nd")]
    pub label: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ForgeVersion {
    pub id: u64,
    #[serde(deserialize_with = "nd")]
    pub version: String,
    #[serde(deserialize_with = "nd")]
    pub link: String,
    #[serde(deserialize_with = "nd")]
    pub content_length: u64,
    #[serde(deserialize_with = "nd")]
    pub spt_version_constraint: String,
    #[serde(deserialize_with = "nd")]
    pub fika_compatibility: String,
    #[serde(deserialize_with = "nd")]
    pub virus_total_links: Vec<VirusTotalLink>,
}

/// One entry of the `/mods/dependencies` tree
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Dependency {
    pub id: u64,
    #[serde(deserialize_with = "nd")]
    pub guid: String,
    #[serde(deserialize_with = "nd")]
    pub name: String,
    #[serde(deserialize_with = "nd")]
    pub slug: String,
    pub latest_compatible_version: Option<ForgeVersion>,
    #[serde(deserialize_with = "nd")]
    pub conflict: bool,
    #[serde(deserialize_with = "nd")]
    pub dependencies: Vec<Dependency>,
}

/// A newer version of an installed mod (`/mods/updates`)
#[derive(Debug, Clone)]
pub struct Update {
    pub mod_id: u64,
    pub version: ForgeVersion,
}

#[derive(Deserialize)]
struct Envelope<T> {
    #[serde(default)]
    success: bool,
    data: Option<T>,
    #[serde(default)]
    message: Option<String>,
}

#[derive(Deserialize)]
struct UpdatesData {
    #[serde(default)]
    updates: Vec<UpdateEntry>,
}

#[derive(Deserialize)]
struct UpdateEntry {
    current_version: UpdateCurrent,
    recommended_version: ForgeVersion,
}

#[derive(Deserialize)]
struct UpdateCurrent {
    mod_id: u64,
}

pub struct SearchPage {
    pub mods: Vec<ForgeMod>,
    pub has_more: bool,
}

#[derive(Deserialize)]
struct Paged<T> {
    #[serde(default)]
    data: Vec<T>,
    #[serde(default)]
    meta: Option<PageMeta>,
}

#[derive(Deserialize)]
struct PageMeta {
    #[serde(default)]
    current_page: u32,
    #[serde(default)]
    last_page: u32,
}

#[derive(Clone)]
pub struct Forge {
    agent: Agent,
    download_agent: Agent,
}

impl Default for Forge {
    fn default() -> Self {
        Self::new()
    }
}

fn make_agent(timeout: Option<Duration>) -> Agent {
    Agent::config_builder()
        .tls_config(TlsConfig::builder().provider(TlsProvider::Rustls).build())
        .http_status_as_error(false)
        .timeout_connect(Some(Duration::from_secs(10)))
        .timeout_global(timeout)
        .max_redirects(10)
        .build()
        .into()
}

/// The API returns descriptions with raw control characters inside strings,
/// which strict JSON parsers reject. Outside strings they are whitespace anyway.
fn sanitize_json(body: &str) -> String {
    body.chars().map(|c| if c < ' ' { ' ' } else { c }).collect()
}

impl Forge {
    pub fn new() -> Self {
        Self { agent: make_agent(Some(Duration::from_secs(20))), download_agent: make_agent(None) }
    }

    fn get_raw(&self, path: &str, query: &[(&str, &str)]) -> Result<String> {
        let mut req = self
            .agent
            .get(&format!("{API}{path}"))
            .header("User-Agent", USER_AGENT)
            .header("Accept", "application/json");
        for (k, v) in query {
            req = req.query(*k, *v);
        }
        let resp = req.call().map_err(|e| anyhow!("sp-mod.com{path}: {e}"))?;
        let status = resp.status();
        let body = resp.into_body().read_to_string().with_context(|| format!("sp-mod.com{path}"))?;
        let body = sanitize_json(&body);
        if !status.is_success() {
            let msg = serde_json::from_str::<Envelope<serde_json::Value>>(&body).ok().and_then(|e| e.message);
            bail!("sp-mod.com{path}: HTTP {status}{}", msg.map(|m| format!(" ({m})")).unwrap_or_default());
        }
        Ok(body)
    }

    fn get<T: DeserializeOwned>(&self, path: &str, query: &[(&str, &str)]) -> Result<T> {
        let body = self.get_raw(path, query)?;
        let env: Envelope<T> = serde_json::from_str(&body).with_context(|| format!("sp-mod.com{path}: bad JSON"))?;
        if !env.success {
            bail!("sp-mod.com{path}: {}", env.message.unwrap_or_else(|| "request failed".into()));
        }
        env.data.ok_or_else(|| anyhow!("sp-mod.com{path}: empty response"))
    }

    /// Mods compatible with `spt_version` (empty = any). Without a query the most
    /// downloaded mods come first, with a query the search relevance decides.
    pub fn search(&self, query: &str, spt_version: &str, page: u32) -> Result<SearchPage> {
        let page = page.max(1).to_string();
        let mut q: Vec<(&str, &str)> = vec![("per_page", "30"), ("page", &page)];
        if !query.trim().is_empty() {
            q.push(("query", query.trim()));
        } else {
            q.push(("sort", "-downloads"));
        }
        if !spt_version.is_empty() {
            q.push(("filter[spt_version]", spt_version));
        }
        let body = self.get_raw("/mods", &q)?;
        let paged: Paged<ForgeMod> = serde_json::from_str(&body).context("sp-mod.com/mods: bad JSON")?;
        let has_more = paged.meta.map(|m| m.current_page < m.last_page).unwrap_or(false);
        Ok(SearchPage { mods: paged.data, has_more })
    }

    pub fn get_mod(&self, id: u64) -> Result<ForgeMod> {
        self.get(&format!("/mod/{id}"), &[])
    }

    /// Newest version that supports `spt_version` (empty = newest overall)
    pub fn latest_version(&self, id: u64, spt_version: &str) -> Result<Option<ForgeVersion>> {
        let mut q: Vec<(&str, &str)> = vec![("sort", "-version"), ("per_page", "1"), ("include", "virus_total_links")];
        if !spt_version.is_empty() {
            q.push(("filter[spt_version]", spt_version));
        }
        let body = self.get_raw(&format!("/mod/{id}/versions"), &q)?;
        let paged: Paged<ForgeVersion> = serde_json::from_str(&body).context("sp-mod.com versions: bad JSON")?;
        Ok(paged.data.into_iter().next())
    }

    /// Every mod `id@version` needs, flattened (dependencies of dependencies
    /// first), without duplicates
    pub fn dependencies(&self, id: u64, version: &str, spt_version: &str) -> Result<Vec<Dependency>> {
        let key = format!("{id}:{version}");
        let data: std::collections::BTreeMap<String, Vec<Dependency>> =
            self.get("/mods/dependencies", &[("mods", &key), ("spt_version", spt_version)])?;
        let mut out: Vec<Dependency> = Vec::new();
        fn walk(list: Vec<Dependency>, out: &mut Vec<Dependency>) {
            for mut d in list {
                walk(std::mem::take(&mut d.dependencies), out);
                if !out.iter().any(|o| o.id == d.id) {
                    out.push(d);
                }
            }
        }
        walk(data.into_values().flatten().collect(), &mut out);
        Ok(out)
    }

    /// Newer versions for installed `(mod id, version)` pairs
    pub fn updates(&self, installed: &[(u64, String)], spt_version: &str) -> Result<Vec<Update>> {
        if installed.is_empty() || spt_version.is_empty() {
            return Ok(Vec::new());
        }
        let mods = installed.iter().map(|(id, v)| format!("{id}:{v}")).collect::<Vec<_>>().join(",");
        let data: UpdatesData = self.get("/mods/updates", &[("mods", &mods), ("spt_version", spt_version)])?;
        Ok(data
            .updates
            .into_iter()
            .map(|u| Update { mod_id: u.current_version.mod_id, version: u.recommended_version })
            .collect())
    }

    /// Downloads a version archive to `dest`. Returns the file name the host
    /// suggested (Content-Disposition), which carries the archive extension.
    pub fn download(&self, url: &str, dest: &Path, mut progress: impl FnMut(u64, u64)) -> Result<String> {
        let resp = self
            .download_agent
            .get(url)
            .header("User-Agent", USER_AGENT)
            .call()
            .map_err(|e| anyhow!("download {url}: {e}"))?;
        if !resp.status().is_success() {
            bail!("download failed: HTTP {}", resp.status());
        }
        let header = |name: &str| resp.headers().get(name).and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
        let content_type = header("content-type");
        if content_type.starts_with("text/html") {
            bail!("the mod is hosted on a site that needs a browser to download; open the mod page instead");
        }
        let file_name = content_disposition_name(&header("content-disposition")).unwrap_or_default();
        let total: u64 = header("content-length").parse().unwrap_or(0);

        if let Some(dir) = dest.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut reader = resp.into_body().into_with_config().limit(u64::MAX).reader();
        let mut file = std::fs::File::create(dest)?;
        let mut buf = vec![0u8; 128 * 1024];
        let mut written = 0u64;
        loop {
            let n = reader.read(&mut buf)?;
            if n == 0 {
                break;
            }
            file.write_all(&buf[..n])?;
            written += n as u64;
            progress(written, total);
        }
        file.sync_all()?;
        Ok(file_name)
    }
}

/// `attachment; filename="X.zip"` / `filename*=UTF-8''X.zip` → `X.zip`
pub fn content_disposition_name(value: &str) -> Option<String> {
    let mut plain = None;
    for part in value.split(';').map(str::trim) {
        if let Some(v) = part.strip_prefix("filename*=") {
            let v = v.rsplit("''").next().unwrap_or(v);
            return Some(v.trim_matches('"').to_string()).filter(|s| !s.is_empty());
        }
        if let Some(v) = part.strip_prefix("filename=") {
            plain = Some(v.trim_matches('"').to_string()).filter(|s| !s.is_empty());
        }
    }
    plain
}

/// Mod id from `1775`, `https://sp-mod.com/mod/1775/wtt-menu-overhaul` or
/// `forge.sp-mod.com/mod/1775`
pub fn parse_mod_ref(input: &str) -> Option<u64> {
    let s = input.trim();
    if !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) {
        return s.parse().ok();
    }
    if !s.contains("sp-mod.com") {
        return None;
    }
    let rest = &s[s.find("/mod/")? + 5..];
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mod_refs() {
        assert_eq!(parse_mod_ref("1775"), Some(1775));
        assert_eq!(parse_mod_ref(" https://sp-mod.com/mod/1775/wtt-menu-overhaul "), Some(1775));
        assert_eq!(parse_mod_ref("forge.sp-mod.com/mod/42"), Some(42));
        assert_eq!(parse_mod_ref("sp-mod.com/mod/download/1775/x/1.3.0"), None);
        assert_eq!(parse_mod_ref("menu overhaul"), None);
    }

    #[test]
    fn disposition() {
        assert_eq!(content_disposition_name("attachment; filename=MoxoPixel-MenuOverhaul.zip").as_deref(), Some("MoxoPixel-MenuOverhaul.zip"));
        assert_eq!(content_disposition_name("attachment; filename=\"a b.7z\"").as_deref(), Some("a b.7z"));
        assert_eq!(content_disposition_name("attachment; filename=x; filename*=UTF-8''y.zip").as_deref(), Some("y.zip"));
        assert_eq!(content_disposition_name("inline"), None);
    }

    #[test]
    fn parses_api_with_nulls_and_control_chars() {
        let body = "{\"success\":true,\"data\":{\"id\":5,\"guid\":\"a.b\",\"name\":\"X\",\"slug\":\"x\",\"teaser\":null,\
                    \"thumbnail\":null,\"downloads\":3,\"detail_url\":\"u\",\"fika_compatibility\":true,\"owner\":{\"name\":\"me\"},\
                    \"description\":\"line1\nline2\"}}";
        let env: Envelope<ForgeMod> = serde_json::from_str(&sanitize_json(body)).unwrap();
        let m = env.data.unwrap();
        assert_eq!((m.id, m.teaser.as_str(), m.author()), (5, "", "me"));
    }
}
