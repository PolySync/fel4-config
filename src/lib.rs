extern crate cmake;
extern crate toml;
use cmake::Config as CmakeConfig;
use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq)]
pub enum ArchHint {
    X86,
    ARM,
    ARMV8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TomlConfig {
    pub target: String,
    pub target_arch: ArchHint,
    pub platform: String,
    pub cmake_target_config: toml::Value,
    pub cmake_profile_config: toml::Value,
    pub cmake_platform_config: toml::Value,
}

/// Returns a TomlConfig generated from a fel4.toml file.
pub fn get_toml_config(path: PathBuf, build_profile: &String) -> TomlConfig {
    let mut manifest_file = File::open(&path)
        .expect(&format!("failed to open manifest file {}", path.display()));

    let mut contents = String::new();

    manifest_file.read_to_string(&mut contents).unwrap();

    let manifest = contents
        .parse::<toml::Value>()
        .expect("failed to parse fel4.toml");

    let fel4_table = match manifest.get("fel4") {
        Some(t) => t,
        None => panic!("fel4.toml is missing fel4 table"),
    };

    let target = match fel4_table.get("target") {
        Some(t) => String::from(t.as_str().unwrap()),
        None => panic!("fel4.toml is missing target key"),
    };

    let platform = match fel4_table.get("platform") {
        Some(t) => String::from(t.as_str().unwrap()),
        None => panic!("fel4.toml is missing platform key"),
    };

    let target_config = match manifest.get(&target) {
        Some(t) => t,
        None => panic!("fel4.toml is missing the target table"),
    };

    TomlConfig {
        target: target.clone(),
        target_arch: if target.contains("arm") {
            ArchHint::ARM
        } else if target.contains("aarch64") {
            ArchHint::ARMV8
        } else if target.contains("x86") {
            ArchHint::X86
        } else {
            panic!("fel4.toml target is not supported");
        },
        platform: platform.clone(),
        cmake_target_config: target_config.clone(),
        cmake_profile_config: match target_config.get(&build_profile) {
            Some(t) => t.clone(),
            None => panic!("fel4.toml is missing build profile table"),
        },
        cmake_platform_config: match target_config.get(&platform) {
            Some(t) => t.clone(),
            None => panic!("fel4.toml is missing target platform table"),
        },
    }
}

/// Configure a CMake build configuration from toml.
///
/// Returns a TomlConfig representation of fel4.toml.
pub fn configure_cmake_build(cmake_config: &mut CmakeConfig) -> TomlConfig {
    let cargo_target = getenv_unwrap("TARGET");

    let root_dir = getenv_unwrap("CARGO_MANIFEST_DIR");

    let root_path = Path::new(&root_dir);

    let kernel_path = root_path.join("deps").join("seL4_kernel");

    let fel4_manifest = PathBuf::from(getenv_unwrap("FEL4_MANIFEST_PATH"));

    println!("cargo:rerun-if-changed={}", fel4_manifest.display());

    // parse fel4.toml
    let toml_config = get_toml_config(fel4_manifest, &getenv_unwrap("PROFILE"));

    if cargo_target != toml_config.target {
        panic!("Cargo is attempting to build for the {} target, however fel4.toml has declared the target to be {}", cargo_target, toml_config.target);
    }

    // CMAKE_TOOLCHAIN_FILE is resolved immediately by CMake
    cmake_config.define("CMAKE_TOOLCHAIN_FILE", kernel_path.join("gcc.cmake"));

    cmake_config.define("KERNEL_PATH", kernel_path);

    // add options from build profile sub-table
    add_cmake_options_from_table(
        &toml_config.cmake_profile_config,
        cmake_config,
    );

    // add options from target sub-table
    add_cmake_options_from_table(
        &toml_config.cmake_target_config,
        cmake_config,
    );

    // add options from platform sub-table
    add_cmake_options_from_table(
        &toml_config.cmake_platform_config,
        cmake_config,
    );

    // seL4 handles these so we clear them to prevent cmake-rs from
    // auto-populating
    cmake_config.define("CMAKE_C_FLAGS", "");
    cmake_config.define("CMAKE_CXX_FLAGS", "");

    // Ninja generator
    cmake_config.generator("Ninja");

    toml_config
}

/// Add CMake configurations from a toml table.
fn add_cmake_options_from_table(
    toml_table: &toml::Value,
    cmake_config: &mut CmakeConfig,
) {
    for (key, value) in toml_table.as_table().unwrap() {
        // ignore other tables within this one
        if value.is_table() {
            continue;
        }

        add_cmake_definition(key, value, cmake_config);
    }
}

/// Add a CMake configuration definition
pub fn add_cmake_definition(
    key: &String,
    value: &toml::Value,
    config: &mut CmakeConfig,
) {
    // booleans use the :<type> syntax, with ON/OFF values
    // everything else is treated as a string
    if value.type_str() == "boolean" {
        if value.as_bool().unwrap() == true {
            config.define(format!("{}:BOOL", key), "ON");
        } else {
            config.define(format!("{}:BOOL", key), "OFF");
        }
    } else if value.type_str() == "integer" {
        config.define(
            key,
            value
                .as_integer()
                .expect(&format!(
                    "failed to convert key '{}' to integer",
                    value
                ))
                .to_string(),
        );
    } else {
        config.define(
            key,
            value
                .as_str()
                .expect(&format!("failed to convert key '{}' to str", value)),
        );
    }
}

/// Return an environment variable as a String.
fn getenv_unwrap(v: &str) -> String {
    match env::var(v) {
        Ok(s) => s,
        Err(..) => panic!("environment variable `{}` not defined", v),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn getenv_unwrap_fails_loudly_on_missing_var() {
        let key = "BLERM";
        std::env::remove_var(key);
        getenv_unwrap(key);
    }

}
