use crate::auth::AuthError;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct Region {
    pub region: String,
    pub shard: String,
}

impl Region {
    pub fn pd_base(&self) -> String {
        format!("https://pd.{}.a.pvp.net", self.shard)
    }

    pub fn glz_base(&self) -> String {
        format!("https://glz-{}-1.{}.a.pvp.net", self.region, self.shard)
    }
}

/// Pull region and shard out of a glz URL found in VALORANT's log, e.g.
/// "https://glz-eu-1.eu.a.pvp.net". Returns (region, shard).
pub fn parse_region_from_log(text: &str) -> Option<Region> {
    let marker = "glz-";
    let start = text.find(marker)?;
    let rest = &text[start + marker.len()..];
    let dash = rest.find("-1.")?;
    let region = &rest[..dash];
    let after = &rest[dash + 3..];
    let dot = after.find(".a.pvp.net")?;
    let shard = &after[..dot];
    if region.is_empty() || shard.is_empty() {
        return None;
    }
    Some(Region {
        region: region.to_string(),
        shard: shard.to_string(),
    })
}

pub fn detect_region_from_log() -> Option<Region> {
    let base = std::env::var("LOCALAPPDATA").ok()?;
    let path = std::path::PathBuf::from(base).join(r"VALORANT\Saved\Logs\ShooterGame.log");
    let text = std::fs::read_to_string(path).ok()?;
    parse_region_from_log(&text)
}

pub fn parse_version(json: &Value) -> Option<String> {
    json.get("data")?
        .get("riotClientVersion")?
        .as_str()
        .map(String::from)
}

pub async fn fetch_client_version() -> Result<String, AuthError> {
    let body: Value = crate::http::pvp_client()
        .get("https://valorant-api.com/v1/version")
        .send()
        .await
        .map_err(|_| AuthError::Http)?
        .json()
        .await
        .map_err(|_| AuthError::Shape)?;
    parse_version(&body).ok_or(AuthError::Shape)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_pvp_bases() {
        let r = Region {
            region: "na".into(),
            shard: "na".into(),
        };
        assert_eq!(r.pd_base(), "https://pd.na.a.pvp.net");
        assert_eq!(r.glz_base(), "https://glz-na-1.na.a.pvp.net");
    }

    #[test]
    fn parses_region_from_log_line() {
        let line = "info: connecting to https://glz-eu-1.eu.a.pvp.net/something";
        let r = parse_region_from_log(line).unwrap();
        assert_eq!(r.region, "eu");
        assert_eq!(r.shard, "eu");
    }

    #[test]
    fn region_from_log_handles_distinct_shard() {
        let line = "https://glz-na-1.na.a.pvp.net";
        let r = parse_region_from_log(line).unwrap();
        assert_eq!(r.region, "na");
        assert_eq!(r.shard, "na");
    }

    #[test]
    fn region_from_log_none_when_absent() {
        assert!(parse_region_from_log("no url here").is_none());
    }

    #[test]
    fn parses_version() {
        let v: Value =
            serde_json::from_str(r#"{"data":{"riotClientVersion":"release-09.00-x"}}"#).unwrap();
        assert_eq!(parse_version(&v).unwrap(), "release-09.00-x");
    }
}
