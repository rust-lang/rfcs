- Feature Name: `min_*_version`
- Start Date: 2020-11-02
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary

[summary]: #summary

This RFC introduces new `cfg` predicates that allow users to declare the minimum versions of particular operating system APIs that a given piece of code supports.

The new predicates are:
* `min_windows_build_version`: the [minimum build version](https://gaijin.at/en/infos/windows-version-numbers) of Windows.
* `min_libc_version`: the version of libc in use.
* `min_kernel_version`: the version of the kernel in use.
* `min_macos_version`: The minium version of macOS in use.

For instance, the standard library's Windows Mutex implementation could potentially take advantage of this mechanism instead of relying on runtime API detection:

```rust 
// Note: this code already only compiles on #[cfg(windows)]
pub unsafe fn unlock(&self) {
    *self.held.get() = false;
    if cfg!(min_windows_build_version >= "6.1.7600") { // `cfg!(min_windows_build_version = "Windows7")` is also possible 
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

When running a binary on a machine on non-"baremetal" targets, external, operating-system provided APIs are often required for the binary to properly run. On Linux, this is a usually a particular version of libc and the kernel, on Android, it's the Android API version, etc. With newer versions of an operating systems comes newer APIs in this set of APIs a program may come to rely on (from here on simply known as the "API version").

Crates including the standard library must account for the *minimum* API version that is required in order for the crate to be able to run. Rust currently has no mechanism for crates to compile different code (or to gracefully fail to compile) depending on the minimum targeted API version. This leads to the following issues:

* Relying on dynamic detection of API support has a runtime cost. The standard library often performs [dynamic API detection](https://github.com/rust-lang/rust/blob/f283d3f02cf3ed261a519afe05cde9e23d1d9278/library/std/src/sys/windows/compat.rs) falling back to older (and less ideal) APIs or forgoing entire features when a certain API is not available. For example, the [current `Mutex` impl](https://github.com/rust-lang/rust/blob/234099d1d12bef9d6e81a296222fbc272dc51d89/library/std/src/sys/windows/mutex.rs#L1-L20) has a Windows XP fallback. Users who only ever intend to run their code on newer versions of Windows will still pay a runtime cost for this dynamic API detection. Providing a mechanism for specifying which minimum API version the user cares about, allows for statically specifying which APIs a binary can use. 
* Certain features cannot be dynamically detected and thus limit possible implementations. The libc crate must use [a raw syscalls on Android for `accept4`](https://github.com/rust-lang/libc/pull/1968), because this was only exposed in libc in version 21 of the Android API. In the future there might be similar changes where there is no way to implement a solution for older versions.
* Trying to compile code with an implicit dependency on a API version greater than what is supported by the target platform leads to linker errors. For example, the `x86_64-pc-windows-msvc` target's rustc implementation requires `SetThreadErrorMode` which was introduced in Windows 7. This means trying to build the compiler on older versions of Windows will fail with [a less than helpful linker error](https://github.com/rust-lang/rust/issues/35471).

While it might be tempting to distill this idea to a singular number across all platforms, each operating system handles how it versions its APIs in sufficiently different way to make it impossible for a singular identifier to be shared across all platforms.

In fact, [a previous revision of this RFC](https://github.com/rust-lang/rfcs/blob/b0f94000a3ddbd159013e100e48cd887ba2a0b54/text/0000-min-target-api-version.md) with such a mechanism was already proposed. It was deemed either unhelpful or incorrect to try to distill such concepts to one number. 

This is why we are proposing the first batch of such platform specific mechanims here. 

# Guide-level explanation

[guide-level-explanation]: #guide-level-explanation

Rust targets are often thought of as monoliths. The thought is that if you compile a binary for that target, that binary should be able to run on any system that fits that target's description. However, this is not actually true. For example, when compiling for `x86_64-pc-windows-msvc` and linking with the standard library, my binary has implicitly taken a dependency on a set of APIs that Windows exposes for certainly functionality. If I try to run my binary on older systems that do not have those APIs, then my binary will fail to run. Platforms usually expose a set of APIs in backward compatible with each release. When compiling for a certain target, you are therefore declaring a dependency on a minimum target API version that you rely on for your binary to run. 

By default, the standard library uses a sensible minimum API version. For example, for `x86_64-pc-windows-msvc` the minimum API version is "6.1.7600" which corresponds to Windows 7. However, there's good reason why you might want control of how your code is compiled depending on what the minimum API version is set as. For instance, if you want to:

* set your crate's minimum API version higher than that of the standard library. 
* change certain implementation details of your crate depending on what a downstream user sets their minimum API version to be.  
* have some sensible compiler error if users of your crate require a lower minimum API version that you require.

This is where certain `cfg` predicates come in that allow you to specify what the minimum target APIs are. These include the following:
* `min_windows_build_version`: the [minimum build version](https://gaijin.at/en/infos/windows-version-numbers) of Windows.
* `min_libc_version`: the version of libc in use.
* `min_kernel_version`: the version of the kernel in use.
* `min_macos_version`: The minium version of macOS in use.


Let's use `min_windows_build_number` as an example. It should be clear how this example would be extended to the other `cfg` predicates. The `min_windows_build_number` allows you to conditionally compile code based on the set minimum API version. For example an implementation of mutex locking on Windows might look like this:

```rust
pub unsafe fn unlock(&self) {
    *self.held.get() = false;
    if cfg!(min_windows_build_version >= "6.0.6000") { // API version greater than Vista
        c::ReleaseSRWLockExclusive(raw(self)) // Use the optimized ReleaseSRWLockExclusive routine
    } else {
        (*self.remutex()).unlock()  // Fall back to an alternative that works on older Windows versions
    }
}
```

End users can set the `min_windows_build_version` in the Cargo configuration file `.cargo/config` under the [`target key`](https://doc.rust-lang.org/cargo/reference/config.html#target) like so:

```toml
[target.x86_64-pc-windows-msvc]
min_windows_build_version = "6.0.6000" # Vista 
```

When using `rustc` directly, this option can be provided as a command line option: `rustc --cfg 'min_windows_build_version="6.0.6000"'`.

If an end user sets their `min_windows_build_version` to an incompatible version then the user receives an error. For instance, in the example above where the user is setting their `min_windows_build_version` to Windows Vista, they will receive an error when linking with the standard library which imposes Windows 7 as its `min_windows_build_version` by default for the `x86_64-pc-windows-msvc` target.

If a library does not explicitly set its `min_windows_build_version`, it will automatically be set to the largest `min_windows_build_version` of all of its transitive dependencies.

When user's code checks against a `min_windows_build_version`, a `>=` check is performed. This means that if the `min_windows_build_version` is "10.0.10240" and the user specifies `#[cfg(min_windows_build_version >= "6.2.9200"]` then the `cfg` predicate will be true and the code it marks will be compiled, since "10.0.10240" is a greater version than "6.2.9200".

For targets where `min_window_build_version` does not make sense (i.e., non-Windows targets), the `cfg` predicate will return false and emit a warning saying that the particular `cfg` predicate is not supported on that target. Therefore, it's important to pair `min_windows_build_version` with a `cfg(windows)` using the existing mechanisms for combining `cfg` predicates.

The above example works exactly the same way with the other platform API `cfg` predicates just with different values and different target support. 

These predicates do not assume any semantic versioning information. The specified predicates are simply listed in order. The only semantics that are assumed is that code compiled with the `cfg` predicates works for all versions greater than or equal to that version.

**Note:** Here it would be important to link to documentation showing the `cfg` predicates and the different version strings that are supported. 

# Reference-level explanation

[reference-level-explanation]: #reference-level-explanation

The various target API version `cfg` predicates allow users to conditionally compile code based on what the minimum platform API version they are willing to support is. In many ways, this predicate functions exactly like other `cfg` predicates with the exception that the predicate implicitly operates on an `>=` comparison and not a plain `=` equality check meaning that the list of target API versions needs to be ordered. 

The implementation does an `Ord` comparison of the provided version string defaulting to false and emitting a warning if the version cannot be parsed. The set version is set by the user through flags to rustc or equivalently through a key in the `target` section of the Cargo configuration file (see the explanation above). If none is provided, the version defaults to maximum of any of the crate's transitive dependencies. 

### Versioning schema

Each API version `cfg` predicate should have its own versioning schema. 

For Windows, it is best to use `<major>.<minor>.<build>` versioning that Microsoft uses to specify OS versions. These version numbers are monotonically increasing. In addition to this aliases for marketing names can be provided for convenience (e.g., "6.1.7600" == "Windows7"). 

An example where another schema may be used entirely is Android which typically only ships major versions (e.g., API Level 21). It is also possible for a vendor to switch versioning schemas entirely (e.g., from date releases to version numbers).

Implementors will likely want to implement `Ord`, `FromStr` and `Dispay` on some enum which represents a given platform's api version numbers. 

### Storing the information in metadata

The values for the various `cfg` predicates must be stored in a crate's metadata if that value is set. If none is set than the value is defaulted to whatever the maximum of all that crate's transitive dependencies is.

### Future compatibility

A given version of the Rust compiler will only know about certain versions of a given API version string. If a compiler does not recognize the given version string, it will issue a warning and the `cfg` predicate will return false. This means as new versions of platforms are introduced, new support must be added to the compiler. As code is written to take advantage of these new version strings, older compilers will not be able to conditionally compile based on them.  

# Drawbacks

[drawbacks]: #drawbacks

There are no known large drawbacks to this proposal. Some small drawbacks include:
* No all targets will strictly follow an ever increasing versioning scheme where more recent versions (i.e., versions with larger version numbers) are supersets of less recent versions. APIs may be deprecated and completely removed between versions. 
   * This proposal does not seek to address these situations. The mechanism does not provide any semantic versioning scheme on the versioning numbers. Maintainers must ensure that any feature they use can work for _all_ versions greater than or equal to the target API version specified.
   * See the [future possibilities](#future-posibilities) section for a possible `max_*` variant which can address these situations.
   * This does mean that code compiled with minimum target API versions is assumed to continue to work with all future versions of a target API. This is not considered a drawback per say since this is currently implicitly true for any target API use. 
* There is the potential for an ever growing number of built-in `cfg` predicates. As support is added for other platforms (e.g., iOS, Android, BSD, etc.), there will be a need for new `cfg` predicates that fit those platforms.
* Incremental complexity of the language

# Rationale and alternatives

[rationale-and-alternatives]: #rationale-and-alternatives

The overall mechanism proposed here builds on other well established primitives in Rust such as `cfg`. 

As previously stated, a mechanism which tries to bridge cross-platform differences under one `min_target_api_version` predicate [was suggested](https://github.com/rust-lang/rfcs/blob/b0f94000a3ddbd159013e100e48cd887ba2a0b54/text/0000-min-target-api-version.md) but was rejected due to different platforms having divergent needs.

To combat the issue of ever increasing number of platform API `cfg` predicates, a new syntax could be introduce like `#[cfg(min_version("windows_build", "6.0.6000")]`

This has the appeal of allowing more features to be added without needing special syntax, but many of the existing `cfg` related features (e.g., [cargo environment variables](https://doc.rust-lang.org/cargo/reference/environment-variables.html)) might need rethinking. 

Small changes could be considered:
* The use of `>=` in cfg annotations is new syntax. Using `=` might be simpler though possibly slightly more confusing. 

# Prior art

[prior-art]: #prior-art

Adding new `cfg` predicates has been proposed and accepted in [previous](https://github.com/rust-lang/rfcs/blob/3071138d4ed510d6dfc1f8e1d7e9d4b099ea12e8/text/2495-min-rust-version.md) [RFCs](https://github.com/rust-lang/rfcs/blob/2b0c911d55cf4095036b984ae30d8b632718241e/text/2523-cfg-path-version.md). This proposal is similar to these previous proposals in mechanics. The actual semantics of this RFC have been discussed several times though this is the first time it is being formally proposed in an RFC. This document hopes to capture the entire state of the discussion up until this point. For a detailed history on the discussion, take a look at the following resources:

* [Pre-RFC Zulip Discussion](https://rust-lang.zulipchat.com/#narrow/stream/242869-t-compiler.2Fwindows/topic/(Pre-RFC.3F).20target-api.2Fos-version)
* [Proof of concept implementation](https://github.com/rust9x/rust/commit/cccfa575a4bb449defdcbf362757f1e161f6cdf5)
* [Discussion as it relates to dropping XP Support](https://rust-lang.zulipchat.com/#narrow/stream/233931-t-compiler.2Fmajor-changes/topic/Drop.20official.20support.20for.20Windows.20XP.20compiler-team.23378)
* [More XP Support Discussion](https://rust-lang.zulipchat.com/#narrow/stream/122651-general/topic/On.20WinXP.20support)

Additionally, other languages like C and C++ feature conditional compilation based on similar predicates:
* For libc various pre-processor macros exist such as `__GLIBC__`
* For Windows with Visual C++, various macros [also exist](https://docs.microsoft.com/en-us/windows/win32/winprog/using-the-windows-headers?redirectedfrom=MSDN)
* For macOS `MAC_OS_X_VERSION_MIN_REQUIRED` can be used.

# Unresolved questions

[unresolved-questions]: #unresolved-questions

There are still some unresolved questions:

### Should there be new targets that only differ from existing targets in their default `min_*_version`?

In the [future possibilities](#future-possibilities) section we discuss the possibility of allowing users specialized versions of the std library depending on the specified `min_*_version` for some platform API version. In the meantime, should we also create new targets that different from the existing targets in that they have different default `min_*_version`s? This would allow users to more directly benefit from this feature. 

### How do custom targets specify ordered valid list of `min_*_version`s?

While it may be possible to have built in compiler support for major targets like `x86_64-pc-windows-msvc`, it is less clear how this should work for custom targets which usually specify such configuration in a target specification JSON file. We propose leaving this for a future RFC. 

# Future possibilities

[future-possibilities]: #future-possibilities

### std aware Cargo

This proposal should work nicely with std aware cargo, allowing for different builds of the std library based on what the `min_*_version` is set to. For now, the standard library shipped with each target through rustup will be set to some sensible default. 

### `max_*_version`

This provides a max bounds on the use of a target API. This would protect against use of later APIs where certain features had been modified or removed.

### More platforms

Currently this is only be suggested for Linux, Windows, and macOS, but of course the same mechanism makes sense on other platforms such as iOS and Android. 