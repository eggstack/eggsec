/// Macro to generate the common `TabState` implementation that delegates to `TabCore`.
///
/// Generates implementations for: `state`, `progress`, `set_error`.
/// The `reset` method is NOT generated because each tab has custom reset logic.
///
/// Usage:
/// ```ignore
/// tab_state_boilerplate!(ReconTab, core: core);
/// ```
#[macro_export]
macro_rules! tab_state_boilerplate {
    ($tab:ty, core: $core:ident) => {
        fn state(&self) -> $crate::tabs::AppState {
            $crate::tabs::core::tab_state_state(&self.$core)
        }

        fn progress(&self) -> f64 {
            $crate::tabs::core::tab_state_progress(&self.$core)
        }

        fn set_error(&mut self, error: $crate::app::tab_error::TabError) {
            $crate::tabs::core::tab_state_set_error(&mut self.$core, error);
        }
    };
}

/// Macro to generate common `TabInput` methods that delegate to `TabCore`.
///
/// Generates implementations for: `handle_copy`, `handle_word_forward`,
/// `handle_word_backward`, `handle_home`, `handle_end`, `handle_top`,
/// `handle_bottom`, `page_up`, `page_down`, `stop`, `primary_target`.
///
/// Note: `handle_char`, `handle_backspace`, and `handle_paste` are NOT generated
/// because some tabs (like scan_ports) need custom validation logic in those methods.
/// Implement them manually in each tab (simple delegation to `core::tab_input_*`).
///
/// Usage:
/// ```ignore
/// tab_input_boilerplate!(ReconTab, core: core, focus: focus_area, Inputs: ReconFocusArea::Inputs, Results: ReconFocusArea::Results);
/// ```
#[macro_export]
macro_rules! tab_input_boilerplate {
    (
        $tab:ty,
        core: $core:ident,
        focus: $focus:ident,
        Inputs: $inputs_variant:expr,
        Results: $results_variant:expr
    ) => {
        fn handle_copy(&mut self) -> Option<String> {
            let running = self.is_running();
            let inputs = self.$focus == $inputs_variant;
            let results = self.$focus == $results_variant;
            $crate::tabs::core::tab_input_copy(&self.$core, running, inputs, results)
        }

        fn handle_word_forward(&mut self) {
            let running = self.is_running();
            let inputs = self.$focus == $inputs_variant;
            $crate::tabs::core::tab_input_word_forward(&mut self.$core, running, inputs);
        }

        fn handle_word_backward(&mut self) {
            let running = self.is_running();
            let inputs = self.$focus == $inputs_variant;
            $crate::tabs::core::tab_input_word_backward(&mut self.$core, running, inputs);
        }

        fn handle_home(&mut self) {
            let running = self.is_running();
            let inputs = self.$focus == $inputs_variant;
            let results = self.$focus == $results_variant;
            $crate::tabs::core::tab_input_home(&mut self.$core, running, inputs, results);
        }

        fn handle_end(&mut self) {
            let running = self.is_running();
            let inputs = self.$focus == $inputs_variant;
            let results = self.$focus == $results_variant;
            $crate::tabs::core::tab_input_end(&mut self.$core, running, inputs, results);
        }

        fn handle_top(&mut self) {
            let running = self.is_running();
            $crate::tabs::core::tab_input_top(&mut self.$core, running);
        }

        fn handle_bottom(&mut self) {
            let running = self.is_running();
            $crate::tabs::core::tab_input_bottom(&mut self.$core, running);
        }

        fn page_up(&mut self, page_size: usize) {
            let running = self.is_running();
            $crate::tabs::core::tab_input_page_up(&mut self.$core, running, page_size);
        }

        fn page_down(&mut self, page_size: usize) {
            let running = self.is_running();
            $crate::tabs::core::tab_input_page_down(&mut self.$core, running, page_size);
        }

        fn stop(&mut self) {
            self.$core.stop();
        }

        fn primary_target(&self) -> Option<String> {
            Some(self.$core.target().to_string())
        }
    };
}

/// Extended macro for tabs with 3 focus areas (Inputs/Options/Results).
///
/// Generates all methods from `tab_input_boilerplate!` plus:
/// `handle_char`, `handle_backspace`, `handle_paste`, `handle_focus_next`,
/// `handle_focus_prev`, `handle_up`, `handle_down`, `handle_left`, `handle_right`,
/// `is_input_focused`, `is_at_left_edge`, `is_at_right_edge`.
///
/// The `Options` area is assumed to have no vertical navigation (up/down are no-ops).
/// For tabs where Options needs custom up/down, override those methods manually.
///
/// Usage:
/// ```ignore
/// tab_input_3area!(
///     GraphQlTab,
///     core: core,
///     focus: focus_area,
///     Inputs: GraphQlFocusArea::Inputs,
///     Options: GraphQlFocusArea::Options,
///     Results: GraphQlFocusArea::Results
/// );
/// ```
#[macro_export]
macro_rules! tab_input_3area {
    (
        $tab:ty,
        core: $core:ident,
        focus: $focus:ident,
        Inputs: $inputs_variant:expr,
        Options: $options_variant:expr,
        Results: $results_variant:expr
    ) => {
        // Delegate all 2-area boilerplate (which is a superset)
        $crate::tab_input_boilerplate!(
            $tab,
            core: $core,
            focus: $focus,
            Inputs: $inputs_variant,
            Results: $results_variant
        );

        fn handle_char(&mut self, c: char) {
            let running = self.is_running();
            let inputs = self.$focus == $inputs_variant;
            $crate::tabs::core::tab_input_char(&mut self.$core, c, running, inputs);
        }

        fn handle_backspace(&mut self) {
            let running = self.is_running();
            let inputs = self.$focus == $inputs_variant;
            $crate::tabs::core::tab_input_backspace(&mut self.$core, running, inputs);
        }

        fn handle_paste(&mut self, text: &str) {
            let running = self.is_running();
            let inputs = self.$focus == $inputs_variant;
            $crate::tabs::core::tab_input_paste(&mut self.$core, text, running, inputs);
        }

        fn handle_focus_next(&mut self) {
            if !self.is_running() {
                self.$focus = $crate::tabs::core::focus_next_3area(
                    &mut self.$core,
                    self.$focus,
                    $inputs_variant,
                    $options_variant,
                    $results_variant,
                );
            }
        }

        fn handle_focus_prev(&mut self) {
            if !self.is_running() {
                self.$focus = $crate::tabs::core::focus_prev_3area(
                    &mut self.$core,
                    self.$focus,
                    $inputs_variant,
                    $options_variant,
                    $results_variant,
                );
            }
        }

        fn handle_up(&mut self) {
            if !self.is_running() {
                $crate::tabs::core::handle_up_3area(
                    &mut self.$core,
                    self.$focus,
                    $inputs_variant,
                    $results_variant,
                );
            }
        }

        fn handle_down(&mut self) {
            if !self.is_running() {
                $crate::tabs::core::handle_down_3area(
                    &mut self.$core,
                    self.$focus,
                    $inputs_variant,
                    $results_variant,
                );
            }
        }

        fn handle_left(&mut self) -> bool {
            if self.is_running() {
                return false;
            }
            if self.$focus == $inputs_variant {
                self.$core.inputs.move_left()
            } else {
                false
            }
        }

        fn handle_right(&mut self) -> bool {
            if self.is_running() {
                return false;
            }
            if self.$focus == $inputs_variant {
                self.$core.inputs.move_right()
            } else {
                false
            }
        }

        fn is_input_focused(&self) -> bool {
            $crate::tabs::core::is_input_focused(self.$focus, $inputs_variant, &self.$core)
        }

        fn is_at_left_edge(&self) -> bool {
            $crate::tabs::core::is_at_left_edge_simple(self.$focus, $inputs_variant, &self.$core)
        }

        fn is_at_right_edge(&self) -> bool {
            $crate::tabs::core::is_at_right_edge_simple(self.$focus, $inputs_variant, &self.$core)
        }
    };
}

/// Macro for tabs with 2 focus areas (Inputs/Results).
///
/// Generates all methods from `tab_input_boilerplate!` plus:
/// `handle_char`, `handle_backspace`, `handle_paste`, `handle_focus_next`,
/// `handle_focus_prev`, `handle_up`, `handle_down`, `handle_left`, `handle_right`,
/// `is_input_focused`, `is_at_left_edge`, `is_at_right_edge`.
///
/// Usage:
/// ```ignore
/// tab_input_2area!(
///     FingerprintTab,
///     core: core,
///     focus: focus_area,
///     Inputs: FingerprintFocusArea::Inputs,
///     Results: FingerprintFocusArea::Results
/// );
/// ```
#[macro_export]
macro_rules! tab_input_2area {
    (
        $tab:ty,
        core: $core:ident,
        focus: $focus:ident,
        Inputs: $inputs_variant:expr,
        Results: $results_variant:expr
    ) => {
        $crate::tab_input_boilerplate!(
            $tab,
            core: $core,
            focus: $focus,
            Inputs: $inputs_variant,
            Results: $results_variant
        );

        fn handle_char(&mut self, c: char) {
            let running = self.is_running();
            let inputs = self.$focus == $inputs_variant;
            $crate::tabs::core::tab_input_char(&mut self.$core, c, running, inputs);
        }

        fn handle_backspace(&mut self) {
            let running = self.is_running();
            let inputs = self.$focus == $inputs_variant;
            $crate::tabs::core::tab_input_backspace(&mut self.$core, running, inputs);
        }

        fn handle_paste(&mut self, text: &str) {
            let running = self.is_running();
            let inputs = self.$focus == $inputs_variant;
            $crate::tabs::core::tab_input_paste(&mut self.$core, text, running, inputs);
        }

        fn handle_focus_next(&mut self) {
            if !self.is_running() {
                self.$focus = $crate::tabs::core::focus_next_2area(
                    &mut self.$core,
                    self.$focus,
                    $inputs_variant,
                    $results_variant,
                );
            }
        }

        fn handle_focus_prev(&mut self) {
            if !self.is_running() {
                self.$focus = $crate::tabs::core::focus_prev_2area(
                    &mut self.$core,
                    self.$focus,
                    $inputs_variant,
                    $results_variant,
                );
            }
        }

        fn handle_up(&mut self) {
            if !self.is_running() {
                $crate::tabs::core::handle_up_2area(
                    &mut self.$core,
                    self.$focus,
                    $inputs_variant,
                    $results_variant,
                );
            }
        }

        fn handle_down(&mut self) {
            if !self.is_running() {
                $crate::tabs::core::handle_down_2area(
                    &mut self.$core,
                    self.$focus,
                    $inputs_variant,
                    $results_variant,
                );
            }
        }

        fn handle_left(&mut self) -> bool {
            let running = self.is_running();
            $crate::tabs::core::handle_left_simple(&mut self.$core, running)
        }

        fn handle_right(&mut self) -> bool {
            let running = self.is_running();
            $crate::tabs::core::handle_right_simple(&mut self.$core, running)
        }

        fn is_input_focused(&self) -> bool {
            $crate::tabs::core::is_input_focused(self.$focus, $inputs_variant, &self.$core)
        }

        fn is_at_left_edge(&self) -> bool {
            $crate::tabs::core::is_at_left_edge_simple(self.$focus, $inputs_variant, &self.$core)
        }

        fn is_at_right_edge(&self) -> bool {
            $crate::tabs::core::is_at_right_edge_simple(self.$focus, $inputs_variant, &self.$core)
        }
    };
}
