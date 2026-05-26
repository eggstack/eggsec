#[allow(dead_code)]
pub(crate) const DOCTOR_SUCCESS: &str = "All checks passed";

#[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
pub async fn handle_doctor(_ctx: &crate::commands::handlers::CommandContext) -> anyhow::Result<()> {
    use crate::plugin::PythonPluginManager;
    use std::io::Write;

    let mut all_ok = true;
    let mut out = std::io::stdout();

    writeln!(out, "=== Slapper Dependency Check ===\n")?;

    #[cfg(feature = "python-plugins")]
    {
        let python_ok = PythonPluginManager::is_python_available();
        if python_ok {
            writeln!(out, "[OK] Python runtime")?;
        } else {
            writeln!(out, "[FAIL] Python runtime - not available or python-plugins feature not enabled")?;
            all_ok = false;
        }
    }

    #[cfg(not(feature = "python-plugins"))]
    {
        writeln!(out, "[SKIP] Python plugins - feature not enabled")?;
    }

    #[cfg(feature = "ruby-plugins")]
    {
        match crate::ruby::RubyPluginClient::new() {
            Ok(_) => {
                writeln!(out, "[OK] Ruby runtime")?;
            }
            Err(e) => {
                writeln!(out, "[FAIL] Ruby runtime: {}", e)?;
                all_ok = false;
            }
        }
    }

    #[cfg(not(feature = "ruby-plugins"))]
    {
        writeln!(out, "[SKIP] Ruby plugins - feature not enabled")?;
    }

    writeln!(out)?;
    if all_ok {
        writeln!(out, "{}", DOCTOR_SUCCESS)?;
        Ok(())
    } else {
        anyhow::bail!("Some dependencies are missing or not properly configured")
    }
}

#[cfg(not(any(feature = "python-plugins", feature = "ruby-plugins")))]
pub async fn handle_doctor(_ctx: &crate::commands::handlers::CommandContext) -> anyhow::Result<()> {
    use std::io::Write;

    let mut out = std::io::stdout();

    writeln!(out, "=== Slapper Dependency Check ===\n")?;
    writeln!(out, "[SKIP] Python plugins - feature not enabled")?;
    writeln!(out, "[SKIP] Ruby plugins - feature not enabled")?;
    writeln!(out, "\nNo plugin features enabled. All core dependencies available.")?;
    Ok(())
}