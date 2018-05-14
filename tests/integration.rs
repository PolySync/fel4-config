extern crate fel4_config;

use fel4_config::*;
use std::path::PathBuf;

#[test]
fn get_full_manifest_happy_path() {
    let fel4_manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_configs/fel4.toml");
    let manifest =
        get_full_manifest(fel4_manifest).expect("Should be able to read the default fel4.toml");
    assert_eq!("artifacts", manifest.artifact_path);
    assert_eq!("targets", manifest.target_specs_path);
    assert_eq!(SupportedTarget::X8664Sel4Fel4, manifest.selected_target);
    assert_eq!(SupportedPlatform::PC99, manifest.selected_platform);
    assert_eq!(2, manifest.targets.len());
    let x86_target = manifest
        .targets
        .get(&SupportedTarget::X8664Sel4Fel4)
        .unwrap();
    assert!(
        x86_target
            .direct_properties
            .iter()
            .any(|p| p.name == "BuildWithCommonSimulationSettings")
    );
    assert_eq!(
        FlatTomlValue::String("x86".to_string()),
        x86_target
            .direct_properties
            .iter()
            .find(|p| p.name == "KernelArch")
            .unwrap()
            .value
    );
    assert_eq!(
        FlatTomlValue::Boolean(true),
        x86_target
            .build_profile_properties
            .get_vec(&BuildProfile::Debug)
            .unwrap()
            .iter()
            .find(|p| p.name == "KernelDebugBuild")
            .unwrap()
            .value
    );
    assert_eq!(
        FlatTomlValue::Boolean(false),
        x86_target
            .build_profile_properties
            .get_vec(&BuildProfile::Release)
            .unwrap()
            .iter()
            .find(|p| p.name == "KernelDebugBuild")
            .unwrap()
            .value
    );
    assert_eq!(
        FlatTomlValue::String("nehalem".to_string()),
        x86_target
            .platform_properties
            .get_vec(&SupportedPlatform::PC99)
            .unwrap()
            .iter()
            .find(|p| p.name == "KernelX86MicroArch")
            .unwrap()
            .value
    );
    let arm_target = manifest.targets.get(&SupportedTarget::ArmSel4Fel4).unwrap();
    assert_eq!(
        FlatTomlValue::String("aarch32".to_string()),
        arm_target
            .direct_properties
            .iter()
            .find(|p| p.name == "KernelArmSel4Arch")
            .unwrap()
            .value
    );
}

#[test]
fn get_resolved_manifest_happy_path() {
    let fel4_manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_configs/fel4.toml");
    let full =
        get_full_manifest(fel4_manifest).expect("Should be able to read the default fel4.toml");
    let manifest = resolve_fel4_config(full, &BuildProfile::Debug)
        .expect("Should have been able to resolve all this");
    assert_eq!(SupportedTarget::X8664Sel4Fel4, manifest.target);
    assert_eq!(SupportedPlatform::PC99, manifest.platform);
    assert_eq!(BuildProfile::Debug, manifest.build_profile);
    assert_eq!(
        &FlatTomlValue::String("x86".to_string()),
        manifest.properties.get("KernelArch").unwrap()
    );
}
