- Feature Name: post_build
- Start Date: 2016-10-26
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

This will add the ability for an extra build script to be run *after* the build completes.

This is a similar concept current build script concept but rather than processing files beforehand,
this script would perform tasks _after_ the crate has completed compilation.

# Motivation
[motivation]: #motivation

This feature will give Rust programmers more flexibility when compiling their crates.

Pre-build scripts as we have today are very useful for performing tasks before the
crate is compiled, such as for compiling C/C++ sources or generating bindings. It
is not currently possible to do anything _after_ Cargo finishes.

## Use cases

On most platforms, once linking is done there is no longer any work to do. This
differs on some systems, such as AVR (which requires linked ELF binaries to be
converted into raw binary blobs).

Currently this would require something like a Makefile which internally
calls `cargo`, which prohibits the crate being published to `crates.io`.

In another case, we could use post-build scripts to generate mountable disk
images for an operating system kernel written in Rust. Again, this is something
that would require an external build system today.

# Detailed design
[design]: #detailed-design

The crate manifest `[package]` table will support another textual field `post-build`.

A normal manifest would look something like this:

```toml
[package]
name = "foobar"

build = "pre-build.rs"
post-build = "post-build.rs"
```

Once `cargo build` finishes compiling a crate, it will check if there is a `post-build` script
configured, and it will compile and run it similarly to the current build script setup.

A post-build script looks like this:

```rust
use std::process::Command;

fn main() {
  let out_dir = env::var("OUT_DIR").unwrap();

    Command::new("avr-objcopy")
        .arg(&format!("-i elf32 -o binary {}/myobject {}/mybinary", out_dir, out_dir))
        .spawn();
}
```

If the build script returns an error code, Cargo should report compilation as failed.

# Drawbacks
[drawbacks]: #drawbacks

* More complexity in the Cargo source
* A corner case that won't be used very often

# Alternatives
[alternatives]: #alternatives

* Use a Makefile-like build system which calls into Cargo
* Expect users to do post-processing manually

If Cargo will not support something given crate authors this ability, people will
be forced to stray-away from the crates ecosystem and use something more custom.

# Unresolved questions
[unresolved]: #unresolved-questions

* What should "crate compilation finished" mean in this regard?
* Should `cargo doc` run the post-build script?
