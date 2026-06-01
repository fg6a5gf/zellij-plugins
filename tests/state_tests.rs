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
