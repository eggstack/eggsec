//! Regression tests: entering a tab must leave a control the operator can drive.
//!
//! Tabs default their focus area to their input area, but `InputGroup::new()`
//! leaves every field unfocused. `App::sync_input_focus_for_current_tab()` calls
//! `TabInput::ensure_input_focus()` on every tab-entry path to repair that.
//!
//! Every tab that generates its `TabInput` impl through the shared macros gets
//! that hook from `tab_input_boilerplate!`. Tabs with a hand-written
//! `impl TabInput` did not — they fell through to the trait's default no-op, so
//! entering them showed a focus ring on an input area with no focused field:
//! `i` flips the mode indicator to insert, the operator types a target, and
//! nothing appears. This is the "dead input" symptom, and it is invisible until
//! you actually try to use the tab.
//!
//! Not every tab opens on its inputs: selector-first tabs (Load, Cluster,
//! Report, Packet, Proxy) legitimately open on a dropdown, and the operator
//! moves into the inputs with `Tab`/`j`. Those tabs report `is_input_focused()`
//! true through their selector, so the assertion holds for them too — what
//! matters is that *something* is focused and can be driven.

use crate::tabs::Tab;

fn is_input_focused(app: &mut crate::App) -> bool {
    let mut tab = app.current_tab;
    tab.as_tab_input(app).is_input_focused()
}

/// Simulate the real tab-entry path: select the tab, reset it, then run the
/// app's own focus-repair hook.
fn enter_tab(app: &mut crate::App, tab: &Tab) {
    app.current_tab = *tab;
    app.reset_current_tab();
    app.sync_input_focus_for_current_tab();
}

/// Tabs that legitimately open on a control which is not an input field, so
/// `is_input_focused()` is correctly `false` while something *is* focused.
///
/// - `Dashboard` is a pure read-only view with no input fields at all.
/// - `History` is a session list plus a details pane; it owns no `InputGroup`,
///   so there is no field to focus. It opens on the list, which renders its
///   selected row and is driven with `j`/`k` and `Enter`.
/// - `Settings` opens on its section list, which renders its own focus
///   indicator (a `▶` marker plus the shared focus border on the pane), so
///   the operator can see and drive the focus with `Enter`/`Right`.
///
/// Selector-first tabs (Load, Cluster, Report, Packet, Proxy) are *not* in this
/// list: they report the focused selector through `is_input_focused()`, which
/// is why they are covered by the assertion below.
fn opens_on_non_input_control(tab: &Tab) -> bool {
    matches!(*tab, Tab::Dashboard | Tab::History | Tab::Settings)
}

#[test]
fn entering_a_tab_leaves_a_drivable_control_focused() {
    let mut app = crate::app::create_test_app();

    for tab in Tab::all() {
        enter_tab(&mut app, tab);

        if opens_on_non_input_control(tab) {
            assert!(
                !is_input_focused(&mut app),
                "{tab:?}: opens on a non-input control, so no input field should \
                 be reported as focused"
            );
            continue;
        }

        assert!(
            is_input_focused(&mut app),
            "{tab:?}: entering the tab left no control focused, so the focus ring \
             points at an input area that silently discards every keystroke \
             (handle_char requires a focused field). Type a target first."
        );
    }
}

#[test]
fn focus_survives_re_entering_the_same_tab() {
    // The hook is documented as idempotent and must never steal a field the
    // operator deliberately moved to, so re-entering a tab is a no-op.
    let mut app = crate::app::create_test_app();

    for tab in Tab::all() {
        if opens_on_non_input_control(tab) {
            continue;
        }
        enter_tab(&mut app, tab);
        let first = is_input_focused(&mut app);

        app.reset_current_tab();
        app.sync_input_focus_for_current_tab();

        assert_eq!(
            first,
            is_input_focused(&mut app),
            "{tab:?}: re-entering the tab changed whether a control is focused"
        );
    }
}
