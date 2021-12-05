- Feature Name: trim-paths
- Start Date: 2021-05-24
- RFC PR: [rust-lang/rfcs#3127](https://github.com/rust-lang/rfcs/pull/3127)
- Rust Issue: N/A

# Summary
[summary]: #summary

Cargo should have a [profile setting](https://doc.rust-lang.org/cargo/reference/profiles.html#profile-settings) named `trim-paths`
to sanitise absolute paths introduced during compilation that may be embedded in the compiled binary executable or library, and optionally in
the separate debug symbols file (depending on `split-debuginfo` settings).

`cargo build` with the default `release` profile should not produce any host filesystem dependent paths into binary executable or library. But
it will retain the paths in separate debug symbols file, if one exists, to help debuggers and profilers locate the source files.

To facilitate this, a new flag named `--remap-path-scope` should be added to `rustc` controlling the behaviour of `--remap-path-prefix`, allowing us to fine
tune the scope of remapping, specifying paths under which context (in macro expansion, in debuginfo or in diagnostics)
should or shouldn't be remapped.

# Motivation
[motivation]: #motivation

## Sanitising local paths that are currently embedded
Currently, executables and libraries built by Cargo have a lot of embedded absolute paths. They most frequently appear in debug information and
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
when a user installs `rust-src` they may want the path to their local copy of source files to be visible. Hence the default behaviour when `rust-src`
is installed should be to embed the local path. These local paths should be then affected by path remappings in the usual way.

## Preserving debuginfo to help debuggers
At the moment, `--remap-path-prefix` will cause paths to source files in debuginfo to be remapped. On platforms where the debuginfo resides in a
separate file from the distributable binary, this may be unnecessary and it prevents debuggers from being able to find the source. Hence `rustc`
should support finer grained control over paths in which contexts should be remapped.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## The rustc book: Command-line arguments

### `--remap-path-scope`: configure the scope of path remapping

When the `--remap-path-prefix` option is passed to rustc then source path prefixes in all output will be affected.
The `--remap-path-scope` argument can be used in conjunction with `--remap-path-prefix` to determine paths in which output context should be affected.
This flag accepts a comma-separated list of values and may be specified multiple times. The valid scopes are:

- `macro` - apply remappings to the expansion of `std::file!()` macro. This is where paths in embedded panic messages come from
- `debuginfo` - apply remappings to debug information
- `diagnostics` - apply remappings to printed compiler diagnostics

## Cargo

`trim-paths` is a profile setting which controls the sanitisation of file paths in compilation outputs. It has three valid options:
- `0` or `false`: no sanitisation at all
- `1`: sanitise only the paths in emitted executable or library binaries. It always affects paths from macros such as panic messages, and in debug information
  only if they will be embedded together with the binary (the default on platforms with ELF binaries, such as Linux and windows-gnu),
  but will not touch them if they are in a separate symbols file (the default on Windows MSVC and macOS)
- `2` or `true`: sanitise paths in all compilation outputs, including compiled executable/library, separate symbols file (if one exists), and compiler diagnostics.

The default release profile uses option `1`. You can also manually override it by specifying this option in `Cargo.toml`:
```toml
[profile.dev]
trim-paths = 2

[profile.release]
trim-paths = 0
```

When a path is in scope for sanitisation, it is replaced with the following rules:

1. Path to the source files of the standard and core library (sysroot) will begin with `/rustc/[rustc commit hash]`.
   E.g. `/home/username/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs` -> 
   `/rustc/fe72845f7bb6a77b9e671e6a4f32fe714962cec4/library/core/src/result.rs`
2. Path to the working directory will be stripped. E.g. `/home/username/crate/src/lib.rs` -> `src/lib.rs`.
3. Path to packages outside of the working directory will be replaced with `[package name]-[version]`. E.g. `/home/username/deps/foo/src/lib.rs` -> `foo-0.1.0/src/lib.rs`

When a path to the source files of the standard and core library is *not* in scope for sanitisation, the emitted path will depend on if `rust-src` component
is present. If it is, then the real path pointing to a copy of the source files on your file system will be emitted; if it isn't, then they will
show up as `/rustc/[rustc commit hash]/library/...` (just like when it is selected for sanitisation). Paths to all other source files will not be affected.

This will not affect any hard-coded paths in the source code.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## `trim-paths` implementation in Cargo

We only need to change the behaviour for `Test` and `Build` compile modes. 

If `trim-paths` is `0` (`false`), no extra flag is supplied to `rustc`.

If `trim-paths` is `1` or `2` (`true`), then two `--remap-path-prefix` arguments are supplied to `rustc`:
- From the path of the local sysroot to `/rustc/[commit hash]`. 
- If the compilation unit is under the working directory, from the the working directory absolute path to empty string.
  If it's outside the working directory, from the absolute path of the package root to `[package name]-[package version]`.

A further `--remap-path-scope` is also supplied for options `1` and `2`:

If `trim-path` is `1`, then it depends on the setting of `split-debuginfo` (whether the setting is explicitly supplied or from the default)
- If `split-debuginfo` is `off`, then `--remap-path-scope=macro,debuginfo`.
- If `split-debuginfo` is `packed` or `unpacked`, then `--remap-path-scope=macro`
This is because we always want to remap panic messages as they will always be embedded in executable/library, but we don't need to touch the separate
symbols file

If `trim-path` is `2` (`true`), all paths will be affected, equivalent to `--remap-path-scope=macro,debuginfo,diagnostics`


Some interactions with compiler-intrinsic macros need to be considered:
1. Path (of the current file) introduced by [`file!()`](https://doc.rust-lang.org/std/macro.file.html) *will* be remapped. **Things may break** if
   the code interacts with its own source file at runtime by using this macro.
2. Path introduced by [`include!()`](https://doc.rust-lang.org/std/macro.include.html) *will* be remapped, given that the included file is under
   the current working directory or a dependency package.

If the user further supplies custom `--remap-path-prefix` arguments via `RUSTFLAGS`
or similar mechanisms, they will take precedence over the one supplied by `trim-paths`. This means that the user-defined remapping arguments must be
supplied *after* Cargo's own remapping.

## Changing handling of sysroot path in `rustc`

The virtualisation of sysroot files to `/rustc/[commit hash]/library/...` was done at compiler bootstraping, specifically when 
`remap-debuginfo = true` in `config.toml`. This is done for Rust distribution on all channels.

At `rustc` runtime (i.e. compiling some code), we try to correlate this virtual path to a real path pointing to the file on the local file system.
Currently the result is represented internally as if the path was remapped by a `--remap-path-prefix`, from local `rust-src` path to the virtual 
path.
Only the virtual name is ever emitted for metadata or codegen. We want to change this behaviour such that, when `rust-src` source files can be
discovered, the virtual path is discarded and therefore the local path will be embedded, unless there is a `--remap-path-prefix` that causes this
local path to be remapped in the usual way.

## Linker arguments

If a separate debuginfo file is to be generated (which can be determined by `split-debuginfo` codegen option), the linker may include an absolute
path to the object into the binary. If the user wants debug information to be remapped, then the inclusion of this absolute path is
undesirable. `rustc` cannot exhaustively control the behaviour of an external program (the linker) specified by the user, but we should
supply appropriate linker options to mitigate this as much as we could.

The linker in use can be determined by the [`linker-flavor`](https://doc.rust-lang.org/rustc/codegen-options/index.html#linker-flavor) flag, itself
generally being inferred by `rustc`. If `debuginfo` is in `--remap-path-scope` and `split-debuginfo` is not `off`, the following linker-specific
options should be emitted:

- When using MSVC linker, `/PDBALTPATH:%_PDB%` should be emitted. This makes the linker embed only the file name of the .pdb file without the path
  to it. 


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

Path to sysroot crates are specially handled by `rustc`. Due to this, the behaviour we currently have is that all such paths are virtualised.
Although good for privacy and reproducibility, some people find it a hinderance for debugging: https://github.com/rust-lang/rust/issues/85463.
Hence the user should be given control on if they want the virtual or local path.

An alternative is to extend the syntax accepted by `--remap-path-prefix` or add a new option called `--remap-path-prefix-scoped` which allows
scoping rules to be explicitly applied to each remapping. This can co-exist with `--remap-path-scope` so it will be discussed further in
[Future possibilities](#future-possibilities) section.

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
  in specific directories for these paths to work. [For instance](https://github.com/rust-lang/rust/issues/87825#issuecomment-920693005), `cargo run`
  invoked under crate root will fail to print meaningful backtrace symbols because the binary and `.pdb` file are under `target/release`, but
  the backtrace library will attempt to find the `.pdb` file from the working directory (crate root), where it doesn't exist. 
- At the time of writing, `rustc` recognises 10 [`linker-flavor`s](https://doc.rust-lang.org/rustc/codegen-options/index.html#linker-flavor).
  We need to find the right option for each to change the embedded path to debug information.
- Should we treat the current working directory the same as other packages? We could have one fewer remapping rule by remapping all
  package roots to `[package name]-[version]`. A minor downside to this is not being able to `Ctrl+click` on paths to files the user is working
  on from panic messages.
- Will these cover all potentially embedded paths? Have we missed anything?
- Should we make this affect more `CompileMode`s, such as `Check`, where the emitted `rmeta` file will also contain absolute paths?

# Future possibilities
[future-possibilities]: #future-possibilities

If it turns out that we want to enable finer grained scoping control on each individual remapping, we could use a `scopes:from=to` syntax.
E.g. `debuginfo,diagnostics:/path/to/src=src` will remove all references to `/path/to/src` from compiler diagnostics and debug information, but
they are retained panic messages. This syntax can be used with either a brand new `--remap-path-prefix-scoped` option, or we could extend the
existing `--remap-path-prefix` option to take in this new syntax.

If we were to extend the existing `--remap-path-prefix`, there may be an ambiguity to whether `:` means a separator between scope list and mapping,
or is it a part of the path; if the first `:` supplied belongs to the path then it would have to be escaped. This coudl be technically breaking.

In any case, future inclusion of this new syntax will not affect `--remap-path-scope` introduced in this RFC. Scopes specified in `--remap-path-scope`
will be used as default for all mappings, and explicit scopes for an individual mapping will take precedence on that mapping.
