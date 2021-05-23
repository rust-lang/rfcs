- Feature Name: trim-path
- Start Date: 2021-05-24
- RFC PR: [rust-lang/rfcs#3127](https://github.com/rust-lang/rfcs/pull/3127)
- Rust Issue: N/A

# Summary
[summary]: #summary

Cargo should have a [profile setting](https://doc.rust-lang.org/cargo/reference/profiles.html#profile-settings) named `trim-path`
to sanitise absolute paths introduced during compilation that may be embedded in the compilation output. This should be enabled by default for 
`release` profile.

# Motivation
[motivation]: #motivation

## Sanitising local paths that are currently embedded
Currently, executables and libraies built by Cargo have a lot of embedded absolute paths. They most frequently appear in debug information and
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
   it may include things that really aren't meant to be public. Without sanitising the path by default, this may be inadvertently leaked.
2. **Build reproducibility**. We would like to make it easier to reproduce binary equivalent builds. While it is not required to maintain
   reproducibility across different environments, removing environment-sensitive information from the build will increase tolerance on the inevitable
   environment differences when trying to verify builds.

## Handling sysroot paths
At the moment, paths to the source files of standard and core libraries, even when they are present, always begin with a virtual prefix in the form
of `/rustc/[SHA1 hash]/library`. This is not an issue when the source files are not present (i.e. when `rust-src` component is not installed), but
when a user installs `rust-src` they expect the path to their local copy of source files to be visible. Hence the user should be given an option for
the local paths to show up in panic messages and backtraces.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

`trim-path` is a profile setting which can be set to either `true` or `false`. This is enabled by default when you do a release build,
such as via `cargo build --release`. You can also manually override it by specifying this option in `Cargo.toml`:
```toml
[profile.dev]
trim-path = true

[profile.release]
trim-path = false
```

With `trim-path` option enabled, the compilation process will not introduce any absolute paths into the build output. Instead, paths containing
certain prefixes will be replaced with something stable by the following rules:

1. Path to the source files of the standard and core library will begin with `/rustc/[rustc version]`.
   E.g. `/home/username/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs` -> 
   `/rustc/1.52.1/library/core/src/result.rs`
2. Path to the working directory will be replaced with `.`. E.g. `/home/username/crate/src/lib.rs` -> `./src/lib.rs`.
3. Path to packages outside of the working directory will be replaced with `[package name]-[version]`. E.g. `/home/username/deps/foo/src/lib.rs` -> `foo-0.1.0/src/lib.rs`

If using MSVC toolchain, path to the .pdb file containing debug information are be embedded as the file name of the .pdb file only, wihtout any path
information.

With `trim-path` option disabled, the embedding of path to the source files of the standard and core library will depend on if `rust-src` component is present. If it is, then the real path pointing to a copy of the source files on your file system will be embedded; if it isn't, then they will
show up as `/rustc/[rustc version]/library/...` (just like when `trim-path` is enabled). Path to all other source files will not be affected.

Note that this will not affect any hard-coded paths in the source code.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## `trim-path` implementation in Cargo
We only need to change the behaviour for `Test` and `Build` compile modes. 

If `trim-path` is enabled, Cargo will emit two `--remap-path-prefix` arguments to `rustc` for each compilation unit. One mapping is from the path of 
the local sysroot to `/rustc/[rust version]`. The other mapping depends on if the package containing the compilation unit is under the working
directory. If it is, then the mapping is from the absolute path to the working directory to `.`. If it's outside the working directory, then the
mapping is from the absolute path of the package root to `[package name]-[package version]`.

Some interactions with compiler-intrinstic macros need to be considered, though these are entirely down to `rustc`'s implementation of
`--remap-path-prefix`:
1. Path (of the current file) introduced by [`file!()`](https://doc.rust-lang.org/std/macro.file.html) *will* be remapped. **Things may break** if
   the code interacts with its own source file at runtime by using this macro.
2. Path introduced by [`include!()`](https://doc.rust-lang.org/std/macro.include.html) *will* be remapped, given that the included file is under
   the current working directory or a dependency package.

If the user further supplies custom `--remap-path-prefix` arguments via `RUSTFLAGS` or similar mechanisms, they will take precedence over the one
supplied by `trim-path`. This means that the user-defined `--remap-path-prefix`s must be supplied *after* Cargo's own remapping.

Additionally, when using MSVC linker, Cargo should emit `/PDBALTPATH:%_PDB%` to the linker via `-C link-arg`. This makes the linker embed
only the file name of the .pdb file without the path to it.

## Changing handling of sysroot path
The virtualisation of sysroot files to `/rustc/[SHA1 hash]/library/...` was done at compiler bootstraping, specifically when 
`remap-debuginfo = true` in `config.toml`. This is done for Rust distribution on all channels.

At `rustc` runtime (i.e. compiling some code), we try to correlate this virtual path to a real path pointing to the file on the local file system.
Currently the result is represented internally as if the path was remapped by `--remap-path-prefix`, holding both the virtual name and local path.
Only the virtual name is ever emitted for metadata or codegen. We want to change this behaviour such that, when `rust-src` source files can be
discovered, the virutal path is discarded and therefore will be embedded unless being remapped by `--remap-path-prefix` in the usual way. The relevant part of the code is here:
https://github.com/rust-lang/rust/blob/d8af907491e20339e41d048d6a32b41ddfa91dfe/compiler/rustc_metadata/src/rmeta/decoder.rs#L1637-L1765

We would also like to change the virtualisation of sysroot to `/rustc/[rustc version]/library/...`, instead of the rustc commit hash. This is shorter and more helpful as an identifier, and makes `trim-path` easier to implement: to make the embedded path the same whether or not `rust-src` is installed, we need to emit the same sysroot virutalisation as was done during bootstrapping. Getting the version number is easier than getting the commit hash. The relevant part of the code is here: https://github.com/rust-lang/rust/blob/d8af907491e20339e41d048d6a32b41ddfa91dfe/src/bootstrap/lib.rs#L831-L834 

# Drawbacks
[drawbacks]: #drawbacks

With `trim-path` enabled, if the `debug` option is simultaneously not `false` (it is turned off by default under `release` profile), paths in
debuginfo will also be remapped. Debuggers will no longer be able to automatically discover and load source files outside of the working directory. 
This can be remidated by [debugger features](https://lldb.llvm.org/use/map.html#miscellaneous) remapping the path back to a filesystem path.

The user also will not be able to `Ctrl+click` on any paths provided in panic messages or backtraces outside of the working directory. But
there shouldn't be any confusion as the combination of pacakge name and version can be used to pinpoint the file.

As mentioned above, `trim-path` may break code that relies on `file!()` to evaluate to an accessible path to the file. Hence enabling
it by default for release builds may be a technically breaking change. Occurances of such use should be extremely rare but should be investigated
via a Crater run. In case this breakage is unacceptable, `trim-path` can be made an opt-in option rather than default in any build profile.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

There has been an issue (https://github.com/rust-lang/rust/issues/40552) asking for path sanitisation to be implemented and enabled by default for 
release builds. It has, over the past 4 years, gained a decent amount of popular support. The remapping rule proposed here is very simple to 
implement.

Path to sysroot crates are specially handled by `rustc`. Due to this, the behaviour we currently have is that all such paths are virtualised.
Although good for privacy and reproducibility, some people find it a hinderance for debugging: https://github.com/rust-lang/rust/issues/85463.
Hence the user should be given control on if they want the virtual or local path.

One alternative for the sysroot handling is to keep the logic in `rustc` largely the same, always emitting the virutalised path by default, and
then introduce an extra option named `--embed-local-sysroot` to embed the local paths if the source files can be found. This inovles adding an extra
option to `rustc` and prevents any uniformity in `--remap-path-prefix`'s handling over sysroot paths, compared to other paths (it currently doesn't
affect sysroot paths at all).

# Prior art
[prior-art]: #prior-art

The name `trim-path` came from the [similar feature](https://golang.org/cmd/go/#hdr-Compile_packages_and_dependencies) in Go. An alternative name
`sanitize-paths` was first considered but the spelling of "sanitise" differs across the pond and down under. It is also not as short and concise.

Go does not enable this by default. Since Go does not differ between debug and release builds, removing absolute paths for all build would be
a hassle for debugging. However this is not an issue for Rust as we have separate debug build profile.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Should the option be called `trim-paths` (plural) instead of `trim-path`? Quite a few other option names are plural, such as `debug-assertions`
  and `overflow-checks`.
- Should we treat the current working directory the same as other packages? We could have one fewer remapping rule by remapping all
  package roots to `[package name]-[version]`. A minor downside to this is not being able to `Ctrl+click` on paths to files the user is working
  on from panic messages.
- Should we use a slightly more complex remapping rule, like distinguishing packages from registry, git and path, as mentioned in https://github.com/rust-lang/rust/issues/40552?
- Will these cover all potentially embedded paths? Have we missed anything?
- Should we make this affect more `CompileMode`s, such as `Check`, where the emitted `rmeta` file will also contain absolute paths?

# Future possibilities
[future-possibilities]: #future-possibilities

N/A