<div align="center">
  <img src="src-tauri/icons/icon.png" width="120" alt="Peek logo" />
  <h1>Peek</h1>
  <p><b>Peek the lobby before you peek the angle.</b></p>
  <p>A fast, lightweight VALORANT match companion. See who you are playing with and against, the moment the match loads.</p>

  <p>
    <a href="https://github.com/6ky/valorant-peek/releases/latest"><img src="https://img.shields.io/github/v/release/6ky/valorant-peek?style=flat-square&color=ff4655&label=release" alt="Latest release" /></a>
    <a href="https://github.com/6ky/valorant-peek/releases"><img src="https://img.shields.io/github/downloads/6ky/valorant-peek/total?style=flat-square&color=ff4655&label=downloads" alt="Downloads" /></a>
    <a href="https://github.com/6ky/valorant-peek/stargazers"><img src="https://img.shields.io/github/stars/6ky/valorant-peek?style=flat-square&color=ff4655" alt="Stars" /></a>
    <a href="LICENSE"><img src="https://img.shields.io/github/license/6ky/valorant-peek?style=flat-square&color=ff4655" alt="License" /></a>
    <img src="https://img.shields.io/badge/platform-Windows-0a0a0a?style=flat-square" alt="Windows" />
    <img src="https://img.shields.io/badge/built%20with-Tauri%20%2B%20Rust-0a0a0a?style=flat-square" alt="Tauri and Rust" />
  </p>

  <img src=".github/screenshots/match.png" width="900" alt="Peek live match roster" />
</div>

---

Peek shows every player's rank, recent form, and threat read during a match, in
a small window you keep on a second monitor or alt-tab to. It is built to be
light and reliable: it runs as its own window (no in-game overlay, no effect on
your FPS) and reads straight from your own running Riot client, so it does not
depend on any third-party website staying up.

## Why Peek

- **Ban-safe by design.** Read only. It talks only to your own local Riot client
  and to Riot's public endpoints. No memory reads, no injection, no overlay.
- **Zero FPS impact.** A separate native window, not an in-game overlay.
- **Deep recent form.** ACS, ADR, KAST, K/D, headshot percent and win rate per
  player, not just a rank badge.
- **Premade detection.** Exact ally parties from presence, plus inferred enemy
  stacks from match history.
- **Encounter memory.** It remembers everyone you have played with or against,
  and your record with them, built up locally over time.
- **Tiny and native.** A roughly 3 MB Rust app, not a browser tab.

|                                          | Peek | Typical web tracker |
| ---------------------------------------- | :--: | :-----------------: |
| Runs off your own client, no outside site |  yes  |         no          |
| No overlay, zero FPS hit                  |  yes  |       varies        |
| ACS / ADR / KAST recent form             |  yes  |      sometimes      |
| Ally and inferred enemy premades         |  yes  |       rarely        |
| Local encounter history                  |  yes  |         no          |
| Native desktop app                       |  yes  |         no          |

## Install

One line in PowerShell grabs the latest release and installs it:

```powershell
irm https://raw.githubusercontent.com/6ky/valorant-peek/main/install.ps1 | iex
```

Or download the `Peek_x.x.x_x64-setup.exe` from the
[releases page](https://github.com/6ky/valorant-peek/releases/latest) and run it.

## Screens

- **Match**: the live roster. Allies stacked above enemies with a team win rate
  comparison between them, or a single lobby list for Deathmatch.
- **Idle**: your own profile when you are in menus, with recent competitive
  history and an RR trend.
- **Standby**: a waiting screen when VALORANT is not running.

## Per player

- Current rank and RR, plus leaderboard rank for Immortal and Radiant
- Peak rank and the act it was reached
- Act win rate and games played
- Recent ACS, ADR, KAST, K/D and headshot percent over the last games (optional)
- Win or loss streak and RR trend over recent games
- Smurf read, a single score from level, rank, games, win rate, and skins
- Party and premade grouping with stack size
- Encounter history: how many times you have seen this player and your record
  with them, built up locally as you play
- Account level, equipped agent, and equipped Vandal skin
- Live map and round score in the header, and a dodge countdown in agent select

All of it is color coded so a smurf, a tilted teammate, or a stacked enemy team
reads at a glance.

## Modes

Competitive, Unrated, Swiftplay, Spike Rush, Deathmatch, Team Deathmatch,
Escalation, Replication, Snowball, and Custom. Competitive versus free-for-all
layout is detected automatically. The recent matches list has a Competitive /
Unrated / All toggle.

## Safety

Peek is read only. It talks to the local Riot client API on your own machine
(the same data your game client already has) and to the public Riot endpoints.
It does not read or modify game memory, inject into the game, or run an in-game
overlay. Match data is fetched once per state change, never on a timer, so it
stays well within normal request rates. This is the same approach used by other
open rank checkers.

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
- **Lobby combat stats**: fetches each player's recent games when a match loads

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
4. Fetch ranks, names, party grouping, and equipped skins once per match, and
   resolve emblems, agent icons, maps, and skins from a locally cached copy of
   valorant-api.com data.
5. Push the assembled table to the UI, refreshing as the match changes.

## Tech

Tauri 2, Rust backend, React and TypeScript frontend. IBM Plex for type.

## Star it

If Peek saves you a dodge or two, a star helps other players find it.

## License

[MIT](LICENSE). Peek is not affiliated with or endorsed by Riot Games. VALORANT
and all related art and trademarks are property of Riot Games, Inc. Use at your
own risk.
