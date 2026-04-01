use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub struct Spinner {
    chars: &'static [&'static str],
    idx: usize,
    pub stop: Arc<AtomicBool>,
    stage: Arc<Mutex<String>>,
}

impl Spinner {
    pub fn new(stop: Arc<AtomicBool>, stage: Arc<Mutex<String>>) -> Self {
        Self {
            chars: &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
            idx: 0,
            stop,
            stage,
        }
    }

    pub fn tick(&mut self) {
        if !self.stop.load(Ordering::Relaxed) {
            if let Ok(stage) = self.stage.lock() {
                eprint!("\r{} {}", self.chars[self.idx], stage);
                self.idx = (self.idx + 1) % self.chars.len();
            }
        }
    }

    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        eprint!("\r                                                      \r");
    }
}
