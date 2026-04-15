## Purpose (why this exists)

This project’s goal is to **validate whether “Bevy + AI (prompt-driven collaboration)” can realistically produce a playable game prototype** with controlled effort and repeatable iteration.  
The focus is not polish — it’s proving a tight loop that is *demoable* and *regression-testable by manual steps*.

## Validation result (conclusion)

Conclusion: **Yes, it’s feasible — and the validation goal was achieved.**

Through iterative, prompt-driven development, this repo reaches a Wolf3D-style baseline gameplay loop, including:

- **Movement & control**: walking and turning
- **UI & flow**: clickable UI, scene/state transitions
- **Rendering & assets**: texture/UI adaptation approach, frame-based animation control
- **Combat core**: hitscan ray checks, decals/impact feedback, taking damage and HP
- **Common systems**: weapons, items/pickups, audio integration, etc.

This demonstrates that even with a code-first engine (Bevy), AI prompts can effectively drive the engineering work from “empty repo” to a playable demo.

## Prompts used (condensed, reusable)

Below is a concise prompt template that repeatedly worked well (replace numbers/asset names as needed).

### 1) Lock the scope (goal + constraints)

- Build a “Wolf3D-like minimal playable demo”
- Must include: main menu → start game → move/shoot → hit feedback → take damage/lose HP → return to menu
- No feature sprawl: every new feature must be demoable and easy to re-check; otherwise reject it
- Only CC0 assets; record sources in `assets/**/README_CC0.txt`

### 2) Iterate in small, verifiable steps

- One iteration = one verifiable point (e.g., ray hit + decal + sound)
- Always provide: approach, key code locations, manual test steps, rollback boundary
- Keep it runnable: after each change, `cargo run` must launch and be playable

### 3) Quality gates (avoid “it runs but isn’t playable”)

- Prevent soft-locks: spawning/exit/obstacles must not overlap or block paths
- Consistent input: zero velocity when no input to prevent drifting
- Clear feedback: hits, damage, pickups, doors, teleports should have obvious feedback (sound/text/animation is fine)

## Expectations for this workflow (what we want next)

We want to turn this “AI prompt-driven development” into a more stable and scalable workflow:

- **Reusable**: standardized prompt/task templates (gameplay → assets → QA)
- **Auditable**: each iteration answers “what / why / how to verify”
- **Extensible**: keep adding weapons, AI, levels, and systems without rewriting the foundation

## Repository

- GitHub: `git@github.com:sconi789/BevyAiWolfenstein.git`

