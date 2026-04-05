
#[cfg(feature = "grpc-api")]
pub mod grpc;
#[cfg(feature = "rest-api")]
pub mod mcp;
#[cfg(feature = "rest-api")]
pub mod openai;
#[cfg(feature = "rest-api")]
pub mod openresponses;
#[cfg(feature = "rest-api")]
pub mod rest;
