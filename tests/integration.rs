extern crate fel4_config;

use std::path::PathBuf;
use fel4_config::*;

#[test]
fn parse_toml_config_happy_path() {
    let fel4_manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_configs/fel4.toml");
    let toml_config = get_toml_config(fel4_manifest, &"debug".to_string());
    assert_eq!("x86_64-sel4-fel4", &toml_config.target)
}