# Pipe Force-ABC Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend zellij-macism plugin with pipe-message-driven force ABC switching for `Ctrl+f`/`Ctrl+y` keybindings, with auto-restore on PaneClosed.

**Architecture:** Add `forced_abc_pane: Option<PaneId>` to State. Add 2 pure functions (`decide_force_abc`, `decide_pane_closed`). Wire `pipe()` lifecycle method and `Event::PaneClosed` handler in main.rs. Modify existing `decide()` to skip Cjk→Abc query when already forced. Update config.kdl keybinds to chain `MessagePlugin` action before launching yazi/harpoon.

**Tech Stack:** Rust, `zellij-tile = =0.44.1`, wasm32-wasip1 bin target.

**Spec:** `docs/superpowers/specs/2026-06-01-pipe-force-abc-design.md`

---

## File Structure

- `src/state.rs` — add `forced_abc_pane` field, add `decide_force_abc()`/`decide_pane_closed()`, modify `decide()` to skip when forced
- `src/main.rs` — add `pipe()` method, add `Event::PaneClosed` arm, add `EventType::PaneClosed` to subscribe
- `tests/state_tests.rs` — 7 new tests
- `~/.config/zellij/config.kdl` — modify `Ctrl+f`/`Ctrl+y` bindings (manual edit, documented in plan)

---

### Task 1: Add `forced_abc_pane` field to State

**Files:**
- Modify: `src/state.rs`

- [ ] **Step 1: Add field to State struct**

Find the `State` struct in `src/state.rs` (currently has `prev_class`, `focused_pane`, `pane_im`). Replace with:

```rust
#[derive(Default)]
pub struct State {
    pub prev_class: Option<ModeClass>,
    pub focused_pane: Option<PaneId>,
    pub pane_im: HashMap<PaneId, String>,
    pub forced_abc_pane: Option<PaneId>,
}
```

- [ ] **Step 2: Verify build still works**

```bash
source $HOME/.cargo/env
cargo build --target wasm32-wasip1 --features wasm-plugin
cargo test --tests
```

Expected: Both pass. 13 existing tests still pass (field has Default, doesn't break anything).

- [ ] **Step 3: Commit**

```bash
git add src/state.rs
git commit -m "state: add forced_abc_pane field"
```

---

### Task 2: TDD — `decide_force_abc()`

**Files:**
- Modify: `tests/state_tests.rs`
- Modify: `src/state.rs`

- [ ] **Step 1: Append 3 failing tests to `tests/state_tests.rs`**

Add at end of file:

```rust
use zellij_macism::state::decide_force_abc;

#[test]
fn force_abc_saves_pane_and_emits_query() {
    let mut s = State::default();
    s.focused_pane = Some(5);
    let act = decide_force_abc(&mut s);
    assert_eq!(act, Action::QueryThenSwitchAbc { pane: 5 });
    assert_eq!(s.forced_abc_pane, Some(5));
}

#[test]
fn force_abc_no_focus_is_noop() {
    let mut s = State::default();
    let act = decide_force_abc(&mut s);
    assert_eq!(act, Action::Noop);
    assert_eq!(s.forced_abc_pane, None);
}

#[test]
fn force_abc_already_forced_is_noop() {
    let mut s = State::default();
    s.focused_pane = Some(5);
    s.forced_abc_pane = Some(3);
    let act = decide_force_abc(&mut s);
    assert_eq!(act, Action::Noop);
    assert_eq!(s.forced_abc_pane, Some(3), "existing forced pane preserved");
}
```

- [ ] **Step 2: Run tests, verify FAIL (function missing)**

```bash
cargo test force_abc
```

Expected: FAIL — `decide_force_abc` not found.

- [ ] **Step 3: Append `decide_force_abc` to `src/state.rs`**

```rust
pub fn decide_force_abc(state: &mut State) -> Action {
    if state.forced_abc_pane.is_some() {
        return Action::Noop;
    }
    let pane = match state.focused_pane {
        Some(p) => p,
        None => return Action::Noop,
    };
    state.forced_abc_pane = Some(pane);
    Action::QueryThenSwitchAbc { pane }
}
```

- [ ] **Step 4: Run tests, verify PASS**

```bash
cargo test
```

Expected: PASS, 16 tests total (13 prior + 3 new).

- [ ] **Step 5: Commit**

```bash
git add src/state.rs tests/state_tests.rs
git commit -m "state: decide_force_abc with reentry guard"
```

---

### Task 3: TDD — `decide_pane_closed()`

**Files:**
- Modify: `tests/state_tests.rs`
- Modify: `src/state.rs`

- [ ] **Step 1: Append 3 failing tests to `tests/state_tests.rs`**

```rust
use zellij_macism::state::decide_pane_closed;

#[test]
fn pane_closed_restores_forced_with_saved_im() {
    let mut s = State::default();
    s.forced_abc_pane = Some(5);
    s.pane_im.insert(5, "im.rime.inputmethod.Squirrel.Hans".into());
    let act = decide_pane_closed(&mut s, "default-cjk");
    assert_eq!(act, Action::Restore {
        pane: 5,
        target: "im.rime.inputmethod.Squirrel.Hans".into(),
    });
    assert_eq!(s.forced_abc_pane, None, "forced state cleared");
}

#[test]
fn pane_closed_falls_back_to_default_when_no_saved() {
    let mut s = State::default();
    s.forced_abc_pane = Some(5);
    let act = decide_pane_closed(&mut s, "default-cjk");
    assert_eq!(act, Action::Restore {
        pane: 5,
        target: "default-cjk".into(),
    });
    assert_eq!(s.forced_abc_pane, None);
}

#[test]
fn pane_closed_no_forced_is_noop() {
    let mut s = State::default();
    let act = decide_pane_closed(&mut s, "default-cjk");
    assert_eq!(act, Action::Noop);
    assert_eq!(s.forced_abc_pane, None);
}
```

- [ ] **Step 2: Run tests, verify FAIL**

```bash
cargo test pane_closed
```

Expected: FAIL — `decide_pane_closed` not found.

- [ ] **Step 3: Append `decide_pane_closed` to `src/state.rs`**

```rust
pub fn decide_pane_closed(state: &mut State, default_cjk: &str) -> Action {
    let pane = match state.forced_abc_pane.take() {
        Some(p) => p,
        None => return Action::Noop,
    };
    let target = state
        .pane_im
        .get(&pane)
        .cloned()
        .unwrap_or_else(|| default_cjk.to_string());
    Action::Restore { pane, target }
}
```

- [ ] **Step 4: Run tests, verify PASS**

```bash
cargo test
```

Expected: PASS, 19 tests total.

- [ ] **Step 5: Commit**

```bash
git add src/state.rs tests/state_tests.rs
git commit -m "state: decide_pane_closed restores forced pane IM"
```

---

### Task 4: TDD — Modify `decide()` to skip Cjk→Abc when forced

**Files:**
- Modify: `tests/state_tests.rs`
- Modify: `src/state.rs`

- [ ] **Step 1: Append failing test to `tests/state_tests.rs`**

```rust
#[test]
fn decide_cjk_to_abc_skips_query_when_forced() {
    let mut s = State::default();
    s.focused_pane = Some(5);
    s.prev_class = Some(ModeClass::Cjk);
    s.forced_abc_pane = Some(5);
    let act = decide(&mut s, ModeClass::Abc, "default-cjk");
    assert_eq!(act, Action::Noop, "must not query when already forced");
    assert_eq!(s.prev_class, Some(ModeClass::Abc), "prev_class still updates");
    assert_eq!(s.forced_abc_pane, Some(5), "forced state preserved");
}
```

- [ ] **Step 2: Run test, verify FAIL**

```bash
cargo test decide_cjk_to_abc_skips_query_when_forced
```

Expected: FAIL — current code emits `QueryThenSwitchAbc` instead of `Noop`.

- [ ] **Step 3: Modify `decide()` in `src/state.rs`**

Find existing `decide()` and replace the `ModeClass::Abc` arm. Full updated function:

```rust
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
        ModeClass::Abc => {
            if state.forced_abc_pane.is_some() {
                return Action::Noop;
            }
            Action::QueryThenSwitchAbc { pane }
        }
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

- [ ] **Step 4: Run all tests, verify PASS**

```bash
cargo test
```

Expected: PASS, 20 tests total. All prior tests still pass (the new check only kicks in when `forced_abc_pane.is_some()`).

- [ ] **Step 5: Commit**

```bash
git add src/state.rs tests/state_tests.rs
git commit -m "state: skip Cjk-to-Abc query when forced (avoid pane_im pollution)"
```

---

### Task 5: Wire `pipe()` method and `Event::PaneClosed` in main.rs

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Verify zellij-tile re-exports `PipeMessage`/`PipeSource`**

```bash
grep -n "PipeMessage\|PipeSource" $HOME/.cargo/registry/src/index.crates.io-*/zellij-tile-0.44.1/src/lib.rs | head -5
```

Expected: `PipeMessage` imported from `zellij_utils::data`. Should be available via `zellij_tile::prelude::*`. If not, add explicit import.

- [ ] **Step 2: Replace `src/main.rs` with full updated implementation**

Read the file first to confirm current contents, then replace. Key changes from current code:
- Add `decide_force_abc`, `decide_pane_closed` to imports from state module
- Add `EventType::PaneClosed` to subscribe list
- Add `Event::PaneClosed` arm in `update()`
- Add `fn pipe()` method

```rust
use std::collections::BTreeMap;
use zellij_tile::prelude::*;
use zellij_macism::state;
use state::{
    apply_query_result, classify_str, decide, decide_force_abc, decide_pane_closed,
    Action, Config, State,
};

#[derive(Default)]
struct MacismPlugin {
    state: State,
    config: Config,
}

register_plugin!(MacismPlugin);

impl ZellijPlugin for MacismPlugin {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        self.config = Config::from_map(&configuration);
        #[cfg(target_arch = "wasm32")]
        {
            request_permission(&[
                PermissionType::ReadApplicationState,
                PermissionType::RunCommands,
            ]);
            subscribe(&[
                EventType::ModeUpdate,
                EventType::PaneUpdate,
                EventType::RunCommandResult,
                EventType::PaneClosed,
            ]);
        }
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
            Event::PaneClosed(_pane_id) => {
                let action = decide_pane_closed(&mut self.state, &self.config.default_cjk);
                self.execute(action);
            }
            _ => {}
        }
        false
    }

    fn pipe(&mut self, msg: PipeMessage) -> bool {
        if matches!(msg.source, PipeSource::Keybind) && msg.name == "force_abc" {
            let action = decide_force_abc(&mut self.state);
            self.execute(action);
        }
        false
    }
}

impl MacismPlugin {
    fn execute(&mut self, action: Action) {
        match action {
            Action::Noop => {}
            #[cfg(target_arch = "wasm32")]
            Action::QueryThenSwitchAbc { pane } => {
                let mut ctx = BTreeMap::new();
                ctx.insert("op".to_string(), "query".to_string());
                ctx.insert("pane".to_string(), pane.to_string());
                run_command(&[&self.config.macism_path], ctx);
            }
            #[cfg(target_arch = "wasm32")]
            Action::Restore { pane, target } => {
                let mut ctx = BTreeMap::new();
                ctx.insert("op".to_string(), "restore".to_string());
                ctx.insert("pane".to_string(), pane.to_string());
                run_command(&[&self.config.macism_path, &target], ctx);
            }
            #[cfg(not(target_arch = "wasm32"))]
            _ => {}
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
                #[cfg(target_arch = "wasm32")]
                self.fire_abc_switch();
            }
            Some("restore") => {
                if exit_code != Some(0) {
                    #[cfg(target_arch = "wasm32")]
                    self.fire_abc_switch();
                }
            }
            _ => {}
        }
    }

    #[cfg(target_arch = "wasm32")]
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

- [ ] **Step 3: Build wasm**

```bash
cargo build --release --target wasm32-wasip1 --features wasm-plugin
```

Expected: PASS. If `PipeMessage`/`PipeSource` not in `prelude::*`, add explicit imports:

```rust
use zellij_tile::shim::{PipeMessage, PipeSource};
```

(Adjust path if different — find via `grep "pub use.*PipeMessage" $HOME/.cargo/registry/src/index.crates.io-*/zellij-tile-0.44.1/src/lib.rs`).

- [ ] **Step 4: Tests still pass**

```bash
cargo test --tests
```

Expected: 20 tests pass.

- [ ] **Step 5: Install wasm**

```bash
cp target/wasm32-wasip1/release/zellij-macism.wasm ~/.config/zellij/plugins/zellij_macism.wasm
```

- [ ] **Step 6: Commit**

```bash
git add src/main.rs
git commit -m "plugin: wire pipe() handler and PaneClosed event"
```

---

### Task 6: Update config.kdl keybinds (manual)

**Files:**
- Modify: `~/.config/zellij/config.kdl` (NOT in repo)

This task touches user config outside the repo. No commit. Manual workflow.

- [ ] **Step 1: Find existing `Ctrl f` binding**

```bash
grep -n -A 5 'bind "Ctrl f"' ~/.config/zellij/config.kdl
```

Expected: shows existing binding with `Run "yazi" { floating true; close_on_exit true; }`.

- [ ] **Step 2: Replace `Ctrl f` binding**

Edit `~/.config/zellij/config.kdl`. Change:

```kdl
bind "Ctrl f" {
    Run "yazi" {
        floating true
        close_on_exit true
    }
}
```

to:

```kdl
bind "Ctrl f" {
    MessagePlugin "file:~/.config/zellij/plugins/zellij_macism.wasm" {
        name "force_abc"
    }
    Run "yazi" {
        floating true
        close_on_exit true
    }
}
```

- [ ] **Step 3: Replace `Ctrl y` binding**

Find:

```kdl
bind "Ctrl y" {
    LaunchOrFocusPlugin "file:~/.config/zellij/plugins/harpoon.wasm" {
        floating true
        move_to_focused_tab true
    }
}
```

Replace with:

```kdl
bind "Ctrl y" {
    MessagePlugin "file:~/.config/zellij/plugins/zellij_macism.wasm" {
        name "force_abc"
    }
    LaunchOrFocusPlugin "file:~/.config/zellij/plugins/harpoon.wasm" {
        floating true
        move_to_focused_tab true
    }
}
```

- [ ] **Step 4: Restart zellij**

```bash
zellij kill-all-sessions
zellij attach --create main
```

Approve any new permission prompts.

---

### Task 7: Manual integration test

**Files:**
- Manual workflow only.

- [ ] **Step 1: Verify plugin loaded without errors**

```bash
tail -50 /var/folders/*/T/zellij-*/zellij-log/zellij.log | grep -i "macism\|error" | head -20
```

Expected: see `Loaded plugin '...zellij_macism.wasm'` lines, no `could not find exported function`, no `failed to load`.

- [ ] **Step 2: Test Ctrl+f flow**

1. In shell pane (Normal mode), set IM to Squirrel: `macism im.rime.inputmethod.Squirrel.Hans`
2. Verify: `macism` outputs `im.rime.inputmethod.Squirrel.Hans`
3. Press `Ctrl+f` (yazi launches floating)
4. In a separate terminal: `macism`
5. Expected: outputs `com.apple.keylayout.ABC`
6. In yazi, press `q` to exit (yazi pane closes)
7. Run `macism` again
8. Expected: outputs `im.rime.inputmethod.Squirrel.Hans`

- [ ] **Step 3: Test Ctrl+y flow**

Same as Step 2 but with `Ctrl+y` (harpoon). Note: harpoon may not auto-close — close it via its own keybind to trigger `PaneClosed`.

- [ ] **Step 4: Test per-pane memory**

1. Open pane A and pane B (`Ctrl+p` → `r`)
2. In A: `macism com.apple.inputmethod.SCIM.ITABC` (Apple Pinyin)
3. In B: `macism im.rime.inputmethod.Squirrel.Hans` (Squirrel)
4. In A, press `Ctrl+f`, exit yazi → verify A restored to Apple Pinyin
5. Switch to B (`Alt+l`), press `Ctrl+f`, exit yazi → verify B restored to Squirrel

- [ ] **Step 5: Test reentry guard**

1. In shell with Squirrel active, press `Ctrl+f` (yazi opens, IM = ABC)
2. Without exiting yazi, press `Ctrl+f` again (yazi already focused; no-op for plugin)
3. Verify pane_im not polluted: exit yazi → Squirrel restored, not ABC

- [ ] **Step 6: Test mode-switch interaction**

1. In shell with Squirrel active, press `Ctrl+f` (yazi opens, IM = ABC, forced=true)
2. Without exiting yazi, press `Ctrl+p` (Pane mode)
3. Verify IM still ABC (no change), Squirrel still saved
4. Press `Esc` (back to Normal — restores from saved IM)
5. Exit yazi (Pane closed → restores from saved IM)
6. Verify final IM = Squirrel, not ABC

---

### Task 8: Update CLAUDE.md and README

**Files:**
- Modify: `CLAUDE.md`
- Modify: `README.md`

- [ ] **Step 1: Update `CLAUDE.md` zellij-macism section**

Find the `## zellij-macism 插件` section. After "### 行为" subsection's existing bullets, append:

```markdown
- **`Ctrl+f`/`Ctrl+y`** (yazi/harpoon)→ keybind 通过 `MessagePlugin` 发 `force_abc` 给 plugin → 切 ABC,记录 pane id;`PaneClosed` 时恢复
```

- [ ] **Step 2: Update `README.md` Behavior section**

Add bullet after existing two:

```markdown
- **Pipe message `force_abc`** (sent via `MessagePlugin` keybind) → query + save current pane's IM → switch ABC. On `PaneClosed`, restore the saved IM.
```

- [ ] **Step 3: Add config example to README**

Append section after "## Configure":

```markdown
## Force ABC on specific keybinds

For tools like yazi or harpoon launched via floating pane, chain `MessagePlugin` action before launch:

\`\`\`kdl
bind "Ctrl f" {
    MessagePlugin "file:~/.config/zellij/plugins/zellij_macism.wasm" {
        name "force_abc"
    }
    Run "yazi" { floating true; close_on_exit true; }
}
\`\`\`

The plugin saves the current IM, switches to ABC, and restores on `PaneClosed`.
```

(Note: in README replace `\`\`\`` with literal triple backticks.)

- [ ] **Step 4: Commit**

```bash
git add CLAUDE.md README.md
git commit -m "docs: document force_abc pipe message and Ctrl+f/Ctrl+y bindings"
```

---

## Self-Review Notes

Spec coverage check:
- `forced_abc_pane` field → Task 1
- `decide_force_abc` + reentry guard → Task 2
- `decide_pane_closed` + auto-clear → Task 3
- `decide()` Cjk→Abc skip when forced → Task 4
- `pipe()` handler + `Event::PaneClosed` arm → Task 5
- `EventType::PaneClosed` subscribe → Task 5 (in subscribe list)
- `PipeSource::Keybind` filter + name=`force_abc` filter → Task 5
- config.kdl keybind chain → Task 6
- All 7 spec tests → Tasks 2/3/4 (3+3+1=7)
- Manual test plan (8 spec items) → Task 7
- Edge cases (reentry, mode-switch interaction) → Task 7 Steps 5-6

No placeholders. Type/method names consistent across tasks (`decide_force_abc`, `decide_pane_closed`, `forced_abc_pane`, `Action::QueryThenSwitchAbc`, `Action::Restore`).
