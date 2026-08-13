# League Director — UI direction

Shipped as four desks (Connect / Look / Cut / Capture) plus a compact HUD that matches the deck.

## Metaphor

**Replay gallery / control room**, not a settings dialog.

- One **stage** (the game) and one **desk** (Director).
- The desk is a physical board: transport, look, timeline.

## Layout

```
┌─ title: League Director · live 12:04 · keys on ───────────┐
│ [Connect] [Look] [Cut] [Capture]     Undo   Reset   HUD   │
├───────────────────────────────────────────────────────────┤
│ Look: FOV · presets · saved looks · Camera/Sky/Fog/DOF    │
│ Cut:  full-width tracks + Visibility / Particles rail     │
│ Capture: folder, codec, marked range, clips               │
└───────────────────────────────────────────────────────────┘
┌─ deck: PLAY −5 +5 K SEQ IN OUT REC     12:04 / 31:03 ─────┐
│ ████████████░░░░  (amber playhead, green In, red Out)     │
└───────────────────────────────────────────────────────────┘
┌─ HUD (always on top) ─────────────────────────────────────┐
│  same deck language, compact                              │
└───────────────────────────────────────────────────────────┘
```

| Desk | Role |
|---|---|
| **Connect** | installs, Watch, LCU/API lamps. Permissions only when missing. Key bindings drawer. |
| **Look** | FOV first, cinematic / gameplay / broadcast / reset, saved looks, Camera / Sky / Fog / DOF groups |
| **Cut** | sequencer hero + visibility + particles as a right rail |
| **Capture** | record folder, remux status, clips. In/Out live on the deck. |

## Color

- Background `#141416`
- Panel `#1C1C20`
- Live green `#3DCF8E`
- Rec red `#E24B4A`
- Keyframe amber `#E8A23A`
- Text `#E8E8EC` / mute `#7A7A82`

## Recording

Webm encodes hang `:2999`. Default Rec is playhead +8s. In/Out on the deck. Webm clamped to 16s. Recover remux before Watch if the API died.

## What we will not do

- Floating Qt-style window maze
- Glassmorphism / marketing grids
- Auto-start a test encode
