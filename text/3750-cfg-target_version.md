- Feature Name: `cfg_target_version`
- Start Date: 2024-12-27
- RFC PR: [rust-lang/rfcs#3750](https://github.com/rust-lang/rfcs/pull/3750)
- Rust Issue: [rust-lang/rust#136866](https://github.com/rust-lang/rust/issues/136866)

# Summary
[summary]: #summary

A new `cfg` predicate `target_version` that allows users to declare the minimum primary (target-defined) API level required/supported by a block.
E.g. `cfg!(target_version("windows", "6.1.7600"))` would match Windows version >= 6.1.7600.

# Motivation
[motivation]: #motivation

Operating systems and their libraries are continually advancing, adding and sometimes removing APIs or otherwise changing behaviour.
Versioning of APIs is common so that developers can target the set of APIs they support.
Crates, including the standard library, must account for various API version requirements for the crate to be able to run.
Rust currently has no mechanism for crates to compile different code (or to gracefully fail to compile) depending on the minimum targeted API version.
This leads to the following issues:

* Relying on dynamic detection of API support has a runtime cost.
The standard library often performs [dynamic API detection](https://github.com/rust-lang/rust/blob/f283d3f02cf3ed261a519afe05cde9e23d1d9278/library/std/src/sys/windows/compat.rs) falling back to older (and less ideal) APIs or forgoing entire features when a certain API is not available.
For example, the [current `Mutex` impl](https://github.com/rust-lang/rust/blob/234099d1d12bef9d6e81a296222fbc272dc51d89/library/std/src/sys/windows/mutex.rs#L1-L20) has a Windows 7 fallback. Users who only ever intend to run their code on newer versions of Windows will still pay a runtime cost for this dynamic API detection.
Providing a mechanism for specifying which minimum API version the user cares about, allows for statically specifying which APIs a binary can use.
* Certain features cannot be dynamically detected and thus limit possible implementations.
The libc crate must use [a raw syscall on Android for `accept4`](https://github.com/rust-lang/libc/pull/1968), because this was only exposed in libc in version 21 of the Android API.
Additionally libstd must dynamically load `signal` for all versions of Android despite it being required only for versions 19 and below.
In the future there might be similar changes where there is no way to implement a solution for older versions.
* Trying to compile code with an implicit dependency on a API version greater than what is supported by the target platform leads to linker errors.
For example, the `x86_64-pc-windows-msvc` target's rustc implementation requires `SetThreadErrorMode` which was introduced in Windows 7.
This means trying to build the compiler on older versions of Windows will fail with [a less than helpful linker error](https://github.com/rust-lang/rust/issues/35471).
* Bumping the minimum supported version of a platform in Rust is a large endeavour.
By adding this feature, we enable [rustc to more gradually raise the supported version](https://github.com/rust-lang/rust/pull/104385#issuecomment-1453520239) or to have more "levels" of version support.
This would allow for having the "default" supported target be higher than the "minimum" supported target.

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

Instead you use the `target_version` predicates to specify the minimum API levels of various parts of the operating system.  For example:

* `target_version("windows", <string>)` would test the [minimum build version](https://gaijin.at/en/infos/windows-version-numbers) of Windows.
* `target_version("libc", <string>)` would test the version of libc.
* `target_version("kernel", <string>)` would test the version of the kernel.

Let’s use `target_version("windows", …)` for a simple example.

```rust
pub fn random_u64() -> u64 {
    let mut rand = 0_u64.to_ne_bytes();
    if cfg!(target_version("windows", "10.0.10240")) {
        // For an API version greater or equal to Windows 10, we use `ProcessPrng`
        unsafe { ProcessPrng(rand.as_mut_ptr(), rand.len()) };
    } else {
        // Otherwise we fallback to `RtlGenRandom`
        unsafe { RtlGenRandom(rand.as_mut_ptr().cast(), rand.len() as u32) };
    }
    u64::from_ne_bytes(rand)
}
```

A more involved example would be to attempt to dynamically load the symbol.
On macOS we use weak linking to do this:

```rust
// Always available under these conditions.
#[cfg(any(
    target_version("macos", "11.0"),
    target_version("ios", "14.0"),
    target_version("tvos", "14.0"),
    target_version("watchos", "7.0"),
    target_version("visionos", "1.0")
))]
let preadv = {
    extern "C" {
        fn preadv(libc::c_int, *const libc::iovec, libc::c_int, off64_t) -> isize;
    }
    Some(preadv)
};

// Otherwise `preadv` needs to be weakly linked.
// We do that using a `weak!` macro, defined elsewhere.
#[cfg(not(any(
    target_version("macos", "11.0"),
    target_version("ios", "14.0"),
    target_version("tvos", "14.0"),
    target_version("watchos", "7.0"),
    target_version("visionos", "1.0")
)))]
weak!(fn preadv(libc::c_int, *const libc::iovec, libc::c_int, off64_t) -> isize);

if let Some(preadv) = preadv {
    preadv(...) // Use preadv, it's available
} else {
    // ... fallback impl
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The `target_version` predicate allows users to conditionally compile code based on the API version supported by the target platform using `cfg`.
It requires a key and a version string.
The key can be either a `target_os` string or else one of a set of target-defined strings.
Version strings are always target defined (see [Versioning Schema][versioning-schema]) and will be compared against the target's supported version.
For example, `#[cfg(target_version("macos", "11.0"))]` has the key `macos` and the minimum version `11.0`, which will match any macOS version greater than or equal to macOS 11 Big Sur.
If a target doesn't support a key, then the `cfg` will always return `false`.

Each target platform will set the minimum API versions it supports for each key.

## The standard library
[the-standard-library]: #the-standard-library

Currently the standard library is pre-compiled meaning that only a single version of each API can be supported, which must be the minimum version.
Third party crates can choose to use a higher API level so long as it's compatible with the baseline API version.
However, there is currently no support for setting a `target_version` version above the target's baseline (see [Future Possibilities][future-possibilities]).

## Versioning Schema
[versioning-schema]: #versioning-schema

Version strings can take on nearly any form and while there are some standard formats, such as semantic versioning or release dates, projects/platforms can change schemas or provide aliases for some or all of their releases.
Because of this diversity in version strings, each target will be responsible for validating the version, and defining comparisons on it.

## Linting
[linting]: #linting

By default `target_version` will be linted by `check_cfg` in a similar way to `target_os`.
That is, all valid values for `target_os` will be accepted as valid keys for `target_version` on all platforms.
The list of additional keys supported by the target will be consulted, which will then be allowed on a per-target basis.

## Future Compatibility
[future-compatibility]: #future-compatibility

The functions for parsing and comparing version strings may need to be updated whenever a new API is added, when the version format changes, or when new aliases need to be added.

# Drawbacks
[drawbacks]: #drawbacks

Each supported platform will need to implement version string parsing logic (or re-use some provided defaults), maintain the logic in response to future changes, and update any version alias tables.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The overall mechanism proposed here builds on other well established primitives in Rust such as `cfg`.
A mechanism which tries to bridge cross-platform differences under one `min_target_api_version` predicate [was suggested](https://github.com/rust-lang/rfcs/blob/b0f94000a3ddbd159013e100e48cd887ba2a0b54/text/0000-min-target-api-version.md) but was rejected due to different platforms having divergent needs.

For many platforms, the `target_os` name and the `target_version` name will be identical.
Even platforms that have multiple possible `versions` relevant to the OS will still have one primary version.
E.g. for `linux` the primary version would refer to the kernel with `libc` being a secondary OS library version.
Therefore it would make sense for the primary target OS version to be a property of `target_os`.
E.g.: `cfg(target_os("macos", min_version = "..."))`.
This means we'd need a more general syntax for `libc` and potentially other versioned libraries where the target OS is ambiguous.

# Prior art
[prior-art]: #prior-art

In C it's common to be able to target different versions based on a preprocessor macro.
For example, on Windows `WINVER` can be used:

```c
// If the minimum version is at least Windows 10
#if (WINVER >= _WIN32_WINNT_WIN10)
// ...
#endif
```

This RFC is a continuation of [RFC #3379](https://github.com/rust-lang/rfcs/pull/3379) more narrowly scoped to just `os_version_min` (renamed to `target_version` in this RFC).
That RFC was in turn an updated version of [this RFC draft](https://github.com/rust-lang/rfcs/pull/3036), with the changes reflecting conversations from the draft review process and [further Zulip discussion](https://rust-lang.zulipchat.com/#narrow/stream/213817-t-lang/topic/CFG.20OS.20Redux.20.28migrated.29/near/294738760).

# Unresolved questions
[unresolved-questions]: #unresolved-questions

Custom targets usually specify their configurations in JSON files.
It is unclear how the target maintainers would add version comparison information to these files.

Bikeshedding the name. `platform_version` and `os_version` are among other suggestions.

Should we draw a distinction between cases where the `target_version` directly implies a specific `target_os` and cases where it doesn't (see alternatives)?

# Future possibilities
[future-possibilities]: #future-possibilities

* The compiler could allow setting a higher minimum OS version than the target's default.
* With the `build-std` feature, each target could optionally support lowering the API version below the default.
