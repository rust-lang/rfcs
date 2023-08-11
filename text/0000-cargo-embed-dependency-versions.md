- Feature Name: `cargo_embed_dependency_versions`
- Start Date: 2019-11-03
- RFC PR: [rust-lang/rfcs#2801](https://github.com/rust-lang/rfcs/pull/2801)
- Rust Issue: None

# Summary
[summary]: #summary

Embed the crate dependency tree in a machine-readable format into compiled binaries so it could be programmatically recovered later.

# Motivation
[motivation]: #motivation

Rust is very promising for security-critical applications due to its safety guarantees, but there currently are gaps in the ecosystem that prevent it. One of them is the lack of any infrastructure for security updates.

Linux distributions alert you if you're running a vulnerable software version and you can opt in to automatic security updates. Cargo not only has no automatic update infrastructure, it doesn't even know which libraries or library versions went into compiling a certain binary, so there's no way to check if your system is vulnerable or not.

The primary use case for this information is cross-referencing versions of the dependencies against [RustSec advisory database](https://github.com/RustSec/advisory-db) and/or third-party databases such as [Common Vulnerabilities and Exposures](https://en.wikipedia.org/wiki/Common_Vulnerabilities_and_Exposures). This also enables use cases such as ensuring a fix in a depencency has been propagated across the entirety of your fleet or preventing binaries with unvetted dependencies from accidentally reaching a production environment - all with zero bookkeeping.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Every time an executable is compiled with Cargo, the dependency tree of the executable is recorded in the binary. This includes the names, versions, dependency kind (build or normal), and origin kind (crates.io, git, local filesystem, custom registry). Development dependencies are not recorded, since they cannot affect the final binary. All filesystem paths and URLs are redacted to preserve privacy. The data is encoded in JSON and compressed with zlib to reduce its size.

This data can be recovered using existing tools like `readelf` or Rust-specific tooling. It can be then used to create a Software Bill of Materials in a common format, or audit the dependency list for known vulnerabilities.

WASM, asm.js and embedded platforms are exempt from this mechanism by default since they have very strict code size requirements. For those platforms we encourage you to use tooling that record the hash of every executable in a database and associates the hash with its Cargo.lock, compiler and LLVM version used for the build.

A per-profile configuration option in `Cargo.toml` can be used to opt out of this behavior if it is not desired (e.g. when building [extremely minimal binaries](https://github.com/johnthagen/min-sized-rust)). The exact name of this option is subject to bikeshedding.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The version information is encoded in an additional arbitrary section of the executable by Cargo. The exact mechanism varies depending on the executable format (ELF, Mach-O, PE, etc.). The section name is `.dep-v0` across all platforms (subject to bikeshedding, but [within 8 bytes](https://github.com/rust-lang/rust/blob/4f7bb9890c0402cd145556ac1929d13d7524959e/compiler/rustc_codegen_ssa/src/back/metadata.rs#L462-L475)). The section name must be changed if breaking changes are made to the format.

The data is encoded in JSON which is compressed with Zlib. All arrays are sorted to not disrupt reproducible builds.

The JSON schema specifying the format is provided below. If you find Rust structures more readable, you can find them [here](https://github.com/rust-secure-code/cargo-auditable/blob/311f9932128667b8b18113becdea276b3d98aace/auditable-serde/src/lib.rs#L99-L172). In case of divergences the JSON schema provided in this RFC takes precedence.

```json
{
    "$schema": "http://json-schema.org/draft-07/schema#",
    "type": "object",
    "required": [
        "packages"
    ],
    "properties": {
        "packages": {
            "type": "array",
            "items": {
                "$ref": "#/definitions/Package"
            }
        }
    },
    "definitions": {
        "DependencyKind": {
            "type": "string",
            "enum": [
                "build",
                "normal"
            ]
        },
        "Package": {
            "description": "A single package in the dependency tree",
            "type": "object",
            "required": [
                "name",
                "source",
                "version"
            ],
            "properties": {
                "dependencies": {
                    "description": "Packages are stored in an ordered array both in the `VersionInfo` struct and in JSON. Here we refer to each package by its index in the array. May be omitted if the list is empty.",
                    "type": "array",
                    "items": {
                        "type": "integer",
                        "format": "uint",
                        "minimum": 0.0
                    }
                },
                "kind": {
                    "description": "\"build\" or \"normal\". May be omitted if set to \"normal\". If it's both a build and a normal dependency, \"normal\" is recorded.",
                    "allOf": [
                        {
                            "$ref": "#/definitions/DependencyKind"
                        }
                    ]
                },
                "name": {
                    "description": "Crate name specified in the `name` field in Cargo.toml file. Examples: \"libc\", \"rand\"",
                    "type": "string"
                },
                "root": {
                    "description": "Whether this is the root package in the dependency tree. There should only be one root package. May be omitted if set to `false`.",
                    "type": "boolean"
                },
                "source": {
                    "description": "Currently \"git\", \"local\", \"crates.io\" or \"registry\". May be extended in the future with other revision control systems, etc.",
                    "allOf": [
                        {
                            "$ref": "#/definitions/Source"
                        }
                    ]
                },
                "version": {
                    "description": "The package's version in the [semantic version](https://semver.org) format.",
                    "type": "string"
                }
            }
        },
        "Source": {
            "description": "Serializes to \"git\", \"local\", \"crates.io\" or \"registry\". May be extended in the future with other revision control systems, etc.",
            "oneOf": [
                {
                    "type": "string",
                    "enum": [
                        "crates.io",
                        "git",
                        "local",
                        "registry"
                    ]
                },
            ]
        }
    }
}
```

Not all compilations targets support embedding this data. Whether the target supports it is recorded in the [target specification JSON](https://doc.rust-lang.org/rustc/targets/custom.html). The exact name of the configuration option is subject to bikeshedding.

# Drawbacks
[drawbacks]: #drawbacks

- Slightly increases the size of the generated binaries. However, the increase is [typically below 1%](https://github.com/rust-lang/rfcs/pull/2801#issuecomment-549184251).
- Adds more platform-specific code to the build process, which needs to be maintained.
- Slightly more work need to be performed at compile time. This implies slightly slower compilation.
  - If the compilation time impact is deemed to be significant, collecting and embedding this data will be disabled by default in debug profile before stabilization. It will be possible to override this default using the per-profile configuration option.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Rationale:

- This way version information is *impossible* to misplace. As long as you have the binary, you can recover the info about dependency versions. The importance of this is impossible to overstate. This allows auditing e.g. a Docker container that you did not build yourself, or a server that somebody's set up a year ago and left no audit trail.
- A malicious actor could lie about the version information. However, doing so requires modifying the binary - and if a malicious actor can do _that,_ you are pwned anyway. So this does not create any additional attack vectors other than exploiting the tool that's recovering the version information, which can be easily sandboxed.
- Any binary authentication that might be deployed automatically applies to the version information. There is no need to separately authenticate it.
- Tooling for extracting information from binaries (such as ELF sections) is already readily available, as are zlib decompressors and JSON parsers. It can be extracted and parsed [in 5 lines of Python](https://github.com/rust-secure-code/cargo-auditable/blob/master/PARSING.md), or even with a shell one-liner in a pinch.
- This enables third parties such as cloud providers to scan your binaries for you. Google Cloud [already provides such a service](https://cloud.google.com/container-registry/docs/get-image-vulnerabilities), Amazon has [an open-source project you can deploy](https://aws.amazon.com/blogs/publicsector/detect-vulnerabilities-in-the-docker-images-in-your-applications/) while Azure [integrates several partner solutions](https://docs.microsoft.com/en-us/azure/security-center/security-center-vulnerability-assessment-recommendations). They do not support this specific format yet, but integration into Trivy was very easy, so adding support will likely be trivial.

Alternatives:

- Do nothing.
  - Identifying vulnerable binaries will remain impossible. We will see increasing number of known vulnerabilities unpatched in production.
- Track version information separately from the binaries, recording it when running `cargo install` and surfacing it through some other Cargo subcommand. When installing not though `cargo install`, rely on Linux package managers to track version information.
  - Identifying vulnerable binaries will remain impossible on all other platforms, as well as on Linux for code compiled with `cargo build`.
  - Verification by third parties will remain impossible.
- Record version information in a `&'static str` in the binary instead if ELF sections, with start/stop markers to allow black-box extraction from the outside.
  - This has been [prototyped](https://github.com/Shnatsel/rust-audit). It has the upside of allowing the binary itself to introspect its version info with little parsing, but the extraction is less efficient, and this is harder to implement and maintain.
- Record version information in an industry standard SBOM format instead of a custom format.
  - This has been prototyped, and we've found the existing formats unsuitable. The primary reasons are a significant binary size increase (the existing formats are quite verbose, not designed for this use case) and issues with reproducible builds (they require timestamps).
  - "SPDX in Zlib in a linker section" is not really an industry-standard format. Adding support for the custom format to [Syft](https://github.com/anchore/syft) was trivial, since it's nearly isomorphic to other SBOM formats, so the custom JSON encoding does not seem to add a lot of overhead to consuming this data.
  - For compatibility with systems that cannot consume this data directly, external tools can be used to convert to industry standard SBOMs. [Syft](https://github.com/anchore/syft) can already do this today.
- Record version information in debug symbols instead of binary sections.
  - Debug information formats are highly platform-specific, complex, and poorly documented. For example, Microsoft provides no documentation for Windows PDB. Extracting it would be considerably more difficult. Parsing debug information would be a major source of complexity and bugs.
  - Some Linux distributions, such as Debian, ship debug symbols separately from the binaries, and do not install the debug symbols by default. We need this information to be included in the binaries, not the debug symbols.
- Provide a Cargo wrapper or plugin to implement this, but do not put it in Cargo itself.
  - Third-party implementations cannot be perfectly reliable because Cargo does not expose sufficient information for a perfectly robust system. For example, custom target specifications are impossible to support. There are also [other corner cases](https://github.com/rust-secure-code/cargo-auditable/issues/124) that appear to be impossible to resolve based on the information from `cargo metadata` alone.
  - When people actually need this information (e.g. to check if they're impacted by a vulnerability) it is too late to reach for third-party tooling - the executables have already been built and deployed, and the information is already lost. As such, this mechanism is ineffective if it's not enabled by default.

# Prior art
[prior-art]: #prior-art

An out-of-tree implementation of this RFC exists, see [`cargo auditable`](https://github.com/rust-secure-code/cargo-auditable/), and has garnered considerable interest. NixOS and Void Linux build all their Rust packages with it today; it is also used in production at Microsoft. Extracting the embedded data is already supported by [`rust-audit-info`](https://crates.io/crates/rust-audit-info) and [Syft](https://github.com/anchore/syft). Auditing such binaries for known vulnerabilities is already supported by [`cargo audit`](https://crates.io/crates/cargo-audit) and [Trivy](https://github.com/aquasecurity/trivy).

The Rust compiler already [embeds](https://github.com/rust-lang/rust/pull/97550) compiler and LLVM version in the executables built with it.

Go compiler embeds `go.mod` dependency information into its compiled binaries. Due to Go binaries generally being far larger than Rust binaries, the binary size is not a constraint, so they embed much more information - e.g. the licence for each package in the dependency tree, which is then read by the [golicense](https://github.com/mitchellh/golicense) tool.

The most common way to manage Ruby apps involves `Gemfile.lock` which can be thought of as a runtime `Cargo.lock`. Some companies have automation searching for these files in production VMs/containers and cross-referencing them against [RubySec](https://rubysec.com/).

Since build system and package management system are usually decoupled, most other languages did not have the opportunity to implement anything like this.

In microservice environments it is fairly typical to expose an HTTP endpoint returning the application version, see e.g. [example from Go cookbook](https://blog.kowalczyk.info/article/vEja/embedding-build-number-in-go-executable.html). However, this typically does not include versions of the dependencies.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- How exactly the initial roll-out should be handled? Following the sparse index example (opt-in on nightly -> default on nightly -> opt-in on stable -> default on stable) sounds like a good idea, but sparse index is target-independent, while this feature is not. So it makes sense to enable it for Tier 1 targets first, and have it gradually expanded to Tier 2, like it was done for LLVM coverage profiling. Does it make sense to have a "stable but opt-in" period in this case?

# Future possibilities
[future-possibilities]: #future-possibilities

- Let the binary itself access this data at runtime.
  - This can be achieved today by running the extraction pipeline on `std::env::current_exe`, but that requires a minimal binary format parser, and access to `/proc` on Unix. 
  - The linker section is already given a symbol in the out-of-tree implementation, named `_AUDITABLE_VERSION_INFO`. It is possible to refer to it and access it. This has downsides such as confusing linker errors when embedding the audit data is disabled, and is out of scope of this initial RFC.
- Record and surface versions of C libraries statically linked into the Rust executable, e.g. OpenSSL.
- Include additional information, e.g. Git revision for dependencies sourced from Git repositories. This is not part of the original RFC because new fields can be added in a backwards-compatible way.
