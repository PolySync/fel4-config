extern crate cmake;
#[macro_use]
extern crate failure;
extern crate multimap;
extern crate toml;
use cmake::Config as CmakeConfig;
use std::borrow::Borrow;
use std::collections::{BTreeMap, HashMap};
use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

mod types;
// TODO - more selective use of types
use multimap::MultiMap;
pub use types::*;

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
}
/// Retrieve the complete contents of the fel4 toml file
pub fn get_full_manifest<P: AsRef<Path>>(path: P) -> Result<FullFel4Manifest, ConfigError> {
    let mut manifest_file = match File::open(&path) {
        Ok(f) => f,
        Err(_) => return Err(ConfigError::FileReadFailure),
    };
    let mut contents = String::new();
    match manifest_file.read_to_string(&mut contents) {
        Ok(_) => {}
        Err(_) => return Err(ConfigError::FileReadFailure),
    };
    let manifest = match contents.parse::<toml::Value>() {
        Ok(m) => m,
        Err(_) => return Err(ConfigError::TomlParseFailure),
    };
    toml_to_full_manifest(manifest)
}

/// Parse the complete contents of the fel4 toml
pub fn toml_to_full_manifest(raw: toml::Value) -> Result<FullFel4Manifest, ConfigError> {
    let fel4_table = raw.get("fel4")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| ConfigError::MissingTable("fel4".into()))?;

    let target: SupportedTarget = fel4_table
        .get("target")
        .and_then(toml::Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ConfigError::MissingRequiredProperty("fel4".into(), "target".into()))?
        .parse()
        .map_err(|e| {
            ConfigError::InvalidValueOption("target", SupportedTarget::target_names(), e)
        })?;
    let platform: SupportedPlatform = fel4_table
        .get("platform")
        .and_then(toml::Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ConfigError::MissingRequiredProperty("fel4".into(), "platform".into()))?
        .parse()
        .map_err(|e| {
            ConfigError::InvalidValueOption("platform", SupportedPlatform::platform_names(), e)
        })?;

    let artifact_path = fel4_table
        .get("artifact-path")
        .and_then(toml::Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            ConfigError::MissingRequiredProperty("fel4".into(), "artifact-path".into())
        })?;
    let specs_path = fel4_table
        .get("target-specs-path")
        .and_then(toml::Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            ConfigError::MissingRequiredProperty("fel4".into(), "target-specs-path".into())
        })?;

    let mut targets: HashMap<SupportedTarget, FullFel4Target> = HashMap::new();
    for curr_target in SupportedTarget::targets() {
        let curr_target_name = curr_target.full_name();
        let curr_target_table = match raw.get(curr_target_name).and_then(toml::Value::as_table) {
            None => continue,
            Some(t) => t,
        };
        let mut build_profile_properties: MultiMap<BuildProfile, FlatTomlProperty> =
            MultiMap::new();
        for profile in BuildProfile::build_profiles() {
            let profile_name = profile.full_name();
            let properties = match curr_target_table
                .get(profile_name)
                .and_then(toml::Value::as_table)
            {
                None => continue,
                Some(t) => extract_flat_properties(t, false).map_err(|prop_name| {
                    ConfigError::UnexpectedStructure(format!(
                        "{}.{}.{}",
                        curr_target_name, profile_name, prop_name
                    ))
                })?,
            };
            build_profile_properties
                .entry(profile)
                .or_insert_vec(properties);
        }
        let mut platform_properties: MultiMap<SupportedPlatform, FlatTomlProperty> =
            MultiMap::new();
        for platform in SupportedPlatform::platforms() {
            let platform_name = platform.full_name();
            let properties = match curr_target_table
                .get(platform_name)
                .and_then(toml::Value::as_table)
            {
                None => continue,
                Some(t) => extract_flat_properties(t, false).map_err(|prop_name| {
                    ConfigError::UnexpectedStructure(format!(
                        "{}.{}.{}",
                        curr_target_name, platform_name, prop_name
                    ))
                })?,
            };
            platform_properties
                .entry(platform)
                .or_insert_vec(properties);
        }
        let direct_properties =
            extract_flat_properties(curr_target_table, true).map_err(|prop_name| {
                ConfigError::UnexpectedStructure(format!("{}.{}", curr_target_name, prop_name))
            })?;

        targets.insert(
            curr_target.clone(),
            FullFel4Target {
                identity: curr_target,
                direct_properties: direct_properties,
                build_profile_properties: build_profile_properties,
                platform_properties: platform_properties,
            },
        );
    }

    // TODO - check for superfluous tables
    // TODO - check for superfluous properties in [fel4]

    Ok(FullFel4Manifest {
        artifact_path: artifact_path.into(),
        target_specs_path: specs_path.into(),
        selected_target: target,
        selected_platform: platform,
        targets: targets,
    })
}

fn extract_flat_properties(
    table: &BTreeMap<String, toml::Value>,
    ignore_structures: bool,
) -> Result<Vec<FlatTomlProperty>, String> {
    let mut v = Vec::new();
    for (prop_name, value) in table {
        let flat_value = match value {
            toml::Value::String(v) => FlatTomlValue::String(v.to_string()),
            toml::Value::Integer(v) => FlatTomlValue::Integer(*v),
            toml::Value::Float(v) => FlatTomlValue::Float(*v),
            toml::Value::Boolean(v) => FlatTomlValue::Boolean(*v),
            toml::Value::Datetime(v) => FlatTomlValue::Datetime(v.clone()),
            toml::Value::Array(_) | toml::Value::Table(_) => {
                if ignore_structures {
                    continue;
                } else {
                    return Err(prop_name.to_string());
                }
            }
        };
        v.push(FlatTomlProperty::new(prop_name.to_string(), flat_value));
    }
    Ok(v)
}

/// Resolve and validate a particular Fel4 configuration for the given `BuildProfile` and the
/// `selected_target` and `selected_platform` found in the `FullFel4Manifest`
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

    Ok(Fel4Config {
        artifact_path: full.borrow().artifact_path.clone(),
        target_specs_path: full.borrow().target_specs_path.clone(),
        target: selected_target,
        platform: full.borrow().selected_platform.clone(),
        build_profile: build_profile.clone(),
        properties: properties,
    })
}

fn add_properties_to_map(
    map: &mut HashMap<String, FlatTomlValue>,
    source: &Vec<FlatTomlProperty>,
) -> Result<(), ConfigError> {
    for p in source {
        match map.insert(p.name.clone(), p.value.clone()) {
            None => {}
            Some(_) => return Err(ConfigError::DuplicateProperty(p.name.clone())),
        }
    }
    Ok(())
}

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
        .map_err(|e| ManifestDiscoveryError::InvalidBuildProfile(e))?;
    Ok((PathBuf::from(manifest_path), build_profile))
}

/// Load, parse, and resolve a Fel4Config
pub fn get_fel4_config<P: AsRef<Path>>(
    fel4_manifest_path: P,
    build_profile: BuildProfile,
) -> Result<Fel4Config, ConfigError> {
    let full_manifest = get_full_manifest(fel4_manifest_path)?;
    resolve_fel4_config(full_manifest, &build_profile)
}

#[derive(Clone, Debug, Fail, PartialEq)]
pub enum CmakeConfigurationError {
    #[fail(display = "Missing the required {} environment variable", _0)]
    MissingRequiredEnvVar(String),
    #[fail(
        display = "Cargo is attempting to build for the {} target, however fel4.toml has declared the target to be {}",
        _0,
        _1
    )]
    CargoTargetToFel4TargetMismatch(String, String),
}

/// Configure a seL4_kernel CMake build configuration with data derived from the fel4.toml manifest
///
/// Assumes the presence of the CARGO_MANIFEST_DIR and TARGET environment variables from cargo
/// Assumes the seL4_kernel is at `${CARGO_MANIFEST_DIR}/deps/seL4_kernel`
pub fn configure_cmake_build(
    cmake_config: &mut CmakeConfig,
    resolved_manifest: &Fel4Config,
) -> Result<(), CmakeConfigurationError> {
    let cargo_manifest_dir = env::var("CARGO_MANIFEST_DIR").map_err(|_| {
        CmakeConfigurationError::MissingRequiredEnvVar("CARGO_MANIFEST_DIR".to_string())
    })?;
    let kernel_path = Path::new(&cargo_manifest_dir)
        .join("deps")
        .join("seL4_kernel");

    // println!("cargo:rerun-if-changed={}", fel4_manifest.display());  // TODO - move this to the calling location
    let cargo_target = env::var("TARGET")
        .map_err(|_| CmakeConfigurationError::MissingRequiredEnvVar("TARGET".to_string()))?;
    if cargo_target != resolved_manifest.target.full_name() {
        return Err(CmakeConfigurationError::CargoTargetToFel4TargetMismatch(
            cargo_target,
            resolved_manifest.target.full_name().to_string(),
        ));
    }

    // CMAKE_TOOLCHAIN_FILE is resolved immediately by CMake
    cmake_config.define("CMAKE_TOOLCHAIN_FILE", kernel_path.join("gcc.cmake"));
    cmake_config.define("KERNEL_PATH", kernel_path);

    add_cmake_definitions(cmake_config, &resolved_manifest.properties);

    // seL4 handles these so we clear them to prevent cmake-rs from
    // auto-populating
    cmake_config.define("CMAKE_C_FLAGS", "");
    cmake_config.define("CMAKE_CXX_FLAGS", "");

    // Ninja generator
    cmake_config.generator("Ninja");
    Ok(())
}

fn add_cmake_definitions(
    cmake_config: &mut CmakeConfig,
    properties: &HashMap<String, FlatTomlValue>,
) {
    for (name, value) in properties {
        add_cmake_definition(cmake_config, name, value);
    }
}

fn add_cmake_definition(config: &mut CmakeConfig, name: &String, value: &FlatTomlValue) {
    match value {
        FlatTomlValue::Boolean(b) => {
            config.define(format!("{}:BOOL", name), if *b { "ON" } else { "OFF" })
        }
        FlatTomlValue::Integer(i) => config.define(name, i.to_string()),
        FlatTomlValue::String(s) => config.define(name, s),
        FlatTomlValue::Float(f) => config.define(name, f.to_string()),
        FlatTomlValue::Datetime(d) => config.define(name, format!("{}", d)),
    };
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

}
