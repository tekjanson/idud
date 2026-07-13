use idud::write_synthetic_understanding;
use std::{fs, path::PathBuf};

#[test]
fn synthetic_understanding_pipeline_writes_expected_artifacts() {
    let temp_dir = std::env::temp_dir().join(format!(
        "idud-e2e-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time should be available")
            .as_nanos()
    ));
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");

    let output_path = temp_dir.join("synthetic_understanding.json");
    let result = write_synthetic_understanding(".", &output_path);
    assert!(
        result.is_ok(),
        "synthetic understanding should be generated: {:?}",
        result
    );

    let json = fs::read_to_string(&output_path).expect("json artifact should be written");
    let understanding: serde_json::Value =
        serde_json::from_str(&json).expect("synthetic understanding JSON should be valid");

    assert_eq!(understanding["repository"].as_str(), Some("idud"));
    let graph_nodes = understanding["graph_nodes"]
        .as_array()
        .expect("graph nodes should exist");
    let graph_edges = understanding["graph_edges"]
        .as_array()
        .expect("graph edges should exist");
    assert!(
        !graph_nodes.is_empty(),
        "synthetic understanding should include graph nodes"
    );
    assert!(
        !graph_edges.is_empty(),
        "synthetic understanding should include graph edges"
    );

    assert!(
        output_path.with_extension("md").exists(),
        "markdown summary should be written"
    );
    assert!(
        output_path.with_extension("html").exists(),
        "html report should be written"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}
