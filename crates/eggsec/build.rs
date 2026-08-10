fn main() {
    #[cfg(feature = "grpc-api")]
    {
        // The generated Rust proto code is checked into source control at
        // src/generated/eggsec.tool.v1.rs and compiled directly via include!().
        // Builds do NOT require protoc for the Rust code.
        //
        // However, tonic-reflection requires a binary file descriptor set
        // (tool_descriptor.bin) which IS generated at build time from the
        // .proto source. This requires protoc to be installed.
        //
        // To regenerate the checked-in Rust file (maintainer task):
        //   1. Install protoc (e.g., `apt install protobuf-compiler`)
        //   2. cargo build --features grpc-api
        //   3. cp target/debug/build/eggsec-*/out/eggsec.tool.v1.rs \
        //        crates/eggsec/src/generated/eggsec.tool.v1.rs

        let out_dir = std::env::var("OUT_DIR").unwrap_or_else(|_| "target/debug/build".to_string());
        let descriptor_path = std::path::Path::new(&out_dir).join("tool_descriptor.bin");

        // Generate file descriptor set for tonic-reflection
        let mut prost_config = prost_build::Config::new();
        prost_config.file_descriptor_set_path(&descriptor_path);

        prost_config
            .compile_protos(&["src/tool/protocol/grpc.proto"], &["src/tool/protocol/"])
            .expect("Failed to compile gRPC proto for file descriptor set");
    }
}
