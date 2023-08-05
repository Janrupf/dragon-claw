fn main() {
    let workspace_root = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
        .parent()
        .unwrap()
        .to_path_buf();

    let protocol_dir = workspace_root.join("proto");
    cargo_emit::rerun_if_changed!(protocol_dir.display());

    // Only build the server
    tonic_build::configure()
        .build_client(false)
        .build_server(true)
        .compile(&[protocol_dir.join("service.proto")], &[protocol_dir])
        .unwrap();
}
