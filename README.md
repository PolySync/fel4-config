# fel4-config

Parsing, transformation and validation for fel4 configuration data

## Overview

The primary purpose of this library is to parse and validate [fel4.toml](examples/exemplar.toml) files,
which contain configuration options relevant to building seL4 applications
with the help of the [cargo-fel4](https://github.com/PolySync/cargo-fel4) tool.

The secondary purpose of this library is to actually assist in applying these
configuration values to the CMake based build process of seL4, as encapsulated
by the [libsel4-sys](https://github.com/PolySync/libsel4-sys) repository.

## Licence

fel4-config is released under the MIT license
