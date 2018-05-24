/// Related to the parsing and representation of the full fel4 manifest
use multimap::MultiMap;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use toml;

use super::ConfigError;
use types::*;
/// The full content of a fel4 manifest
#[derive(Clone, Debug, PartialEq)]
pub struct FullFel4Manifest {
    pub artifact_path: String,
    pub target_specs_path: String,
    pub selected_target: SupportedTarget,
    pub selected_platform: SupportedPlatform,
    pub targets: HashMap<SupportedTarget, FullFel4Target>,
}

/// The full content of a target within a fel4 manifest
#[derive(Clone, Debug, PartialEq)]
pub struct FullFel4Target {
    pub identity: SupportedTarget,
    pub direct_properties: Vec<FlatTomlProperty>,
    pub build_profile_properties: MultiMap<BuildProfile, FlatTomlProperty>,
    pub platform_properties: MultiMap<SupportedPlatform, FlatTomlProperty>,
}

/// Retrieve the complete contents of the fel4 toml from a file
pub fn get_full_manifest<P: AsRef<Path>>(path: P) -> Result<FullFel4Manifest, ConfigError> {
    let mut manifest_file = File::open(&path).map_err(|_| ConfigError::FileReadFailure)?;
    let mut toml_string = String::new();
    let _size = manifest_file
        .read_to_string(&mut toml_string)
        .map_err(|_| ConfigError::FileReadFailure)?;
    parse_full_manifest(toml_string)
}

/// Retrieve the complete contents of the fel4 toml from a string
pub fn parse_full_manifest<S: AsRef<str>>(toml_string: S) -> Result<FullFel4Manifest, ConfigError> {
    let manifest = toml_string
        .as_ref()
        .parse::<toml::Value>()
        .map_err(|_| ConfigError::TomlParseFailure)?;
    toml_to_full_manifest(&manifest)
}

#[derive(Clone, Debug, PartialEq)]
struct Fel4Header {
    pub artifact_path: String,
    pub target_specs_path: String,
    pub selected_target: SupportedTarget,
    pub selected_platform: SupportedPlatform,
}

/// Internal convenience to break out the header table parsing
fn parse_fel4_header(raw: &toml::Value) -> Result<Fel4Header, ConfigError> {
    let fel4_table = raw
        .get("fel4")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| ConfigError::MissingTable("fel4".into()))?;

    has_only_approved_substructures(fel4_table, None)
        .map_err(|name| ConfigError::UnexpectedStructure(format!("fel4.{}", name)))?;

    let selected_target: SupportedTarget = fel4_table
        .get("target")
        .and_then(toml::Value::as_str)
        .and_then(|s| if s.is_empty() { None } else { Some(s) })
        .ok_or_else(|| ConfigError::MissingRequiredProperty("fel4".into(), "target".into()))?
        .parse()
        .map_err(|e| {
            ConfigError::InvalidValueOption("target", SupportedTarget::target_names(), e)
        })?;
    let selected_platform: SupportedPlatform = fel4_table
        .get("platform")
        .and_then(toml::Value::as_str)
        .and_then(|s| if s.is_empty() { None } else { Some(s) })
        .ok_or_else(|| ConfigError::MissingRequiredProperty("fel4".into(), "platform".into()))?
        .parse()
        .map_err(|e| {
            ConfigError::InvalidValueOption("platform", SupportedPlatform::platform_names(), e)
        })?;

    let artifact_path = fel4_table
        .get("artifact-path")
        .ok_or_else(|| ConfigError::MissingRequiredProperty("fel4".into(), "artifact-path".into()))
        .and_then(|o| {
            o.as_str()
                .ok_or_else(|| ConfigError::NonStringProperty("artifact-path"))
        })
        .and_then(|s| {
            if s.is_empty() {
                Err(ConfigError::MissingRequiredProperty(
                    "fel4".into(),
                    "artifact-path".into(),
                ))
            } else {
                Ok(s)
            }
        })?
        .to_string();
    let target_specs_path = fel4_table
        .get("target-specs-path")
        .ok_or_else(|| {
            ConfigError::MissingRequiredProperty("fel4".into(), "target-specs-path".into())
        })
        .and_then(|o| {
            o.as_str()
                .ok_or_else(|| ConfigError::NonStringProperty("target-specs-path"))
        })
        .and_then(|s| {
            if s.is_empty() {
                Err(ConfigError::MissingRequiredProperty(
                    "fel4".into(),
                    "target-specs-path".into(),
                ))
            } else {
                Ok(s)
            }
        })?
        .to_string();
    Ok(Fel4Header {
        artifact_path,
        target_specs_path,
        selected_target,
        selected_platform,
    })
}

/// Parse the complete contents of the fel4 toml
pub fn toml_to_full_manifest(raw: &toml::Value) -> Result<FullFel4Manifest, ConfigError> {
    let Fel4Header {
        artifact_path,
        target_specs_path,
        selected_target,
        selected_platform,
    } = parse_fel4_header(&raw)?;

    // Parse the target subtables
    let allowed_target_subtable_names: HashSet<String> = SupportedPlatform::platform_names()
        .into_iter()
        .chain(BuildProfile::build_profile_names().into_iter())
        .collect();
    let mut targets: HashMap<SupportedTarget, FullFel4Target> = HashMap::new();
    for curr_target in SupportedTarget::targets() {
        let curr_target_name = curr_target.full_name();
        let curr_target_table = match raw.get(curr_target_name).and_then(toml::Value::as_table) {
            None => continue,
            Some(t) => t,
        };
        has_only_approved_substructures(curr_target_table, Some(&allowed_target_subtable_names))
            .map_err(|prop_name| {
                ConfigError::UnexpectedStructure(format!("{}.{}", curr_target_name, prop_name))
            })?;

        let mut build_profile_properties: MultiMap<BuildProfile, FlatTomlProperty> =
            MultiMap::new();
        for profile in BuildProfile::build_profiles() {
            let profile_name = profile.full_name();
            let properties = match curr_target_table
                .get(profile_name)
                .and_then(toml::Value::as_table)
            {
                None => continue,
                Some(t) => extract_flat_properties(t).map_err(|prop_name| {
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
                Some(t) => extract_flat_properties(t).map_err(|prop_name| {
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

        let table_minus_approved_subtables = curr_target_table
            .iter()
            .filter(|&(k, _)| !allowed_target_subtable_names.contains(&(*k).clone()))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let direct_properties =
            extract_flat_properties(&table_minus_approved_subtables).map_err(|prop_name| {
                ConfigError::UnexpectedStructure(format!("{}.{}", curr_target_name, prop_name))
            })?;

        targets.insert(
            curr_target,
            FullFel4Target {
                identity: curr_target,
                direct_properties,
                build_profile_properties,
                platform_properties,
            },
        );
    }

    Ok(FullFel4Manifest {
        artifact_path,
        target_specs_path,
        selected_target,
        selected_platform,
        targets,
    })
}

fn has_only_approved_substructures(
    map: &BTreeMap<String, toml::Value>,
    approved_substructures: Option<&HashSet<String>>,
) -> Result<(), String> {
    for (k, v) in map {
        match v {
            &toml::Value::Array(_) | toml::Value::Table(_) => {
                if let Some(substructure_whitelist) = approved_substructures {
                    if substructure_whitelist.contains(k) {
                        continue;
                    } else {
                        return Err(k.to_string());
                    }
                } else {
                    return Err(k.to_string());
                }
            }
            _ => continue,
        }
    }
    Ok(())
}

fn extract_flat_properties(
    table: &BTreeMap<String, toml::Value>,
) -> Result<Vec<FlatTomlProperty>, String> {
    let mut v = Vec::new();
    for (prop_name, value) in table {
        let flat_value = match value {
            &toml::Value::String(ref v) => FlatTomlValue::String(v.to_string()),
            &toml::Value::Integer(v) => FlatTomlValue::Integer(v),
            &toml::Value::Float(v) => FlatTomlValue::Float(v),
            &toml::Value::Boolean(v) => FlatTomlValue::Boolean(v),
            &toml::Value::Datetime(ref v) => FlatTomlValue::Datetime(v.clone()),
            &toml::Value::Array(_) | toml::Value::Table(_) => {
                return Err(prop_name.to_string());
            }
        };
        v.push(FlatTomlProperty::new(prop_name.to_string(), flat_value));
    }
    Ok(v)
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn bogus_file_unreadable() {
        assert_eq!(
            Err(ConfigError::FileReadFailure),
            get_full_manifest(PathBuf::from("path/to/nowhere"))
        );
    }

    #[test]
    fn non_toml_file_unparseable() {
        assert_eq!(
            Err(ConfigError::TomlParseFailure),
            parse_full_manifest("<hey>not toml</hey>")
        );
    }

    #[test]
    fn toml_file_without_fel4_table() {
        assert_eq!(
            Err(ConfigError::MissingTable("fel4".into())),
            parse_full_manifest(
                "just = true
            some = \"unrelated property\""
            )
        );
    }

    #[test]
    fn fel4_table_missing_target() {
        assert_eq!(
            Err(ConfigError::MissingRequiredProperty(
                "fel4".into(),
                "target".into()
            )),
            parse_full_manifest(
                r#"[fel4]
            wrong_properties = true"#
            )
        );
    }

    #[test]
    fn fel4_table_invalid_target() {
        assert_eq!(
            Err(ConfigError::InvalidValueOption(
                "target",
                SupportedTarget::target_names(),
                "wrong".into()
            )),
            parse_full_manifest(
                r#"[fel4]
            target = "wrong""#
            )
        );
    }

    #[test]
    fn fel4_table_missing_platform() {
        assert_eq!(
            Err(ConfigError::MissingRequiredProperty(
                "fel4".into(),
                "platform".into()
            )),
            parse_full_manifest(
                r#"[fel4]
            target = "x86_64-sel4-fel4""#
            )
        );
    }

    #[test]
    fn fel4_table_invalid_platform() {
        assert_eq!(
            Err(ConfigError::InvalidValueOption(
                "platform",
                SupportedPlatform::platform_names(),
                "wrong".into()
            )),
            parse_full_manifest(
                r#"[fel4]
            target = "x86_64-sel4-fel4"
            platform = "wrong""#
            )
        );
    }

    #[test]
    fn fel4_table_missing_artifact_path() {
        assert_eq!(
            Err(ConfigError::MissingRequiredProperty(
                "fel4".into(),
                "artifact-path".into()
            )),
            parse_full_manifest(
                r#"[fel4]
            target = "x86_64-sel4-fel4"
            platform = "pc99""#
            )
        );
    }

    #[test]
    fn fel4_table_missing_target_specs_path() {
        assert_eq!(
            Err(ConfigError::MissingRequiredProperty(
                "fel4".into(),
                "target-specs-path".into()
            )),
            parse_full_manifest(
                r#"[fel4]
            target = "x86_64-sel4-fel4"
            platform = "pc99"
            artifact-path = "artifacts/path/nested""#
            )
        );
    }

    #[test]
    fn wrong_type_target_specs_path_in_fel4() {
        assert_eq!(
            Err(ConfigError::NonStringProperty("target-specs-path")),
            parse_full_manifest(
                r#"[fel4]
            target = "x86_64-sel4-fel4"
            platform = "pc99"
            artifact-path = "somewhere"
            target-specs-path = true
            "#
            )
        );
    }

    #[test]
    fn wrong_type_artifact_path_in_fel4() {
        assert_eq!(
            Err(ConfigError::NonStringProperty("artifact-path")),
            parse_full_manifest(
                r#"[fel4]
            target = "x86_64-sel4-fel4"
            platform = "pc99"
            artifact-path = true
            target-specs-path = "where/are/rust/targets"
            "#
            )
        );
    }

    #[test]
    fn unexpected_structure_in_fel4() {
        assert_eq!(
            Err(ConfigError::UnexpectedStructure("fel4.custom".into())),
            parse_full_manifest(
                r#"[fel4]
            target = "x86_64-sel4-fel4"
            platform = "pc99"
            artifact-path = "artifacts/path/nested"
            target-specs-path = "where/are/rust/targets"
            [fel4.custom]
            SomeProp = "hello"
            "#
            )
        );
    }

    #[test]
    fn unexpected_structure_in_target() {
        assert_eq!(
            Err(ConfigError::UnexpectedStructure(
                "x86_64-sel4-fel4.custom".into()
            )),
            parse_full_manifest(
                r#"[fel4]
            target = "x86_64-sel4-fel4"
            platform = "pc99"
            artifact-path = "artifacts/path/nested"
            target-specs-path = "where/are/rust/targets"
            [x86_64-sel4-fel4]
            SomeProp = "hello"
            [x86_64-sel4-fel4.custom]
            NestedProp = true
            "#
            )
        );
    }

    #[test]
    fn unexpected_structure_in_target_platform() {
        assert_eq!(
            Err(ConfigError::UnexpectedStructure(
                "x86_64-sel4-fel4.pc99.custom".into()
            )),
            parse_full_manifest(
                r#"[fel4]
            target = "x86_64-sel4-fel4"
            platform = "pc99"
            artifact-path = "artifacts/path/nested"
            target-specs-path = "where/are/rust/targets"
            [x86_64-sel4-fel4]
            SomeProp = "hello"
            [x86_64-sel4-fel4.pc99]
            SomethingPlatformy = true
            [x86_64-sel4-fel4.pc99.custom]
            DeepNesting = true
            "#
            )
        );
    }

    #[test]
    fn unexpected_structure_in_target_build_profile() {
        assert_eq!(
            Err(ConfigError::UnexpectedStructure(
                "x86_64-sel4-fel4.debug.custom".into()
            )),
            parse_full_manifest(
                r#"[fel4]
            target = "x86_64-sel4-fel4"
            platform = "pc99"
            artifact-path = "artifacts/path/nested"
            target-specs-path = "where/are/rust/targets"
            [x86_64-sel4-fel4]
            SomeProp = "hello"
            [x86_64-sel4-fel4.debug]
            KernelPrinting = true
            [x86_64-sel4-fel4.debug.custom]
            DeepNesting = true
            "#
            )
        );
    }
}
