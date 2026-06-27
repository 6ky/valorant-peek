use crate::model::MatchView;
use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};

/// The single shared "Peek" Discord application id, baked in so every user
/// gets the same branding and logo without registering their own app. Devs can
/// override it with the PEEK_DISCORD_APP_ID environment variable.
const DEFAULT_APP_ID: &str = "1519865652763033752";

pub fn resolve_app_id() -> String {
    match std::env::var("PEEK_DISCORD_APP_ID") {
        Ok(id) if !id.is_empty() => id,
        _ => DEFAULT_APP_ID.to_string(),
    }
}

// Re-send the presence every this many updates even when unchanged. This
// keep-alive refreshes the activity and, if Discord has restarted, makes the
// write fail so we notice the dropped connection and reconnect.
const KEEPALIVE_EVERY: u32 = 10;
// Sentinel dedup key used while presence is turned off in settings.
const OFF_KEY: &str = "__rpc_off__";

pub struct Rpc {
    client: Option<DiscordIpcClient>,
    app_id: String,
    start: i64,
    last_key: String,
    ticks: u32,
}

impl Rpc {
    pub fn new(app_id: String, start: i64) -> Self {
        Self {
            client: None,
            app_id,
            start,
            last_key: String::new(),
            ticks: 0,
        }
    }

    pub fn has_app_id(&self) -> bool {
        !self.app_id.is_empty()
    }

    fn ensure_connected(&mut self) -> bool {
        if self.client.is_some() {
            return true;
        }
        if let Ok(mut client) = DiscordIpcClient::new(&self.app_id) {
            if client.connect().is_ok() {
                self.client = Some(client);
                // Force the next update to re-apply the activity on this fresh
                // connection, even if the status text has not changed.
                self.last_key.clear();
                return true;
            }
        }
        false
    }

    pub fn update(&mut self, view: &MatchView, enabled: bool) {
        if !self.has_app_id() {
            return;
        }
        if !enabled {
            // Clear the presence once when turned off in settings.
            if self.last_key != OFF_KEY {
                if self.ensure_connected() {
                    if let Some(client) = self.client.as_mut() {
                        let _ = client.clear_activity();
                    }
                }
                self.last_key = OFF_KEY.to_string();
            }
            return;
        }
        if !self.ensure_connected() {
            return;
        }
        self.ticks = self.ticks.wrapping_add(1);

        let (details, state) = activity_text(view);
        let (large_image, large_text) = large_asset(view);
        let rank = rank_asset(view);
        let psize = party_size(view);
        let key = format!(
            "{details}|{state}|{large_image}|{}|{}",
            rank.as_ref().map(|r| r.0.as_str()).unwrap_or(""),
            psize.unwrap_or(0)
        );

        // Send on change, plus a periodic keep-alive resend. The keep-alive
        // refreshes the presence and surfaces a dropped connection as an error.
        let keepalive = self.ticks % KEEPALIVE_EVERY == 0;
        if key == self.last_key && !keepalive {
            return;
        }

        let client = self.client.as_mut().unwrap();
        let result = if details.is_empty() {
            client.clear_activity()
        } else {
            // Map splash as the large image and the rank icon as the small one.
            // Discord accepts external image urls in these fields, so the
            // valorant-api urls already on the view go straight through without
            // any uploaded assets.
            let mut assets = activity::Assets::new()
                .large_image(&large_image)
                .large_text(&large_text);
            if let Some((url, text)) = rank.as_ref() {
                assets = assets.small_image(url).small_text(text);
            }
            let timestamps = activity::Timestamps::new().start(self.start);
            let mut act = activity::Activity::new()
                .assets(assets)
                .timestamps(timestamps)
                .details(&details);
            if !state.is_empty() {
                act = act.state(&state);
            }
            if let Some(size) = psize {
                act = act.party(activity::Party::new().size([size.min(5) as i32, 5]));
            }
            client.set_activity(act)
        };

        match result {
            Ok(_) => self.last_key = key,
            Err(_) => {
                // Connection dropped; reconnect on the next update and force a
                // resend by clearing the dedup key.
                self.client = None;
                self.last_key.clear();
            }
        }
    }
}

fn self_rank(view: &MatchView) -> String {
    match &view.me {
        Some(me) if me.rank_tier > 0 => format!("{} - {} RR", me.rank_name, me.rr),
        Some(me) => me.rank_name.clone(),
        None => String::new(),
    }
}

fn activity_text(view: &MatchView) -> (String, String) {
    let mut details = if view.activity.is_empty() {
        "Idle".to_string()
    } else {
        view.activity.clone()
    };
    // Show the live round score in the top line once a game is underway.
    if view.ally_score > 0 || view.enemy_score > 0 {
        details = format!("{details}  {} - {}", view.ally_score, view.enemy_score);
    }
    (details, self_rank(view))
}

// Large image: the live map splash while in a match, else the bundled logo.
fn large_asset(view: &MatchView) -> (String, String) {
    if view.map_image.is_empty() {
        ("logo".to_string(), "Peek".to_string())
    } else if view.map.is_empty() {
        (view.map_image.clone(), "Peek".to_string())
    } else {
        (view.map_image.clone(), view.map.clone())
    }
}

// Small image: the player's current rank icon and name, when ranked.
fn rank_asset(view: &MatchView) -> Option<(String, String)> {
    let me = view.me.as_ref()?;
    if me.rank_tier == 0 || me.rank_icon.is_empty() {
        return None;
    }
    Some((me.rank_icon.clone(), me.rank_name.clone()))
}

// Party size for the "(n of 5)" bracket, when the roster knows it.
fn party_size(view: &MatchView) -> Option<u32> {
    match &view.me {
        Some(me) if me.party_size >= 1 => Some(me.party_size),
        _ => None,
    }
}
