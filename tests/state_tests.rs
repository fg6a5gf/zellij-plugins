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
