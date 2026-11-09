//! Wire types of the SPT launcher protocol.
//!
//! Field names follow the server's JSON. The response wrappers (`Response`,
//! `Profiles`) and the bundle manifest are PascalCase in the official launcher's
//! models; camelCase aliases are accepted as well, in case a server build
//! serialises them differently.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct MiniProfile {
    pub username: String,
    pub nickname: String,
    pub side: String,
    #[serde(rename = "currlvl")]
    pub current_level: i64,
    #[serde(rename = "currexp")]
    pub current_exp: i64,
    #[serde(rename = "prevexp")]
    pub previous_exp: i64,
    #[serde(rename = "nextlvl")]
    pub next_level: i64,
    #[serde(rename = "maxlvl")]
    pub max_level: i64,
    #[serde(rename = "profileId")]
    pub profile_id: String,
    pub edition: String,
    #[serde(rename = "sptData")]
    pub spt_data: SptData,
    #[serde(rename = "invalidOrUnloadableProfile")]
    pub invalid: bool,
    #[serde(rename = "profileCurrency")]
    pub currency: ProfileCurrency,
    /// Fresh profile: the character still has to be created in game
    pub wipe: bool,
}

impl MiniProfile {
    /// Progress towards the next level, 0..=100
    pub fn xp_percent(&self) -> f64 {
        let span = self.next_level - self.previous_exp;
        if span <= 0 {
            return if self.current_level >= self.max_level && self.max_level > 0 { 100.0 } else { 0.0 };
        }
        (((self.current_exp - self.previous_exp) as f64 / span as f64) * 100.0).clamp(0.0, 100.0)
    }

    pub fn display_name(&self) -> &str {
        if self.nickname.is_empty() || self.wipe {
            &self.username
        } else {
            &self.nickname
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct SptData {
    pub version: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct ProfileCurrency {
    pub roubles: i64,
    pub euros: i64,
    pub dollars: i64,
    pub gp: i64,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct ModPage {
    pub name: String,
    #[serde(rename = "homePage")]
    pub home_page: String,
    pub description: Option<String>,
}

/// `{ "Response": T }`
#[derive(Debug, Deserialize)]
pub struct Wrapped<T> {
    #[serde(rename = "Response", alias = "response")]
    pub response: Option<T>,
}

/// Register / remove / wipe: `{ "Response": bool, "Profiles": [...] }`
#[derive(Debug, Deserialize)]
pub struct ProfilesResult {
    #[serde(rename = "Response", alias = "response", default)]
    pub response: bool,
    #[serde(rename = "Profiles", alias = "profiles", default)]
    pub profiles: Vec<MiniProfile>,
}

#[derive(Debug, Serialize)]
pub struct LoginRequest<'a> {
    pub username: &'a str,
}

#[derive(Debug, Serialize)]
pub struct RegisterRequest<'a> {
    pub username: &'a str,
    pub edition: &'a str,
}

/// One entry of `/singleplayer/bundles`
#[derive(Debug, Clone, Deserialize)]
pub struct BundleManifestItem {
    #[serde(rename = "FileName", alias = "fileName", default)]
    pub file_name: String,
    #[serde(rename = "ModPath", alias = "modPath", default)]
    pub mod_path: String,
    #[serde(rename = "Crc", alias = "crc", default)]
    pub crc: u32,
    #[serde(rename = "Size", alias = "size", default)]
    pub size: u64,
    #[serde(rename = "ModifiedUtcTicks", alias = "modifiedUtcTicks", default)]
    pub modified_utc_ticks: i64,
}

/// Profile editions offered by `/launcher/v2/types`: name -> description
pub type Editions = BTreeMap<String, String>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_profiles_response() {
        let json = r#"{"Response":[{"username":"deathedit","nickname":"Deathedit","side":"Usec",
            "currlvl":23,"currexp":100,"prevexp":50,"nextlvl":150,"maxlvl":79,"profileId":"abc",
            "edition":"Standard","sptData":{"version":"4.1.6"},"invalidOrUnloadableProfile":false,
            "profileCurrency":{"roubles":1000,"euros":0,"dollars":0,"gp":0},
            "profileStats":{"overall":{},"pmc":{},"scav":{}},"wipe":false}]}"#;
        let w: Wrapped<Vec<MiniProfile>> = serde_json::from_str(json).unwrap();
        let p = &w.response.unwrap()[0];
        assert_eq!(p.profile_id, "abc");
        assert_eq!(p.currency.roubles, 1000);
        assert_eq!(p.xp_percent(), 50.0);
    }

    #[test]
    fn parses_manifest_both_cases() {
        let a: Vec<BundleManifestItem> =
            serde_json::from_str(r#"[{"FileName":"a.bundle","ModPath":"user/mods/x","Crc":1,"Size":2,"ModifiedUtcTicks":3,"Dependencies":[]}]"#).unwrap();
        let b: Vec<BundleManifestItem> =
            serde_json::from_str(r#"[{"fileName":"a.bundle","modPath":"user/mods/x","crc":1,"size":2,"modifiedUtcTicks":3}]"#).unwrap();
        assert_eq!(a[0].file_name, b[0].file_name);
        assert_eq!(a[0].size, b[0].size);
    }
}
