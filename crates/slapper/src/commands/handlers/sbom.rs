use crate::cli::{SbomArgs, SbomCommand};
use anyhow::{Context, Result};

pub async fn handle_sbom(_ctx: &crate::commands::CommandContext, args: SbomArgs) -> Result<()> {
    match args.command {
        SbomCommand::Generate(gen_args) => {
            let gen = crate::supply_chain::sbom::SbomGenerator::new();

            let report = if std::path::Path::new(&gen_args.project).join("Cargo.toml").exists() {
                gen.generate_from_cargo(&gen_args.project)?
            } else if std::path::Path::new(&gen_args.project).join("package.json").exists() {
                gen.generate_from_npm(&gen_args.project)?
            } else if std::path::Path::new(&gen_args.project).join("requirements.txt").exists() {
                gen.generate_from_requirements(&gen_args.project)?
            } else {
                return Err(crate::error::SlapperError::Validation(
                    "No supported manifest file found (Cargo.toml, package.json, requirements.txt)".to_string(),
                ).into());
            };

            let output = match gen_args.format.as_str() {
                "cyclonedx" => gen.export_cyclonedx(&report)?,
                "spdx" => gen.export_spdx(&report)?,
                "json" => serde_json::to_string_pretty(&report)?,
                _ => serde_json::to_string_pretty(&report)?,
            };

            if let Some(ref output_file) = gen_args.output {
                tokio::fs::write(output_file, &output).await
                    .with_context(|| format!("Failed to write output to {}", output_file))?;
                eprintln!("SBOM written to {}", output_file);
            } else {
                println!("{}", output);
            }
        }
        SbomCommand::CheckTyposquat(typo_args) => {
            let detector = crate::supply_chain::typosquat::TyposquatDetector::new(typo_args.threshold);

            let mut packages = Vec::new();

            let cargo_toml = std::path::Path::new(&typo_args.project).join("Cargo.toml");
            if cargo_toml.exists() {
                let content = std::fs::read_to_string(&cargo_toml)?;
                let mut in_deps = false;
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed == "[dependencies]" {
                        in_deps = true;
                        continue;
                    }
                    if trimmed.starts_with('[') {
                        in_deps = false;
                        continue;
                    }
                    if in_deps {
                        if let Some((name, _)) = trimmed.split_once('=') {
                            packages.push(name.trim().to_string());
                        }
                    }
                }
            }

            let package_json = std::path::Path::new(&typo_args.project).join("package.json");
            if package_json.exists() {
                let content = std::fs::read_to_string(&package_json)?;
                let json: serde_json::Value = serde_json::from_str(&content)?;
                if let Some(deps) = json.get("dependencies").and_then(|v| v.as_object()) {
                    for name in deps.keys() {
                        packages.push(name.clone());
                    }
                }
            }

            let req_file = std::path::Path::new(&typo_args.project).join("requirements.txt");
            if req_file.exists() {
                let content = std::fs::read_to_string(&req_file)?;
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('-') {
                        continue;
                    }
                    let name = if trimmed.contains("==") {
                        trimmed.split("==").next().unwrap_or(trimmed).trim()
                    } else if trimmed.contains(">=") {
                        trimmed.split(">=").next().unwrap_or(trimmed).trim()
                    } else {
                        trimmed
                    };
                    packages.push(name.to_string());
                }
            }

            let report = detector.check_packages(&packages)?;

            println!("Typosquat Analysis: {}", typo_args.project);
            println!("Packages checked: {}", report.packages_checked);
            println!("Suspicious packages: {}", report.suspicious_packages.len());
            println!("Risk level: {:?}", report.risk_level);
            println!();

            for finding in &report.suspicious_packages {
                println!("[{}] {} -> {} (similarity: {:.2})",
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
