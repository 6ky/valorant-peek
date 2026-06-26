use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

#[derive(Default)]
pub struct StaticData {
    pub tiers: HashMap<u32, String>,
    pub tier_icons: HashMap<u32, String>,
    pub agents: HashMap<String, String>,
    pub agent_icons: HashMap<String, String>,
    pub maps: HashMap<String, String>,
    pub card_arts: HashMap<String, String>,
    pub season_labels: HashMap<String, String>,
}

impl StaticData {
    pub fn rank_name(&self, tier: u32) -> String {
        self.tiers
            .get(&tier)
            .cloned()
            .unwrap_or_else(|| "Unranked".to_string())
    }

    pub fn rank_icon(&self, tier: u32) -> String {
        self.tier_icons.get(&tier).cloned().unwrap_or_default()
    }

    pub fn agent_name(&self, id: &str) -> String {
        self.agents.get(id).cloned().unwrap_or_default()
    }

    pub fn agent_icon(&self, id: &str) -> String {
        self.agent_icons.get(id).cloned().unwrap_or_default()
    }

    pub fn map_name(&self, map_url: &str) -> String {
        self.maps.get(map_url).cloned().unwrap_or_default()
    }

    pub fn card_art(&self, id: &str) -> String {
        self.card_arts.get(id).cloned().unwrap_or_default()
    }

    pub fn season_label(&self, uuid: &str) -> String {
        self.season_labels.get(uuid).cloned().unwrap_or_default()
    }
}

pub fn parse_tiers(json: &Value) -> HashMap<u32, String> {
    let mut out = HashMap::new();
    let episodes = json.get("data").and_then(|d| d.as_array());
    for episode in episodes.into_iter().flatten() {
        let tiers = episode.get("tiers").and_then(|t| t.as_array());
        for tier in tiers.into_iter().flatten() {
            if let (Some(n), Some(name)) = (
                tier.get("tier").and_then(|v| v.as_u64()),
                tier.get("tierName").and_then(|v| v.as_str()),
            ) {
                out.insert(n as u32, name.to_string());
            }
        }
    }
    out
}

pub fn parse_tier_icons(json: &Value) -> HashMap<u32, String> {
    let mut out = HashMap::new();
    let episodes = json.get("data").and_then(|d| d.as_array());
    for episode in episodes.into_iter().flatten() {
        let tiers = episode.get("tiers").and_then(|t| t.as_array());
        for tier in tiers.into_iter().flatten() {
            if let (Some(n), Some(icon)) = (
                tier.get("tier").and_then(|v| v.as_u64()),
                tier.get("largeIcon").and_then(|v| v.as_str()),
            ) {
                out.insert(n as u32, icon.to_string());
            }
        }
    }
    out
}

pub fn parse_agents(json: &Value) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let agents = json.get("data").and_then(|d| d.as_array());
    for agent in agents.into_iter().flatten() {
        if let (Some(uuid), Some(name)) = (
            agent.get("uuid").and_then(|v| v.as_str()),
            agent.get("displayName").and_then(|v| v.as_str()),
        ) {
            out.insert(uuid.to_string(), name.to_string());
        }
    }
    out
}

pub fn parse_agent_icons(json: &Value) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let agents = json.get("data").and_then(|d| d.as_array());
    for agent in agents.into_iter().flatten() {
        if let (Some(uuid), Some(icon)) = (
            agent.get("uuid").and_then(|v| v.as_str()),
            agent.get("displayIcon").and_then(|v| v.as_str()),
        ) {
            out.insert(uuid.to_string(), icon.to_string());
        }
    }
    out
}

pub fn parse_maps(json: &Value) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let maps = json.get("data").and_then(|d| d.as_array());
    for map in maps.into_iter().flatten() {
        if let (Some(url), Some(name)) = (
            map.get("mapUrl").and_then(|v| v.as_str()),
            map.get("displayName").and_then(|v| v.as_str()),
        ) {
            out.insert(url.to_string(), name.to_string());
        }
    }
    out
}

pub fn parse_player_cards(json: &Value) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let cards = json.get("data").and_then(|d| d.as_array());
    for card in cards.into_iter().flatten() {
        if let (Some(uuid), Some(art)) = (
            card.get("uuid").and_then(|v| v.as_str()),
            card.get("wideArt").and_then(|v| v.as_str()),
        ) {
            out.insert(uuid.to_string(), art.to_string());
        }
    }
    out
}

fn extract_num(name: &str) -> Option<u32> {
    let last = name.split_whitespace().last()?;
    if let Ok(n) = last.parse::<u32>() {
        return Some(n);
    }
    match last.to_uppercase().as_str() {
        "I" => Some(1),
        "II" => Some(2),
        "III" => Some(3),
        "IV" => Some(4),
        "V" => Some(5),
        "VI" => Some(6),
        "VII" => Some(7),
        _ => None,
    }
}

/// Short label for a parent season: "E8" for "EPISODE 8", or "V26" for the
/// newer version-based seasons.
fn parent_label(name: &str) -> Option<String> {
    let t = name.trim();
    if t.to_uppercase().contains("EPISODE") {
        return extract_num(t).map(|n| format!("E{n}"));
    }
    if t.len() >= 2 && t.starts_with('V') && t[1..].chars().all(|c| c.is_ascii_digit()) {
        return Some(t.to_string());
    }
    None
}

/// Map each competitive act uuid to a short label like "E8A2" or "V26A4".
pub fn parse_seasons(json: &Value) -> HashMap<String, String> {
    let arr = match json.get("data").and_then(|d| d.as_array()) {
        Some(a) => a,
        None => return HashMap::new(),
    };

    // Top-level seasons (parentUuid null) are episodes or version seasons.
    let mut parents: HashMap<String, String> = HashMap::new();
    for s in arr {
        if s.get("parentUuid").and_then(|v| v.as_str()).is_none() {
            let name = s.get("displayName").and_then(|v| v.as_str()).unwrap_or("");
            let uuid = s.get("uuid").and_then(|v| v.as_str()).unwrap_or("");
            if let Some(label) = parent_label(name) {
                if !uuid.is_empty() {
                    parents.insert(uuid.to_string(), label);
                }
            }
        }
    }

    let mut out = HashMap::new();
    for s in arr {
        let name = s.get("displayName").and_then(|v| v.as_str()).unwrap_or("");
        let uuid = s.get("uuid").and_then(|v| v.as_str()).unwrap_or("");
        let parent = s.get("parentUuid").and_then(|v| v.as_str()).unwrap_or("");
        if name.to_uppercase().contains("ACT") {
            if let (Some(act), Some(label)) = (extract_num(name), parents.get(parent)) {
                out.insert(uuid.to_string(), format!("{label}A{act}"));
            }
        }
    }
    out
}

async fn fetch_json(url: &str) -> Option<Value> {
    crate::http::pvp_client()
        .get(url)
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()
}

fn read_cached(path: &Path) -> Option<Value> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

pub async fn load_or_fetch(cache_dir: &Path) -> StaticData {
    let _ = std::fs::create_dir_all(cache_dir);
    let tiers_path = cache_dir.join("competitivetiers.json");
    let agents_path = cache_dir.join("agents.json");

    let tiers_json = match read_cached(&tiers_path) {
        Some(v) => v,
        None => {
            let v = fetch_json("https://valorant-api.com/v1/competitivetiers")
                .await
                .unwrap_or_else(|| serde_json::json!({"data": []}));
            if let Ok(text) = serde_json::to_string(&v) {
                let _ = std::fs::write(&tiers_path, text);
            }
            v
        }
    };

    let agents_json = match read_cached(&agents_path) {
        Some(v) => v,
        None => {
            let v = fetch_json("https://valorant-api.com/v1/agents?isPlayableCharacter=true")
                .await
                .unwrap_or_else(|| serde_json::json!({"data": []}));
            if let Ok(text) = serde_json::to_string(&v) {
                let _ = std::fs::write(&agents_path, text);
            }
            v
        }
    };

    let maps_path = cache_dir.join("maps.json");
    let maps_json = match read_cached(&maps_path) {
        Some(v) => v,
        None => {
            let v = fetch_json("https://valorant-api.com/v1/maps")
                .await
                .unwrap_or_else(|| serde_json::json!({"data": []}));
            if let Ok(text) = serde_json::to_string(&v) {
                let _ = std::fs::write(&maps_path, text);
            }
            v
        }
    };

    let cards_path = cache_dir.join("playercards.json");
    let cards_json = match read_cached(&cards_path) {
        Some(v) => v,
        None => {
            let v = fetch_json("https://valorant-api.com/v1/playercards")
                .await
                .unwrap_or_else(|| serde_json::json!({"data": []}));
            if let Ok(text) = serde_json::to_string(&v) {
                let _ = std::fs::write(&cards_path, text);
            }
            v
        }
    };

    let seasons_path = cache_dir.join("seasons.json");
    let seasons_json = match read_cached(&seasons_path) {
        Some(v) => v,
        None => {
            let v = fetch_json("https://valorant-api.com/v1/seasons")
                .await
                .unwrap_or_else(|| serde_json::json!({"data": []}));
            if let Ok(text) = serde_json::to_string(&v) {
                let _ = std::fs::write(&seasons_path, text);
            }
            v
        }
    };

    StaticData {
        tiers: parse_tiers(&tiers_json),
        tier_icons: parse_tier_icons(&tiers_json),
        agents: parse_agents(&agents_json),
        agent_icons: parse_agent_icons(&agents_json),
        maps: parse_maps(&maps_json),
        card_arts: parse_player_cards(&cards_json),
        season_labels: parse_seasons(&seasons_json),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_tier_and_agent_names() {
        let tiers: Value = serde_json::from_str(
            r#"{"data":[{"tiers":[{"tier":21,"tierName":"IMMORTAL 3"},{"tier":0,"tierName":"UNRANKED"}]}]}"#,
        )
        .unwrap();
        let agents: Value = serde_json::from_str(
            r#"{"data":[{"uuid":"abc","displayName":"Jett","isPlayableCharacter":true}]}"#,
        )
        .unwrap();
        let sd = StaticData {
            tiers: parse_tiers(&tiers),
            agents: parse_agents(&agents),
            ..Default::default()
        };
        assert_eq!(sd.rank_name(21), "IMMORTAL 3");
        assert_eq!(sd.rank_name(999), "Unranked");
        assert_eq!(sd.agent_name("abc"), "Jett");
        assert_eq!(sd.agent_name("missing"), "");
    }

    #[test]
    fn seasons_map_old_and_new_naming() {
        let v: Value = serde_json::from_str(
            r#"{"data":[
                {"uuid":"ep8","displayName":"EPISODE 8","parentUuid":null},
                {"uuid":"act-a","displayName":"ACT II","parentUuid":"ep8"},
                {"uuid":"v26","displayName":"V26","parentUuid":null},
                {"uuid":"act-b","displayName":"ACT IV","parentUuid":"v26"}
            ]}"#,
        )
        .unwrap();
        let m = parse_seasons(&v);
        assert_eq!(m.get("act-a").unwrap(), "E8A2");
        assert_eq!(m.get("act-b").unwrap(), "V26A4");
    }

    #[test]
    fn maps_url_to_name() {
        let v: Value = serde_json::from_str(
            r#"{"data":[{"mapUrl":"/Game/Maps/Ascent/Ascent","displayName":"Ascent"}]}"#,
        )
        .unwrap();
        let maps = parse_maps(&v);
        assert_eq!(maps.get("/Game/Maps/Ascent/Ascent").unwrap(), "Ascent");
    }
}
