use crate::lockfile::Lockfile;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::Value;

#[derive(Default)]
pub struct Presence {
    pub loop_state: String,
    pub queue_id: String,
    pub party_state: String,
    pub provisioning_flow: String,
    pub is_idle: bool,
}

/// The decoded private presence blob comes in two shapes depending on the
/// client build: fields nested under "matchPresenceData"/"partyPresenceData",
/// or flat at the root. Read from whichever has the field.
pub fn parse_private(decoded: &Value) -> Presence {
    let lookup = |key: &str| {
        decoded
            .get("matchPresenceData")
            .and_then(|d| d.get(key))
            .or_else(|| decoded.get("partyPresenceData").and_then(|d| d.get(key)))
            .or_else(|| decoded.get(key))
    };
    let read = |key: &str| {
        lookup(key)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };
    Presence {
        loop_state: read("sessionLoopState"),
        queue_id: read("queueId"),
        party_state: read("partyState"),
        provisioning_flow: read("provisioningFlow"),
        is_idle: lookup("isIdle").and_then(|v| v.as_bool()).unwrap_or(false),
    }
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

/// A human description of what the player is currently doing, for the header
/// and Discord presence.
pub fn describe_activity(p: &Presence, mode: &str) -> String {
    match p.loop_state.as_str() {
        "INGAME" => format!("Playing {mode}"),
        "PREGAME" => format!("Agent Select - {mode}"),
        "MENUS" => {
            if p.provisioning_flow.eq_ignore_ascii_case("CustomGame")
                || p.party_state == "CUSTOM_GAME_SETUP"
            {
                "In a custom lobby".to_string()
            } else if p.party_state == "MATCHMAKING" {
                format!("In queue - {mode}")
            } else if p.is_idle {
                "Away".to_string()
            } else {
                "In the lobby".to_string()
            }
        }
        _ => "Idle".to_string(),
    }
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
    Some(parse_private(&decoded))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_flat_private() {
        let v: Value = serde_json::from_str(
            r#"{"sessionLoopState":"MENUS","queueId":"competitive","partyState":"MATCHMAKING"}"#,
        )
        .unwrap();
        let p = parse_private(&v);
        assert_eq!(p.loop_state, "MENUS");
        assert_eq!(p.queue_id, "competitive");
        assert_eq!(p.party_state, "MATCHMAKING");
    }

    #[test]
    fn parses_nested_private() {
        let v: Value = serde_json::from_str(
            r#"{"matchPresenceData":{"sessionLoopState":"PREGAME","queueId":"deathmatch"},"partyPresenceData":{"partyState":"DEFAULT"}}"#,
        )
        .unwrap();
        let p = parse_private(&v);
        assert_eq!(p.loop_state, "PREGAME");
        assert_eq!(p.queue_id, "deathmatch");
        assert_eq!(p.party_state, "DEFAULT");
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
