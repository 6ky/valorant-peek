use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// What we remember about a player across matches.
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Record {
    pub name: String,
    pub rank_tier: u32,
    pub last_seen: i64,
    pub seen: u32,
    pub wins: u32,
    pub losses: u32,
}

/// Local history of every player we have shared a match with, and our record
/// in those matches. Persisted to disk so it carries across sessions. Holds no
/// network logic: the caller feeds it rosters and outcomes.
#[derive(Serialize, Deserialize, Default)]
pub struct EncounterStore {
    players: HashMap<String, Record>,
    // Match ids already counted, so a match seen on several polls is recorded
    // once.
    seen_matches: HashSet<String>,
    // Match ids whose win or loss has already been applied.
    outcome_matches: HashSet<String>,
    // Match id to the other players in it, waiting for the result to land.
    pending: HashMap<String, Vec<String>>,
    #[serde(skip)]
    path: PathBuf,
}

impl EncounterStore {
    pub fn load(path: PathBuf) -> Self {
        let mut store: EncounterStore = std::fs::read_to_string(&path)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        store.path = path;
        store
    }

    fn save(&self) {
        if let Ok(text) = serde_json::to_string(self) {
            let _ = std::fs::write(&self.path, text);
        }
    }

    /// Counts as they stand for a player, not including the current match when
    /// called before record_seen.
    pub fn prior(&self, puuid: &str) -> (u32, u32, u32) {
        match self.players.get(puuid) {
            Some(r) => (r.seen, r.wins, r.losses),
            None => (0, 0, 0),
        }
    }

    /// Count a match once. `players` is every other player in it (exclude the
    /// signed-in user). The result is recorded later through apply_outcome.
    pub fn record_seen(&mut self, match_id: &str, players: &[(String, String, u32)], now: i64) {
        if match_id.is_empty() || self.seen_matches.contains(match_id) {
            return;
        }
        self.seen_matches.insert(match_id.to_string());
        let mut puuids = Vec::with_capacity(players.len());
        for (puuid, name, tier) in players {
            let r = self.players.entry(puuid.clone()).or_default();
            r.seen += 1;
            r.last_seen = now;
            if !name.is_empty() {
                r.name = name.clone();
            }
            if *tier > 0 {
                r.rank_tier = *tier;
            }
            puuids.push(puuid.clone());
        }
        self.pending.insert(match_id.to_string(), puuids);
        self.save();
    }

    /// Apply a win or loss for a match we counted earlier.
    pub fn apply_outcome(&mut self, match_id: &str, won: bool) {
        if match_id.is_empty() || self.outcome_matches.contains(match_id) {
            return;
        }
        let puuids = match self.pending.remove(match_id) {
            Some(p) => p,
            None => return,
        };
        self.outcome_matches.insert(match_id.to_string());
        for puuid in puuids {
            if let Some(r) = self.players.get_mut(&puuid) {
                if won {
                    r.wins += 1;
                } else {
                    r.losses += 1;
                }
            }
        }
        self.save();
    }

    /// Match ids still waiting for a result, so the caller knows which outcomes
    /// to look up.
    pub fn pending_ids(&self) -> Vec<String> {
        self.pending.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store() -> EncounterStore {
        let mut dir = std::env::temp_dir();
        dir.push(format!("peek-enc-test-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&dir);
        EncounterStore::load(dir)
    }

    #[test]
    fn counts_and_records_outcome() {
        let mut s = temp_store();
        let roster = vec![
            ("p1".to_string(), "Ace#NA1".to_string(), 24u32),
            ("p2".to_string(), "Bee#NA1".to_string(), 18u32),
        ];
        assert_eq!(s.prior("p1"), (0, 0, 0));
        s.record_seen("m1", &roster, 100);
        assert_eq!(s.prior("p1"), (1, 0, 0));
        // Same match again does not double count.
        s.record_seen("m1", &roster, 200);
        assert_eq!(s.prior("p1"), (1, 0, 0));
        s.apply_outcome("m1", true);
        assert_eq!(s.prior("p1"), (1, 1, 0));
        // Outcome applied once.
        s.apply_outcome("m1", true);
        assert_eq!(s.prior("p1"), (1, 1, 0));

        s.record_seen("m2", &roster, 300);
        s.apply_outcome("m2", false);
        assert_eq!(s.prior("p1"), (2, 1, 1));
    }

    #[test]
    fn persists_across_load() {
        let mut dir = std::env::temp_dir();
        dir.push(format!("peek-enc-persist-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&dir);
        {
            let mut s = EncounterStore::load(dir.clone());
            s.record_seen("m1", &[("p1".to_string(), "Ace#NA1".to_string(), 24)], 100);
        }
        let s2 = EncounterStore::load(dir.clone());
        assert_eq!(s2.prior("p1"), (1, 0, 0));
        let _ = std::fs::remove_file(&dir);
    }
}
