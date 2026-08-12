# League Director — UI direction (not implemented yet)

Approve this before we restyle the live app.

## Problem

The current UI is a dark egui prototype: eight equal tabs, raw sliders, debug chrome. It works. It does not feel like a broadcast desk.

## Metaphor

**Replay gallery / control room**, not a settings dialog.

- One **stage** (the game) and one **desk** (Director).
- The desk is a physical board: transport, look, timeline. Not a website.

## Layout

```
┌─ title: League Director · live 12:04 · keys on ───────────┐
│ [Connect] [Look] [Cut] [Capture]     Undo   Reset   HUD   │
├──────────────┬────────────────────────────────────────────┤
│ left rail    │ main                                       │
│ replay list  │   transport (play, ±5, K, seq)             │
│ watch        │   FOV / cam / presets                      │
│ LCU · API    │   timeline tracks                          │
│ permissions  │                                            │
│ (only if bad)│                                            │
└──────────────┴────────────────────────────────────────────┘
┌─ HUD (always on top, ~420×72) ────────────────────────────┐
│  ▶  −5  +5  K  SEQ   │  12:04.1 / 31:03   keys  Undo Reset│
└───────────────────────────────────────────────────────────┘
```

Four desks instead of eight tabs:

| Desk | Today |
|---|---|
| **Connect** | installs, Watch, LCU/API lights. Permissions only when missing. |
| **Look** | camera, FOV, sky, fog, DOF, cinematic / gameplay / reset |
| **Cut** | sequencer + visibility + particles as drawers |
| **Capture** | record, clips, remux status |

## HUD (borderless)

Always-on-top, no debug `lsappinfo` dump. Type large enough to read at a glance. Undo / Reset / keys lamp. This can ship in a small pass before the full restyle.

## Color

Keep it dark. Not purple-gradient SaaS.

- Background `#161618`
- Panel `#1E1E22`
- Live green `#3DCF8E`
- Rec red `#E24B4A`
- Keyframe amber `#E8A23A`
- Text `#E6E6EA` / mute `#8B8B93`

No rainbow sliders. One accent (amber) for the playhead and primary actions.

## Type

- UI: system UI (already what egui uses) — 13px body, 11px mute
- Timecode: tabular / monospaced

## What we will not do

- Floating Qt-style window maze (the Python original)
- Glassmorphism, hero gradients, 12-column marketing grid
- Restyle before this direction is signed off

## First implementation slice (after OK)

1. HUD compact (can start earlier)
2. Top bar + 4 desks
3. Connect without false Fix
4. Look + Cut
5. Capture
