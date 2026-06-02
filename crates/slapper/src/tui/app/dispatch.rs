use crate::tui::tabs::HistoryTab;
use crate::tui::tabs::TabInput;
use parking_lot::MutexGuard;
use std::ops::{Deref, DerefMut};

pub enum TabDispatcher<'a> {
    Standard(&'a mut dyn TabInput),
    LockedHistory(MutexGuard<'a, HistoryTab>),
}

impl<'a> TabDispatcher<'a> {
    pub fn new(tab_input: &'a mut dyn TabInput) -> Self {
        Self::Standard(tab_input)
    }

    pub fn new_locked(history: MutexGuard<'a, HistoryTab>) -> Self {
        Self::LockedHistory(history)
    }
}

impl<'a> Deref for TabDispatcher<'a> {
    type Target = dyn TabInput + 'a;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Standard(t) => *t,
            Self::LockedHistory(h) => &**h,
        }
    }
}

impl<'a> DerefMut for TabDispatcher<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Standard(t) => *t,
            Self::LockedHistory(h) => &mut **h,
        }
    }
}
