use crate::cli::{SbomArgs, SbomCommand};
use anyhow::{Context, Result};

fn validate_project_path(project_path: &str) -> Result<std::path::PathBuf> {
    let base = std::path::Path::new(".");
    crate::utils::validation::validate_path_string(base, project_path)
}

pub async fn handle_sbom(_ctx: &crate::commands::CommandContext, args: SbomArgs) -> Result<()> {
    match args.command {
        SbomCommand::Generate(gen_args) => {
            let project_path = validate_project_path(&gen_args.project)?;

            let gen = crate::supply_chain::sbom::SbomGenerator::new();
            let format = match gen_args.format.as_str() {
                "spdx" => crate::supply_chain::sbom::SbomFormat::Spdx,
                _ => crate::supply_chain::sbom::SbomFormat::CycloneDx,
            };

            let path_str = project_path.to_str().ok_or_else(|| {
                anyhow::anyhow!("Invalid path: {}", project_path.display())
            })?;

            let report =
                if project_path.join("Cargo.toml").exists() {
                    gen.generate_from_cargo(path_str, format.clone())?
                } else if project_path.join("package.json").exists() {
                    gen.generate_from_npm(path_str, format.clone())?
                } else if project_path.join("requirements.txt").exists() {
                    gen.generate_from_requirements(path_str, format.clone())?
                } else {
                    return Err(crate::error::SlapperError::Validation(
                    "No supported manifest file found (Cargo.toml, package.json, requirements.txt)"
                        .to_string(),
                )
                .into());
                };

            let output = match gen_args.format.as_str() {
                "cyclonedx" => gen.export_cyclonedx(&report)?,
                "spdx" => gen.export_spdx(&report)?,
                "json" => serde_json::to_string_pretty(&report)?,
                _ => serde_json::to_string_pretty(&report)?,
            };

            if let Some(ref output_file) = gen_args.output {
                let output_path = validate_project_path(output_file)?;
                tokio::fs::write(
                    output_path.to_str().ok_or_else(|| {
                        anyhow::anyhow!("Invalid path: {}", output_path.display())
                    })?,
                    &output,
                )
                .await
                .with_context(|| format!("Failed to write output to {}", output_file))?;
                eprintln!("SBOM written to {}", output_file);
            } else {
                println!("{}", output);
            }
        }
        SbomCommand::CheckTyposquat(typo_args) => {
            let project_path = validate_project_path(&typo_args.project)?;
            let detector =
                crate::supply_chain::typosquat::TyposquatDetector::new(typo_args.threshold);

            let packages = crate::supply_chain::scanner::collect_package_names(&project_path)?;

            let report = detector.check_packages(&packages)?;

            println!("Typosquat Analysis: {}", typo_args.project);
            println!("Packages checked: {}", report.packages_checked);
            println!("Suspicious packages: {}", report.suspicious_packages.len());
            println!("Risk level: {:?}", report.risk_level);
            println!();

            for finding in &report.suspicious_packages {
                println!(
                    "[{}] {} -> {} (similarity: {:.2})",
                    finding.severity.as_str().to_uppercase(),
                    finding.package_name,
                    finding.suspected_target,
                    finding.similarity_score
                );
                println!("  Techniques: {:?}", finding.techniques);
                println!("  {}", finding.recommendation);
                println!();
            }
        }
    }

    Ok(())
}
