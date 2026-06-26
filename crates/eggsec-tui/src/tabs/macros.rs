/// Macro to generate a struct from checkbox values, mapping each checkbox to a named field.
///
/// This eliminates the verbose `get_options()` pattern where each checkbox is manually
/// mapped by index to a struct field.
///
/// Usage:
/// ```ignore
/// checkbox_options_struct! {
///     pub struct ReconOptions {
///         no_tech: "Skip Tech Detection",
///         no_dns: "Skip DNS Lookup",
///         no_geo: "Skip Geolocation",
///     }
/// }
/// ```
#[macro_export]
macro_rules! checkbox_options_struct {
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident {
            $( $field:ident : $label:literal ),* $(,)?
        }
    ) => {
        $(#[$meta])*
        $vis struct $name {
            $( pub $field: bool, )*
        }

        impl $name {
            /// Number of options in this struct.
            pub const COUNT: usize = $crate::count_fields!( $( $field ),* );

            /// Labels for each option, in order.
            pub const LABELS: &'static [&'static str] = &[ $( $label ),* ];

            /// Create from a slice of checkbox checked values.
            /// Extra checkboxes are ignored; missing ones default to false.
            pub fn from_checkboxes(checkboxes: &[bool]) -> Self {
                let mut iter = checkboxes.iter();
                Self {
                    $(
                        $field: iter.next().copied().unwrap_or(false),
                    )*
                }
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self { $( $field: false, )*
                }
            }
        }
    };
}

/// Helper macro to count the number of fields passed to `checkbox_options_struct!`.
#[macro_export]
macro_rules! count_fields {
    () => { 0usize };
    ($head:ident $(, $tail:ident)*) => { 1usize + $crate::count_fields!($($tail),*) };
}

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

/// Unified macro for tabs with 2 or 3 focus areas.
///
/// Generates all `TabInput` methods except `handle_enter` and `handle_escape`.
/// When called with 2 areas (Inputs/Results), behaves like `tab_input_2area!`.
/// When called with 3 areas (Inputs/Options/Results), behaves like `tab_input_3area!`.
///
/// Usage:
/// ```ignore
/// // 2-area:
/// tab_input_areas!(
///     FingerprintTab,
///     core: core,
///     focus: focus_area,
///     Inputs: FingerprintFocusArea::Inputs,
///     Results: FingerprintFocusArea::Results
/// );
///
/// // 3-area:
/// tab_input_areas!(
///     ReconTab,
///     core: core,
///     focus: focus_area,
///     Inputs: ReconFocusArea::Inputs,
///     Options: ReconFocusArea::Options,
///     Results: ReconFocusArea::Results
/// );
/// ```
#[macro_export]
macro_rules! tab_input_areas {
    // 2-area variant
    (
        $tab:ty,
        core: $core:ident,
        focus: $focus:ident,
        Inputs: $inputs_variant:expr,
        Results: $results_variant:expr
    ) => {
        $crate::tab_input_2area!(
            $tab,
            core: $core,
            focus: $focus,
            Inputs: $inputs_variant,
            Results: $results_variant
        );
    };
    // 3-area variant
    (
        $tab:ty,
        core: $core:ident,
        focus: $focus:ident,
        Inputs: $inputs_variant:expr,
        Options: $options_variant:expr,
        Results: $results_variant:expr
    ) => {
        $crate::tab_input_3area!(
            $tab,
            core: $core,
            focus: $focus,
            Inputs: $inputs_variant,
            Options: $options_variant,
            Results: $results_variant
        );
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
            self.$focus = $crate::tabs::core::focus_next_3area(
                &mut self.$core,
                self.$focus,
                $inputs_variant,
                $options_variant,
                $results_variant,
            );
        }

        fn handle_focus_prev(&mut self) {
            self.$focus = $crate::tabs::core::focus_prev_3area(
                &mut self.$core,
                self.$focus,
                $inputs_variant,
                $options_variant,
                $results_variant,
            );
        }

        fn handle_up(&mut self) {
            $crate::tabs::core::handle_up_3area(
                &mut self.$core,
                self.$focus,
                $inputs_variant,
                $results_variant,
            );
        }

        fn handle_down(&mut self) {
            $crate::tabs::core::handle_down_3area(
                &mut self.$core,
                self.$focus,
                $inputs_variant,
                $results_variant,
            );
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

/// Unified macro to generate `handle_escape` for tabs with any number of focus areas.
///
/// Supports three strategies:
/// - `simple`: 2-area tabs (Inputs/Results). Stops if running, blurs inputs, focuses inputs from Results.
/// - `three_area`: 3-area tabs (Inputs/Options/Results). Stops if running, blurs/focuses as appropriate.
/// - `to_first`: N-area tabs. Stops if running, returns to the first area.
///
/// Usage:
/// ```ignore
/// // 2-area:
/// tab_escape!(MyTab, core: core, focus: focus_area,
///     strategy: simple, Inputs: MyFocusArea::Inputs);
///
/// // 3-area:
/// tab_escape!(MyTab, core: core, focus: focus_area,
///     strategy: three_area, Inputs: A::Inputs, Options: A::Options, Results: A::Results);
///
/// // N-area (to first):
/// tab_escape!(MyTab, core: core, focus: focus_area,
///     strategy: to_first, First: MyFocusArea::Inputs);
/// ```
#[macro_export]
macro_rules! tab_escape {
    ($tab:ty, core: $core:ident, focus: $focus:ident,
     strategy: simple, Inputs: $inputs_variant:expr) => {
        fn handle_escape(&mut self) {
            self.$focus = $crate::tabs::core::handle_escape_simple(
                &mut self.$core,
                self.$focus,
                $inputs_variant,
            );
        }
    };
    ($tab:ty, core: $core:ident, focus: $focus:ident,
     strategy: three_area, Inputs: $inputs_variant:expr, Options: $options_variant:expr, Results: $results_variant:expr) => {
        fn handle_escape(&mut self) {
            self.$focus = $crate::tabs::core::handle_escape_3area(
                &mut self.$core,
                self.$focus,
                $inputs_variant,
                $options_variant,
                $results_variant,
            );
        }
    };
    ($tab:ty, core: $core:ident, focus: $focus:ident,
     strategy: to_first, First: $first:expr) => {
        fn handle_escape(&mut self) {
            self.$focus = $crate::tabs::core::handle_escape_to_first(
                &mut self.$core,
                self.$focus,
                $first,
            );
        }
    };
}

/// Deprecated: Use `tab_escape!` with `strategy: simple` instead.
#[macro_export]
macro_rules! tab_escape_2area {
    ($tab:ty, core: $core:ident, focus: $focus:ident, Inputs: $inputs_variant:expr) => {
        $crate::tab_escape!($tab, core: $core, focus: $focus, strategy: simple, Inputs: $inputs_variant);
    };
}

/// Deprecated: Use `tab_escape!` with `strategy: three_area` instead.
#[macro_export]
macro_rules! tab_escape_3area {
    ($tab:ty, core: $core:ident, focus: $focus:ident, Inputs: $inputs_variant:expr, Options: $options_variant:expr, Results: $results_variant:expr) => {
        $crate::tab_escape!($tab, core: $core, focus: $focus, strategy: three_area, Inputs: $inputs_variant, Options: $options_variant, Results: $results_variant);
    };
}

/// Deprecated: Use `tab_escape!` with `strategy: to_first` instead.
#[macro_export]
macro_rules! tab_escape_to_first {
    ($tab:ty, core: $core:ident, focus: $focus:ident, $first:expr) => {
        $crate::tab_escape!($tab, core: $core, focus: $focus, strategy: to_first, First: $first);
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
            self.$focus = $crate::tabs::core::focus_next_2area(
                &mut self.$core,
                self.$focus,
                $inputs_variant,
                $results_variant,
            );
        }

        fn handle_focus_prev(&mut self) {
            self.$focus = $crate::tabs::core::focus_prev_2area(
                &mut self.$core,
                self.$focus,
                $inputs_variant,
                $results_variant,
            );
        }

        fn handle_up(&mut self) {
            $crate::tabs::core::handle_up_2area(
                &mut self.$core,
                self.$focus,
                $inputs_variant,
                $results_variant,
            );
        }

        fn handle_down(&mut self) {
            $crate::tabs::core::handle_down_2area(
                &mut self.$core,
                self.$focus,
                $inputs_variant,
                $results_variant,
            );
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

/// Macro for N-area tabs (3+ focus areas in sequence).
///
/// Generates all methods from `tab_input_boilerplate!` plus:
/// `handle_char`, `handle_backspace`, `handle_paste`, `handle_focus_next`,
/// `handle_focus_prev`, `handle_up`, `handle_down`, `handle_left`, `handle_right`,
/// `is_input_focused`, `is_at_left_edge`, `is_at_right_edge`.
///
/// The first variant is treated as Inputs, the last as Results. Middle variants
/// are selector/option areas with no vertical navigation (up/down are no-ops there).
///
/// Usage:
/// ```ignore
/// tab_input_narea!(
///     StressTab,
///     core: core,
///     focus: focus_area,
///     areas: [StressFocusArea::Inputs, StressFocusArea::TypeSelector, StressFocusArea::Results]
/// );
/// ```
#[macro_export]
macro_rules! tab_input_narea {
    (
        $tab:ty,
        core: $core:ident,
        focus: $focus:ident,
        areas: [ $($area:expr),+ $(,)? ]
    ) => {
        $crate::tab_input_boilerplate!(
            $tab,
            core: $core,
            focus: $focus,
            Inputs: $crate::first_area!($($area),+),
            Results: $crate::last_area!($($area),+)
        );

        fn handle_char(&mut self, c: char) {
            let running = self.is_running();
            let inputs = self.$focus == $crate::first_area!($($area),+);
            $crate::tabs::core::tab_input_char(&mut self.$core, c, running, inputs);
        }

        fn handle_backspace(&mut self) {
            let running = self.is_running();
            let inputs = self.$focus == $crate::first_area!($($area),+);
            $crate::tabs::core::tab_input_backspace(&mut self.$core, running, inputs);
        }

        fn handle_paste(&mut self, text: &str) {
            let running = self.is_running();
            let inputs = self.$focus == $crate::first_area!($($area),+);
            $crate::tabs::core::tab_input_paste(&mut self.$core, text, running, inputs);
        }

        fn handle_focus_next(&mut self) {
            let areas = $crate::narea_slice!($($area),+);
            self.$focus = $crate::tabs::core::focus_next_n(
                &mut self.$core,
                self.$focus,
                areas,
            );
        }

        fn handle_focus_prev(&mut self) {
            let areas = $crate::narea_slice!($($area),+);
            self.$focus = $crate::tabs::core::focus_prev_n(
                &mut self.$core,
                self.$focus,
                areas,
            );
        }

        fn handle_up(&mut self) {
            let areas = $crate::narea_slice!($($area),+);
            $crate::tabs::core::handle_up_n(
                &mut self.$core,
                self.$focus,
                areas,
            );
        }

        fn handle_down(&mut self) {
            let areas = $crate::narea_slice!($($area),+);
            $crate::tabs::core::handle_down_n(
                &mut self.$core,
                self.$focus,
                areas,
            );
        }

        fn handle_left(&mut self) -> bool {
            if self.is_running() {
                return false;
            }
            $crate::tabs::core::handle_left_n(&mut self.$core, self.$focus, $crate::first_area!($($area),+))
        }

        fn handle_right(&mut self) -> bool {
            if self.is_running() {
                return false;
            }
            $crate::tabs::core::handle_right_n(&mut self.$core, self.$focus, $crate::first_area!($($area),+))
        }

        fn is_input_focused(&self) -> bool {
            $crate::tabs::core::is_input_focused(self.$focus, $crate::first_area!($($area),+), &self.$core)
        }

        fn is_at_left_edge(&self) -> bool {
            $crate::tabs::core::is_at_left_edge_simple(self.$focus, $crate::first_area!($($area),+), &self.$core)
        }

        fn is_at_right_edge(&self) -> bool {
            $crate::tabs::core::is_at_right_edge_simple(self.$focus, $crate::first_area!($($area),+), &self.$core)
        }
    };
}

/// Macro for tabs with a custom focus enum (Inputs variant + Results variant).
///
/// Generates all TabInput delegation methods. The tab must:
/// - Embed `TabCore` as the named field `$core`
/// - Have a focus enum field `$focus`
/// - Have an Inputs variant that maps to the first InputGroup field
/// - Have a Results variant that represents the results area
///
/// The tab must manually implement: `handle_focus_next`, `handle_focus_prev`,
/// `handle_enter`, `handle_escape`, `handle_up`, `handle_down`.
///
/// Usage:
/// ```ignore
/// tab_input_custom!(
///     AuthTab,
///     core: core,
///     focus: focus_area,
///     Inputs: AuthFocusArea::Target,
///     Results: AuthFocusArea::Results
/// );
/// ```
#[macro_export]
macro_rules! tab_input_custom {
    (
        $tab:ty,
        core: $core:ident,
        focus: $focus:ident,
        Inputs: $inputs_variant:expr,
        Results: $results_variant:expr
    ) => {
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

        fn page_up(&mut self, page_size: usize) {
            let running = self.is_running();
            $crate::tabs::core::tab_input_page_up(&mut self.$core, running, page_size);
        }

        fn page_down(&mut self, page_size: usize) {
            let running = self.is_running();
            $crate::tabs::core::tab_input_page_down(&mut self.$core, running, page_size);
        }

        fn primary_target(&self) -> Option<String> {
            Some(self.$core.target().to_string())
        }

        fn handle_copy(&mut self) -> Option<String> {
            let running = self.is_running();
            let inputs = self.$focus == $inputs_variant;
            let results = self.$focus == $results_variant;
            $crate::tabs::core::tab_input_copy(&self.$core, running, inputs, results)
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

#[macro_export]
macro_rules! tab_input_indexed {
    (
        $tab:ty,
        core: $core:ident,
        focus: $focus:ident,
        InputAreas: $input_areas:expr,
        Results: $results_variant:expr
    ) => {
        fn handle_char(&mut self, c: char) {
            let running = self.is_running();
            let inputs = $crate::tabs::core::is_indexed_input_area(self.$focus, $input_areas);
            $crate::tabs::core::tab_input_char(&mut self.$core, c, running, inputs);
        }

        fn handle_backspace(&mut self) {
            let running = self.is_running();
            let inputs = $crate::tabs::core::is_indexed_input_area(self.$focus, $input_areas);
            $crate::tabs::core::tab_input_backspace(&mut self.$core, running, inputs);
        }

        fn handle_paste(&mut self, text: &str) {
            let running = self.is_running();
            let inputs = $crate::tabs::core::is_indexed_input_area(self.$focus, $input_areas);
            $crate::tabs::core::tab_input_paste(&mut self.$core, text, running, inputs);
        }

        fn handle_word_forward(&mut self) {
            let running = self.is_running();
            let inputs = $crate::tabs::core::is_indexed_input_area(self.$focus, $input_areas);
            $crate::tabs::core::tab_input_word_forward(&mut self.$core, running, inputs);
        }

        fn handle_word_backward(&mut self) {
            let running = self.is_running();
            let inputs = $crate::tabs::core::is_indexed_input_area(self.$focus, $input_areas);
            $crate::tabs::core::tab_input_word_backward(&mut self.$core, running, inputs);
        }

        fn handle_home(&mut self) {
            let running = self.is_running();
            let inputs = $crate::tabs::core::is_indexed_input_area(self.$focus, $input_areas);
            let results = self.$focus == $results_variant;
            $crate::tabs::core::tab_input_home(&mut self.$core, running, inputs, results);
        }

        fn handle_end(&mut self) {
            let running = self.is_running();
            let inputs = $crate::tabs::core::is_indexed_input_area(self.$focus, $input_areas);
            let results = self.$focus == $results_variant;
            $crate::tabs::core::tab_input_end(&mut self.$core, running, inputs, results);
        }

        fn page_up(&mut self, page_size: usize) {
            let running = self.is_running();
            $crate::tabs::core::tab_input_page_up(&mut self.$core, running, page_size);
        }

        fn page_down(&mut self, page_size: usize) {
            let running = self.is_running();
            $crate::tabs::core::tab_input_page_down(&mut self.$core, running, page_size);
        }

        fn primary_target(&self) -> Option<String> {
            Some(self.$core.target().to_string())
        }

        fn handle_copy(&mut self) -> Option<String> {
            let running = self.is_running();
            let inputs = $crate::tabs::core::is_indexed_input_area(self.$focus, $input_areas);
            let results = self.$focus == $results_variant;
            $crate::tabs::core::tab_input_copy(&self.$core, running, inputs, results)
        }

        fn handle_left(&mut self) -> bool {
            if self.is_running() {
                return false;
            }
            if $crate::tabs::core::is_indexed_input_area(self.$focus, $input_areas) {
                self.$core.inputs.move_left()
            } else {
                false
            }
        }

        fn handle_right(&mut self) -> bool {
            if self.is_running() {
                return false;
            }
            if $crate::tabs::core::is_indexed_input_area(self.$focus, $input_areas) {
                self.$core.inputs.move_right()
            } else {
                false
            }
        }

        fn is_input_focused(&self) -> bool {
            $crate::tabs::core::is_indexed_input_area(self.$focus, $input_areas)
                && self.$core.inputs.is_focused()
        }

        fn is_at_left_edge(&self) -> bool {
            if $crate::tabs::core::is_indexed_input_area(self.$focus, $input_areas) {
                self.$core.inputs.is_at_left_edge()
            } else {
                true
            }
        }

        fn is_at_right_edge(&self) -> bool {
            if $crate::tabs::core::is_indexed_input_area(self.$focus, $input_areas) {
                self.$core.inputs.is_at_right_edge()
            } else {
                true
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    checkbox_options_struct! {
        #[derive(Debug, Clone, PartialEq, Eq)]
        struct TestOptions {
            opt_a: "Option A",
            opt_b: "Option B",
            opt_c: "Option C",
        }
    }

    #[test]
    fn test_checkbox_options_struct_count() {
        assert_eq!(TestOptions::COUNT, 3);
    }

    #[test]
    fn test_checkbox_options_struct_labels() {
        assert_eq!(TestOptions::LABELS, &["Option A", "Option B", "Option C"]);
    }

    #[test]
    fn test_checkbox_options_struct_from_checkboxes_all_true() {
        let opts = TestOptions::from_checkboxes(&[true, true, true]);
        assert!(opts.opt_a);
        assert!(opts.opt_b);
        assert!(opts.opt_c);
    }

    #[test]
    fn test_checkbox_options_struct_from_checkboxes_all_false() {
        let opts = TestOptions::from_checkboxes(&[false, false, false]);
        assert!(!opts.opt_a);
        assert!(!opts.opt_b);
        assert!(!opts.opt_c);
    }

    #[test]
    fn test_checkbox_options_struct_from_checkboxes_mixed() {
        let opts = TestOptions::from_checkboxes(&[true, false, true]);
        assert!(opts.opt_a);
        assert!(!opts.opt_b);
        assert!(opts.opt_c);
    }

    #[test]
    fn test_checkbox_options_struct_from_checkboxes_empty() {
        let opts = TestOptions::from_checkboxes(&[]);
        assert!(!opts.opt_a);
        assert!(!opts.opt_b);
        assert!(!opts.opt_c);
    }

    #[test]
    fn test_checkbox_options_struct_from_checkboxes_partial() {
        let opts = TestOptions::from_checkboxes(&[true]);
        assert!(opts.opt_a);
        assert!(!opts.opt_b);
        assert!(!opts.opt_c);
    }

    #[test]
    fn test_checkbox_options_struct_from_checkboxes_extra_ignored() {
        let opts = TestOptions::from_checkboxes(&[true, false, true, true, false]);
        assert!(opts.opt_a);
        assert!(!opts.opt_b);
        assert!(opts.opt_c);
    }

    #[test]
    fn test_checkbox_options_struct_default() {
        let opts = TestOptions::default();
        assert!(!opts.opt_a);
        assert!(!opts.opt_b);
        assert!(!opts.opt_c);
    }

    #[test]
    fn test_checkbox_options_struct_labels_count_matches() {
        assert_eq!(TestOptions::LABELS.len(), TestOptions::COUNT);
    }

    #[test]
    fn test_recon_options_labels_match_checkbox_count() {
        use super::super::recon::ReconOptions;
        assert_eq!(ReconOptions::LABELS.len(), ReconOptions::COUNT);
        assert_eq!(ReconOptions::COUNT, 16);
    }
}
