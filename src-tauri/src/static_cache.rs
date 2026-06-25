use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

pub struct StaticData {
    pub tiers: HashMap<u32, String>,
    pub agents: HashMap<String, String>,
}

impl StaticData {
    pub fn rank_name(&self, tier: u32) -> String {
        self.tiers
            .get(&tier)
            .cloned()
            .unwrap_or_else(|| "Unranked".to_string())
    }

    pub fn agent_name(&self, id: &str) -> String {
        self.agents.get(id).cloned().unwrap_or_default()
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

    StaticData {
        tiers: parse_tiers(&tiers_json),
        agents: parse_agents(&agents_json),
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
        };
        assert_eq!(sd.rank_name(21), "IMMORTAL 3");
        assert_eq!(sd.rank_name(999), "Unranked");
        assert_eq!(sd.agent_name("abc"), "Jett");
        assert_eq!(sd.agent_name("missing"), "");
    }
}
