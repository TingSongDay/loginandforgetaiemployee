# NeoHUman
## Master Development Plan

**Version:** 5.0  
**Date:** March 17, 2026  
**Status:** Detailed MVP Execution Plan  
**Target Runtime Base:** `https://github.com/TingSongDay/zeroclawagent`  
**Target Deployment:** `NeoHUman Station` on a dedicated always-on MacBook Neo

---

## 1. Objective

Build `NeoHUman Station`, a narrow local AI employee appliance on top of the ZeroClaw fork, that can:

1. run on one dedicated always-on MacBook Neo
2. manage a visible dual-surface station from the beginning
3. keep one fixed left automation surface and one right operator panel open at all times
4. let a human complete first login, QR scan, 2FA, CAPTCHA, and recovery on the left surface
5. restore the left surface browser session on later runs
6. read messages from WhatsApp Web running in the left managed kiosk browser
7. detect a newly arrived inbound message without replaying old history
8. send one deterministic reply for one inbound trigger
9. expose operator state, intervention prompts, and controls through the right NeoHUman panel

This MVP is not a general-purpose browser agent. It is a visible dual-surface inbox appliance:

- **Left automation surface:** fixed WhatsApp Web kiosk browser used for deterministic automation
- **Right operator panel:** local NeoHUman AI chat and status surface used by the human operator

---

## 2. Ground Rules

### 2.1 Delivery Rule

Each step must:

1. be implemented in isolation
2. have its own test coverage
3. pass unit tests and validation
4. be merged only after the step gate is green

Do not begin the next step until the current step passes its gate.

### 2.2 Scope Rule

Do not strip large parts of ZeroClaw on day one.

For MVP speed:

- first disable unused capabilities by config and build profile
- then isolate the station appliance path
- only remove unused modules after the MVP path is stable

This avoids breaking the fork before the kiosk browser workflow is proven.

### 2.3 Interaction Rule

Use this execution stack in order:

1. DOM and state probes
2. fixed-window mapped interactions
3. screenshots only for diagnostics or recovery

The happy path must not depend on screenshot-driven reasoning.

### 2.4 Human Handoff Rule

The human is responsible for:

- first login
- QR scan
- CAPTCHA
- 2FA
- suspicious-login checkpoints
- recovery from unexpected UI changes

The product must preserve one shared managed browser environment for the left WhatsApp surface so both human and agent can use the same persistent session when needed.

### 2.5 Kiosk Rule

Treat the left WhatsApp browser as a controlled kiosk surface.

That means:

- one app-owned browser profile
- one fixed browser binary
- one canonical window origin and size
- one canonical zoom
- one standardized display scaling assumption
- one canonical WhatsApp Web workspace state

Before any mapped interaction, the runtime must:

1. re-anchor the left window to canonical bounds
2. verify actual placement and expected UI anchors
3. refuse to guess if verification fails
4. attempt deterministic recovery before escalating

The right operator panel is explicitly excluded from kiosk invariants.

---

## 3. Technical Assumptions

### 3.1 Platform

- One dedicated `NeoHUman Station` MacBook Neo
- Always on
- Connected to power and network
- Left automation surface is a visible Brave or Chromium-compatible browser window
- Right operator panel is a visible local NeoHUman app or web UI window
- App data under `~/Library/Application Support/NeoHUman/`

### 3.2 Display and Kiosk Assumptions

- Canonical left-surface bounds default to `(0,0)` and `1920x1080`
- Browser zoom is fixed and not treated as a variable
- Retina or display scaling is standardized as an install prerequisite
- Sidebar width drift is out of scope because the kiosk environment is controlled
- If WhatsApp changes the UI materially, NeoHUman will be updated rather than adapting to arbitrary drift at runtime

### 3.3 ZeroClaw Baseline

The fork already provides the right seams:

- browser tool registration in `src/tools/mod.rs`
- browser config in `src/config/schema.rs`
- browser backend implementation in `src/tools/browser.rs`
- security policy in `src/security/policy.rs`
- runtime abstractions in `src/runtime/`
- test layers in `tests/component`, `tests/integration`, `tests/system`, `tests/live`

### 3.4 Browser Strategy

For MVP:

- use `rust_native` as the production behavior target
- keep `agent_browser` available only for development iteration and debugging
- use a managed Brave or Chromium-compatible binary for the left kiosk surface

Why:

- `rust_native` gives tighter control over browser version, executable path, window size, window origin, and profile ownership
- a managed Chromium-family browser is required for deterministic kiosk behavior
- `agent_browser` remains useful for debugging but is not the runtime to optimize around

### 3.5 Product Shape

The delivered product is `NeoHUman Station`:

- a branded local desktop appliance for the dedicated MacBook Neo
- with one local supervisor runtime
- controlling one managed left kiosk browser and one independent right operator panel
- with one isolated browser profile and session store for the left WhatsApp surface

Surface model for MVP:

- left automation surface = WhatsApp Web managed kiosk browser
- right operator panel = local NeoHUman AI chat, controls, and status surface

The customer does not manually install ZeroClaw.

---

## 4. Success Criteria

The MVP is done only when all of the following are true:

1. The appliance launches the left kiosk browser with fixed runtime settings every time.
2. The appliance launches the right operator panel independently of kiosk geometry.
3. The left kiosk browser restores to canonical geometry before mapped interactions.
4. The operator can log in manually once on the left surface and the session is saved.
5. The left kiosk browser session restores after restart.
6. The left surface can read its active WhatsApp Web conversation state.
7. The left surface can distinguish a new inbound message from already-seen history.
8. The left surface can send one deterministic reply for one inbound trigger.
9. Duplicate replies are prevented across restarts.
10. The appliance exposes clear states for `ready`, `login_required`, `challenge_required`, `paused`, and `error`.
11. The right operator panel shows station state, last action, last message, and intervention requirements.
12. Screenshots are not required for the normal send path.
13. The local `NeoHUman` supervisor service restarts cleanly after process failure.
14. All phase gates below are green.

---

## 5. Delivery Sequence

## Phase 0. Baseline and Test Harness

### Step 0.1 Freeze the fork baseline

**Goal**

Create a stable starting point in `zeroclawagent` before any product work.

**Implementation**

- create `mvp/browser-messaging` branch in the fork
- record baseline commit SHA in internal notes
- confirm the repo builds on the target macOS machine
- confirm current tests run and document existing failures

**Code touchpoints**

- none yet, baseline only

**Unit tests**

- no new tests

**Validation**

- run `cargo test --test component`
- run `cargo test --test integration`
- run `cargo test --test system`
- record current failures and mark them as inherited or blocker

**Exit gate**

- baseline build reproducible on macOS
- known test status documented
- no uninvestigated failures remain

---

### Step 0.2 Add an MVP test lane

**Goal**

Create a dedicated test lane for the dual-surface appliance workflow so future work does not disappear inside ZeroClaw's broad test surface.

**Implementation**

- add new test files:
  - `tests/component/messaging_mvp_config.rs`
  - `tests/integration/messaging_mvp_flow.rs`
  - `tests/system/messaging_mvp_runtime.rs`
- add test helpers under `tests/support/` for mock browser state, kiosk placement fixtures, mapped interaction fixtures, and message fixtures
- define naming convention for all MVP tests

**Code touchpoints**

- `tests/component/`
- `tests/integration/`
- `tests/system/`
- `tests/support/`

**Unit tests**

- add initial placeholder tests that verify the new test harness compiles and runs

**Validation**

- run only the new test files
- confirm CI or local command list is documented for MVP test execution

**Exit gate**

- dedicated MVP test lane exists
- new tests compile and run cleanly

---

## Phase 1. Product Slice and Config Skeleton

### Step 1.1 Introduce a dedicated station config section

**Goal**

Create a first-class configuration surface for the station appliance instead of overloading generic browser settings.

**Implementation**

- add or revise a `station` config section in `src/config/schema.rs`
- include:
  - `enabled`
  - `platform`
  - `message_poll_interval_ms`
  - `reply_mode`
  - `operator_display_name`
  - `manual_intervention_timeout_secs`
  - `left_surface`
  - `right_panel`
- define the left surface as kiosk-specific config, not a symmetric worker
- define the right panel as operator-surface config, not a browser worker
- document the config in local docs

**Public interfaces**

`StationConfig` must evolve from symmetric worker tiles to an asymmetric model:

- `left_surface`
  - `browser_binary_path`
  - `profile_name`
  - `workspace_name`
  - `canonical_window_origin_x`
  - `canonical_window_origin_y`
  - `canonical_viewport_width`
  - `canonical_viewport_height`
  - `zoom_percent`
  - `display_scale_mode`
  - `snap_back_before_interaction`
  - `preflight_verification_enabled`
- `right_panel`
  - `enabled`
  - `runtime_mode`
  - `local_url_or_path`
  - optional visual defaults only
  - no canonical geometry requirement

**Code touchpoints**

- `src/config/schema.rs`
- config docs or generated schema docs

**Unit tests**

- parse default station config
- parse explicit left-surface kiosk config
- parse explicit right-panel config
- reject invalid canonical geometry
- reject empty required strings

**Validation**

- run new component tests for config parsing
- verify config serialization round-trip

**Exit gate**

- station config is stable
- invalid config fails early with clear errors

---

### Step 1.2 Register an explicit station runtime mode

**Goal**

Prevent the MVP from depending on ZeroClaw's broader agent loop by defining a narrow runtime mode for the station appliance.

**Implementation**

- add a new runtime entry path, for example:
  - `src/appliance/mod.rs`
  - `src/appliance/runtime.rs`
  - `src/appliance/state.rs`
  - `src/appliance/supervisor.rs`
- define the core state machine:
  - `booting`
  - `login_required`
  - `challenge_required`
  - `ready`
  - `processing_message`
  - `paused`
  - `error`
- define a supervisor model that owns:
  - left automation surface registry
  - right operator panel lifecycle
  - station health
  - intervention state
- add a CLI entry or startup entrypoint for this mode

**Code touchpoints**

- `src/main.rs`
- `src/lib.rs`
- new `src/appliance/` module

**Unit tests**

- state transitions are valid
- invalid transitions are rejected
- default startup state is correct
- left surface and right panel initialization are deterministic

**Validation**

- run component tests for state machine transitions
- confirm the new runtime mode starts the kiosk surface and operator panel without invoking unrelated channels

**Exit gate**

- a dedicated station runtime exists
- station state machine is test-covered

---

## Phase 2. Managed Kiosk Browser Foundation

### Step 2.1 Define the managed kiosk browser contract

**Goal**

Make the fixed left-browser environment explicit and testable.

**Implementation**

- add a managed kiosk browser config model with:
  - surface id
  - browser binary path
  - persistent user data dir
  - canonical viewport width
  - canonical viewport height
  - canonical zoom
  - canonical window x/y origin
  - fixed timezone
  - fixed locale
  - fixed user agent
  - display scale mode
  - headed mode only for MVP
  - snap-back enable flag
  - preflight verification enable flag
- define one canonical WhatsApp Web landing or workspace state for automation
- keep right-panel config outside the kiosk contract

**Runtime guarantees**

- before coordinate-based action, the runtime re-anchors the left window to canonical bounds
- after re-anchor, the runtime verifies placement and expected UI anchors
- if verification fails, the runtime does not guess
- the runtime first attempts deterministic recovery
- screenshots are captured only when recovery fails or evidence is needed

**Code touchpoints**

- `src/config/schema.rs`
- `src/appliance/`

**Unit tests**

- defaults are correct
- invalid dimensions fail
- invalid browser path fails validation
- invalid display scale mode fails validation

**Validation**

- verify config docs and schema output

**Exit gate**

- managed kiosk browser contract is stable and validated

---

### Step 2.2 Build a kiosk browser runtime abstraction

**Goal**

Stop coupling the appliance directly to the current `BrowserTool` implementation details.

**Implementation**

- add or revise a trait such as `ManagedBrowserRuntime`
- methods should cover:
  - `launch()`
  - `connect()`
  - `open_url()`
  - `snapshot()`
  - `click()`
  - `fill()`
  - `type_text()`
  - `get_text()`
  - `screenshot()`
  - `ensure_canonical_placement()`
  - `verify_canonical_placement()`
  - `preflight_check()`
  - `capture_recovery_artifacts()`
  - `close()`
- create two implementations:
  - development adapter using existing `BrowserTool`
  - production-oriented adapter targeting `rust_native`
- create a placement manager component that:
  - owns canonical bounds for the left kiosk surface
  - repositions the left window if it drifts
  - verifies that the right panel is not subject to kiosk enforcement

**Code touchpoints**

- new `src/appliance/browser_runtime.rs`
- new `src/appliance/tile_manager.rs` or successor placement manager
- `src/tools/browser.rs`

**Unit tests**

- trait contract tests using mocks
- launcher returns deterministic kiosk config object
- browser action mapping is correct
- canonical placement mapping is correct

**Validation**

- integration test with mocked runtime proves the supervisor can launch the kiosk surface and operator panel correctly

**Exit gate**

- station runtime does not depend on raw tool-call JSON
- browser interactions and canonical placement are abstracted and mockable

---

### Step 2.3 Launch the left kiosk browser with fixed invariants

**Goal**

Make left-surface browser launch reproducible.

**Implementation**

- enforce on the left surface:
  - canonical window origin `(0,0)` by default
  - canonical viewport `1920x1080` by default
  - fixed zoom
  - fixed locale
  - fixed timezone
  - fixed user agent
  - fixed persistent user data dir
  - fixed browser executable
- use an app-owned Brave or Chromium-compatible profile path
- capture launch metadata in logs
- launch the right operator panel separately with no placement enforcement

**Code touchpoints**

- `src/tools/browser.rs`
- `src/appliance/browser_runtime.rs`
- `src/appliance/runtime.rs`
- placement manager module

**Unit tests**

- launch options built correctly
- missing binary path yields clear error
- user data dir is deterministic
- canonical coordinates are deterministic

**Validation**

- manual validation on target Mac:
  - left browser launches at canonical bounds
  - right panel launches independently
  - relaunch preserves the correct profile
  - viewport is unchanged across restarts

**Exit gate**

- managed kiosk browser launch is deterministic on the target machine

---

### Step 2.4 Add snap-back and re-anchor behavior

**Goal**

Ensure the left surface can be manually moved and still be restored before automation.

**Implementation**

- detect left-window drift from canonical bounds
- reset the left window to canonical bounds before every mapped interaction
- verify actual bounds after snap-back
- fail closed if bounds cannot be restored
- leave the right operator panel untouched

**Code touchpoints**

- `src/appliance/browser_runtime.rs`
- placement manager module
- `src/appliance/runtime.rs`

**Unit tests**

- drift is detected
- snap-back resets bounds
- failed re-anchor blocks mapped actions
- right panel is never included in snap-back logic

**Validation**

- manually move or resize the left window, then trigger an action and confirm snap-back occurs before interaction

**Exit gate**

- left-window drift is corrected deterministically before mapped actions

---

### Step 2.5 Add a preflight verifier

**Goal**

Confirm the left kiosk surface is in a safe known state before mapped interactions.

**Implementation**

- add a preflight check that verifies:
  - canonical placement
  - expected WhatsApp Web state
  - presence of required UI anchors
  - absence of blocking modal or unknown overlay
- reject action execution when in login, modal, or unknown states

**Code touchpoints**

- `src/appliance/browser_runtime.rs`
- `src/appliance/runtime.rs`
- platform driver module introduced later

**Unit tests**

- preflight passes in expected state
- preflight fails on login state
- preflight fails on modal or overlay state
- preflight failure blocks mapped interactions

**Validation**

- manual validation against expected and blocked states in a real browser

**Exit gate**

- mapped actions are gated by deterministic preflight checks

---

## Phase 3. Session Persistence

### Step 3.1 Implement appliance-owned session storage

**Goal**

Persist authentication and browser state in a way the appliance owns and can restore.

**Implementation**

- create a session manager module:
  - `src/appliance/session_store.rs`
- store:
  - left surface id
  - browser user data dir reference
  - session metadata
  - last successful login timestamp
  - session validity markers
- do not store raw secrets in plain text
- define corruption handling and reset flow

**Code touchpoints**

- `src/appliance/session_store.rs`
- `src/config/schema.rs`
- possibly `src/security/secrets.rs`

**Unit tests**

- session metadata writes and reads
- corrupted session metadata fails safely
- reset deletes only appliance-owned state
- right-panel state and left-session state remain isolated

**Validation**

- manual test:
  - log in once on the left surface
  - restart appliance
  - confirm the left session remains active

**Exit gate**

- session restore works across process restart
- session corruption produces `login_required`, not undefined behavior

---

### Step 3.2 Add explicit login and challenge states

**Goal**

Make operator intervention first-class instead of implicit failure.

**Implementation**

- add login detection hooks
- add challenge detection hooks
- wire station state transitions:
  - `ready -> login_required`
  - `ready -> challenge_required`
  - `challenge_required -> ready`
- add operator-visible reason codes
- make intervention state specific to the left WhatsApp surface while surfacing prompts in the right panel

**Code touchpoints**

- `src/appliance/runtime.rs`
- `src/appliance/state.rs`
- platform-specific selector logic module introduced in next phase

**Unit tests**

- login-required detection from mocked browser state
- challenge-required detection from mocked browser state
- state transitions preserve last error context
- right-panel status remains available during intervention states

**Validation**

- manual validation using forced logout or session invalidation

**Exit gate**

- operator intervention states are deterministic and test-covered

---

## Phase 4. WhatsApp Web Platform Connector

### Step 4.1 Create a platform-driver interface

**Goal**

Separate generic station runtime from site-specific message logic.

**Implementation**

- add trait such as `MessagingPlatformDriver`
- required methods:
  - `open_workspace()`
  - `detect_login_state()`
  - `detect_challenge_state()`
  - `detect_ui_state()`
  - `list_visible_messages()`
  - `extract_message_id()`
  - `extract_message_text()`
  - `extract_message_direction()`
  - `focus_reply_box()`
  - `send_reply()`
  - `perform_mapped_action()`
- implement one driver only for MVP, targeting WhatsApp Web first
- allow the trait shape to stay generic enough for later reuse

**Required WhatsApp state categories**

- `login_required`
- `chat_list_visible`
- `chat_open`
- `search_open`
- `modal_open`
- `unexpected_overlay`
- `error_or_unknown`

**Code touchpoints**

- new `src/appliance/platforms/mod.rs`
- new `src/appliance/platforms/whatsapp_web.rs`

**Unit tests**

- trait mock tests
- state classification tests
- message normalization tests

**Validation**

- integration test with fixture DOM snapshots or browser action mocks

**Exit gate**

- station runtime is platform-agnostic
- one real WhatsApp Web driver exists for MVP

---

### Step 4.2 Define and freeze selector maps

**Goal**

Make selector drift manageable and testable.

**Implementation**

- create platform selector configuration for:
  - conversation list
  - message list
  - incoming message node
  - outgoing message node
  - reply input
  - send button
  - login markers
  - challenge markers
  - modal markers
  - overlay markers
  - search input
  - chat open markers
- keep selectors in config or structured constants, not inline strings

**Code touchpoints**

- `src/appliance/platforms/whatsapp_web.rs`
- optional selector config file

**Unit tests**

- selector config loads correctly
- selector set is complete
- missing required selectors fail fast

**Validation**

- manual browser inspection against the target site
- confirm selectors resolve on the current WhatsApp Web version

**Exit gate**

- selector map is explicit, versioned, and validated

---

### Step 4.3 Add a mapped-interaction layer

**Goal**

Define fast deterministic actions for the controlled kiosk surface.

**Implementation**

- define a mapped interaction specification for WhatsApp Web
- enumerate deterministic actions:
  - focus search
  - search chat
  - open chat
  - focus composer
  - type message
  - send message
  - dismiss known modal
- each action must define:
  - required preflight state
  - primary execution path
  - verification of success
  - recovery behavior if verification fails

**Execution order**

1. DOM and state detection to identify current state
2. mapped interaction for fast action execution
3. targeted DOM verification after action
4. screenshot and log capture only on recovery or failure

**Code touchpoints**

- `src/appliance/platforms/whatsapp_web.rs`
- `src/appliance/browser_runtime.rs`
- `src/appliance/runtime.rs`

**Unit tests**

- mapped action routing by state
- post-action verification succeeds in happy paths
- recovery routing triggers when verification fails

**Validation**

- integration tests on mocked WhatsApp states
- manual validation in the live kiosk browser

**Exit gate**

- fast mapped interactions are deterministic and verifiable

---

## Phase 5. Conversation Reading

### Step 5.1 Implement normalized message extraction

**Goal**

Read the active WhatsApp conversation and convert it into stable message records.

**Implementation**

- create `MessageRecord` model:
  - `surface_id`
  - `platform_message_id`
  - `timestamp`
  - `direction`
  - `author`
  - `text`
  - `raw_fingerprint`
- extract messages from browser state
- normalize whitespace and duplicate formatting noise

**Code touchpoints**

- `src/appliance/messages.rs`
- `src/appliance/platforms/whatsapp_web.rs`

**Unit tests**

- normalize incoming messages
- normalize outgoing messages
- handle empty text safely
- stable ID generation when platform IDs are absent

**Validation**

- integration test using fixture conversations
- manual validation against a live test account

**Exit gate**

- active conversation can be read into normalized records reliably

---

### Step 5.2 Persist last-seen conversation state

**Goal**

Prepare for new-message detection and duplicate prevention.

**Implementation**

- create local conversation checkpoint store
- persist:
  - left surface id
  - last seen message ID
  - last seen inbound message ID
  - last processed message hash
  - last reply correlation key

**Code touchpoints**

- `src/appliance/dedupe_store.rs`
- `src/appliance/messages.rs`

**Unit tests**

- checkpoint save and load
- idempotent update behavior
- corruption handling
- left surface checkpoints remain isolated from operator-panel state

**Validation**

- restart test proves state survives process restart

**Exit gate**

- last-seen state persists correctly and is safe to reuse

---

## Phase 6. New Message Detection

### Step 6.1 Implement inbound delta detection

**Goal**

Determine whether a truly new inbound message has arrived.

**Implementation**

- compare current normalized message list to checkpoint store
- evaluate the left surface independently of right-panel state
- detect only inbound messages not yet processed
- ignore:
  - previously seen history
  - the agent's own outgoing messages
  - transient duplicate DOM nodes

**Code touchpoints**

- `src/appliance/detector.rs`
- `src/appliance/messages.rs`
- `src/appliance/dedupe_store.rs`
- `src/appliance/supervisor.rs`

**Unit tests**

- new inbound message detected
- no false positive on restart
- no false positive on outgoing message
- duplicate DOM node ignored

**Validation**

- integration test with simulated conversation evolution
- live validation with second test account sending messages

**Exit gate**

- detector never re-processes the same inbound message in test scenarios

---

### Step 6.2 Add polling loop with backoff

**Goal**

Turn detection into a stable long-running loop.

**Implementation**

- add supervisor poller
- poll on configured interval
- add backoff for transient browser failures
- add max retry threshold before entering `error`

**Code touchpoints**

- `src/appliance/runtime.rs`
- `src/appliance/poller.rs`
- `src/appliance/supervisor.rs`

**Unit tests**

- polling interval honored
- transient failures back off
- fatal failures escalate to `error`
- right-panel availability does not mask left-surface failure

**Validation**

- system test running for at least 30 minutes with the kiosk browser and operator panel active without memory growth or duplicate processing

**Exit gate**

- poller is stable under repeated read cycles

---

## Phase 7. Reply Engine

### Step 7.1 Implement deterministic reply generation

**Goal**

Start with rule-based replies, not free-form AI generation.

**Implementation**

- add a reply engine interface
- start with deterministic modes only:
  - static reply
  - rule table
  - template with variable substitution
- capture response decision in logs
- surface the chosen reply and rationale in the right operator panel

**Code touchpoints**

- `src/appliance/reply_engine.rs`
- `src/config/schema.rs`

**Unit tests**

- reply selected for known message
- no reply for unsupported message
- variables interpolate correctly

**Validation**

- integration test with message fixtures and expected outputs

**Exit gate**

- reply engine is deterministic, testable, and explainable

---

### Step 7.2 Implement send path and duplicate protection

**Goal**

Send exactly one reply for one inbound trigger.

**Implementation**

- use the platform driver to focus input and send
- use mapped interactions only after preflight passes
- write send outcome to dedupe store before or after send with a safe two-step design
- prevent duplicate sends across restart or retry

**Code touchpoints**

- `src/appliance/runtime.rs`
- `src/appliance/reply_engine.rs`
- `src/appliance/dedupe_store.rs`
- `src/appliance/platforms/whatsapp_web.rs`

**Unit tests**

- successful send marks message processed
- retry after crash does not duplicate reply
- send failure does not falsely mark complete
- preflight failure blocks send

**Validation**

- live validation with a second account
- crash-and-restart test during send path

**Exit gate**

- one inbound message produces at most one reply in all tested restart scenarios

---

## Phase 8. Human Handoff and Safety

### Step 8.1 Add manual pause, resume, and takeover states

**Goal**

Let the operator safely intervene without fighting the agent.

**Implementation**

- add commands or UI hooks for:
  - `pause`
  - `resume`
  - `mark_login_complete`
  - `mark_challenge_complete`
- when paused, stop send actions but keep status visible
- show intervention prompts in the right panel
- keep the left surface available for direct human recovery when needed

**Operator recovery list**

- login expired
- QR login required
- modal or overlay blocking interaction
- unexpected UI drift
- browser relaunch needed
- preflight verification failure

**Code touchpoints**

- `src/appliance/runtime.rs`
- CLI entrypoints or right-panel integration

**Unit tests**

- pause prevents send
- resume returns to ready
- manual completion clears intervention state
- intervention state appears in operator panel status

**Validation**

- manual validation during live browser session

**Exit gate**

- operator can safely take over and return control

---

### Step 8.2 Restrict shell and browser scope for station mode

**Goal**

Prevent the MVP from behaving like a general agent with broad host power.

**Implementation**

- define a strict station security profile:
  - narrow browser allowlist
  - minimal shell allowlist
  - no general arbitrary file editing
- ensure station mode does not expose unrelated tools by default
- ensure coordinate-based actions are scoped only to the left kiosk browser

**Code touchpoints**

- `src/security/policy.rs`
- `src/tools/mod.rs`
- `src/appliance/runtime.rs`

**Unit tests**

- station mode rejects disallowed shell commands
- station mode exposes only intended tools
- out-of-scope browser domains are blocked
- right-panel window is not eligible for mapped kiosk actions

**Validation**

- component tests against security policy
- manual negative tests for blocked commands and domains

**Exit gate**

- station runtime is materially narrower than stock ZeroClaw

---

## Phase 9. Local Reliability

### Step 9.1 Add crash recovery and watchdog behavior

**Goal**

Keep `NeoHUman` running unattended on a dedicated MacBook Neo.

**Implementation**

- detect browser crash
- detect runtime panic or fatal loop exit
- persist enough state to resume safely
- add restart policy hooks for macOS launchd or equivalent
- relaunch the left kiosk browser without losing dedupe state when possible
- keep the right operator panel recoverable independently when possible

**Code touchpoints**

- `src/appliance/runtime.rs`
- `src/appliance/watchdog.rs`
- packaging assets added later

**Unit tests**

- crash state transitions to recoverable restart path
- restart does not drop dedupe state
- kiosk browser crash is isolated from operator-panel state when possible

**Validation**

- system test with forced browser process kill
- manual restart validation on target machine

**Exit gate**

- the station can recover from kiosk browser crash without duplicate reply behavior

---

### Step 9.2 Add observability for supportability

**Goal**

Make failures diagnosable without turning the MVP into a large monitoring project.

**Implementation**

- structured logs for:
  - station boot
  - kiosk browser launch
  - canonical placement verification
  - snap-back
  - session restore
  - login required
  - challenge required
  - new message detected
  - mapped action executed
  - reply sent
  - reply blocked
  - recovery path entered
- write failure screenshots only when selectors or recovery fail
- include state transition logs
- include surface id in all station logs

**Code touchpoints**

- `src/appliance/runtime.rs`
- `src/appliance/logging.rs`
- existing observability hooks where appropriate

**Unit tests**

- log events emitted for key transitions
- screenshot artifact path generation is deterministic

**Validation**

- manual failure injection and log review

**Exit gate**

- support artifacts exist for all critical failure classes

---

## Phase 10. Packaging for the Dedicated Mac

### Step 10.1 Package as a local appliance

**Goal**

Ship the MVP as your product, not as a developer repo.

**Implementation**

- define local filesystem layout:
  - supervisor binary
  - app binary
  - managed browser binary
  - left-surface browser profile dir
  - session metadata
  - logs
  - screenshots
  - right-panel assets or local bundle
- create startup wrapper for station mode
- define launchd startup behavior
- document display scaling prerequisites for kiosk reliability

**Code touchpoints**

- packaging scripts
- installer assets
- runtime paths in station config

**Unit tests**

- path resolution tests
- startup config generation tests

**Validation**

- install on a clean target MacBook Neo
- reboot machine
- confirm station restarts automatically
- confirm left kiosk browser and right panel both relaunch correctly

**Exit gate**

- a non-developer can install and launch the station locally

---

### Step 10.2 Add operator-facing status surface

**Goal**

Give the operator minimal visibility without building a large admin product.

**Implementation**

- expose at least:
  - station health
  - current state
  - last successful run
  - last inbound message time
  - last reply time
  - relogin required
  - paused
  - last mapped action
  - last recovery path
- show this in the right operator panel
- keep CLI or local status file as fallback

**Code touchpoints**

- `src/appliance/status.rs`
- operator panel UI or local shell
- CLI or status command

**Unit tests**

- state renders correctly
- status snapshot serializes correctly

**Validation**

- manual operator walkthrough

**Exit gate**

- operator can see whether the station is healthy without reading raw logs

---

## Phase 11. Live MVP Qualification

### Step 11.1 Controlled live test on WhatsApp Web

**Goal**

Prove the end-to-end workflow on the real target platform.

**Implementation**

- use one operator account and one test sender account
- execute scripted scenarios:
  - first login on left kiosk surface
  - restart after login
  - session reuse on restart
  - manual move or resize of left window, then confirm snap-back before next action
  - inbound message detection on WhatsApp Web
  - deterministic reply send
  - duplicate prevention across restart
  - crash or restart during pending send
  - modal or overlay recovery
  - challenge state
  - right operator panel remains usable regardless of its window position

**Unit tests**

- none new; this is live qualification

**Validation**

- run a written live test checklist
- capture evidence:
  - timestamps
  - screenshots
  - log excerpts

**Exit gate**

- all live scenarios pass on the target MacBook Neo with the controlled WhatsApp kiosk surface and operator panel

---

### Step 11.2 Strip unused surfaces only after MVP qualification

**Goal**

Reduce maintenance and attack surface after the product path is proven.

**Implementation**

- disable or remove unused tools and channels from the packaged build
- preserve upstream mergeability where possible
- avoid deep invasive refactors unless necessary

**Code touchpoints**

- `src/tools/mod.rs`
- `src/channels/mod.rs`
- feature flags and packaging config

**Unit tests**

- packaged build still exposes station mode
- removed surfaces are not accessible in the release profile

**Validation**

- compare packaged binary behavior before and after stripping

**Exit gate**

- product build is smaller and narrower without breaking the proven workflow

---

## 6. Test Strategy by Layer

### 6.1 Component Tests

Use for:

- config parsing
- selector validation
- state transitions
- reply rules
- dedupe store
- session metadata
- kiosk geometry validation
- snap-back logic
- WhatsApp state classification
- mapped action verification

Suggested locations:

- `tests/component/messaging_mvp_config.rs`
- `tests/component/messaging_mvp_state.rs`
- `tests/component/messaging_mvp_reply.rs`
- `tests/component/messaging_mvp_dedupe.rs`
- `tests/component/messaging_mvp_browser.rs`
- `tests/component/messaging_mvp_platform.rs`

### 6.2 Integration Tests

Use for:

- station runtime plus mocked browser runtime
- conversation read flow
- new message detection flow
- send flow
- restart dedupe behavior
- left-surface action flow without screenshots in the happy path
- manual left-window drift followed by automatic snap-back
- overlay or modal interruption and recovery

Suggested locations:

- `tests/integration/messaging_mvp_flow.rs`
- `tests/integration/messaging_mvp_restart.rs`

### 6.3 System Tests

Use for:

- long-running poll loop
- crash recovery
- browser relaunch
- status output
- canonical placement enforcement
- restart-safe dedupe behavior on WhatsApp Web

Suggested locations:

- `tests/system/messaging_mvp_runtime.rs`
- `tests/system/messaging_mvp_recovery.rs`

### 6.4 Live Tests

Use for:

- real browser
- real WhatsApp Web
- real login
- real inbound message
- real reply
- real canonical left-window behavior
- real operator panel behavior

Suggested locations:

- `tests/live/messaging_mvp_target.rs`

### 6.5 Manual Validation

Required for:

- first login
- QR and challenge flow
- browser visual invariants
- snap-back verification
- operator handoff
- install and reboot behavior

Suggested location:

- `tests/manual/messaging_mvp_checklist.md`

---

## 7. Phase Gate Checklist

Every phase must satisfy all of the following before the next phase starts:

1. New code has unit tests.
2. Relevant component, integration, and system tests pass.
3. Manual validation for that phase is recorded.
4. Known defects are either fixed or explicitly accepted as out of scope.
5. Logs and failure modes are understandable.
6. No regression is introduced in the prior MVP path.

If any gate fails, stop and fix the current phase before continuing.

---

## 8. Build Order Summary

Execute in this order:

1. Baseline and MVP test lane
2. Station config and runtime state machine
3. Managed kiosk browser contract, runtime abstraction, and placement manager
4. Session persistence and intervention states
5. WhatsApp Web driver, selector map, and mapped interactions
6. Conversation reading and checkpoint store
7. New inbound message detection
8. Deterministic reply engine and send path
9. Human handoff and safety restrictions
10. Crash recovery and observability
11. Packaging and operator status panel
12. Live qualification
13. Post-qualification stripping and hardening

---

## 9. Immediate Next Step

Start with **Phase 0.2 and Step 1.1**, not browser coding.

Reason:

- the MVP needs a dedicated dual-surface station test lane and runtime config first
- otherwise the kiosk browser, operator panel, and mapped interaction work will sprawl across generic ZeroClaw modules without clear boundaries

The first implementation branch should therefore produce:

1. station config schema with asymmetric left-surface and right-panel modeling
2. supervisor and station runtime state machine
3. dedicated component and integration test files

Only after those are green should kiosk browser runtime and placement work begin.
