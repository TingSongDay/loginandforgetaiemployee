# MVP Buildup Summary Report

The `zeroclawagent` branch now has a real NeoHUman MVP runtime path instead of just a plan scaffold. The station runtime can launch two worker sessions through a managed browser abstraction, derive worker state from live platform checks at boot, persist worker session and control state, normalize visible messages, track dedupe state, make deterministic reply decisions, and run a polling loop that is structured for exactly-once reply behavior across restarts. Worker-scoped operator controls are also wired in through the CLI: `pause`, `resume`, `mark_login_complete`, and `mark_challenge_complete`.

## What Was Added

- Managed browser launch plans now carry real worker-specific invariants: profile dir, viewport, window origin, locale, timezone, user agent, zoom, and optional browser binary override.
- The station boot path now uses live platform truth instead of metadata-only assumptions:
  - launch browser
  - open platform workspace
  - detect `login_required` / `challenge_required` / `ready`
  - persist resulting worker state
- A full message pipeline now exists under `src/appliance/`:
  - `messages.rs`: normalized message records
  - `dedupe_store.rs`: persisted last-seen / processed / pending-reply state
  - `detector.rs`: inbound delta detection
  - `reply_engine.rs`: deterministic reply generation
  - `poller.rs`: long-running worker polling loop
- Station controls are exposed through the CLI and persisted through session metadata.
- Launch metadata is written per worker so runtime configuration is inspectable.

## Current MVP Capabilities

- Two-worker station model remains fixed: left and right worker tiles.
- Boot-time worker status is now driven by live platform detection.
- Session metadata and worker control state survive process restarts.
- Existing conversation history can be bootstrapped without replaying old messages.
- New inbound messages can be distinguished from already-seen messages in automated tests.
- Duplicate send prevention is modeled through pending-reply staging and processed-message commit.
- Deterministic replies are available as the first reply mode.

## Verification

The following test commands were run successfully:

```bash
cargo test --test component messaging_mvp_ -- --nocapture
cargo test --test integration messaging_mvp_ -- --nocapture
cargo test --test system messaging_mvp_ -- --nocapture
cargo test --tests
```

## Important Limits

This is still an MVP buildup, not a signed-off production inbox operator.

- The live/manual macOS validation gate from `MASTER_DEVELOPMENT_PLAN.md` is still pending.
- Real WeChat session reuse, real browser tiling behavior on the target Mac, and exactly-once reply behavior against a live account are not yet manually validated.
- The runtime structure is now in place, but the final confidence step is live station validation on the dedicated MacBook Neo.

## Recommended Next Step

Run the manual MVP gate on the target machine:

1. Launch both worker windows visibly.
2. Complete first login per worker.
3. Restart the station and confirm session reuse.
4. Send a fresh inbound message to one worker.
5. Confirm one deterministic reply only.
6. Repeat with restart during or after send to validate dedupe behavior.
