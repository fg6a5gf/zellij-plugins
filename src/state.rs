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
