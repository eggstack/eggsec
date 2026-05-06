#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlEvent {
    FocusNext,
    FocusPrev,
    Enter,
    Escape,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    Char(char),
    Backspace,
    Paste(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlOutcome {
    Handled,
    Ignored,
    FocusChanged,
    ActionRequested,
}

impl ControlOutcome {
    pub fn is_handled(&self) -> bool {
        matches!(self, ControlOutcome::Handled | ControlOutcome::FocusChanged | ControlOutcome::ActionRequested)
    }
}

pub trait ControlHandler {
    fn handle(&mut self, event: ControlEvent) -> ControlOutcome;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_outcome_is_handled_for_all_positive_cases() {
        assert!(ControlOutcome::Handled.is_handled());
        assert!(ControlOutcome::FocusChanged.is_handled());
        assert!(ControlOutcome::ActionRequested.is_handled());
        assert!(!ControlOutcome::Ignored.is_handled());
    }

    #[test]
    fn control_event_derives_eq() {
        let e1 = ControlEvent::Up;
        let e2 = ControlEvent::Up;
        let e3 = ControlEvent::Down;
        assert_eq!(e1, e2);
        assert_ne!(e1, e3);
    }

    #[test]
    fn control_event_char_carrys_value() {
        let e = ControlEvent::Char('a');
        match e {
            ControlEvent::Char(c) => assert_eq!(c, 'a'),
            _ => panic!("Expected ControlEvent::Char('a')"),
        }
    }

    #[test]
    fn control_event_paste_carrys_string() {
        let e = ControlEvent::Paste("hello".to_string());
        match e {
            ControlEvent::Paste(s) => assert_eq!(s, "hello"),
            _ => panic!("Expected ControlEvent::Paste(...)"),
        }
    }
}