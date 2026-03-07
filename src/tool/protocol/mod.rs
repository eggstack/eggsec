#![allow(dead_code)]

#[cfg(feature = "rest-api")]
pub mod rest;

#[cfg(feature = "grpc-api")]
pub mod grpc;

#[cfg(feature = "mcp-server")]
pub mod mcp;
