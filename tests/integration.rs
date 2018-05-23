extern crate fel4_config;
extern crate tempfile;

use fel4_config::*;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn get_full_manifest_happy_path() {
    let fel4_manifest = write_exemplar_toml_to_temp_file();
    let manifest = get_full_manifest(fel4_manifest.path())
        .expect("Should be able to read the default fel4.toml");
    assert_eq!("artifacts", manifest.artifact_path);
    assert_eq!("target_specs", manifest.target_specs_path);
    assert_eq!(SupportedTarget::X8664Sel4Fel4, manifest.selected_target);
    assert_eq!(SupportedPlatform::PC99, manifest.selected_platform);
    assert_eq!(3, manifest.targets.len());
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
    let armv7_target = manifest
        .targets
        .get(&SupportedTarget::Armv7Sel4Fel4)
        .unwrap();
    assert_eq!(
        FlatTomlValue::String("aarch32".to_string()),
        armv7_target
            .direct_properties
            .iter()
            .find(|p| p.name == "KernelArmSel4Arch")
            .unwrap()
            .value
    );
    let aarch64_target = manifest
        .targets
        .get(&SupportedTarget::Aarch64Sel4Fel4)
        .unwrap();
    assert_eq!(
        FlatTomlValue::String("aarch64".to_string()),
        aarch64_target
            .direct_properties
            .iter()
            .find(|p| p.name == "KernelArmSel4Arch")
            .unwrap()
            .value
    );
}

fn write_exemplar_toml_to_temp_file() -> NamedTempFile {
    let mut tmpfile = NamedTempFile::new().unwrap();
    write!(tmpfile, "{}", get_exemplar_default_toml()).unwrap();
    tmpfile.flush().unwrap();
    tmpfile
}

#[test]
fn get_resolved_x86_64_manifest_happy_path() {
    let manifest_file = write_exemplar_toml_to_temp_file();
    let full = get_full_manifest(manifest_file.path())
        .expect("Should be able to read the default fel4.toml");
    let config = resolve_fel4_config(full, &BuildProfile::Debug)
        .expect("Should have been able to resolve all this");
    assert_eq!(SupportedTarget::X8664Sel4Fel4, config.target);
    assert_eq!(SupportedPlatform::PC99, config.platform);
    assert_eq!(BuildProfile::Debug, config.build_profile);
    assert_eq!(
        &FlatTomlValue::String("x86".to_string()),
        config.properties.get("KernelArch").unwrap()
    );
}

#[test]
fn get_resolved_armv7_manifest_happy_path() {
    let manifest_file = write_exemplar_toml_to_temp_file();
    let mut full = get_full_manifest(manifest_file.path())
        .expect("Should be able to read the default fel4.toml");
    full.selected_target = SupportedTarget::Armv7Sel4Fel4;
    full.selected_platform = SupportedPlatform::Sabre;
    let config = resolve_fel4_config(full, &BuildProfile::Debug)
        .expect("Should have been able to resolve all this");
    assert_eq!(SupportedTarget::Armv7Sel4Fel4, config.target);
    assert_eq!(SupportedPlatform::Sabre, config.platform);
    assert_eq!(BuildProfile::Debug, config.build_profile);
    assert_eq!(
        &FlatTomlValue::String("arm".to_string()),
        config.properties.get("KernelArch").unwrap()
    );
}

#[test]
fn get_resolved_aarch64_manifest_happy_path() {
    let manifest_file = write_exemplar_toml_to_temp_file();
    let mut full = get_full_manifest(manifest_file.path())
        .expect("Should be able to read the default fel4.toml");
    full.selected_target = SupportedTarget::Aarch64Sel4Fel4;
    full.selected_platform = SupportedPlatform::Tx1;
    let config = resolve_fel4_config(full, &BuildProfile::Debug)
        .expect("Should have been able to resolve all this");
    assert_eq!(SupportedTarget::Aarch64Sel4Fel4, config.target);
    assert_eq!(SupportedPlatform::Tx1, config.platform);
    assert_eq!(BuildProfile::Debug, config.build_profile);
    assert_eq!(
        &FlatTomlValue::String("arm".to_string()),
        config.properties.get("KernelArch").unwrap()
    );
}
