# Peek

Peek the lobby before you peek the angle.

A lightweight VALORANT match companion. While you are in a match it shows every
player's rank, peak rank, RR, account level, equipped agent, and party grouping
in a small, fast window you keep on a second monitor or alt-tab to.

It is built to be light and reliable: it runs as a separate window (no in-game
overlay, no effect on your FPS), and it reads data straight from your own running
Riot client, so it does not depend on any third-party website staying up.

## Safety

Peek is read-only. It talks to the local Riot client API on your own
machine (the same data your game client already has) and to the public Riot PVP
endpoints. It does not read or modify game memory, inject into the game, or run
an in-game overlay. It is the same approach used by existing open rank checkers.

Riot's terms broadly disallow third-party tools, so this lives in the same
tolerated grey area as similar projects. There is no anti-cheat detection here
and no history of bans for local-API rank checkers, but you use it at your own
risk. This project is not affiliated with or endorsed by Riot Games.

## Requirements

- Windows
- WebView2 runtime (already present on current Windows installs)

To build from source:

- [Rust](https://rustup.rs) (stable)
- [Node.js](https://nodejs.org) 18 or newer

## Run from source

```
npm install
npm run tauri dev
```

VALORANT must be running. Enemy ranks become visible from Agent Select onward,
which is a Riot restriction, not a limitation of this tool.

## Build a release

```
npm run tauri build
```

The installer and executable are produced under `src-tauri/target/release`.

## Region

The region and shard are detected automatically from VALORANT's log. If
detection is wrong, override it with environment variables before launching:

```
VAL_REGION=eu
VAL_SHARD=eu
```

Common regions: `na`, `eu`, `ap`, `kr`, `br`, `latam`.

## How it works

1. Read the local client lockfile for the API port and password.
2. Get the access and entitlements tokens and your PUUID from the local API.
3. Detect match state (pre-game or in-game) and collect the players.
4. Fetch ranks, names, and party grouping; resolve agent and rank names from a
   locally cached copy of valorant-api.com data.
5. Push the assembled table to the UI, refreshing every few seconds.

## Roadmap

- Win rate and recent form
- Equipped skins / loadout viewer
- Agent pools and a simple threat read
- Local history of players you have seen before
