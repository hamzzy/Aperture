fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Use vendored protoc so we don't rely on system protoc
    let protoc = protoc_bin_vendored::protoc_bin_path().expect("vendored protoc");
    std::env::set_var("PROTOC", protoc);

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile(&["proto/aperture.proto"], &["proto"])?;
    Ok(())
}
