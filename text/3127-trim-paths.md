- Feature Name: trim-paths
- Start Date: 2021-05-24
- RFC PR: [rust-lang/rfcs#3127](https://github.com/rust-lang/rfcs/pull/3127)
- Rust Issue: [rust-lang/rust#111540](https://github.com/rust-lang/rust/issues/111540)

# Summary
[summary]: #summary

Cargo should have a [profile setting](https://doc.rust-lang.org/cargo/reference/profiles.html#profile-settings) named `trim-paths`
to sanitise absolute paths introduced during compilation that may be embedded in the compiled binary executable or library.

`cargo build` with the default `release` profile should not produce any host filesystem dependent paths into binary executable or library. But
it will retain the paths inside separate debug symbols file, if one exists, to help debuggers and profilers locate the source files.

To facilitate this, a new flag named `--remap-path-scope` should be added to `rustc` controlling the behaviour of `--remap-path-prefix`, allowing us to fine
tune the scope of remapping, specifying paths under which context (in macro expansion, in debuginfo or in diagnostics)
should or shouldn't be remapped.

# Motivation
[motivation]: #motivation

## Sanitising local paths that are currently embedded
Currently, executables and libraries built by Rust and Cargo have a lot of embedded absolute paths. They most frequently appear in debug information and
panic messages (pointing to the panic location source file). As an example, consider the following package:

`Cargo.toml`:

```toml
[package]
name = "rfc"
version = "0.1.0"
edition = "2018"

[dependencies]
rand = "0.8.0"
```
`src/main.rs`

```rust
use rand::prelude::*;
    
fn main() {
    let r: f64 = rand::thread_rng().gen();
    println!("{}", r);
}
```

Then run

```bash
$ cargo build --release
$ strings target/release/rfc | grep $HOME
```

We will see some absolute paths pointing to dependency crates downloaded by Cargo, containing our username:

```
could not initialize thread_rng: /home/username/.cargo/registry/src/github.com-1ecc6299db9ec823/rand-0.8.3/src/rngs/thread.rs
/home/username/.cargo/registry/src/github.com-1ecc6299db9ec823/rand_chacha-0.3.0/src/guts.rsdescription() is deprecated; use Display
/home/username/.cargo/registry/src/github.com-1ecc6299db9ec823/getrandom-0.2.2/src/util_libc.rs
```

This is undesirable for the following reasons:

1. **Privacy**. `release` binaries may be distributed, and anyone could then see the builder's local OS account username.
   Additionally, some CI (such as [GitLab CI](https://docs.gitlab.com/runner/best_practice/#build-directory)) checks out the repo under a path where
   non-public information is included. Without sanitising the path by default, this may be inadvertently leaked.
2. **Build reproducibility**. We would like to make it easier to reproduce binary equivalent builds. While it is not required to maintain
   reproducibility across different environments, removing environment-sensitive information from the build will increase the tolerance on the
   inevitable environment differences. This helps with build verification, as well as producing deterministic builds when using a distributed build
   system.

## Handling sysroot paths
At the moment, paths to the source files of standard and core libraries, even when they are present, always begin with a virtual prefix in the form
of `/rustc/[SHA1 hash]/library`. This is not an issue when the source files are not present (i.e. when `rust-src` component is not installed), but
when a user installs `rust-src` they may want the path to their local copy of source files to be visible. Sometimes this is simply impossible as the path originated from the pre-compiled std and core and outside of rustc's control, but the local path should be used where possible.
Hence the default behaviour when `rust-src` is installed should be to use the local path. These local paths should be then affected by path remappings in the usual way.

## Preserving debuginfo to help debuggers
At the moment, `--remap-path-prefix` will cause paths to source files in debuginfo to be remapped. On platforms where the debuginfo resides in a
separate file from the distributable binary, this may be unnecessary and it prevents debuggers from being able to find the source. Hence `rustc`
should support finer grained control over paths in which contexts should be remapped.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## The rustc book: Command-line arguments

### `--remap-path-scope`: configure the scope of path remapping

When the `--remap-path-prefix` option is passed to rustc, source path prefixes in all output will be affected by default.
The `--remap-path-scope` argument can be used in conjunction with `--remap-path-prefix` to determine paths in which output context should be affected.
This flag accepts a comma-separated list of values and may be specified multiple times, in which case the scopes are aggregated together. The valid scopes are:

- `macro` - apply remappings to the expansion of `std::file!()` macro. This is where paths in embedded panic messages come from
- `diagnostics` - apply remappings to printed compiler diagnostics
- `unsplit-debuginfo` - apply remappings to debug information only when they are written to compiled executables or libraries, but not when they are in split debuginfo files
- `split-debuginfo` - apply remappings to debug information only when they are written to split debug information files, but not in compiled executables or libraries 
- `split-debuginfo-path` - apply remappings to the paths pointing to split debug information files. Does nothing when these files are not generated.
- `object` - an alias for `macro,unsplit-debuginfo,split-debuginfo-path`. This ensures all paths in compiled executables or libraries are remapped, but not elsewhere.
- `all` and `true` - an alias for all of the above, also equivalent to supplying only `--remap-path-prefix` without `--remap-path-scope`.

Debug information are written to split files when the separate codegen option `-C split-debuginfo=packed` or `unpacked` (whether by default or explicitly set).

Note: this RFC is not a commitment to stabilizing all of these options; stabilization will evaluate each option and see if that option carries enough value to stabilize.

## Cargo

`trim-paths` is a profile setting which enables and controls the sanitisation of file paths in build outputs. It is a simplified version of rustc's `--remap-path-scope`. It takes a comma separated list of the following values:

- `none` and `false` - disable path sanitisation
- `macro` - sanitise paths in the expansion of `std::file!()` macro. This is where paths in embedded panic messages come from
- `diagnostics` - sanitise paths in printed compiler diagnostics
- `object` - sanitise paths in compiled executables or libraries
- `all` and `true` - sanitise paths in all possible locations

Note: this RFC is not a commitment to stabilizing all of these options; stabilization will evaluate each option and see if that option carries enough value to stabilize.

It is defaulted to `none` for debug profiles, and `object` for release profiles. You can manually override it by specifying this option in `Cargo.toml`:
```toml
[profile.dev]
trim-paths = "all"

[profile.release]
trim-paths = "none"
```

The default release profile setting (`object`) sanitises only the paths in emitted executable or library files. It always affects paths from macros such as panic messages, and in debug information
  only if they will be embedded together with the binary (the default on platforms with ELF binaries, such as Linux and windows-gnu),
  but will not touch them if they are in separate files (the default on Windows MSVC and macOS). But the paths to these separate files are sanitised.

If `trim-paths` is not `none` or `false`, then the following paths are sanitised if they appear in a selected scope:

1. Path to the source files of the standard and core library (sysroot) will begin with `/rustc/[rustc commit hash]`.
   E.g. `/home/username/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs` -> 
   `/rustc/fe72845f7bb6a77b9e671e6a4f32fe714962cec4/library/core/src/result.rs`
2. Path to the current package will be stripped. E.g. `/home/username/crate/src/lib.rs` -> `src/lib.rs`.
3. Path to dependency packages will be replaced with `[package name]-[version]`. E.g. `/home/username/deps/foo/src/lib.rs` -> `foo-0.1.0/src/lib.rs`

When a path to the source files of the standard and core library is *not* in scope for sanitisation, the emitted path will depend on if `rust-src` component
is present. If it is, then some paths will point to the copy of the source files on your file system; if it isn't, then they will
show up as `/rustc/[rustc commit hash]/library/...` (just like when it is selected for sanitisation). Paths to all other source files will not be affected.

This will not affect any hard-coded paths in the source code, such as in strings.

### Environment variables Cargo sets for build scripts
* `CARGO_TRIM_PATHS` - The value of `trim-paths` profile option. If the build script introduces absolute paths to built artefacts (such as
by invoking a compiler), the user may request them to be sanitised in different types of artefacts. Common paths requiring sanitisation
include `OUT_DIR` and `CARGO_MANIFEST_DIR`, plus any other introduced by the build script, such as include directories.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## `trim-paths` implementation in Cargo

If `trim-paths` is `none` (`false`), no extra flag is supplied to `rustc`.

If `trim-paths` is anything else, then its value is supplied directly to `rustc`'s `--remap-path-scope` option, along with two `--remap-path-prefix` arguments:
- From the path of the local sysroot to `/rustc/[commit hash]`. 
- For the the current package (where the current working directory is in), from the the absolute path of the package root to empty string.
  For other packages, from the absolute path of the package root to `[package name]-[package version]`.

The default value of `trim-paths` is `object` for release profile. As a result, panic messages (which are always embedded) are sanitised. If debug information is embedded, then they are sanitised; if they are split then they are kept untouched, but the paths to these split files are sanitised.

Some interactions with compiler-intrinsic macros need to be considered:
1. Path (of the current file) introduced by [`file!()`](https://doc.rust-lang.org/std/macro.file.html) *will* be remapped. **Things may break** if
   the code interacts with its own source file at runtime by using this macro.
2. Path introduced by [`include!()`](https://doc.rust-lang.org/std/macro.include.html) *will* be remapped, given that the included file is under
   the current package or a dependency package.

If the user further supplies custom `--remap-path-prefix` arguments via `RUSTFLAGS`
or similar mechanisms, they will take precedence over the one supplied by `trim-paths`. This means that the user-defined remapping arguments must be
supplied *after* Cargo's own remapping.

## Changing handling of sysroot path in `rustc`

The remapping of sysroot paths to `/rustc/[commit hash]/library/...` was done when std and core libraries are compiled by Rust's release CI. Unless [`build-std`](https://doc.rust-lang.org/cargo/reference/unstable.html#build-std) is specified, these pre-compiled artifacts are used.

Most of the time, these paths are never handled by `rustc`, since they are in the debuginfo of pre-compiled binaries to be directly copied by the linker. However, sometimes (such as when compiling monomorphised functions), `rustc` does pick up these metadata. When this happens, `rustc` tries to correlate this virtual path to a real path pointing to the file on the local file system.
Currently the result is represented internally as if the path was remapped by a `--remap-path-prefix`, from local `rust-src` path to the virtual 
path `/rustc/[commit hash]/library/...`.
Only the virtual path is ever emitted for metadata or codegen. We want to change this behaviour such that, when `rust-src` source files can be
discovered, the virtual path is discarded and therefore the local path will be embedded, unless there is a `--remap-path-prefix` that causes this
local path to be remapped in the usual way.

## Split Debuginfo

When debug information are not embedded in the binary (i.e. `split-debuginfo` is not `off`), absolute paths to various files containing debug
information are embedded into the binary instead. Such as the absolute path to `.pdb` file (MSVC, `packed`), `.dwo` files (ELF, `unpacked`), 
and `.o` files (ELF, `packed`). This can be undesirable. As such, `split-debuginfo-path` is made specifically for these embedded paths.

On macOS and ELF platforms, these paths are introduced by `rustc` during codegen. With MSVC, however, the path to `.pdb` file is generated and
embedded into the binary by the linker `link.exe`. The linker has a `/PDBALTPATH` option allows us to change the embedded path written to the
binary, which could be supplied by `rustc`

# Usage examples

## Alice wants to ship her binaries, but doesn't want others to see her username

It works out of the box!

```console
Alice$ cargo build --release
```

## Bob wants to profile his program and see the original function names in the report

He needs the debug information emitted and preserved, so he changes his `Cargo.toml` file

```toml
[profile.release]
trim-paths = "none"
debuginfo = 1
```

```console
Bob$ cargo build --release && perf record cargo run --release
```

## Eve wants to symbolicate her users' crash reports from binaries without debug information

She needs to use the `split-debuginfo` feature to produce a separate file containing debug information

```toml
[profile.release]
split-debuginfo = "packed"
debuginfo = 1
```

Again, the default works fine.

```console
Eve$ cargo build --release
```

She can ship her binary like Alice, without worrying about leaking usernames.

## Hana needs to compile a C program in their build script

They can consult `CARGO_TRIM_PATHS` in their build script to find out paths in what places the user wants sanitised

```rust
// in build.rs
use std::env;
use std::process::Command;

let mut gcc = Command::new("gcc");
let out_dir = env::var("OUT_DIR").unwrap();
let scope = env::var("CARGO_TRIM_PATHS").unwrap();

if scope != "none" && scope != "false" {
   // Runtime working directory of the build script
   let cwd = env::var("CARGO_MANIFEST_DIR").unwrap();
   let gcc_scope = match scope.as_str() {
      "macro" => "-fmacro-prefix-map",
      _       => "-ffile-prefix-map",
   };
   gcc.args([&format!("{gcc_scope}={cwd}=redacted"), &format!("{gcc_scope}={out_dir}=redacted")]);
}

gcc.args(["-std=c11", &format!("-o={out_dir}/lib.o"), "lib.c"]);

let output = gcc.output();

//... do stuff
```

```console
Hana$ cargo build --release
```

# Drawbacks
[drawbacks]: #drawbacks

The user will not be able to `Ctrl+click` on any paths provided in panic messages or backtraces outside of the working directory. But
there shouldn't be any confusion as the combination of package name and version can be used to pinpoint the file.

As mentioned above, `trim-paths` may break code that relies on `std::file!()` to evaluate to an accessible path to the file. Hence enabling
it by default for release builds may be a technically breaking change. Occurrences of such use should be extremely rare but should be investigated
via a Crater run. In case this breakage is unacceptable, `trim-paths` can be made an opt-in option rather than default in any build profile.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

There has been an issue (https://github.com/rust-lang/rust/issues/40552) asking for path sanitisation to be implemented and enabled by default for 
release builds. It has, over the past 4 years, gained a decent amount of popular support. The remapping rule proposed here is very simple to 
implement.

Paths to sysroot crates are specially handled by `rustc`. Due to this, the behaviour we currently have is that all such paths are virtualised.
Although good for privacy and reproducibility, some people find it a hindrance for debugging: https://github.com/rust-lang/rust/issues/85463.
Hence the user should be given control on if they want the virtual or local path.

An alternative is to extend the syntax accepted by `--remap-path-prefix` or add a new option called `--remap-path-prefix-scoped` which allows
scoping rules to be explicitly applied to each remapping. This can co-exist with `--remap-path-scope` so it will be discussed further in
[Future possibilities](#future-possibilities) section.

## Rationale for the `--remap-path-scope` options
There are quite a few options available for `--remap-path-scope`. Not all of them are expected to have meaningful use-cases in their own right.
Some are only added for completeness, that is, the behaviour of `--remap-path-scope=all` (or the original `--remap-path-prefix` on its own) is
the same as specifying all individual scopes. In the future, we expect some of the scopes to be removed as independent options, while preserving
the behaviour of `--remap-path-scope=all` and the stable `--remap-path-prefix`, which is "Remap source names in all output".

- `macro` is primarily meant for panic messages embedded in binaries.
- `diagnostics` is unlikely to be used on its own as it only affects console outputs, but is required for completeness. See [#87745](https://github.com/rust-lang/rust/issues/87745).
- `unsplit-debuginfo` is used to sanitise debuginfo embedded in binaries.
- `split-debuginfo` is used to sanitise debuginfo separate from binaries. This may be used when debuginfo files are separate and the author
still wants to distribute them.
- `split-debuginfo-path` is used to sanitise the path embedded in binaries pointing to separate debuginfo files. This is likely needed in all
contexts where `unsplit-debuginfo` is used, but it's technically a separate piece of information inserted by the linker, not rustc.
- `object` is a shorthand for the most common use-case: sanitise everything in binaries, but nowhere else. 
- `all` and `true` preserves the documented behaviour of `--remap-path-prefix`.

# Prior art
[prior-art]: #prior-art

The name `trim-paths` came from the [similar feature](https://golang.org/cmd/go/#hdr-Compile_packages_and_dependencies) in Go. An alternative name
`sanitize-paths` was first considered but the spelling of "sanitise" differs across the pond and down under. It is also not as short and concise.

Go does not enable this by default. Since Go does not differ between debug and release builds, removing absolute paths for all build would be
a hassle for debugging. However this is not an issue for Rust as we have separate debug build profile.

GCC and Clang both have a flag equivalent to `--remap-path-prefix`, but they also both have two separate flags one for only macro expansion and
the other for only debuginfo: https://reproducible-builds.org/docs/build-path/. This is the origin of the `--remap-path-scope` idea.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Should we use a slightly more complex remapping rule, like distinguishing packages from registry, git and path, as proposed in
  [Issue #40552](https://github.com/rust-lang/rust/issues/40552)?
- With debug information in separate files, debuggers and Rust's own backtrace rely on the path embedded in the binary to find these files to display
  source code lines, columns and symbols etc. If we sanitise these paths to relative paths, then debuggers and backtrace must be invoked
  in specific directories for these paths to work. [For instance](https://github.com/rust-lang/rust/issues/87825#issuecomment-920693005), if the
  absolute path to the `.pdb` file is sanitised to the relative `target/release/foo.pdb`, then the binary must be invoked under the crate root as
  `target/release/foo` to allow the correct backtrace to be displayed.
- Should we treat the current package the same as other packages? We could have one fewer remapping rule by remapping all
  package roots to `[package name]-[version]`. A minor downside to this is not being able to `Ctrl+click` on paths to files the user is working
  on from panic messages.
- Will these cover all potentially embedded paths? Have we missed anything?

# Future possibilities
[future-possibilities]: #future-possibilities

## Per-mapping scope control
If it turns out that we want to enable finer grained scoping control on each individual remapping, we could use a `scopes:from=to` syntax.
E.g. `split-debuginfo,unsplit-debuginfo,diagnostics:/path/to/src=src` will remove all references to `/path/to/src` from compiler diagnostics and debug information, but
they are retained in panic messages.

How exactly this new syntax will look like is, of course, up to further discussion. Using comma as a separator for scopes may look ambiguous as `macro,diagnostics:/path/from=to` could be interpreted as `macro`
and `diagnostics:/path/from=to`.

This syntax can be used with either a brand new `--remap-path-prefix-scoped` option, or we could extend the
existing `--remap-path-prefix` option to take in this new syntax.

If we were to extend the existing `--remap-path-prefix`, there may be an ambiguity to whether `:` means a separator between scope list and mapping,
or is it a part of the path; if the first `:` supplied belongs to the path then it would have to be escaped. This could be technically breaking.

In any case, future inclusion of this new syntax will not affect `--remap-path-scope` introduced in this RFC. Scopes specified in `--remap-path-scope`
will be used as default for all mappings, and explicit scopes for an individual mapping will take precedence on that mapping.

## Sysroot paths uniformity
Since some virtualised sysroot paths are hardcoded in the pre-compiled debuginfo, while the others can be resolved back to a local path with `rust-src`, the user may see them interleaved
```
   0: rust_begin_unwind
             at /rustc/881c1ac408d93bb7adaa3a51dabab9266e82eee8/library/std/src/panicking.rs:493:5
   1: core::panicking::panic_fmt
             at /rustc/881c1ac408d93bb7adaa3a51dabab9266e82eee8/library/core/src/panicking.rs:92:14
   2: core::result::unwrap_failed
             at /rustc/881c1ac408d93bb7adaa3a51dabab9266e82eee8/library/core/src/result.rs:1355:5
   3: core::result::Result<T,E>::unwrap
             at /home/jonas/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:1037:23
```

This is not very nice. It is infeasible to fix up the pre-compiled debuginfo before linking to fully remove the virtual paths, so demapping needs to happen when it is displayed (in this case, when the backtrace is printed). This is out of scope of this RFC but it may be something we want to do separately in the future.
