fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("DOCS_RS").is_ok() {
        tonic_build::configure()
            .protoc_arg("--experimental_allow_proto3_optional")
            .compile_protos(&["proto/gateway.proto"], &["proto"])?;
    } else {
        tonic_build::compile_protos("proto/gateway.proto")?;
    }
    Ok(())
}
