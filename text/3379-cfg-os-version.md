- Feature Name: `cfg-os-version`
- Start Date: 2022-10-31
- RFC PR: TBD
- Rust Issue: TBD

# Summary
[summary]: #summary

A new `cfg` key-value option `target_os_version`, and new predicates `os_version_eq`, `os_version_min`, and `os_version_range` that allow users to declare the primary (target-defined) API level required/supported by a block.  A second version of the predicates would take an additional key argument allowing targets to specify the versions of different OS components, e.g. kernel and libc versions.

For instance, the standard library's Windows Mutex implementation could potentially take advantage of this mechanism instead of relying on runtime API detection:

```rust
pub unsafe fn unlock(&self) {
    *self.held.get() = false;
    if cfg!(os_version_min("windows", "6.1.7600")) { // `cfg!(os_version_min("Windows7"))` is also possible
        c::ReleaseSRWLockExclusive(raw(self))
    } else {
        match kind() {
            Kind::SRWLock => c::ReleaseSRWLockExclusive(raw(self)),
            Kind::CriticalSection => (*self.remutex()).unlock(),
        }
    }
}
```

# Motivation
[motivation]: #motivation

The target API version is the version number of the "API set" that a particular binary relies on in order to run properly.  An API set is the set of APIs that a host operating system makes available for use by binaries running on that platform.  Newer versions of a platform may either add or remove APIs from the API set.

Crates including the standard library must account for various API version requirements for the crate to be able to run.  Rust currently has no mechanism for crates to compile different code (or to gracefully fail to compile) depending on the minimum targeted API version. This leads to the following issues:

* Relying on dynamic detection of API support has a runtime cost. The standard library often performs [dynamic API detection](https://github.com/rust-lang/rust/blob/f283d3f02cf3ed261a519afe05cde9e23d1d9278/library/std/src/sys/windows/compat.rs) falling back to older (and less ideal) APIs or forgoing entire features when a certain API is not available. For example, the [current `Mutex` impl](https://github.com/rust-lang/rust/blob/234099d1d12bef9d6e81a296222fbc272dc51d89/library/std/src/sys/windows/mutex.rs#L1-L20) has a Windows XP fallback. Users who only ever intend to run their code on newer versions of Windows will still pay a runtime cost for this dynamic API detection. Providing a mechanism for specifying which minimum API version the user cares about, allows for statically specifying which APIs a binary can use.
* Certain features cannot be dynamically detected and thus limit possible implementations. The libc crate must use [a raw syscalls on Android for `accept4`](https://github.com/rust-lang/libc/pull/1968), because this was only exposed in libc in version 21 of the Android API.  Additionally libstd must dynamically load `signal` for all versions of Android despite it being required only for versions 19 and below. In the future there might be similar changes where there is no way to implement a solution for older versions.
* Trying to compile code with an implicit dependency on a API version greater than what is supported by the target platform leads to linker errors. For example, the `x86_64-pc-windows-msvc` target's rustc implementation requires `SetThreadErrorMode` which was introduced in Windows 7. This means trying to build the compiler on older versions of Windows will fail with [a less than helpful linker error](https://github.com/rust-lang/rust/issues/35471).


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Rust targets are often thought of as monoliths.  The thought is that if you compile a binary for that target, that binary should be able to run on any system that fits that target's description. However, this is not actually true.  For example, when compiling for `x86_64-pc-windows-msvc` and linking with the standard library, my binary has implicitly taken a dependency on a set of APIs that Windows exposes for certain functionality.  If I try to run my binary on older systems that do not have those APIs, then my binary will fail to run.  When compiling for a certain target, you are therefore declaring a dependency on a minimum target API version that you rely on for your binary to run.

By default, the standard library uses a sensible minimum API version.  For example, for `x86_64-pc-windows-msvc` the minimum API version is "6.1.7600" which corresponds to Windows 7.  However, there are good reasons why you might want control of how your code is compiled depending on what the minimum API version is set as.  For instance, if you want to:

* set your crate's minimum API version higher than that of the standard library.
* change certain implementation details of your crate depending on what a downstream user sets their minimum API version to be.
* have some sensible compiler error if users of your crate require a lower minimum API version that you require.

In cases like these you can use the `os_version_min` and `os_version_range` predicates to specify the minimum API levels of various parts of the operating system.  For example:

* `os_version_min(“windows”, <string>)` would test the [minimum build version](https://gaijin.at/en/infos/windows-version-numbers) of Windows.
* `os_version_min(“libc”, <string>)` would test the version of libc in use.
* `os_version_min(“kernel”, <string>)` would test the version of the kernel in use.

Let’s use `os_version_min(“windows”, …)` as an example.  It should be clear how this example would be extended to the other `cfg` predicates. The predicate allows you to conditionally compile code based on the set minimum API version. For example an implementation of mutex locking on Windows might look like this:

```rust
pub unsafe fn unlock(&self) {
    *self.held.get() = false;
    if cfg!(os_version_min(“windows”, "6.0.6000") { // API version greater than Vista
        c::ReleaseSRWLockExclusive(raw(self)) // Use the optimized ReleaseSRWLockExclusive routine
    } else {
        (*self.remutex()).unlock()  // Fall back to an alternative that works on older Windows versions
    }
}
```

Crate authors can set the API requirements of their Cargo manifest file under the  [`target key`](https://doc.rust-lang.org/cargo/reference/config.html#target) like so (suggested variable name/syntax only):

```toml
[target.x86_64-pc-windows-msvc]
min_os_version_windows = "6.0.6000" # Vista
```

If a crate specifies a version value lower than that of one of it's dependencies an error will be issued.

When compiling, the user can provide the API levels to compile for: `rustc --cfg 'target_os_version.windows="6.0.6000"'`.

If an end user sets their `target_os_version.windows` to an incompatible version then the user receives an error. For instance, in the example above where the user is setting their `min_os_version_windows` to Windows Vista, they will receive an error when linking with the standard library which imposes Windows 7 as its minimum `target_os_version.windows` by default for the `x86_64-pc-windows-msvc` target.

If a library does not explicitly set its `min_os_target_windows` value, it will automatically be set to the largest `min_windows_build_version` of all of its transitive dependencies.

The `os_version_min` predicate evaluates to `true` when the provided string is greater than or equal to the target OS version and the marked code will be compiled.

For targets where `os_version_min(“windows”, …)` does not make sense (i.e., non-Windows targets), the `cfg` predicate will return `false` and emit a warning saying that the particular `cfg` predicate is not supported on that target. Therefore, it's important to pair `os_version_min(“windows”, …)` with a `cfg(windows)` using the existing mechanisms for combining `cfg` predicates.

The above example works exactly the same way with the other platform API `cfg` predicates just with different values and different target support.

These predicates do not assume any semantic versioning information. The specified predicates are simply listed in order. The only semantics that are assumed is that code compiled with the `cfg` predicates works for all versions greater than or equal to that version.

**Note:** Here it would be important to link to documentation showing the `cfg` predicates and the different version strings that are supported.


# Implementation
[implementation]: #implementation

The various target API version `cfg` predicates allow users to conditionally compile code based on the API version supported by the target platform.  Each platform is responsible for defining a default key, a set of keys it supports, and functions that are able to compare the version strings they use.  A set of comparison functions can be provided by `rustc` for common formats such as 2- and 3-part semantic versioning.  When a platform detects a key it doesn’t support it will return `false` and emit a warning.

When a target is being built the actual API versions will be set via the following methods, in decreasing order of precedence:
* Command line arguments to `rustc` and/or `cargo`
* Cargo.toml target sections
* Target platform defaultsi

## Versioning Schema

Version strings can take on nearly any form and while there are some standard formats, such as semantic versioning or release dates, projects can change schemas or provide aliases for some or all of their releases.  Because of this diversity in version strings each platform will be responsible for defining a type implementing `FromStr`, `Display`, and `Ord` for each key they support (or using one of the pre-defined types).

## Storing Information in Metadata

The values for the various `cfg` requirements must be stored in a crate’s metadata if the value is set.  If no value is set the platform’s default API versions are used.  If no value is set it will default to the maximum of all the crate’s transitive dependencies’ requirements.

## Future Compatibility

The functions for parsing and comparing version strings will need to be updated whenever a new API is added, when the version format changes, or when new aliases need to be added.

# Drawbacks
[drawbacks]: #drawbacks

Each supported platform will need to implement version string parsing logic (or re-use some provided defaults), maintain the logic in response to future changes, and update any version alias tables.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The overall mechanism proposed here builds on other well established primitives in Rust such as `cfg`.

As previously stated, a mechanism which tries to bridge cross-platform differences under one `min_target_api_version` predicate [was suggested](https://github.com/rust-lang/rfcs/blob/b0f94000a3ddbd159013e100e48cd887ba2a0b54/text/0000-min-target-api-version.md) but was rejected due to different platforms having divergent needs.

# Prior art
[prior-art]: #prior-art

This RFC is largely an updated version of [this RFC draft](https://github.com/rust-lang/rfcs/pull/3036), with the changes reflecting conversations from the draft review process and [further Zulip discussion](https://rust-lang.zulipchat.com/#narrow/stream/213817-t-lang/topic/CFG.20OS.20Redux.20.28migrated.29/near/294738760).

# Unresolved questions
[unresolved-questions]: #unresolved-questions

Custom targets usually specify their configurations in JSON files.  It is unclear how the target maintainers would add functions, types, and version compatibility information to these files.

# Future possibilities
[future-possibilities]: #future-possibilities

## std-Aware Cargo

This proposal should work nicely with std aware cargo, allowing for different builds of the std library based on what the `target_os_version.*` values are set to. For now, the standard library shipped with each target through rustup will be set to some sensible default.
