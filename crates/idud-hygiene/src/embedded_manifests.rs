use std::path::PathBuf;

pub const EMBEDDED_MANIFESTS: &[(&str, &str)] = &[
    (
        "crates/idud-hygiene/golden_patterns/analysis_layer_hygiene.json",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/golden_patterns/analysis_layer_hygiene.json"
        )),
    ),
    (
        "crates/idud-hygiene/golden_patterns/api_service_boundary.json",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/golden_patterns/api_service_boundary.json"
        )),
    ),
    (
        "crates/idud-hygiene/golden_patterns/architecture_hygiene.json",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/golden_patterns/architecture_hygiene.json"
        )),
    ),
    (
        "crates/idud-hygiene/golden_patterns/controller_service_boundary.json",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/golden_patterns/controller_service_boundary.json"
        )),
    ),
    (
        "crates/idud-hygiene/golden_patterns/controller_service_pattern.json",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/golden_patterns/controller_service_pattern.json"
        )),
    ),
    (
        "crates/idud-hygiene/golden_patterns/error_propagation_boundary.json",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/golden_patterns/error_propagation_boundary.json"
        )),
    ),
    (
        "crates/idud-hygiene/golden_patterns/event_driven_boundary.json",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/golden_patterns/event_driven_boundary.json"
        )),
    ),
    (
        "crates/idud-hygiene/golden_patterns/idud_hygiene_self_hygiene.json",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/golden_patterns/idud_hygiene_self_hygiene.json"
        )),
    ),
    (
        "crates/idud-hygiene/golden_patterns/layered_architecture_hygiene.json",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/golden_patterns/layered_architecture_hygiene.json"
        )),
    ),
    (
        "crates/idud-hygiene/golden_patterns/naming_conventions.json",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/golden_patterns/naming_conventions.json"
        )),
    ),
    (
        "crates/idud-hygiene/golden_patterns/pattern_catalog_hygiene.json",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/golden_patterns/pattern_catalog_hygiene.json"
        )),
    ),
    (
        "crates/idud-hygiene/golden_patterns/pattern_registry.json",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/golden_patterns/pattern_registry.json"
        )),
    ),
    (
        "crates/idud-hygiene/golden_patterns/pipeline_layer_hygiene.json",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/golden_patterns/pipeline_layer_hygiene.json"
        )),
    ),
    (
        "crates/idud-hygiene/golden_patterns/production_runtime_hygiene.json",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/golden_patterns/production_runtime_hygiene.json"
        )),
    ),
    (
        "crates/idud-hygiene/golden_patterns/repository_service_boundary.json",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/golden_patterns/repository_service_boundary.json"
        )),
    ),
    (
        "crates/idud-hygiene/golden_patterns/side_effect_boundary.json",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/golden_patterns/side_effect_boundary.json"
        )),
    ),
    (
        "crates/idud-hygiene/golden_patterns/training_layer_hygiene.json",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/golden_patterns/training_layer_hygiene.json"
        )),
    ),
    (
        "crates/idud-hygiene/golden_patterns/ui_layer_hygiene.json",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/golden_patterns/ui_layer_hygiene.json"
        )),
    ),
    (
        "crates/idud-hygiene/golden_patterns/usecase_workflow_boundary.json",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/golden_patterns/usecase_workflow_boundary.json"
        )),
    ),
];

pub fn embedded_manifest_content(path: impl AsRef<std::path::Path>) -> Option<&'static str> {
    let path = path.as_ref();
    let candidate = path.to_string_lossy().replace('\\', "/");
    let candidate = candidate.strip_prefix('/').unwrap_or(&candidate);
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();

    EMBEDDED_MANIFESTS
        .iter()
        .find_map(|(stored_path, content)| {
            let stored_path = stored_path.replace('\\', "/");
            let stored_path = stored_path.strip_prefix('/').unwrap_or(&stored_path);
            if stored_path == candidate
                || stored_path.ends_with(&format!("/{filename}"))
                || stored_path == filename
            {
                Some(*content)
            } else {
                None
            }
        })
}

pub fn embedded_manifest_paths() -> Vec<PathBuf> {
    EMBEDDED_MANIFESTS
        .iter()
        .map(|(stored_path, _)| PathBuf::from(*stored_path))
        .collect()
}
