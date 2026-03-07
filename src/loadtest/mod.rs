pub mod metrics;
pub mod runner;

use anyhow::Result;

use crate::cli::LoadArgs;
use crate::config::SlapperConfig;

pub use runner::LoadTestRunner;

pub async fn run_cli(args: LoadArgs, config: &SlapperConfig) -> Result<()> {
    if args.verbose {
        eprintln!("Starting load test against {} with {} concurrent connections", 
            args.url, args.concurrency);
    }
    
    let runner = LoadTestRunner::from_args_with_config(args.clone(), config)?;
    let results = runner.run().await?;

    let output = if args.json {
        serde_json::to_string_pretty(&results)?
    } else {
        format!("\n{}", results)
    };
    
    if let Some(ref output_file) = args.output {
        std::fs::write(output_file, &output)?;
        if args.verbose {
            eprintln!("Results written to {}", output_file);
        }
    } else {
        println!("{}", output);
    }
    
    if args.verbose {
        eprintln!("Load test complete: {} requests, {} errors", 
            results.total_requests, results.failed_requests);
    }

    Ok(())
}
