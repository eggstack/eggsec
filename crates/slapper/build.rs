fn main() {
    #[cfg(feature = "grpc-api")]
    {
        // Use tonic_prost_build for proto compilation with tonic service generation
        let out_dir = std::env::var("OUT_DIR").unwrap_or_else(|_| "target/debug/build".to_string());
        let descriptor_path = std::path::Path::new(&out_dir).join("tool_descriptor.bin");

        let builder = tonic_prost_build::configure()
            .build_server(true)
            .build_client(true)
            .file_descriptor_set_path(&descriptor_path);

        builder
            .compile_protos(&["src/tool/protocol/grpc.proto"], &["src/tool/protocol/"])
            .expect("Failed to compile gRPC proto files");
    }
}
