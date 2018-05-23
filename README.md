# fel4-config

Parsing, transformation and validation for feL4 manifests.

## Overview

The primary purpose of this library is to parse and validate [fel4.toml](examples/exemplar.toml)
files, which contain configuration options relevant to building seL4
with the help of the [cargo-fel4](https://github.com/PolySync/cargo-fel4) tool.

The secondary purpose of this library is to actually assist in applying these
configuration values to the CMake based build process of seL4, as encapsulated
by the [libsel4-sys](https://github.com/PolySync/libsel4-sys) repository.

## Getting Started

### Dependencies

`fel4-config` manages its dependencies through its Cargo.toml file, as usual for Rust projects.

### Building

`fel4-config` should build on the stable or nightly Rust toolchains.

```
cargo build
```

## Usage

feL4 manifest files are typically named `fel4.toml` and live at the root directory of a
feL4 project.  You typically don't have to manufacture them from scratch, as the
cargo-fel4 tool will generate a complete manifest as part of the `cargo fel4 new` command.


A feL4 manifest consists of a `[fel4]` header section followed by target-specific tables.

The `[fel4]` table selects the build-target-and-platform pair that your project will be built for,
along with some book-keeping

```toml
[fel4]
# The Rust build target triple that your feL4 project has selected
# Currently "x86_64-sel4-fel4" and "armv7-sel4-fel4" are the available options
target = "x86_64-sel4-fel4"

# The associated platform for your build target.
# "pc99" is available in combination with the "x86_64-sel4-fel4" target
# "sabre" is available in combination with the "armv7-sel4-fel4" target
platform = "pc99"

# The path relative to your project root dir where feL4 output build artifacts will be stored
artifact-path = "artifacts"

# The path relative to your project root where the Rust target JSON specifications are stored
# `cargo fel4 new` will generate these specifications for you by default
target-specs-path = "target_specs"
```

For the target triple you have selected, there ought to be a toml table and a few nested subtables.

```toml

# The top-level target table,
[x86_64-sel4-fel4]
BuildWithCommonSimulationSettings = true
KernelOptimisation = "-O2"
# ... Snip ... many more configuration options are possible

# A subtable with configuration options specific to the selected plaform, [$TARGET.$PLATFORM]
# Even if multiple platforms are defined in the toml for a target, only the options
# from the subtable matching the platform selected in the [fel4] header table's
# "platform" field will be applied to the final configuration.
[x86_64-sel4-fel4.pc99]
KernelX86MicroArch = "nehalem"
LibPlatSupportX86ConsoleDevice = "com1"

# [$TARGET.debug], a subtable with options only applied for debug builds
[x86_64-sel4-fel4.debug]
KernelDebugBuild = true
KernelPrinting = true

# [$TARGET.debug], a subtable with options only applied for release builds
[x86_64-sel4-fel4.release]
KernelDebugBuild = false
KernelPrinting = false

```

### Examples

You can find a complete example in this repository at [examples/exemplar.toml](examples/exemplar.toml).

### API

There are two key types provided by `fel4-config`, `FullFel4Manifest` and `Fel4Config`.

`FullFel4Manifest` represents the entire contents of a fel4.toml,
and can be produced by means of `get_full_manifest(::std::path::Path::new("./fel4.toml"))?` or `parse_full_manifest`.
These methods conduct parsing and basic validation of the manifest contents.

`Fel4Config` represents a coalesced subset of the contents of a manifest,
applied for a particular target, platform, and build profile. You can
create a `Fel4Config` from a `FullFel4Manifest` using `resolve_fel4_config`.

```rust
let full:FullFel4Manifest = get_full_manifest(manifest_file.path())
    .expect("Should be able to read the fel4.toml file");
let config:Fel4Config = resolve_fel4_config(full, &BuildProfile::Debug)
    .expect("Should have been able to resolve a config");
```

`Fel4Config` contains a resolved, deduplicated set of configuration properties.

Current applications include use in `libsel4-sys` CMake configuration, `cargo-fel4` code generation, and so forth.

See the generated Rust documents for details on individual types and functions.

```
cargo doc --open
```

## Tests

### Test Dependencies

Managed through Cargo.toml `[dev-dependencies]`

### Running Tests

Tests are executable in the usual way for Rust projects:

```
cargo test
```

# License

fel4-config is released under the MIT license
