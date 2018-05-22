use std::collections::HashMap;
use std::str::FromStr;
use toml;

/// Fel4 configuration for a particular target, platform, and build profile
/// tuple resolved from a FullFel4Target
#[derive(Clone, Debug, PartialEq)]
pub struct Fel4Config {
    pub artifact_path: String,
    pub target_specs_path: String,
    pub target: SupportedTarget,
    pub platform: SupportedPlatform,
    pub build_profile: BuildProfile,
    pub properties: HashMap<String, FlatTomlValue>,
}

/// A single toml key-value pair where the value only includes non-nestable
/// structures
#[derive(PartialEq, Clone, Debug)]
pub struct FlatTomlProperty {
    pub name: String,
    pub value: FlatTomlValue,
}

impl FlatTomlProperty {
    pub fn new(name: String, value: FlatTomlValue) -> Self {
        FlatTomlProperty { name, value }
    }
}

/// A subset of `toml::Value` that only includes non-nestable structures
#[derive(PartialEq, Clone, Debug)]
pub enum FlatTomlValue {
    /// Represents a TOML string
    String(String),
    /// Represents a TOML integer
    Integer(i64),
    /// Represents a TOML float
    Float(f64),
    /// Represents a TOML boolean
    Boolean(bool),
    /// Represents a TOML datetime,
    Datetime(toml::value::Datetime),
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum SupportedTarget {
    X8664Sel4Fel4,
    Armv7Sel4Fel4,
}

const TARGET_X86_64_SEL4_FEL4: &str = "x86_64-sel4-fel4";
const TARGET_ARMV7_SEL4_FEL4: &str = "armv7-sel4-fel4";

impl SupportedTarget {
    pub fn full_name(&self) -> &'static str {
        match *self {
            SupportedTarget::X8664Sel4Fel4 => TARGET_X86_64_SEL4_FEL4,
            SupportedTarget::Armv7Sel4Fel4 => TARGET_ARMV7_SEL4_FEL4,
        }
    }

    pub fn targets() -> Vec<SupportedTarget> {
        vec![SupportedTarget::X8664Sel4Fel4, SupportedTarget::Armv7Sel4Fel4]
    }

    pub fn target_names() -> Vec<String> {
        SupportedTarget::targets()
            .iter()
            .map(|t| t.full_name().into())
            .collect()
    }
}

impl FromStr for SupportedTarget {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        match s {
            TARGET_X86_64_SEL4_FEL4 => Ok(SupportedTarget::X8664Sel4Fel4),
            TARGET_ARMV7_SEL4_FEL4 => Ok(SupportedTarget::Armv7Sel4Fel4),
            _ => Err(s.to_string()),
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum SupportedPlatform {
    PC99,
    Sabre,
}

const PLATFORM_PC99: &str = "pc99";
const PLATFORM_SABRE: &str = "sabre";

impl SupportedPlatform {
    pub fn full_name(&self) -> &'static str {
        match *self {
            SupportedPlatform::PC99 => PLATFORM_PC99,
            SupportedPlatform::Sabre => PLATFORM_SABRE,
        }
    }

    pub fn platforms() -> Vec<SupportedPlatform> {
        vec![SupportedPlatform::PC99, SupportedPlatform::Sabre]
    }

    pub fn platform_names() -> Vec<String> {
        SupportedPlatform::platforms()
            .iter()
            .map(|t| t.full_name().into())
            .collect()
    }
}

impl FromStr for SupportedPlatform {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        match s {
            PLATFORM_PC99 => Ok(SupportedPlatform::PC99),
            PLATFORM_SABRE => Ok(SupportedPlatform::Sabre),
            _ => Err(s.to_string()),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub enum BuildProfile {
    Debug,
    Release,
}
const BUILD_PROFILE_DEBUG: &str = "debug";
const BUILD_PROFILE_RELEASE: &str = "release";
impl BuildProfile {
    pub fn full_name(&self) -> &'static str {
        match *self {
            BuildProfile::Debug => BUILD_PROFILE_DEBUG,
            BuildProfile::Release => BUILD_PROFILE_RELEASE,
        }
    }

    pub fn build_profiles() -> Vec<BuildProfile> {
        vec![BuildProfile::Debug, BuildProfile::Release]
    }

    pub fn build_profile_names() -> Vec<String> {
        BuildProfile::build_profiles()
            .iter()
            .map(|t| t.full_name().into())
            .collect()
    }
}

impl FromStr for BuildProfile {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        match s {
            BUILD_PROFILE_DEBUG => Ok(BuildProfile::Debug),
            BUILD_PROFILE_RELEASE => Ok(BuildProfile::Release),
            _ => Err(s.to_string()),
        }
    }
}
