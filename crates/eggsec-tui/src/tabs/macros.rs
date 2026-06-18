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
