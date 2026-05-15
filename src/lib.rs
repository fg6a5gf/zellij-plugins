use std::collections::BTreeMap;
use zellij_tile::prelude::*;

pub mod state;

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
