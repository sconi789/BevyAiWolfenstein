## Bevy + AI Wolf3D Demo (validation project)

**Languages**: [中文](README.zh.md) | [日本語](README.ja.md) | [English](README.en.md)

## Goal & result

This repo validates a simple question: **Can Bevy + AI (natural-language, prompt-driven development) produce a playable 3D FPS demo?**  
Result: **Yes — validation achieved.** The demo includes a playable loop, clickable UI/state transitions, texture/adaptation approach, audio, hitscan ray hits, decals/impact feedback, taking damage + HP, frame animation control, weapons/items, etc.

Repo: `git@github.com:sconi789/BevyAiWolfenstein.git`

## Controls (source-of-truth: code)

- **Move**: W / A / S / D
- **Sprint**: Left Shift
- **Look**: Arrow keys (← → ↓ ↑)
- **Shoot**: Space (hold to fire)
- **Interact / open door**: E
- **Weapon switch**: 1 (pistol), 2 (shotgun, after unlocked)
- **Use medkit**: H (when you have medkits and HP is not full)
- **Minimap**: M (toggle exists in code; if UI is not wired, ignore)
- **Debug**:
  - F1: enemy preview toggle
  - `[` / `]`: preview frame step

## Architecture (Rust / Bevy / Hermes / Harness)

- **Rust + Bevy 0.14**: engine/runtime (rendering, UI, audio)
- **bevy_rapier3d**: physics + ray casting (hitscan)
- **`crates/hermes` (Hermes)**: a tiny event bus used to log decisions/outputs by role (Producer/Designer/Programmer/Art/QA) and topic (Gate/Gameplay/QA…)
- **`crates/harness` (Harness)**: gates & acceptance. It validates `assets/level_plan.yaml` at startup and fails fast if the spec is invalid
- **Asset policy**: CC0-only assets under `assets/`, with sources recorded in `assets/**/README_CC0.txt`

Useful files:
- `assets/level_plan.yaml` (validated by Harness)
- `docs/AI-FPS-DEMO-PROMPTS.md` (prompt/workflow notes)
- `docs/ABOUT.*.md` (tri-lingual in-game “About” text)

## How Hermes helped keep direction & reduce repeated mistakes

- **Make direction/gates explicit**: log “what is allowed / rejected” as `ProducerGate` events instead of leaving it implicit in chat history
- **Bake regression into prompts**: every iteration must remain runnable (`cargo run`) and verifiable quickly, otherwise rollback/reject
- **Lower handoff cost across “roles”**: when the AI switches roles, Hermes logs act as a shared, auditable record of decisions and outcomes

## How AI was used to find and apply free (CC0) assets

Working constraints:
- CC0 only
- After adding an asset, write its source URL + filename into `assets/**/README_CC0.txt`

Common sources:
- OpenGameArt: `https://opengameart.org/`
- Kenney: `https://kenney.nl/assets`
- Noto CJK: `https://github.com/googlefonts/noto-cjk`

## Prompting to match frame animations (UV / sprite sheet)

The key is to turn “frame index → UV range” plus rules like **v-flip**, frame order, and per-frame timing into explicit constraints, then iterate via prompts until the preview looks correct.  
This repo includes a simple visual verification loop (enemy preview toggle + frame stepping) so alignment can be adjusted quickly.

