use anyhow::Result;
use crate::cli::ai_analyze::AiAnalyzeArgs;

pub async fn handle_ai_analyze(_args: AiAnalyzeArgs) -> Result<()> {
    eprintln!("AI analysis requires ai-integration feature and API configuration.");
    Ok(())
}
