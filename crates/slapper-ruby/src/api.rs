#[cfg(feature = "ruby-plugins")]
use magnus::{prelude::*, Error, Ruby};

#[cfg(feature = "ruby-plugins")]
static BLOCKING_RUNTIME: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();

#[cfg(feature = "ruby-plugins")]
fn get_blocking_runtime() -> &'static tokio::runtime::Runtime {
    BLOCKING_RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("ruby-async")
            .build()
            .unwrap()
    })
}

#[cfg(feature = "ruby-plugins")]
fn runtime_error(ruby: &Ruby, msg: impl Into<std::borrow::Cow<'static, str>>) -> Error {
    Error::new(ruby.exception_runtime_error(), msg)
}

#[cfg(feature = "ruby-plugins")]
pub fn register_api(ruby: &Ruby) -> Result<(), Error> {
    let slapper = ruby.define_module("Slapper")?;

    register_reporting_api(ruby, &slapper)?;

    Ok(())
}

#[cfg(feature = "ruby-plugins")]
fn register_reporting_api(_ruby: &Ruby, slapper: &magnus::RModule) -> Result<(), Error> {
    let report = slapper.define_module("Report")?;

    report.define_module_function("finding", magnus::function!(report_finding, 4))?;
    report.define_module_function("vulnerability", magnus::function!(report_vulnerability, 5))?;
    report.define_module_function("info", magnus::function!(report_info, 2))?;
    report.define_module_function("success", magnus::function!(report_success, 2))?;
    report.define_module_function("warning", magnus::function!(report_warning, 2))?;
    report.define_module_function("error", magnus::function!(report_error, 2))?;

    Ok(())
}

#[cfg(feature = "ruby-plugins")]
fn report_finding(
    _ruby: &Ruby,
    severity: String,
    finding_type: String,
    description: String,
    location: String,
) -> Result<(), Error> {
    tracing::info!(
        severity = %severity,
        type = %finding_type,
        location = %location,
        "{}", description
    );
    Ok(())
}

#[cfg(feature = "ruby-plugins")]
fn report_vulnerability(
    _ruby: &Ruby,
    severity: String,
    vuln_type: String,
    description: String,
    location: String,
    _cve: String,
) -> Result<(), Error> {
    tracing::warn!(
        severity = %severity,
        type = %vuln_type,
        location = %location,
        "{}", description
    );
    Ok(())
}

#[cfg(feature = "ruby-plugins")]
fn report_info(_ruby: &Ruby, title: String, message: String) -> Result<(), Error> {
    tracing::info!("[{}] {}", title, message);
    Ok(())
}

#[cfg(feature = "ruby-plugins")]
fn report_success(_ruby: &Ruby, title: String, message: String) -> Result<(), Error> {
    tracing::info!("[SUCCESS] {}: {}", title, message);
    Ok(())
}

#[cfg(feature = "ruby-plugins")]
fn report_warning(_ruby: &Ruby, title: String, message: String) -> Result<(), Error> {
    tracing::warn!("[WARNING] {}: {}", title, message);
    Ok(())
}

#[cfg(feature = "ruby-plugins")]
fn report_error(_ruby: &Ruby, title: String, message: String) -> Result<(), Error> {
    tracing::error!("[ERROR] {}: {}", title, message);
    Ok(())
}

pub struct SlapperApi;

impl SlapperApi {
    #[cfg(feature = "ruby-plugins")]
    pub fn register(ruby: &Ruby) -> Result<(), Error> {
        register_api(ruby)
    }

    #[cfg(not(feature = "ruby-plugins"))]
    pub fn register() -> Result<(), anyhow::Error> {
        Ok(())
    }
}
