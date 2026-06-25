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
    fn parses_version() {
        let v: Value =
            serde_json::from_str(r#"{"data":{"riotClientVersion":"release-09.00-x"}}"#).unwrap();
        assert_eq!(parse_version(&v).unwrap(), "release-09.00-x");
    }
}
