mod icon;

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

    // Process the icons
    let icon_meta = workspace_root.join("assets/icon/icon-meta.json");
    cargo_emit::rerun_if_changed!(icon_meta.display());

    let icon_processor =
        icon::IconProcessor::from_file(icon_meta).expect("Failed to construct icon processor");
    icon_processor.process("generic-icon").unwrap();
    icon_processor.process("windows-icon").unwrap();
}
