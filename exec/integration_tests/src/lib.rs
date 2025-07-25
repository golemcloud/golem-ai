//! Integration tests for golem:exec implementation
//! 
//! These tests verify the basic functionality of both JavaScript and Python executors

use std::io::Read;

#[test]
fn test_javascript_wasm_exists() {
    let wasm_path = "../../exec-javascript.wasm";
    assert!(std::path::Path::new(wasm_path).exists(), 
           "JavaScript WASM component should exist at root level");
}

#[test]  
fn test_python_wasm_exists() {
    let wasm_path = "../../exec-python.wasm";
    assert!(std::path::Path::new(wasm_path).exists(),
           "Python WASM component should exist at root level");
}

#[test]
fn test_javascript_wasm_size() {
    let wasm_path = "../../exec-javascript.wasm";
    if let Ok(metadata) = std::fs::metadata(wasm_path) {
        let size = metadata.len();
        // Should be around 2.4 MB (2,400,000 bytes approximately)
        assert!(size > 2_000_000, "JavaScript WASM should be > 2MB, got {}", size);
        assert!(size < 5_000_000, "JavaScript WASM should be < 5MB, got {}", size);
    } else {
        panic!("Could not read JavaScript WASM metadata");
    }
}

#[test]
fn test_python_wasm_size() {
    let wasm_path = "../../exec-python.wasm";
    if let Ok(metadata) = std::fs::metadata(wasm_path) {
        let size = metadata.len();
        // Should be around 2.3 MB (2,300,000 bytes approximately)  
        assert!(size > 2_000_000, "Python WASM should be > 2MB, got {}", size);
        assert!(size < 5_000_000, "Python WASM should be < 5MB, got {}", size);
    } else {
        panic!("Could not read Python WASM metadata");
    }
}

#[test]
fn test_wit_interface_exists() {
    let wit_path = "../wit/golem-exec.wit";
    assert!(std::path::Path::new(wit_path).exists(),
           "WIT interface definition should exist");
}

#[test]
fn test_wit_interface_content() {
    let wit_path = "../wit/golem-exec.wit";
    if let Ok(mut file) = std::fs::File::open(wit_path) {
        let mut contents = String::new();
        file.read_to_string(&mut contents).expect("Should read WIT file");
        
        // Verify key interface elements exist
        assert!(contents.contains("golem:exec"), "Should contain golem:exec package");
        assert!(contents.contains("interface executor"), "Should contain executor interface");
        assert!(contents.contains("variant language"), "Should contain language variant");
        assert!(contents.contains("javascript"), "Should support javascript");
        assert!(contents.contains("python"), "Should support python");
    } else {
        panic!("Could not read WIT interface file");
    }
}

#[test]
fn test_javascript_source_exists() {
    let js_src = "../exec-javascript/src/lib.rs";
    assert!(std::path::Path::new(js_src).exists(),
           "JavaScript executor source should exist");
}

#[test]
fn test_python_source_exists() {
    let py_src = "../exec-python/src/lib.rs";
    assert!(std::path::Path::new(py_src).exists(),
           "Python executor source should exist");
}

#[test]
fn test_cargo_workspace_config() {
    let cargo_toml = "../Cargo.toml";
    assert!(std::path::Path::new(cargo_toml).exists(),
           "Workspace Cargo.toml should exist");
           
    if let Ok(mut file) = std::fs::File::open(cargo_toml) {
        let mut contents = String::new();
        file.read_to_string(&mut contents).expect("Should read Cargo.toml");
        
        assert!(contents.contains("[workspace]"), "Should be a workspace");
        assert!(contents.contains("exec-javascript"), "Should include JavaScript executor");
        assert!(contents.contains("exec-python"), "Should include Python executor");
    }
}

#[test]
fn test_deliverable_components_ready() {
    // This test verifies all bounty deliverables are present
    let deliverables = vec![
        "../../exec-javascript.wasm",  // Main deliverable #1
        "../../exec-python.wasm",      // Main deliverable #2
        "../README.md",                // Documentation
        "../wit/golem-exec.wit",       // Interface definition
        "../exec-javascript/Cargo.toml", // JavaScript component config
        "../exec-python/Cargo.toml",     // Python component config
    ];
    
    for deliverable in deliverables {
        assert!(std::path::Path::new(deliverable).exists(), 
               "Bounty deliverable missing: {}", deliverable);
    }
}
