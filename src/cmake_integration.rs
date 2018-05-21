use cmake::Config as CmakeConfig;
use std::collections::HashMap;
/// Utilities for configuring the sel4_kernel CMake build based
/// on fel4 configuration data
///
use std::env;
use std::path::Path;
use types::*;
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
/// Configure a seL4_kernel CMake build configuration with data derived from
/// the fel4.toml manifest
///
/// Assumes `cargo_target` is a rust build target option
/// Assumes the seL4_kernel is at `${cargo_manifest_dir}/deps/seL4_kernel`
pub fn configure_cmake_build<P: AsRef<Path>>(
    cmake_config: &mut CmakeConfig,
    fel4_config: &Fel4Config,
    cargo_manifest_dir: P,
    cargo_target: &str,
) -> Result<(), CmakeConfigurationError> {
    let kernel_path = cargo_manifest_dir.as_ref().join("deps").join("seL4_kernel");

    if cargo_target != fel4_config.target.full_name() {
        return Err(CmakeConfigurationError::CargoTargetToFel4TargetMismatch(
            cargo_target.to_string(),
            fel4_config.target.full_name().to_string(),
        ));
    }

    // CMAKE_TOOLCHAIN_FILE is resolved immediately by CMake
    cmake_config.define("CMAKE_TOOLCHAIN_FILE", kernel_path.join("gcc.cmake"));
    cmake_config.define("KERNEL_PATH", kernel_path);

    add_cmake_definitions(cmake_config, &fel4_config.properties);

    // Supply additional cross compilation toolchain guidance for arm,
    // since the seL4-CMake inferred option doesn't support hardware floating point
    if fel4_config.target == SupportedTarget::ArmSel4Fel4 {
        cmake_config.define("CROSS_COMPILER_PREFIX", "arm-linux-gnueabihf-");
    }

    // seL4 handles these so we clear them to prevent cmake-rs from
    // auto-populating
    cmake_config.define("CMAKE_C_FLAGS", "");
    cmake_config.define("CMAKE_CXX_FLAGS", "");

    // Ninja generator
    cmake_config.generator("Ninja");
    Ok(())
}

/// Configure a seL4_kernel CMake build configuration with data derived from
/// the fel4.toml manifest and choice environment variables.
///
/// Assumes the presence of the CARGO_MANIFEST_DIR and TARGET environment
/// variables from cargo Assumes the seL4_kernel is at
/// `${CARGO_MANIFEST_DIR}/deps/seL4_kernel`
pub fn configure_cmake_build_from_env(
    cmake_config: &mut CmakeConfig,
    fel4_config: &Fel4Config,
) -> Result<(), CmakeConfigurationError> {
    let cargo_manifest_dir = env::var("CARGO_MANIFEST_DIR").map_err(|_| {
        CmakeConfigurationError::MissingRequiredEnvVar("CARGO_MANIFEST_DIR".to_string())
    })?;
    let cargo_target = env::var("TARGET")
        .map_err(|_| CmakeConfigurationError::MissingRequiredEnvVar("TARGET".to_string()))?;
    configure_cmake_build(cmake_config, fel4_config, cargo_manifest_dir, &cargo_target)
}

fn add_cmake_definitions(
    cmake_config: &mut CmakeConfig,
    properties: &HashMap<String, FlatTomlValue>,
) {
    for (name, value) in properties {
        add_cmake_definition(cmake_config, name, value);
    }
}

fn add_cmake_definition(config: &mut CmakeConfig, name: &str, value: &FlatTomlValue) {
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
    use super::super::*;
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn sanity_check_exemplar_cmake_configuration() {
        let mut c = CmakeConfig::new(PathBuf::from("./somewhere/bogus"));
        let full = parse_full_manifest(get_exemplar_default_toml())
            .expect("Should be able to get the default fel4.toml");
        let fel4_config =
            resolve_fel4_config(full, &BuildProfile::Debug).expect("Trouble in config resolution");
        let r = configure_cmake_build(
            &mut c,
            &fel4_config,
            Path::new("./some/repo"),
            "x86_64-sel4-fel4",
        );
        assert_eq!(Ok(()), r);
    }

    #[test]
    fn sanity_check_exemplar_cmake_configuration_target_mismatch() {
        let mut c = CmakeConfig::new(PathBuf::from("./somewhere/bogus"));
        let full = parse_full_manifest(get_exemplar_default_toml())
            .expect("Should be able to get the default fel4.toml");
        let fel4_config =
            resolve_fel4_config(full, &BuildProfile::Debug).expect("Trouble in config resolution");
        let r = configure_cmake_build(
            &mut c,
            &fel4_config,
            Path::new("./some/repo"),
            "arm-sel4-fel4",
        );
        assert_eq!(
            Err(CmakeConfigurationError::CargoTargetToFel4TargetMismatch(
                "arm-sel4-fel4".to_string(),
                "x86_64-sel4-fel4".to_string()
            )),
            r
        );
    }

    // TODO - better testing after environment variable usage is factored out of
    // configure_cmake_build
}
