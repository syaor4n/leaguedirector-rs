# League Director

Native macOS desk for the League of Legends [Replay API](https://developer.riotgames.com/docs/lol). Independent Rust rewrite of the Apache 2.0 [RiotGames/leaguedirector](https://github.com/RiotGames/leaguedirector) project.

**Not published or endorsed by Riot Games.**

## Download

Latest: **[League Director 0.6.0](https://github.com/syaor4n/leaguedirector-rs/releases/tag/v0.6.0)** (`League.Director-0.6.0.dmg`).

The `.app` is ad-hoc signed, not notarized. First open: right-click → **Open**. Drag it into `/Applications`. Do not run `cargo` if you want hotkeys to stick — TCC must attach to **League Director.app**.

## First launch

In **System Settings → Privacy & Security**, grant **League Director.app**:

- Accessibility
- Input Monitoring
- Files and Folders (Documents)

The app can prompt for Accessibility and Input Monitoring by name. Open Settings only if that prompt never appears.

Put League in **borderless** window mode. Exclusive Metal fullscreen swallows keys and hides the HUD.

1. Connect: tick your League install (`EnableReplayApi=1` in `game.cfg`).
2. Watch a `.rofl` from the list, or **Open .rofl…**. Director copies it into `Contents/LoL/Replays` and asks the League Client (LCU) to play it. Do **not** launch `LeagueofLegends` with the file as an argument — that crashes.
3. Wait until the **Game** and **API** lamps are on. LCU `204` is not a live replay; Director waits for the game process **and** `https://127.0.0.1:2999`.

## Desks

The bottom **deck** is always there: play / ±5s / K / SEQ / In / Out / Rec, plus a scrubber.

| Desk | What it is |
|---|---|
| **Connect** | Installs, replay list, Watch, LCU / Game / API lamps. Permissions only when missing. Key bindings drawer. |
| **Look** | FOV first. Cinematic / Gameplay / Broadcast / Reset. Saved looks. Camera / Sky / Fog / DOF. |
| **Cut** | Sequencer (POS / ROT / FOV / SPD / FOG / DOF / SKY / NEAR). Visibility and particles on the right rail. |
| **Capture** | Folder, codec, marked range, remux status, clip list. In / Out live on the deck. |

The **HUD** is a compact always-on-top copy of the deck. Leave it up while League is frontmost.

## Filming

- **Look** — FOV, then a preset. Save a look (grade only: FOV, sky, fog, DOF — not camera pose) to `~/Documents/LeagueDirector/looks`.
- **Cut** — `K` keyframes camera + FOV. Drag diamonds on the tracks. Play sequence from the deck.
- **Capture** — mark **In** / **Out** on the deck, or leave them empty. **Rec** without marks is playhead + 8s. **CLR** clears marks.

Webm encodes hang the Replay API. Clips are clamped to **16 seconds**. Prefer the League folder `Contents/LoL/Replays/director-captures` (the game can write there; Documents is often blocked by TCC). Director remuxes leftover `.webm.tmp` with bundled `ffmpeg`. If `:2999` dies after a take, use **Recover remux** before Watch again.

## Shortcuts

Work in the Director window **and** while **League of Legends** is frontmost (not while you are in Finder, Chrome, etc.). Remap under Connect → Key bindings.

| Key | Action |
|---|---|
| `Space` | Play / pause |
| `←` / `→` | −5s / +5s |
| `K` | Keyframe camera + FOV (opens Cut) |
| `Enter` | Play sequence |
| `⌘Z` / `⇧⌘Z` | Undo / redo look or sequence |
| `R` | Record toggle (same as deck Rec) |

## Skyboxes

Drop `.dds` files in `~/Documents/LeagueDirector/skyboxes`. Riot textures are not redistributed.

## Build from source

Needs Rust 1.85+, a League of Legends install, and the League Client to Watch a `.rofl`. `ffmpeg` is optional when running from cargo (the `.app` bundles it).

```bash
cargo run --release
```

Package:

```bash
cargo install cargo-bundle
./scripts/macos-bundle.sh
# → target/release/bundle/osx/League Director.app
#    ad-hoc signed, id `dev.leaguedirector.app`
```

Then **Install to /Applications** on the Connect desk, or copy the `.app` yourself.

## License

Apache 2.0 — see `LICENSE` and `NOTICE`.
Riot skybox textures are not included.
