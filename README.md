# fel4-config

Parsing, transformation and validation for fel4 manifests.

## Overview

The primary purpose of this library is to parse and validate [fel4.toml](examples/exemplar.toml)
files, which contain configuration options relevant to building seL4
with the help of the [cargo-fel4](https://github.com/PolySync/cargo-fel4) tool.

The secondary purpose of this library is to actually assist in applying these
configuration values to the CMake based build process of seL4, as encapsulated
by the [libsel4-sys](https://github.com/PolySync/libsel4-sys) repository.

## fel4.toml

fel4 manifest files are typically named `fel4.toml` and live at the root directory of a
fel4 project.  You typically don't have to manufacture them from scratch, as the
cargo-fel4 tool will generate a complete manifest as part of the `cargo fel4 new` command.

You can find an example in this repository at [examples/exemplar.toml](examples/exemplar.toml).

A fel4 manifest consists of a `[fel4]` header section followed by target-specific tables.

The `[fel4]` table selects the build-target-and-platform pair that your project will be built for,
along with some book-keeping

```toml
[fel4]
# The Rust build target triple that your fel4 project has selected
# Currently "x86_64-sel4-fel4" and "armv7-sel4-fel4" are the available options
target = "x86_64-sel4-fel4"

# The associated platform for your build target.
# "pc99" is available in combination with the "x86_64-sel4-fel4" target
# "sabre" is available in combination with the "armv7-sel4-fel4" target
platform = "pc99"

# The path relative to your project root dir where fel4 output build artifacts will be stored
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


## License

fel4-config is released under the MIT license
