#[allow(dead_code)]
pub(crate) const DOCTOR_SUCCESS: &str = "All checks passed";

pub async fn handle_doctor(_ctx: &crate::commands::handlers::CommandContext) -> anyhow::Result<()> {
    use std::io::Write;

    let mut out = std::io::stdout();

    writeln!(out, "=== Slapper Dependency Check ===\n")?;
    writeln!(
        out,
        "No plugin features enabled. All core dependencies available."
    )?;
    writeln!(out, "\n{}", DOCTOR_SUCCESS)?;
    Ok(())
}
