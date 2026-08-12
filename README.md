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
