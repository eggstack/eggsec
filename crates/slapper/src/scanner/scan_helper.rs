use indicatif::{ProgressBar, ProgressStyle};
use std::sync::Arc;
use tokio::sync::Semaphore;

pub struct ScanProgress {
    pub bar: ProgressBar,
    pub total: usize,
    pub completed: usize,
}

impl ScanProgress {
    pub fn new(total: usize, message: &str) -> Self {
        let bar = ProgressBar::new(total as u64);
        bar.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) | {msg}")
                .unwrap()
                .progress_chars("#>-"),
        );
        bar.set_message(message.to_string());
        Self {
            bar,
            total,
            completed: 0,
        }
    }

    pub fn inc(&mut self) {
        self.completed += 1;
        self.bar.inc(1);
    }

    pub fn set_message(&self, msg: &str) {
        self.bar.set_message(msg.to_string());
    }

    pub fn finish(self) {
        self.bar.finish_and_clear();
    }
}

pub fn create_progress_bar(total: usize, template: &str, message: &str) -> ProgressBar {
    let bar = ProgressBar::new(total as u64);
    bar.set_style(
        ProgressStyle::default_bar()
            .template(template)
            .unwrap()
            .progress_chars("#>-"),
    );
    bar.set_message(message.to_string());
    bar
}

pub const DEFAULT_SCAN_TEMPLATE: &str =
    "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})";
pub const DEFAULT_SCAN_TEMPLATE_WITH_MSG: &str =
    "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) | {msg}";

pub struct ScannerConfig {
    pub concurrency: usize,
    pub progress_template: &'static str,
    pub progress_message: &'static str,
}

impl Default for ScannerConfig {
    fn default() -> Self {
        Self {
            concurrency: 10,
            progress_template: DEFAULT_SCAN_TEMPLATE,
            progress_message: "scanning...",
        }
    }
}

impl ScannerConfig {
    pub fn new(concurrency: usize) -> Self {
        Self {
            concurrency,
            ..Default::default()
        }
    }

    pub fn with_progress(mut self, template: &'static str, message: &'static str) -> Self {
        self.progress_template = template;
        self.progress_message = message;
        self
    }
}
