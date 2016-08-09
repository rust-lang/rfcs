- Feature Name: target_bundles
- Start Date: 2016-08
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Combine distribution of standard libraries and targets into bundles for targeting a particular
platform. Such bundle is all you need from the Rust side to cross-compile for your target.

The major ideas of this RFC is to:

1. Make JSON targets as full featured as they need to be in order to support specifying targets as
   custom as they need to be;
2. Convert current built-in targets to the JSON targets;
3. Change distribution of libstd to include a corresponding JSON target.

# Motivation
[motivation]: #motivation

Currently there’s two different ways rustc targets are distributed: built-in targets and custom
JSON targets. Built-in targets are very inflexible – they cannot be changed without changing and
recompiling the compiler itself. On the other side of the spectrum, custom JSON targets are easy to
adjust and adapt, but feature wise are very limited and rarely are suitable for the more uncommon
use-cases.

We’ve observed a considerable amount of desire by users of the language to customize targets they
use in the ways currently not supported by our current infrastructure (sans making changes to the
compiler itself, of course), and noted that the current scheme is not very feasible in the long
run. This RFC should go a great amount towards fixing the issues.

Then, there also is a strong need to be able to inspect arbitrary parts of target specification,
regardless of their origin. For example, in a cross-compilation setting, when the crate uses a
build.rs script, `#[cfg]` variables are for the host, rather than the target. This way the author
is forced to parse the target triple and figure out particularities of the target on their own, as
rustc does not provide any way to inspect any of the built-in targets.

# Detailed design
[design]: #detailed-design

## What constitutes a fully working target?

In order to have any meaningful discussion about targets we need to decide on what constitutes a
full, complete target.

Currently to compile for a specific target a number of pieces are necessary: compiler knows
information about the target in some way, there is a set of rust standard libraries compiled for
the target, system has native library dependencies for the target and there exists a linker which
is capable of linking code for the target.

In this RFC we will *not* propose how to call custom linkers or “extend” capabilities of the LLVM
used.

## Changes to target format

### Comments and reuse

There has been attempt already to migrate built-in targets to JSON targets, but it didn’t go all
the way because of [loss of comments][comments]. One proposal was to migrate towards TOML, however
another, [more elegant solution][jsmin] which allows us to keep using JSON was proposed by Douglas
Crockford himself:

> Suppose you are using JSON to keep configuration files, which you would like to annotate. Go
> ahead and insert all the comments you like. Then pipe it through JSMin before handing it to your
> JSON parser.

This RFC proposes to use similar scheme: adjust the build system to remove comments from all the
JSON target specifications before packaging them for use by rustc. This way we get to keep using
JSON and can keep on having comments in the checked-in target specifications.

Similar preprocessing step could be used to implement some form of target inheritance so the
duplication between built-in targets could be reduced greatly.

[comments]: https://github.com/rust-lang/rust/pull/34980#issuecomment-234683183
[jsmin]: https://plus.google.com/+DouglasCrockfordEsq/posts/RK8qyGVaGSr

### Target quirks and cfg

Currently we have a few keys dealing with the configuration variables only: `target_os`,
`target_env`, `target_vendor`, `target_family`, `target_endian`, `target_pointer_width`¹, etc;
Then there’s options from which some configuration variables are derived, but they are also used to
tweak compilation: `is_like_windows`, `is_like_osx`, `is_like_solaris`, etc.

This RFC proposes replacing these keys with a different set of keys which explicitly control
configuration variables and keys which control compilation details:

* `cfg: {"target_env": "msvc", windows: null }` would result in two cofiguration variables
`#[cfg(windows)]` and `#[cfg(target_env = "msvc")]` evaluating to true, but no variables provided
by the `cfg` key would get used to influence the behaviour of the compiler; and
* `debuginfo: ["CodeView", 1]` some targets require different debuginfo format than what LLVM
generates by default. MSVC targets want CodeView version 1, OS X and Android want Dwarf version 2,
while LLVM appears to use the highest supported version of Dwarf by default.
* similarly for many other variables which tweak the way compilation is done.

¹: Technically, `target_pointer_width` is used in trans, but it does not provide any extra
information over `data_layout`, which is what it should be using.

This would allow selectively reusing various conditional implementations that are present in the
compiler for a custom target, when none of the `is_like_*` variables would fully suit the target.
Moreover, being able to specify arbitrary cfg variables would allow easily adapting for various
miniscule details related to the targets. For example, the targets for ARM CPUs with NEON support
could export `target_has_neon` without any extra language support.

That being said, it might make sense to have a whitelist of stable options and cfg variables and
keep everything else unstable for some duration.

### Proposed JSON key-values

```js
{
    // REQUIRED
    "llvm-target": "x86_64-unknown-linux-gnu",              // LLVM target triple (does not need to match with rustc triple)
    "data-layout": "e-m:e-i64:64-f80:128-n8:16:32:64-S128", // DataLayout for the target
    "arch": "x86_64",                                       // Architecture of the target

    // OPTIONAL
    // Configuration variables injected into the compilation units.
    "cfg": {
        "target_os": "linux",
        "target_family": "unix",
        "target_arch": "x86_64",
        "target_endian": "little",
        "target_pointer_width": "64",
        "target_env": "gnu",
        "target_vendor": "unknown",
        "target_has_atomic": ["8", "16", "32", "64"],       // any of #[cfg(target_has_atomic={"8","16","32","64"}] work.
        "target_has_atomic_ptr": null,
        "target_thread_local": null,
        "unix": null
    },

    // Type of, and the linker used
    "linker-kind": "gnu",                                   // previously linker_is_gnu: bool
    "linker": "cc",
    "ar": "ar",
    "archive-format": "gnu",

    // Various stuff adjusting how compilation is done
    "function-sections": true,
    "dynamic-linking": true,
    "disable-redzone": false,
    "obj-is-bitcode": false,
    "allow-asm": true,
    "allows-weak-linkage": true,
    "no-default-libraries": true,
    "custom-unwind-resume": false,                          // might make sense to merge into below
    "eh-method": "dwarf",                                   // NEW: what EH method to use
    "dll-storage-attrs": false,                             // NEW: should use dll storage attrs?
    "debug-info": ["Dwarf", 4],                             // NEW: what debug info format
    "system-abi": "C",                                      // NEW: what "system" ABI means
    "c-abi-kind": "cabi_x86_64",                            // NEW: what C-ABI implementation to use

    // Various stuff adjusting how linkage is done
    "pre-link-args": ["-Wl,--as-needed", "-Wl,-z,noexecstack"],
    "post-link-args": [],
    "pre-link-objects-dll": [],
    "pre-link-objects-exe": [],
    "post-link-objects": [],
    "late-link-args": [],
    "gc-sections-args": [],                                 // NEW: how to strip sections
    "rpath-prefix": "$ORIGIN",                              // CHANGED: to allow specifying rpath prefix, null to disable rpath altogether
    "no-compiler-rt": false,
    "metadata-section": ".note.rustc",                      // NEW: name of section for metadata
    "has-frameworks": false,                                // NEW: is concept of frameworks supported?
    "position-independent-executables": true,               // should become a plain linker argument?
    "lib_allocation_crate": "alloc_system",
    "exe_allocation_crate": "alloc_jemalloc",
    // should become templates? `lib{}.so` is much nicer
    "dll_prefix": "lib",
    "dll_suffix": ".so",
    "exe_suffix": "",
    "staticlib_prefix": "lib",
    "staticlib_suffix": ".a",

    // LLVM options
    "cpu": "x86-64",                                        // CPU of the target
    "features": "",                                         // LLVM features
    "relocation_model": "pic",
    "code_model": "default",
    "eliminate-frame-pointer": true,

    // Apparently OS X is so much of a special case that even moving all the special cases into
    // configuration is painful.
    is_like_osx: false,

    // is_like_solaris is handled by the extra gc-sections key
    // is_like_msvc is handled by the extra metadata-section, linker-kind, eh-method,
    // dll-storage-attrs, debug-info keys
    // is_like_windows is handled by the extra system-abi and c-abi-kind keys
    // is_like_android is handled by the extra debug-info key
}
```

## Distribution of targets

Currently every built-in target is distributed along with the rustc compiler. This is suboptimal,
because in majority of cases users are interested in targets which rustc can compile for, rather
than the built-in targets rustc knows about, therefore distributing targets built-in into rustc is
providing no benefits.

Native libraries and linkers aside, it is obvious there’s little sense in distributing targets
separately from the standard libraries for them. This RFC proposes to change the distribution so
the rust-std and rust (not rustc or rust-docs) packages begin including the JSON target description
for the target which the standard library targets. The target JSONs would get installed in
`$sysroot/share/rustc/targets/` or a similar directory, and the `RUST_TARGET_PATH` environment
variable would be adjusted to include this directory by default.

This scheme allows users to easily produce and distribute custom standard library and target
combinations, thus removing the need to land every single target as a built-in to rustc. Moreover,
under this scheme, if rustc reports knowing about a target, it is very likely it will be able to
compile for it as well, instead of reporting a confusing

> error: can't find crate for `std` [E0463]

# Drawbacks
[drawbacks]: #drawbacks

This RFC does not attempt to solve issues surrounding linker invocation and native library
discovery, especially during cross-compilation. Instead the code built-in into rustc is still relied
onto to deal with these problems. On the other hand, it is likely that there isn’t much more
necessary than rustc knowing how to invoke the few linkers it already knows about in order to cover
the great majority of use cases.

The users will be able to build the standard library-target bundles, but only for nightly versions
of rustc compilers, because of the number of unstable features necessary to build a libstd. On the
upside, it should become as easy as `cd src/libstd && cargo build --release
--target=/path/to/custom-target.json && build_bundle`. If there’s a desire to make “bundles” work
with stable rustc, the target would still be submitted upstream.

`#![no_core]` users will have to download the rust-std even if they have no use for libraries in
there.

Proposed change to add a key for each option, instead of having an umbrella `is_like_*` keys, will
result in big increase of such options. All of these are optional and should have sane default
values, though.

# Alternatives
[alternatives]: #alternatives

This RFC is still very viable without the proposed changes to JSON keys (the “target quirks and
cfg” as well as the “proposed JSON key-values” sections).

# Unresolved questions
[unresolved]: #unresolved-questions
