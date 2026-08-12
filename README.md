# League Director (Rust)

Native client for the League of Legends [Replay API](https://developer.riotgames.com/docs/lol).
Independent Rust rewrite of the Apache 2.0 [RiotGames/leaguedirector](https://github.com/RiotGames/leaguedirector) project.

**This software is not published or endorsed by Riot Games.**

## Requirements

- Rust 1.85+
- League of Legends (macOS)
- `EnableReplayApi=1` in `game.cfg` (the app can write this)
- League Client running to open a `.rofl` (LCU)
- `ffmpeg` recommended to remux the game's `.webm.tmp` files

Repo: [github.com/syaor4n/leaguedirector-rs](https://github.com/syaor4n/leaguedirector-rs)

## Run (dev)

```bash
cd leaguedirector-rs
cargo run --release
```

## Package the `.app`

```bash
cargo install cargo-bundle
./scripts/macos-bundle.sh
# → target/release/bundle/osx/League Director.app  (ad-hoc signed, id `dev.leaguedirector.app`)
```

In System Settings, grant **League Director.app** (not `cargo`):

- Privacy & Security → Accessibility
- Privacy & Security → Input Monitoring
- Privacy & Security → Files and Folders (Documents)

## First launch (macOS)

1. `./scripts/macos-bundle.sh` then **Install to /Applications** (Connect tab), or copy the `.app` yourself.
2. Open **that** app (not `cargo run`) and grant Accessibility, Input Monitoring, and Files and Folders.
3. Tick your League install, Watch a `.rofl`. Director waits for the game process **and** `https://127.0.0.1:2999` (LCU 204 is not enough). If the game dies at loading, the Connect tab shows the r3dlog tail and Retry.
4. Keep the HUD on top while filming. Do not start another recording while the API is down after an encode.

Global hotkeys fire only while **League of Legends** (the game) is frontmost. Remap them in the Keys tab.

Recording: the game API often stalls while encoding. Director starts a watchdog and remuxes `.webm.tmp` → `.webm` (bundled `ffmpeg` if present). Clips show up in the Recording tab.

## Usage

1. Connect tab: tick your League install (`EnableReplayApi=1`).
2. Watch a `.rofl` from the list, or **Open .rofl…**. The app copies it into `Contents/LoL/Replays` and asks LCU to play it. Do not launch `LeagueofLegends` with the file as an argument (crash).
3. The app connects to `https://127.0.0.1:2999`.

Shortcuts (Director window **or** League frontmost): `Space` play/pause · `←`/`→` ±5 s · `K` keyframe · `⌘Z` undo · `Enter` play sequence.

Recording: prefer `Contents/LoL/Replays/director-captures` (the game can write there). The app remuxes `.webm.tmp` → `.webm` with ffmpeg.

Skyboxes: drop `.dds` files in `~/Documents/LeagueDirector/skyboxes` (Riot textures are not redistributed).

## License

Apache 2.0 — see `LICENSE` and `NOTICE`.
Riot skybox textures are not included.
