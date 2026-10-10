fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "open_workspace",
            "read_workspace",
            "mutate_workspace",
            "reveal_target",
            "resolve_target",
        ]),
    ))
    .expect("failed to build desktop resources");
}
