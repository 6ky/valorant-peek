use crate::lockfile::Lockfile;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::Value;

pub struct Presence {
    pub loop_state: String,
    pub queue_id: String,
}

/// The decoded private presence blob comes in two shapes depending on the
/// client build: fields nested under "matchPresenceData", or flat at the root.
pub fn parse_private(decoded: &Value) -> (String, String) {
    let nested = decoded.get("matchPresenceData");
    let read = |key: &str| {
        nested
            .and_then(|n| n.get(key))
            .or_else(|| decoded.get(key))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };
    (read("sessionLoopState"), read("queueId"))
}

pub fn mode_name(queue_id: &str) -> String {
    let name = match queue_id {
        "competitive" => "Competitive",
        "unrated" => "Unrated",
        "swiftplay" => "Swiftplay",
        "spikerush" => "Spike Rush",
        "deathmatch" => "Deathmatch",
        "ggteam" => "Escalation",
        "onefa" => "Replication",
        "hurm" => "Team Deathmatch",
        "snowball" => "Snowball Fight",
        "newmap" => "New Map",
        "custom" | "" => "Custom",
        other => other,
    };
    name.to_string()
}

/// Free-for-all modes have no two-team structure, so ally/enemy grouping
/// does not apply.
pub fn is_ffa(queue_id: &str) -> bool {
    queue_id == "deathmatch"
}

pub async fn fetch_self_presence(lf: &Lockfile, puuid: &str) -> Option<Presence> {
    let url = format!("https://127.0.0.1:{}/chat/v4/presences", lf.port);
    let body: Value = crate::http::local_client()
        .get(url)
        .header("Authorization", crate::auth::basic_auth_header(&lf.password))
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()?;
    let presences = body.get("presences")?.as_array()?;
    let me = presences
        .iter()
        .find(|p| p.get("puuid").and_then(|v| v.as_str()) == Some(puuid))?;
    let private_b64 = me.get("private").and_then(|v| v.as_str())?;
    let decoded_bytes = STANDARD.decode(private_b64).ok()?;
    let decoded: Value = serde_json::from_slice(&decoded_bytes).ok()?;
    let (loop_state, queue_id) = parse_private(&decoded);
    Some(Presence {
        loop_state,
        queue_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_flat_private() {
        let v: Value =
            serde_json::from_str(r#"{"sessionLoopState":"INGAME","queueId":"competitive"}"#)
                .unwrap();
        let (state, queue) = parse_private(&v);
        assert_eq!(state, "INGAME");
        assert_eq!(queue, "competitive");
    }

    #[test]
    fn parses_nested_private() {
        let v: Value = serde_json::from_str(
            r#"{"matchPresenceData":{"sessionLoopState":"PREGAME","queueId":"deathmatch"}}"#,
        )
        .unwrap();
        let (state, queue) = parse_private(&v);
        assert_eq!(state, "PREGAME");
        assert_eq!(queue, "deathmatch");
    }

    #[test]
    fn maps_known_and_unknown_modes() {
        assert_eq!(mode_name("hurm"), "Team Deathmatch");
        assert_eq!(mode_name("ggteam"), "Escalation");
        assert_eq!(mode_name(""), "Custom");
        assert_eq!(mode_name("somethingnew"), "somethingnew");
    }

    #[test]
    fn flags_deathmatch_as_ffa() {
        assert!(is_ffa("deathmatch"));
        assert!(!is_ffa("hurm"));
        assert!(!is_ffa("competitive"));
    }
}
