<div align="center">
  <img src="src-tauri/icons/icon.png" width="120" alt="Peek logo" />
  <h1>Peek</h1>
  <p><b>Peek the lobby before you peek the angle.</b></p>
  <p>A fast, lightweight VALORANT match companion. See everyone's rank, the moment the match loads.</p>
</div>

---

Peek shows every player's rank, peak rank, win rate, agent, and party grouping
during a match, in a small window you keep on a second monitor or alt-tab to. It
is built to be light and reliable: it runs as its own window (no in-game overlay,
no effect on your FPS) and reads straight from your own running Riot client, so
it does not depend on any third-party website staying up.

## Features

- **Live roster** with real rank emblems and agent icons, color coded by tier
- **Ally / enemy split**, with free-for-all handling for Deathmatch
- **Every mode**: Competitive, Unrated, Swiftplay, Spike Rush, Deathmatch,
  Team Deathmatch, Escalation, Replication, Snowball, Custom
- **Per player stats**: current rank and RR, peak rank, act win/loss record,
  account level, leaderboard rank for Immortal and Radiant, party grouping
- **Your profile** when idle, with recent competitive RR history
- **Privacy aware**: respects streamer mode and hidden account level
- **Discord Rich Presence** showing your match state, mode, rank, and elapsed time
- **Frameless window** with a custom title bar, system tray, and settings
- Runs at around 40 MB of memory

## Safety

Peek is read only. It talks to the local Riot client API on your own machine
(the same data your game client already has) and to the public Riot endpoints.
It does not read or modify game memory, inject into the game, or run an in-game
overlay. This is the same approach used by other open rank checkers.

Riot's terms broadly disallow third party tools, so this lives in the same
tolerated grey area as similar projects. There is no anti-cheat detection here
and no history of bans for local-API rank checkers, but you use it at your own
risk. This project is not affiliated with or endorsed by Riot Games.

## Requirements

- Windows
- WebView2 runtime (already present on current Windows installs)

To build from source you also need [Rust](https://rustup.rs) (stable) and
[Node.js](https://nodejs.org) 18 or newer.

## Run from source

```bash
npm install
npm run tauri dev
```

VALORANT must be running. Enemy ranks become visible from Agent Select onward,
which is a Riot restriction, not a limit of this tool.

## Build

```bash
npm run tauri build
```

The installer and executable are written to `src-tauri/target/release`.

## Settings

Open the gear in the title bar to set:

- **Close button**: ask each time, minimize to tray, or quit
- **Always on top**
- **Discord Rich Presence** on or off

## Region

Region and shard are detected from VALORANT's log automatically. If detection is
wrong, override it before launching:

```
VAL_REGION=eu
VAL_SHARD=eu
```

Common regions: `na`, `eu`, `ap`, `kr`, `br`, `latam`.

## How it works

1. Read the local client lockfile for the API port and password.
2. Get the access and entitlements tokens and your PUUID from the local API.
3. Read presence to detect game state and queue, then collect the players.
4. Fetch ranks, names, and party grouping once per match, and resolve emblems
   and agent icons from a locally cached copy of valorant-api.com data.
5. Push the assembled table to the UI, refreshing as the match changes.

## Tech

Tauri 2, Rust backend, React and TypeScript frontend. Geist for type.

## Disclaimer

Not affiliated with or endorsed by Riot Games. VALORANT is a trademark of Riot
Games, Inc. Use at your own risk.
