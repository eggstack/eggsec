use indicatif::ProgressStyle;
use std::sync::LazyLock;

pub static DEFAULT_PROGRESS_STYLE: LazyLock<ProgressStyle> =
    LazyLock::new(ProgressStyle::default_bar);

pub static SCAN_PROGRESS_STYLE: LazyLock<ProgressStyle> = LazyLock::new(|| {
    ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] {bar:40} {pos}/{len} {msg}")
        .unwrap_or_else(|_| ProgressStyle::default_bar())
});

pub static PORT_SCAN_PROGRESS_STYLE: LazyLock<ProgressStyle> = LazyLock::new(|| {
    ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] Scanning {pos} ports... {msg}")
        .unwrap_or_else(|_| ProgressStyle::default_bar())
});

pub static ENDPOINT_DISCOVERY_STYLE: LazyLock<ProgressStyle> = LazyLock::new(|| {
    ProgressStyle::default_bar()
        .template("{spinner:.cyan} [{elapsed_precise}] {bar:40} {pos}/{len} {msg}")
        .unwrap_or_else(|_| ProgressStyle::default_bar())
});

pub static FINGERPRINT_STYLE: LazyLock<ProgressStyle> = LazyLock::new(|| {
    ProgressStyle::default_bar()
        .template("{spinner:.yellow} [{elapsed_precise}] Fingerprinting {pos} hosts...")
        .unwrap_or_else(|_| ProgressStyle::default_bar())
});

pub static FUZZ_PROGRESS_STYLE: LazyLock<ProgressStyle> = LazyLock::new(|| {
    ProgressStyle::default_bar()
        .template("{spinner:.red} [{elapsed_precise}] Fuzzing {bar:40} {pos}/{len} {msg}")
        .unwrap_or_else(|_| ProgressStyle::default_bar())
});

pub static LOADTEST_STYLE: LazyLock<ProgressStyle> = LazyLock::new(|| {
    ProgressStyle::default_bar()
        .template("{spinner:.blue} [{elapsed_precise}] {bar:40} {pos}/{len} Requests {msg}")
        .unwrap_or_else(|_| ProgressStyle::default_bar())
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_progress_style_created() {
        let _style = &*DEFAULT_PROGRESS_STYLE;
    }

    #[test]
    fn test_scan_progress_style_created() {
        let _style = &*SCAN_PROGRESS_STYLE;
    }

    #[test]
    fn test_port_scan_progress_style_created() {
        let _style = &*PORT_SCAN_PROGRESS_STYLE;
    }

    #[test]
    fn test_fuzz_progress_style_created() {
        let _style = &*FUZZ_PROGRESS_STYLE;
    }
}
