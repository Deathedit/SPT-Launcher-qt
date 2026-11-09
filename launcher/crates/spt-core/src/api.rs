//! Client for the SPT server's launcher endpoints.
//!
//! Every JSON body is zlib-compressed in both directions. Requests with a body
//! are PUT, the rest GET. The server uses a self-signed certificate, so
//! verification is disabled (as in the official launcher).

use std::io::{Read, Write};
use std::path::Path;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use serde::de::DeserializeOwned;
use serde::Serialize;
use ureq::tls::{TlsConfig, TlsProvider};
use ureq::Agent;

use crate::models::*;

pub mod urls {
    pub const PING: &str = "/launcher/v2/ping";
    pub const TYPES: &str = "/launcher/v2/types";
    pub const LOGIN: &str = "/launcher/v2/login";
    pub const REGISTER: &str = "/launcher/v2/register";
    pub const REMOVE: &str = "/launcher/v2/remove";
    pub const VERSION: &str = "/launcher/v2/version";
    pub const MOD_PAGES: &str = "/launcher/v2/mod-pages";
    pub const PROFILES: &str = "/launcher/v2/profiles";
    pub const WIPE: &str = "/launcher/v2/wipe";
    pub const BUNDLES: &str = "/singleplayer/bundles";
    pub const BUNDLE_FILE: &str = "/files/bundle/";
}

pub const LOCAL_HOST: &str = "127.0.0.1:6969";

#[derive(Clone)]
pub struct Client {
    host: String,
    agent: Agent,
    /// Agent without a global timeout, for large downloads
    download_agent: Agent,
}

fn make_agent(timeout: Option<Duration>) -> Agent {
    let config = Agent::config_builder()
        .tls_config(TlsConfig::builder().provider(TlsProvider::Rustls).disable_verification(true).build())
        .http_status_as_error(false)
        .timeout_connect(Some(Duration::from_secs(5)))
        .timeout_global(timeout)
        .build();
    config.into()
}

impl Client {
    /// `host` is `ip:port`, without scheme
    pub fn new(host: &str) -> Self {
        Self {
            host: normalize_host(host),
            agent: make_agent(Some(Duration::from_secs(10))),
            download_agent: make_agent(None),
        }
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn backend_url(&self) -> String {
        format!("https://{}", self.host)
    }

    fn url(&self, path: &str) -> String {
        format!("https://{}{}", self.host, path)
    }

    fn read_zlib(resp: ureq::http::Response<ureq::Body>, path: &str) -> Result<String> {
        let status = resp.status();
        let raw = resp
            .into_body()
            .into_with_config()
            .limit(64 * 1024 * 1024)
            .read_to_vec()
            .with_context(|| format!("reading {path}"))?;
        if !status.is_success() {
            bail!("{path}: HTTP {status}");
        }
        let mut out = String::new();
        ZlibDecoder::new(raw.as_slice())
            .read_to_string(&mut out)
            .with_context(|| format!("{path}: response is not zlib JSON"))?;
        Ok(out)
    }

    pub fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let resp = self.agent.get(self.url(path)).call().map_err(|e| anyhow!("GET {path}: {e}"))?;
        let body = Self::read_zlib(resp, path)?;
        serde_json::from_str(&body).with_context(|| format!("{path}: unexpected JSON"))
    }

    pub fn put<T: DeserializeOwned, B: Serialize>(&self, path: &str, body: &B) -> Result<T> {
        let mut enc = ZlibEncoder::new(Vec::new(), Compression::best());
        enc.write_all(serde_json::to_string(body)?.as_bytes())?;
        let data = enc.finish()?;
        let resp = self.agent.put(self.url(path)).send(&data[..]).map_err(|e| anyhow!("PUT {path}: {e}"))?;
        let body = Self::read_zlib(resp, path)?;
        serde_json::from_str(&body).with_context(|| format!("{path}: unexpected JSON"))
    }

    pub fn ping(&self) -> Result<()> {
        let w: Wrapped<String> = self.get(urls::PING)?;
        match w.response.as_deref() {
            Some("Pong!") => Ok(()),
            other => bail!("unexpected ping answer: {other:?}"),
        }
    }

    /// Server version, e.g. `4.1.6` (a dev build adds ` - <text>`)
    pub fn version(&self) -> Result<String> {
        let w: Wrapped<String> = self.get(urls::VERSION)?;
        w.response.ok_or_else(|| anyhow!("empty version"))
    }

    pub fn profiles(&self) -> Result<Vec<MiniProfile>> {
        let w: Wrapped<Vec<MiniProfile>> = self.get(urls::PROFILES)?;
        Ok(w.response.unwrap_or_default())
    }

    pub fn editions(&self) -> Result<Editions> {
        let w: Wrapped<Editions> = self.get(urls::TYPES)?;
        Ok(w.response.unwrap_or_default())
    }

    pub fn mod_pages(&self) -> Result<Vec<ModPage>> {
        let w: Wrapped<Vec<ModPage>> = self.get(urls::MOD_PAGES)?;
        Ok(w.response.unwrap_or_default())
    }

    pub fn login(&self, username: &str) -> Result<()> {
        let w: Wrapped<bool> = self.put(urls::LOGIN, &LoginRequest { username })?;
        if w.response == Some(true) {
            Ok(())
        } else {
            bail!("the server refused the login for \"{username}\"")
        }
    }

    pub fn register(&self, username: &str, edition: &str) -> Result<Vec<MiniProfile>> {
        let r: ProfilesResult = self.put(urls::REGISTER, &RegisterRequest { username, edition })?;
        if !r.response {
            bail!("the server refused to create \"{username}\" (username taken?)");
        }
        Ok(r.profiles)
    }

    pub fn remove(&self, username: &str) -> Result<Vec<MiniProfile>> {
        let r: ProfilesResult = self.put(urls::REMOVE, &LoginRequest { username })?;
        if !r.response {
            bail!("the server refused to remove \"{username}\"");
        }
        Ok(r.profiles)
    }

    pub fn wipe(&self, username: &str, edition: &str) -> Result<Vec<MiniProfile>> {
        let r: ProfilesResult = self.put(urls::WIPE, &RegisterRequest { username, edition })?;
        if !r.response {
            bail!("the server refused to wipe \"{username}\"");
        }
        Ok(r.profiles)
    }

    pub fn bundle_manifest(&self) -> Result<Vec<BundleManifestItem>> {
        // Plain array, not wrapped in { Response }
        self.get(urls::BUNDLES)
    }

    /// Streams `/files/bundle/<name>` (not zlib-wrapped) to `dest` via a `.part` file.
    /// `progress` gets the bytes written so far.
    pub fn download_bundle(&self, file_name: &str, dest: &Path, mut progress: impl FnMut(u64)) -> Result<()> {
        let url = self.url(&format!("{}{}", urls::BUNDLE_FILE, encode_path(file_name)));
        if let Some(dir) = dest.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let part = dest.with_extension(format!(
            "{}part",
            dest.extension().map(|e| format!("{}.", e.to_string_lossy())).unwrap_or_default()
        ));
        let result = (|| -> Result<()> {
            let resp = self.download_agent.get(&url).call().map_err(|e| anyhow!("GET {url}: {e}"))?;
            if !resp.status().is_success() {
                bail!("{file_name}: HTTP {}", resp.status());
            }
            let mut reader = resp.into_body().into_with_config().limit(u64::MAX).reader();
            let mut file = std::fs::File::create(&part)?;
            let mut buf = vec![0u8; 128 * 1024];
            let mut written = 0u64;
            loop {
                let n = reader.read(&mut buf)?;
                if n == 0 {
                    break;
                }
                file.write_all(&buf[..n])?;
                written += n as u64;
                progress(written);
            }
            file.sync_all()?;
            std::fs::rename(&part, dest)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&part);
        }
        result
    }
}

/// Accepts `host:port`, `https://host:port/` etc.
pub fn normalize_host(input: &str) -> String {
    let s = input.trim();
    let s = s.strip_prefix("https://").or_else(|| s.strip_prefix("http://")).unwrap_or(s);
    s.trim_end_matches('/').to_string()
}

/// Percent-encodes everything except unreserved characters and `/`
fn encode_path(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hosts() {
        assert_eq!(normalize_host(" https://1.2.3.4:6969/ "), "1.2.3.4:6969");
        assert_eq!(normalize_host("127.0.0.1:6969"), "127.0.0.1:6969");
    }

    #[test]
    fn encodes() {
        assert_eq!(encode_path("assets/a b.bundle"), "assets/a%20b.bundle");
    }
}
