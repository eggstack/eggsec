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

        // Verify reference copy is in sync with generated code
        let generated_path = std::path::Path::new(&out_dir).join("eggsec.tool.v1.rs");
        let reference_path = std::path::Path::new("src/generated/eggsec.tool.v1.rs");

        if generated_path.exists() && reference_path.exists() {
            let generated = std::fs::read_to_string(&generated_path).unwrap_or_default();
            let reference = std::fs::read_to_string(reference_path).unwrap_or_default();

            if generated != reference {
                println!(
                    "cargo::warning=Reference copy src/generated/eggsec.tool.v1.rs is out of sync with generated code. Run: cp {} src/generated/eggsec.tool.v1.rs",
                    generated_path.display()
                );
            }
        }
    }
}
