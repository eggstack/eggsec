#![allow(dead_code)]

pub mod traits;
pub mod request;
pub mod response;
pub mod registry;
pub mod dispatcher;

pub mod implementations;

#[cfg(feature = "rest-api")]
pub mod protocol;

pub use traits::{SecurityTool, ToolCategory, ToolCapability, ParameterDef, ParameterType};
pub use request::{ToolRequest, Target, TargetType, RequestOptions};
pub use response::{ToolResponse, ResponseStatus, ResponseMetadata, ToolError};
pub use registry::ToolRegistry;
pub use dispatcher::ToolDispatcher;

use crate::error::SlapperError;

pub type ToolResult<T> = Result<T, SlapperError>;

pub fn create_default_registry() -> ToolRegistry {
    let registry = ToolRegistry::new();
    
    registry.register(crate::tool::implementations::recon::ReconTool::new()).ok();
    registry.register(crate::tool::implementations::scanner::ScannerTool::ports()).ok();
    registry.register(crate::tool::implementations::scanner::ScannerTool::fingerprint()).ok();
    registry.register(crate::tool::implementations::scanner::ScannerTool::endpoints()).ok();
    registry.register(crate::tool::implementations::fuzzer::FuzzerTool::new()).ok();
    registry.register(crate::tool::implementations::loadtest::LoadTestTool::new()).ok();
    registry.register(crate::tool::implementations::waf::WafTool::detect()).ok();
    registry.register(crate::tool::implementations::waf::WafTool::bypass()).ok();
    registry.register(crate::tool::implementations::waf::WafTool::stress()).ok();
    registry.register(crate::tool::implementations::pipeline::PipelineTool::new()).ok();
    
    registry
}
