# Zellij Macism Plugin Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Zellij background plugin that auto-switches macOS input method via `macism` based on Zellij input mode, with per-pane CJK IM memory.

**Architecture:** Single Rust crate compiled to `wasm32-wasip1`. Pure-function `decide()` core (testable without Zellij), thin event-handling shell. Subscribes to `ModeUpdate`, `PaneUpdate`, `RunCommandResult`. Uses `run_command()` to invoke `macism` binary.

**Tech Stack:** Rust, `zellij-tile` crate, `wasm32-wasip1` target, `macism` CLI on macOS.

**Spec:** `docs/superpowers/specs/2026-05-15-zellij-macism-plugin-design.md`

---

## File Structure

- `Cargo.toml` — crate metadata, `cdylib` lib target, `zellij-tile` dep
- `rust-toolchain.toml` — pin toolchain, declare wasm target
- `.gitignore` — exclude `target/`
- `src/lib.rs` — Zellij plugin entry point: `register_plugin!`, `ZellijPlugin` impl, event dispatch
- `src/state.rs` — pure state machine: `ModeClass`, `Action`, `decide()`, config parsing. No Zellij imports. Fully unit-testable.
- `tests/state_tests.rs` — integration tests for `decide()` covering all transitions
- `README.md` — install + config instructions
- `docs/superpowers/plans/2026-05-15-zellij-macism-plugin.md` — this plan

`src/lib.rs` stays small (~80 LOC): event match → call into `state.rs` → execute Zellij API calls. `src/state.rs` holds all logic.

---

### Task 1: Scaffold Rust workspace

**Files:**
- Create: `Cargo.toml`
- Create: `rust-toolchain.toml`
- Create: `.gitignore`
- Create: `src/lib.rs` (stub)

- [ ] **Step 1: Create `.gitignore`**

```
/target
**/*.rs.bk
Cargo.lock
```

(Cargo.lock excluded since this is a library. If you later want to commit it for reproducibility, remove that line.)

- [ ] **Step 2: Create `rust-toolchain.toml`**

```toml
[toolchain]
channel = "stable"
targets = ["wasm32-wasip1"]
```

- [ ] **Step 3: Create `Cargo.toml`**

```toml
[package]
name = "zellij-macism"
version = "0.1.0"
edition = "2021"
description = "Zellij plugin that switches macOS input method via macism on mode change"
license = "MIT"

[lib]
crate-type = ["cdylib"]

[dependencies]
zellij-tile = "0.42"

[profile.release]
opt-level = "z"
lto = true
strip = true
```

- [ ] **Step 4: Create `src/lib.rs` stub**

```rust
use std::collections::BTreeMap;
use zellij_tile::prelude::*;

#[derive(Default)]
struct MacismPlugin;

register_plugin!(MacismPlugin);

impl ZellijPlugin for MacismPlugin {
    fn load(&mut self, _configuration: BTreeMap<String, String>) {}
    fn update(&mut self, _event: Event) -> bool { false }
}
```

- [ ] **Step 5: Verify build**

Run: `cargo build --target wasm32-wasip1`
Expected: PASS, produces `target/wasm32-wasip1/debug/zellij_macism.wasm`. (If `zellij-tile` 0.42 is not on crates.io, run `cargo search zellij-tile` and pin to the latest published version.)

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml rust-toolchain.toml .gitignore src/lib.rs
git commit -m "scaffold: zellij-macism crate skeleton"
```

---

### Task 2: Define `ModeClass`, `Action`, `Config` types in `state.rs`

**Files:**
- Create: `src/state.rs`
- Modify: `src/lib.rs` (add `mod state;`)

- [ ] **Step 1: Create `src/state.rs` with types**

```rust
use std::collections::BTreeMap;
use std::collections::HashMap;

pub const DEFAULT_CJK: &str = "im.rime.inputmethod.Squirrel.Hans";
pub const DEFAULT_ABC: &str = "com.apple.keylayout.ABC";
pub const DEFAULT_MACISM: &str = "macism";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModeClass {
    Cjk,
    Abc,
}

pub type PaneId = u32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Noop,
    QueryThenSwitchAbc { pane: PaneId },
    Restore { pane: PaneId, target: String },
}

#[derive(Debug, Clone)]
pub struct Config {
    pub default_cjk: String,
    pub abc: String,
    pub macism_path: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_cjk: DEFAULT_CJK.to_string(),
            abc: DEFAULT_ABC.to_string(),
            macism_path: DEFAULT_MACISM.to_string(),
        }
    }
}

impl Config {
    pub fn from_map(m: &BTreeMap<String, String>) -> Self {
        let d = Config::default();
        Self {
            default_cjk: m.get("default_cjk").cloned().unwrap_or(d.default_cjk),
            abc: m.get("abc").cloned().unwrap_or(d.abc),
            macism_path: m.get("macism_path").cloned().unwrap_or(d.macism_path),
        }
    }
}

#[derive(Default)]
pub struct State {
    pub prev_class: Option<ModeClass>,
    pub focused_pane: Option<PaneId>,
    pub pane_im: HashMap<PaneId, String>,
}
```

- [ ] **Step 2: Wire module in `src/lib.rs`**

Replace `src/lib.rs` contents with:

```rust
use std::collections::BTreeMap;
use zellij_tile::prelude::*;

mod state;

#[derive(Default)]
struct MacismPlugin {
    state: state::State,
    config: state::Config,
}

register_plugin!(MacismPlugin);

impl ZellijPlugin for MacismPlugin {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        self.config = state::Config::from_map(&configuration);
    }
    fn update(&mut self, _event: Event) -> bool { false }
}
```

- [ ] **Step 3: Verify build**

Run: `cargo build --target wasm32-wasip1`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add src/state.rs src/lib.rs
git commit -m "state: types for mode class, action, config"
```

---

### Task 3: Implement and test `classify()` and `decide()` pure logic

**Files:**
- Modify: `src/state.rs` (add `classify`, `decide`)
- Create: `tests/state_tests.rs`

- [ ] **Step 1: Write failing tests in `tests/state_tests.rs`**

```rust
use zellij_macism::state::{classify_str, decide, Action, ModeClass, State};

#[test]
fn cjk_modes_classified_correctly() {
    assert_eq!(classify_str("Normal"), ModeClass::Cjk);
    assert_eq!(classify_str("Locked"), ModeClass::Cjk);
}

#[test]
fn other_modes_classified_as_abc() {
    for m in ["Pane", "Tab", "Resize", "Move", "Scroll", "Session",
              "RenamePane", "RenameTab", "Tmux", "Search"] {
        assert_eq!(classify_str(m), ModeClass::Abc, "mode {m}");
    }
}

#[test]
fn first_update_into_cjk_restores_default_when_no_saved() {
    let mut s = State::default();
    s.focused_pane = Some(7);
    let act = decide(&mut s, ModeClass::Cjk, "default-cjk");
    assert_eq!(act, Action::Restore { pane: 7, target: "default-cjk".into() });
    assert_eq!(s.prev_class, Some(ModeClass::Cjk));
}

#[test]
fn first_update_into_abc_queries_then_switches() {
    let mut s = State::default();
    s.focused_pane = Some(3);
    let act = decide(&mut s, ModeClass::Abc, "default-cjk");
    assert_eq!(act, Action::QueryThenSwitchAbc { pane: 3 });
    assert_eq!(s.prev_class, Some(ModeClass::Abc));
}

#[test]
fn same_class_transition_is_noop() {
    let mut s = State::default();
    s.focused_pane = Some(1);
    s.prev_class = Some(ModeClass::Cjk);
    let act = decide(&mut s, ModeClass::Cjk, "default-cjk");
    assert_eq!(act, Action::Noop);
}

#[test]
fn cjk_to_abc_emits_query_then_switch() {
    let mut s = State::default();
    s.focused_pane = Some(2);
    s.prev_class = Some(ModeClass::Cjk);
    let act = decide(&mut s, ModeClass::Abc, "default-cjk");
    assert_eq!(act, Action::QueryThenSwitchAbc { pane: 2 });
}

#[test]
fn abc_to_cjk_uses_saved_im_when_present() {
    let mut s = State::default();
    s.focused_pane = Some(5);
    s.prev_class = Some(ModeClass::Abc);
    s.pane_im.insert(5, "saved-im".into());
    let act = decide(&mut s, ModeClass::Cjk, "default-cjk");
    assert_eq!(act, Action::Restore { pane: 5, target: "saved-im".into() });
}

#[test]
fn abc_to_cjk_falls_back_to_default_when_no_save() {
    let mut s = State::default();
    s.focused_pane = Some(9);
    s.prev_class = Some(ModeClass::Abc);
    let act = decide(&mut s, ModeClass::Cjk, "default-cjk");
    assert_eq!(act, Action::Restore { pane: 9, target: "default-cjk".into() });
}

#[test]
fn no_focused_pane_yields_noop() {
    let mut s = State::default();
    let act = decide(&mut s, ModeClass::Cjk, "default-cjk");
    assert_eq!(act, Action::Noop);
    assert_eq!(s.prev_class, None, "prev_class not advanced when noop");
}
```

Add to `Cargo.toml` (under `[lib]` section, top-level):

```toml
[lib]
crate-type = ["cdylib", "rlib"]
```

`rlib` is required so integration tests can link against the crate.

- [ ] **Step 2: Run tests, verify they fail to compile**

Run: `cargo test --target $(rustc -vV | sed -n 's|host: ||p')`
Expected: FAIL — `classify_str` and `decide` not found.

(Use host target for tests; wasm32 cannot run unit tests.)

- [ ] **Step 3: Implement `classify_str` and `decide` in `src/state.rs`**

Append to `src/state.rs`:

```rust
pub fn classify_str(mode: &str) -> ModeClass {
    match mode {
        "Normal" | "Locked" => ModeClass::Cjk,
        _ => ModeClass::Abc,
    }
}

pub fn decide(state: &mut State, new_class: ModeClass, default_cjk: &str) -> Action {
    let pane = match state.focused_pane {
        Some(p) => p,
        None => return Action::Noop,
    };

    if state.prev_class == Some(new_class) {
        return Action::Noop;
    }

    state.prev_class = Some(new_class);

    match new_class {
        ModeClass::Abc => Action::QueryThenSwitchAbc { pane },
        ModeClass::Cjk => {
            let target = state
                .pane_im
                .get(&pane)
                .cloned()
                .unwrap_or_else(|| default_cjk.to_string());
            Action::Restore { pane, target }
        }
    }
}
```

Make `state` module public in `src/lib.rs`. Replace `mod state;` with:

```rust
pub mod state;
```

- [ ] **Step 4: Run tests, verify they pass**

Run: `cargo test`
Expected: PASS, 8 tests.

- [ ] **Step 5: Verify wasm build still works**

Run: `cargo build --target wasm32-wasip1`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml src/lib.rs src/state.rs tests/state_tests.rs
git commit -m "state: classify and decide pure functions with tests"
```

---

### Task 4: Add `apply_query_result()` and tests for IM-save logic

**Files:**
- Modify: `src/state.rs`
- Modify: `tests/state_tests.rs`

This handles the result of a `macism` query: parse stdout, save into `pane_im`.

- [ ] **Step 1: Add failing tests to `tests/state_tests.rs`**

```rust
use zellij_macism::state::apply_query_result;

#[test]
fn query_success_saves_trimmed_im() {
    let mut s = State::default();
    apply_query_result(&mut s, 4, Some(0), "im.rime.inputmethod.Squirrel.Hans\n");
    assert_eq!(s.pane_im.get(&4).map(String::as_str),
               Some("im.rime.inputmethod.Squirrel.Hans"));
}

#[test]
fn query_failure_does_not_save() {
    let mut s = State::default();
    apply_query_result(&mut s, 4, Some(127), "");
    assert!(s.pane_im.get(&4).is_none());
}

#[test]
fn query_empty_stdout_does_not_save() {
    let mut s = State::default();
    apply_query_result(&mut s, 4, Some(0), "   \n");
    assert!(s.pane_im.get(&4).is_none());
}

#[test]
fn query_overwrites_previous_save() {
    let mut s = State::default();
    s.pane_im.insert(4, "old".into());
    apply_query_result(&mut s, 4, Some(0), "new\n");
    assert_eq!(s.pane_im.get(&4).map(String::as_str), Some("new"));
}
```

- [ ] **Step 2: Run tests, verify failure**

Run: `cargo test apply_query_result`
Expected: FAIL — function not found.

- [ ] **Step 3: Implement `apply_query_result` in `src/state.rs`**

```rust
pub fn apply_query_result(
    state: &mut State,
    pane: PaneId,
    exit_code: Option<i32>,
    stdout: &str,
) {
    if exit_code != Some(0) {
        return;
    }
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return;
    }
    state.pane_im.insert(pane, trimmed.to_string());
}
```

- [ ] **Step 4: Run tests, verify pass**

Run: `cargo test`
Expected: PASS, 12 tests total.

- [ ] **Step 5: Commit**

```bash
git add src/state.rs tests/state_tests.rs
git commit -m "state: apply_query_result saves trimmed IM on success"
```

---

### Task 5: Wire `ModeUpdate` and `PaneUpdate` event handlers in `lib.rs`

**Files:**
- Modify: `src/lib.rs`

The plugin must request permissions, subscribe to events, and translate `ModeUpdate`/`PaneUpdate` into calls into `state::decide()`, then execute the resulting `Action` via Zellij APIs.

- [ ] **Step 1: Replace `src/lib.rs` with full implementation**

```rust
use std::collections::BTreeMap;
use zellij_tile::prelude::*;

pub mod state;

use state::{apply_query_result, classify_str, decide, Action, Config, ModeClass, State};

#[derive(Default)]
struct MacismPlugin {
    state: State,
    config: Config,
}

register_plugin!(MacismPlugin);

impl ZellijPlugin for MacismPlugin {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        self.config = Config::from_map(&configuration);
        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::RunCommands,
        ]);
        subscribe(&[
            EventType::ModeUpdate,
            EventType::PaneUpdate,
            EventType::RunCommandResult,
        ]);
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::ModeUpdate(info) => {
                let mode_str = format!("{:?}", info.mode);
                let class = classify_str(&mode_str);
                let action = decide(&mut self.state, class, &self.config.default_cjk);
                self.execute(action);
            }
            Event::PaneUpdate(pane_manifest) => {
                self.state.focused_pane = focused_pane_id(&pane_manifest);
            }
            Event::RunCommandResult(exit_code, stdout, _stderr, ctx) => {
                self.handle_run_result(exit_code, &stdout, &ctx);
            }
            _ => {}
        }
        false
    }
}

impl MacismPlugin {
    fn execute(&mut self, action: Action) {
        match action {
            Action::Noop => {}
            Action::QueryThenSwitchAbc { pane } => {
                let mut ctx = BTreeMap::new();
                ctx.insert("op".to_string(), "query".to_string());
                ctx.insert("pane".to_string(), pane.to_string());
                run_command(&[&self.config.macism_path], ctx);
            }
            Action::Restore { pane, target } => {
                let mut ctx = BTreeMap::new();
                ctx.insert("op".to_string(), "restore".to_string());
                ctx.insert("pane".to_string(), pane.to_string());
                run_command(&[&self.config.macism_path, &target], ctx);
            }
        }
    }

    fn handle_run_result(
        &mut self,
        exit_code: Option<i32>,
        stdout: &[u8],
        ctx: &BTreeMap<String, String>,
    ) {
        let stdout_str = String::from_utf8_lossy(stdout);
        match ctx.get("op").map(String::as_str) {
            Some("query") => {
                if let Some(pane) = ctx.get("pane").and_then(|s| s.parse::<u32>().ok()) {
                    apply_query_result(&mut self.state, pane, exit_code, &stdout_str);
                }
                self.fire_abc_switch();
            }
            Some("restore") => {
                if exit_code != Some(0) {
                    self.fire_abc_switch();
                }
            }
            _ => {}
        }
    }

    fn fire_abc_switch(&self) {
        run_command(
            &[&self.config.macism_path, &self.config.abc],
            BTreeMap::new(),
        );
    }
}

fn focused_pane_id(manifest: &PaneManifest) -> Option<state::PaneId> {
    for (_tab, panes) in manifest.panes.iter() {
        for p in panes {
            if p.is_focused {
                return Some(p.id as state::PaneId);
            }
        }
    }
    None
}
```

Note: `RunCommandResult` payload is `(Option<i32>, Vec<u8>, Vec<u8>, BTreeMap<String, String>)`. If your `zellij-tile` version differs, run `cargo doc --open -p zellij-tile` and adjust the destructure pattern in the `match`. Same for `PaneManifest` field shape — adjust `focused_pane_id` if iteration yields a different structure.

- [ ] **Step 2: Build wasm**

Run: `cargo build --release --target wasm32-wasip1`
Expected: PASS. Output at `target/wasm32-wasip1/release/zellij_macism.wasm`.

If build fails on `Event::RunCommandResult` destructuring, look up the variant in your `zellij-tile` version (`cargo doc --open -p zellij-tile`, navigate to `Event` enum) and update the pattern. Same for `PaneManifest`.

- [ ] **Step 3: Run unit tests still pass**

Run: `cargo test`
Expected: PASS, 12 tests.

- [ ] **Step 4: Commit**

```bash
git add src/lib.rs
git commit -m "plugin: wire ModeUpdate/PaneUpdate/RunCommandResult handlers"
```

---

### Task 6: Manual integration test

**Files:**
- Manual workflow only.

- [ ] **Step 1: Verify `macism` is installed**

Run: `which macism && macism`
Expected: prints path and current IM ID. If missing: `brew install laishulu/homebrew-macism/macism` (per README).

- [ ] **Step 2: Copy wasm into Zellij plugin dir**

```bash
mkdir -p ~/.config/zellij/plugins
cp target/wasm32-wasip1/release/zellij_macism.wasm ~/.config/zellij/plugins/
```

- [ ] **Step 3: Add plugin to `~/.config/zellij/config.kdl`**

```kdl
load_plugins {
    "file:~/.config/zellij/plugins/zellij_macism.wasm" {
        default_cjk "im.rime.inputmethod.Squirrel.Hans"
        abc "com.apple.keylayout.ABC"
        macism_path "/opt/homebrew/bin/macism"
    }
}
```

(Use absolute `macism_path` because WASI plugins inherit a sanitized PATH.)

- [ ] **Step 4: Restart Zellij session**

```bash
zellij kill-all-sessions
zellij attach --create main
```

- [ ] **Step 5: Approve permission prompt**

On first load, Zellij prompts to approve `ReadApplicationState` + `RunCommands`. Approve.

- [ ] **Step 6: Test single-pane switch**

1. In Normal mode, switch IM to Squirrel via `Cmd+Space` (or `macism im.rime.inputmethod.Squirrel.Hans` in another terminal).
2. Verify: `macism` prints `im.rime.inputmethod.Squirrel.Hans`.
3. Press `Ctrl+p` (Pane mode). Run `macism` in a new shell.
   Expected: prints `com.apple.keylayout.ABC`.
4. Press `Esc` (back to Normal). Run `macism`.
   Expected: prints `im.rime.inputmethod.Squirrel.Hans`.

- [ ] **Step 7: Test per-pane memory**

1. Open a second pane (`Ctrl+p` → `r`).
2. In pane B, set IM to a different CJK (e.g. Sogou: `macism com.sogou.inputmethod.sogou.pinyin`).
3. Press `Ctrl+t` (Tab mode), then `Esc`. Verify pane B's IM is restored to Sogou.
4. Switch focus to pane A (`Alt+h`). Toggle mode same way. Verify pane A restored to Squirrel.

- [ ] **Step 8: Test failure fallback**

1. In `config.kdl`, change `macism_path` to a non-existent path: `/tmp/nope`.
2. Restart Zellij. Toggle modes.
3. IM should not change (commands fail), no Zellij crash. Restore correct path after.

- [ ] **Step 9: Commit any config changes if version-controlled**

If `~/.config/zellij/config.kdl` lives in dotfiles repo, commit there. Otherwise skip.

---

### Task 7: Write README

**Files:**
- Create: `README.md`

- [ ] **Step 1: Create `README.md`**

```markdown
# zellij-macism

Zellij plugin that auto-switches macOS input method via [macism](https://github.com/laishulu/macism) based on Zellij input mode. Per-pane CJK IM memory.

## Behavior

- **Normal / Locked mode** → restores the CJK IM the pane last used (or `default_cjk` if none).
- **Any other mode** (Pane / Tab / Resize / Move / Scroll / Session / Rename* / Tmux / Search) → switches to ABC (English).

## Requirements

- macOS
- [macism](https://github.com/laishulu/macism) installed and on PATH (or supply absolute path via `macism_path` config).
- Zellij with plugin permission support.

## Build

```bash
rustup target add wasm32-wasip1
cargo build --release --target wasm32-wasip1
```

Output: `target/wasm32-wasip1/release/zellij_macism.wasm`.

## Install

```bash
mkdir -p ~/.config/zellij/plugins
cp target/wasm32-wasip1/release/zellij_macism.wasm ~/.config/zellij/plugins/
```

## Configure

Add to `~/.config/zellij/config.kdl`:

```kdl
load_plugins {
    "file:~/.config/zellij/plugins/zellij_macism.wasm" {
        default_cjk "im.rime.inputmethod.Squirrel.Hans"
        abc "com.apple.keylayout.ABC"
        macism_path "/opt/homebrew/bin/macism"
    }
}
```

All three keys are optional; defaults shown above.

## Permissions

On first load, Zellij will prompt for `ReadApplicationState` and `RunCommands`. Approve both.

## Test

```bash
cargo test
```
```

- [ ] **Step 2: Commit**

```bash
git add README.md
git commit -m "docs: README with build/install/config"
```

---

## Self-Review Notes

Spec coverage:
- Per-pane IM memory → Task 3 (`pane_im` map + `decide()`), Task 4 (`apply_query_result`)
- Mode classification → Task 3 (`classify_str`)
- Default CJK fallback → Task 3 test `abc_to_cjk_falls_back_to_default_when_no_save`
- ABC fallback on restore failure → Task 5 (`handle_run_result` → `fire_abc_switch` on non-zero restore exit)
- Config from kdl → Task 2 (`Config::from_map`)
- Permissions → Task 5 (`request_permission`)
- Subscribed events match spec → Task 5
- `PaneUpdate` only updates `focused_pane`, no IM action → Task 5 (no call to `decide` in PaneUpdate branch)
- Stale results acceptable → no special handling needed; `apply_query_result` writes by pane id from ctx, not current focus
- Manual test plan → Task 6
- Build/install docs → Task 7

No placeholders. Type names consistent (`PaneId = u32`, `Action`, `ModeClass`, `Config`, `State`).
