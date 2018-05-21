extern crate cmake;
#[macro_use]
extern crate failure;
extern crate multimap;
extern crate toml;
use std::borrow::Borrow;
use std::collections::{HashMap, HashSet};
use std::env;
use std::path::{Path, PathBuf};

mod cmake_integration;
mod manifest;
mod types;
// TODO - more selective use of types
pub use cmake_integration::*;
pub use manifest::*;
pub use types::*;

/// Convenience function for getting a quick-working fel4.toml example
pub fn get_exemplar_default_toml() -> &'static str {
    include_str!("../examples/exemplar.toml")
}

/// All the things that could go wrong when reading fel4 configuration data
#[derive(Clone, Debug, Fail, PartialEq)]
pub enum ConfigError {
    #[fail(display = "Unable to read the fel4 manifest file")]
    FileReadFailure,
    #[fail(display = "The fel4 manifest file is unparseable as toml")]
    TomlParseFailure,
    #[fail(display = "The fel4 manifest file is missing the {} table", _0)]
    MissingTable(String),
    #[fail(display = "The fel4 manifest file contained an unexpected table or array {}", _0)]
    UnexpectedStructure(String),
    #[fail(display = "The [{}] table requires the {} property, but it is absent.", _0, _1)]
    MissingRequiredProperty(String, String),
    #[fail(display = "The {} property should be specified as a string, but is not", _0)]
    NonStringProperty(&'static str),
    #[fail(display = "The {} property should be one of {:?}, but is instead {}", _0, _1, _2)]
    InvalidValueOption(&'static str, Vec<String>, String),
    #[fail(
        display = "The fel4 manifest had a duplicate property {} when resolved to a canonical set",
        _0
    )]
    DuplicateProperty(String),
    #[fail(display = "The {} property was supplied, but is not on the permitted whitelist", _0)]
    NonWhitelistProperty(String),
}

/// Resolve and validate a particular Fel4 configuration for the given
/// `BuildProfile` and the `selected_target` and `selected_platform` found in
/// the `FullFel4Manifest`
pub fn resolve_fel4_config<M: Borrow<FullFel4Manifest>>(
    full: M,
    build_profile: &BuildProfile,
) -> Result<Fel4Config, ConfigError> {
    let selected_target = full.borrow().selected_target.clone();
    let platform = full.borrow().selected_platform.clone();
    let target = full.borrow()
        .targets
        .get(&selected_target)
        .ok_or_else(|| ConfigError::MissingTable(selected_target.full_name().to_string()))?;

    let mut properties = HashMap::new();
    add_properties_to_map(&mut properties, &target.direct_properties)?;
    let profile_properties = target
        .build_profile_properties
        .get_vec(build_profile)
        .ok_or_else(|| {
            ConfigError::MissingTable(format!(
                "{}.{}",
                selected_target.full_name(),
                build_profile.full_name()
            ))
        })?;
    add_properties_to_map(&mut properties, profile_properties)?;

    let platform_properties = target
        .platform_properties
        .get_vec(&platform)
        .ok_or_else(|| {
            ConfigError::MissingTable(format!(
                "{}.{}",
                selected_target.full_name(),
                platform.full_name()
            ))
        })?;
    add_properties_to_map(&mut properties, platform_properties)?;
    let whitelist: HashSet<String> = ALL_PROPERTIES_WHITELIST
        .iter()
        .map(|s| s.to_string())
        .collect();
    for k in properties.keys() {
        if !whitelist.contains(k) {
            return Err(ConfigError::NonWhitelistProperty(k.to_string()));
        }
    }

    Ok(Fel4Config {
        artifact_path: full.borrow().artifact_path.clone(),
        target_specs_path: full.borrow().target_specs_path.clone(),
        target: selected_target,
        platform: full.borrow().selected_platform.clone(),
        build_profile: build_profile.clone(),
        properties,
    })
}

fn add_properties_to_map(
    map: &mut HashMap<String, FlatTomlValue>,
    source: &[FlatTomlProperty],
) -> Result<(), ConfigError> {
    for p in source {
        match map.insert(p.name.clone(), p.value.clone()) {
            None => {}
            Some(_) => return Err(ConfigError::DuplicateProperty(p.name.clone())),
        }
    }
    Ok(())
}

const ALL_PROPERTIES_WHITELIST: &'static [&'static str] = &[
    "BuildWithCommonSimulationSettings",
    "KernelOptimisation",
    "KernelVerificationBuild",
    "KernelBenchmarks",
    "KernelFastpath",
    "LibSel4FunctionAttributes",
    "KernelNumDomains",
    "HardwareDebugAPI",
    "KernelColourPrinting",
    "KernelFWholeProgram",
    "KernelResetChunkBits",
    "LibSel4DebugAllocBufferEntries",
    "LibSel4DebugFunctionInstrumentation",
    "KernelNumPriorities",
    "KernelStackBits",
    "KernelTimeSlice",
    "KernelTimerTickMS",
    "KernelUserStackTraceLength",
    "KernelArch",
    "KernelX86Sel4Arch",
    "KernelMaxNumNodes",
    "KernelRetypeFanOutLimit",
    "KernelRootCNodeSizeBits",
    "KernelMaxNumBootinfoUntypedCaps",
    "KernelSupportPCID",
    "KernelCacheLnSz",
    "KernelDebugDisablePrefetchers",
    "KernelExportPMCUser",
    "KernelFPU",
    "KernelFPUMaxRestoresSinceSwitch",
    "KernelFSGSBase",
    "KernelHugePage",
    "KernelIOMMU",
    "KernelIRQController",
    "KernelIRQReporting",
    "KernelLAPICMode",
    "KernelMaxNumIOAPIC",
    "KernelMaxNumWorkUnitsPerPreemption",
    "KernelMultiboot1Header",
    "KernelMultiboot2Header",
    "KernelMultibootGFXMode",
    "KernelSkimWindow",
    "KernelSyscall",
    "KernelVTX",
    "KernelX86DangerousMSR",
    "KernelX86IBPBOnContextSwitch",
    "KernelX86IBRSMode",
    "KernelX86RSBOnContextSwitch",
    "KernelXSaveSize",
    "LinkPageSize",
    "UserLinkerGCSections",
    "KernelX86MicroArch",
    "LibPlatSupportX86ConsoleDevice",
    "KernelDebugBuild",
    "KernelPrinting",
    "KernelArmSel4Arch",
    "KernelAArch32FPUEnableContextSwitch",
    "KernelDebugDisableBranchPrediction",
    "KernelIPCBufferLocation",
    "KernelARMPlatform",
    "ElfloaderImage",
    "ElfloaderMode",
    "ElfloaderErrata764369",
    "KernelArmEnableA9Prefetcher",
    "KernelArmExportPMUUser",
    "KernelDebugDisableL2Cache",
];
/// Things that can go wrong when trying to rely on environment variables
/// to locate the fel4 manifest and its parameterization.
#[derive(Clone, Debug, Fail, PartialEq)]
pub enum ManifestDiscoveryError {
    #[fail(display = "Required environment variable {} was absent", _0)]
    MissingEnvVar(String),
    #[fail(
        display = "The PROFILE environment variable had a value {} that could not be interpreted as a BuildProfile instance",
        _0
    )]
    InvalidBuildProfile(String),
}

/// Read environment variables to discover the information necessary to
/// read and resolve a `Fel4Config`
pub fn infer_manifest_location_from_env() -> Result<(PathBuf, BuildProfile), ManifestDiscoveryError>
{
    let manifest_path = env::var("FEL4_MANIFEST_PATH")
        .map_err(|_| ManifestDiscoveryError::MissingEnvVar("FEL4_MANIFEST_PATH".to_string()))?;
    let raw_profile = env::var("PROFILE")
        .map_err(|_| ManifestDiscoveryError::MissingEnvVar("PROFILE".to_string()))?;
    let build_profile: BuildProfile = raw_profile
        .parse()
        .map_err(ManifestDiscoveryError::InvalidBuildProfile)?;
    Ok((PathBuf::from(manifest_path), build_profile))
}

/// Load, parse, and resolve a Fel4Config
pub fn get_fel4_config<P: AsRef<Path>>(
    fel4_manifest_path: P,
    build_profile: &BuildProfile,
) -> Result<Fel4Config, ConfigError> {
    let full_manifest = get_full_manifest(fel4_manifest_path)?;
    resolve_fel4_config(full_manifest, build_profile)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infer_manifest_location_from_env_happy_path() {
        std::env::set_var("PROFILE", "debug");
        std::env::set_var("FEL4_MANIFEST_PATH", "./somewhere/else");
        let (p, b) = infer_manifest_location_from_env().expect("Oh no");
        assert_eq!(PathBuf::from("./somewhere/else"), p);
        assert_eq!(BuildProfile::Debug, b);
    }

    #[test]
    fn exemplar_toml_is_fully_valid() {
        let full = parse_full_manifest(get_exemplar_default_toml())
            .expect("Should be able to get the default fel4.toml");
        let _ = resolve_fel4_config(full, &BuildProfile::Debug)
            .expect("Should be able to resolve config");
    }

    #[test]
    fn exemplar_toml_calls_return_identical() {
        let a = get_exemplar_default_toml();
        let b = get_exemplar_default_toml();
        assert_eq!(a, b);
    }

    #[test]
    fn missing_selected_target_get_caught_in_config_resolution() {
        let manifest = parse_full_manifest(
            r#"[fel4]
            target = "x86_64-sel4-fel4"
            platform = "pc99"
            artifact-path = "artifacts/path/nested"
            target-specs-path = "where/are/rust/targets"
            [arm-sel4-fel4]
            KernelOptimisation = "-O2"
            [arm-sel4-fel4.debug]
            KernelPrinting = true
            "#,
        ).expect("Should have been able to parse manifest");
        assert_eq!(
            Err(ConfigError::MissingTable("x86_64-sel4-fel4".into())),
            resolve_fel4_config(manifest, &BuildProfile::Debug)
        );
    }

    #[test]
    fn duplicate_property_gets_caught_in_config_resolution() {
        let manifest = parse_full_manifest(
            r#"[fel4]
            target = "x86_64-sel4-fel4"
            platform = "pc99"
            artifact-path = "artifacts/path/nested"
            target-specs-path = "where/are/rust/targets"

            [x86_64-sel4-fel4]
            KernelPrinting = false

            [x86_64-sel4-fel4.debug]
            KernelPrinting = true

            [x86_64-sel4-fel4.pc99]
            KernelX86MicroArch = "nehalem"
            "#,
        ).expect("Should have been able to parse manifest");
        assert_eq!(
            Err(ConfigError::DuplicateProperty("KernelPrinting".into())),
            resolve_fel4_config(manifest, &BuildProfile::Debug)
        );
    }

    #[test]
    fn non_whitelist_property_gets_caught_in_config_resolution() {
        let manifest = parse_full_manifest(
            r#"[fel4]
            target = "x86_64-sel4-fel4"
            platform = "pc99"
            artifact-path = "artifacts/path/nested"
            target-specs-path = "where/are/rust/targets"

            [x86_64-sel4-fel4]
            KernelArch = "x86"

            [x86_64-sel4-fel4.debug]
            KernelPrinting = true

            [x86_64-sel4-fel4.pc99]
            SomeUnallowedProperty = "foo"
            "#,
        ).expect("Should have been able to parse manifest");
        assert_eq!(
            Err(ConfigError::NonWhitelistProperty(
                "SomeUnallowedProperty".into()
            )),
            resolve_fel4_config(manifest, &BuildProfile::Debug)
        );
    }

}
