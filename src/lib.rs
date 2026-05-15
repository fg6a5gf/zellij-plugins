use std::collections::BTreeMap;
use zellij_tile::prelude::*;

#[derive(Default)]
struct MacismPlugin;

register_plugin!(MacismPlugin);

impl ZellijPlugin for MacismPlugin {
    fn load(&mut self, _configuration: BTreeMap<String, String>) {}
    fn update(&mut self, _event: Event) -> bool { false }
}
