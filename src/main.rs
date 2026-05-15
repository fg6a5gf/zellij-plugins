use std::collections::BTreeMap;
use zellij_tile::prelude::*;
use zellij_macism::state;
use state::{apply_query_result, classify_str, decide, Action, Config, State};

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
            _ => {}
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
