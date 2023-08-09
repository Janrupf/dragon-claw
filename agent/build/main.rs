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

    // Generate installer icons
    icon_processor.process("wix-installer-banner").unwrap();
    icon_processor.process("wix-installer-dialog").unwrap();

    // We always generate the icons to ensure the build works, but only consume them
    // when targeting windows.
    let windows_icons = icon_processor.process("windows-icon").unwrap();

    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        // Targeting windows, generate windows specific resources
        let icon_resource = windows_icons.get_output("icon").unwrap();
        winres::WindowsResource::new()
            .set_icon(&icon_resource.display().to_string())
            .compile()
            .expect("Failed to compile windows resources");
    }
}
