- Feature Name: `cfg_os_version_min`
- Start Date: 2024-12-27
- RFC PR: [rust-lang/rfcs#3750](https://github.com/rust-lang/rfcs/pull/3750)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

A new `cfg` predicate `os_version_min` that allows users to declare the minimum primary (target-defined) API level required/supported by a block.
E.g. `cfg!(os_version_min("windows", "6.1.7600"))` would match Windows version >= 6.1.7600.

# Motivation
[motivation]: #motivation

The target API version is the version number of the "API set" that a particular binary relies on in order to run properly.  An API set is the set of APIs that a host operating system makes available for use by binaries running on that platform.  Newer versions of a platform may either add or remove APIs from the API set.

Crates including the standard library must account for various API version requirements for the crate to be able to run.  Rust currently has no mechanism for crates to compile different code (or to gracefully fail to compile) depending on the minimum targeted API version. This leads to the following issues:

* Relying on dynamic detection of API support has a runtime cost. The standard library often performs [dynamic API detection](https://github.com/rust-lang/rust/blob/f283d3f02cf3ed261a519afe05cde9e23d1d9278/library/std/src/sys/windows/compat.rs) falling back to older (and less ideal) APIs or forgoing entire features when a certain API is not available. For example, the [current `Mutex` impl](https://github.com/rust-lang/rust/blob/234099d1d12bef9d6e81a296222fbc272dc51d89/library/std/src/sys/windows/mutex.rs#L1-L20) has a Windows XP fallback. Users who only ever intend to run their code on newer versions of Windows will still pay a runtime cost for this dynamic API detection. Providing a mechanism for specifying which minimum API version the user cares about, allows for statically specifying which APIs a binary can use.
* Certain features cannot be dynamically detected and thus limit possible implementations. The libc crate must use [a raw syscalls on Android for `accept4`](https://github.com/rust-lang/libc/pull/1968), because this was only exposed in libc in version 21 of the Android API.  Additionally libstd must dynamically load `signal` for all versions of Android despite it being required only for versions 19 and below. In the future there might be similar changes where there is no way to implement a solution for older versions.
* Trying to compile code with an implicit dependency on a API version greater than what is supported by the target platform leads to linker errors. For example, the `x86_64-pc-windows-msvc` target's rustc implementation requires `SetThreadErrorMode` which was introduced in Windows 7. This means trying to build the compiler on older versions of Windows will fail with [a less than helpful linker error](https://github.com/rust-lang/rust/issues/35471).

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Rust targets are often thought of as monoliths.
The thought is that if you compile a binary for that target, that binary should be able to run on any system that fits that target's description.
However, this is not actually true.
For example, when compiling for `x86_64-pc-windows-msvc` and linking with the standard library, my binary has implicitly taken a dependency on a set of APIs that Windows exposes for certain functionality.
If I try to run my binary on older systems that do not have those APIs, then my binary will fail to run.
When compiling for a certain target, you are therefore declaring a dependency on a minimum target API version that you rely on for your binary to run.

Each standard library target uses a sensible minimum API version. for `x86_64-pc-windows-msvc` the minimum API version is "10.0.10240" which corresponds to Windows 10's initial release.
For `x86_64-win7-pc-windows-msvc` the minimum API version is "6.1.7600" which corresponds to Windows 7.
However, inferring the API version from the target name isn't ideal especially as it can change over time.

Instead you use the `os_version_min` predicates to specify the minimum API levels of various parts of the operating system.  For example:

* `os_version_min(“windows”, <string>)` would test the [minimum build version](https://gaijin.at/en/infos/windows-version-numbers) of Windows.
* `os_version_min(“libc”, <string>)` would test the version of libc.
* `os_version_min(“kernel”, <string>)` would test the version of the kernel.

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

For targets where `os_version_min(“windows”, …)` does not make sense (i.e., non-Windows targets), the `cfg` predicate will return `false` and emit a warning saying that the particular `cfg` predicate is not supported on that target. Therefore, it's important to pair `os_version_min(“windows”, …)` with a `cfg(windows)` using the existing mechanisms for combining `cfg` predicates.

The above example works exactly the same way with the other platform API `cfg` predicates just with different values and different target support.

These predicates do not assume any semantic versioning information. The specified predicates are simply listed in order. The only semantics that are assumed is that code compiled with the `cfg` predicates works for all versions greater than or equal to that version.

**Note:** Here it would be important to link to documentation showing the `cfg` predicates and the different version strings that are supported.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The `os_version_min` predicate allows users to conditionally compile code based on the API version supported by the target platform.
Each platform is responsible for defining a default key, a set of keys it supports, and functions that are able to compare the version strings they use.
A set of comparison functions can be provided by `rustc` for common formats such as 2- and 3-part semantic versioning.
When a platform detects a key it doesn’t support it will return `false` and emit a warning.

Each target platform will set the minimum API versions it supports.

## Versioning Schema

Version strings can take on nearly any form and while there are some standard formats, such as semantic versioning or release dates, projects can change schemas or provide aliases for some or all of their releases.
Because of this diversity in version strings each platform will be responsible for defining a type implementing `FromStr`, `Display`, and `Ord` for each key they support (or using one of the pre-defined types).

## Future Compatibility

The functions for parsing and comparing version strings will need to be updated whenever a new API is added, when the version format changes, or when new aliases need to be added.

# Drawbacks
[drawbacks]: #drawbacks

Each supported platform will need to implement version string parsing logic (or re-use some provided defaults), maintain the logic in response to future changes, and update any version alias tables.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The overall mechanism proposed here builds on other well established primitives in Rust such as `cfg`.
A mechanism which tries to bridge cross-platform differences under one `min_target_api_version` predicate [was suggested](https://github.com/rust-lang/rfcs/blob/b0f94000a3ddbd159013e100e48cd887ba2a0b54/text/0000-min-target-api-version.md) but was rejected due to different platforms having divergent needs.

For many platforms, the `target_os` name and the `os_version_min` name will be identical.
Even platforms that have multiple possible `versions` relevant to the OS will still have one primary version.
E.g. for `linux` the primary version would refer to the kernel with `libc` being a secondary OS library version.
Therefore it would be possible to simplify the syntax for the primary target OS version.
E.g.: `cfg(target_os("macos", min_version = "..."))` or by having `os_version_min("macos", "...")` imply `#[cfg(target_os = "macos")]`.
This means we'd need a more general syntax for `libc` and potentially other versioned libraries where the target OS is ambiguous.

# Prior art
[prior-art]: #prior-art

The Swift package manager has a way to [specify the supported platforms for a given package](https://docs.swift.org/package-manager/PackageDescription/PackageDescription.html#supportedplatform).

This RFC is largely a version of [RFC #3379](https://github.com/rust-lang/rfcs/pull/3379) more narrowly scoped to just the most minimal lang changes.
That RFC was in turn an updated version of [this RFC draft](https://github.com/rust-lang/rfcs/pull/3036), with the changes reflecting conversations from the draft review process and [further Zulip discussion](https://rust-lang.zulipchat.com/#narrow/stream/213817-t-lang/topic/CFG.20OS.20Redux.20.28migrated.29/near/294738760).

# Unresolved questions
[unresolved-questions]: #unresolved-questions

Custom targets usually specify their configurations in JSON files.
It is unclear how the target maintainers would add functions, types, and version compatibility information to these files.

What exactly should the syntax be?
Should we draw a distinction between cases where the `os_version_min` directly implies a specific `target_os` and cases where it doesn't (see alternatives)?

# Future possibilities
[future-possibilities]: #future-possibilities

The compiler could allow setting a higher minimum OS version than the target's default.
With the `build-std` feature, each target could optionally support lowering the API version below the default.
