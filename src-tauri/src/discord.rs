use crate::model::{MatchState, MatchView};
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

    pub fn enabled(&self) -> bool {
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

    pub fn update(&mut self, view: &MatchView) {
        if !self.enabled() || !self.ensure_connected() {
            return;
        }
        self.ticks = self.ticks.wrapping_add(1);

        let (details, state) = activity_text(view);
        let key = format!("{details}|{state}");

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
            let assets = activity::Assets::new().large_image("logo").large_text("Peek");
            let timestamps = activity::Timestamps::new().start(self.start);
            let mut act = activity::Activity::new()
                .assets(assets)
                .timestamps(timestamps)
                .details(&details);
            if !state.is_empty() {
                act = act.state(&state);
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
    let rank = self_rank(view);
    let mode = if view.mode.is_empty() {
        "match".to_string()
    } else {
        view.mode.clone()
    };
    match view.state {
        MatchState::CoreGame => (format!("In a {mode} match"), rank),
        MatchState::PreGame => (format!("Agent Select - {mode}"), rank),
        MatchState::Menu => ("In the menu".to_string(), rank),
        MatchState::NoGame => ("Idle".to_string(), "Not in game".to_string()),
    }
}
