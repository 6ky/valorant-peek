# Contributing

Peek is a small project, so the process is light.

## Before a big change

For anything past a small fix, open an issue first so we can agree on the shape
of it before you spend time on it. Bug fixes and small improvements can go
straight to a pull request.

## Building

```bash
npm install
npm run tauri dev
```

VALORANT has to be running for there to be anything to show. You also need
[Rust](https://rustup.rs) (stable) and [Node.js](https://nodejs.org) 18 or
newer. The backend is Rust under `src-tauri`, the frontend is React and
TypeScript under `src`.

## The one hard rule

Peek is read only and stays that way. It talks to the local Riot client and to
Riot's public endpoints, nothing else. Do not add anything that reads or writes
game memory, injects into the game, runs an overlay, or ships data to a third
party. That line is the whole reason the app is safe to run, so a pull request
that crosses it will not be merged.

## Style

Match the code around you. Keep it plain: short comments that say why rather
than what, no decorative noise, no em dashes in prose. Run `cargo check` and
`tsc` before you open a pull request and make sure both come back clean.

## Pull requests

Keep each one focused on a single thing. Say what it does and how you tested it.
If it touches the UI, a before and after screenshot helps a lot.
