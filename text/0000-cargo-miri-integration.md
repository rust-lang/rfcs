- Feature Name: `cargo_miri_integration`
- Start Date: 2020-04-01
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

We propose that `cargo` should allow customizing build steps and dependency resolution arbitrarily by injecting Rust code directly into `cargo` and executing it with `miri`.
This allows:

1. Proper sandboxing of `build.rs` scripts, with the added bonus of checking build scripts for undefined behavior.
2. The ability to make arbitrary changes to the build process, such as implementing the entire `rustc` bootstrapping process as a build script or implementing a custom SAT solver for dependency resolution.
3. An appreciable performance degradation to `cargo`, which will hopefully incentivize people to cull their dependency graphs a bit.


# Motivation
[motivation]: #motivation

`cargo` is a fantastic tool. Everyone knows that. It makes building rust code and managing dependencies so easy. However, it has a few limitations in it's current form.

1. `cargo` is very limited in what it can do. Projects that require additional or custom build steps simply won't work. For example, `rustc` must use an additional bootstrapping tool written in a mix of python and rust because `cargo` has no way to specify the need for bootstrapping. Another example is building an OS kernel, where one might need to use weird linkage or manipulate the resulting ELF object into a disk image. Usually, one must resort to hacking together additional wrappers around `cargo`, such as `cargo-xtask`, or using `cargo` as a step within a larger build system such as a `makefile`, `bazel`, or `rustc`'s `bootstrap`.

2. In some cases, one can use `build.rs` scripts. These allow some customization of the build process, such as building and linking a non-rust library into a primarily rust project. Many `*-sys` crates do this. However, this is a bit dangerous as `build.rs` scripts can run arbitrary code. There have been prior calls for sandboxing `build.rs` scripts for security, but doing so is technically complex.

3. `cargo` is too fast and easy. Sometimes I just add random, unneeded dependencies to my projects because it feels so good. But this ease of dependency manage is not all roses and candy. Each dependency adds more transitive dependencies. It's not too unusal for your crate to end up depending on multiple versions of some popular crates. Pretty soon, you're up to your ears in direct and transitive dependencies. How did my simple command line calculator program end up depnding on an FFT library, `io_uring`, and _two different_ Paxos implementations? I gave up trying to figure it out.

These are the problems we aim to fix in this RFC.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

To start a new project, you can use `cargo new`. This will create a fresh directory with a stub crate and new `Cargo.toml` manifest.

When you want to build your project, you can do:

```sh
cargo build
```

This will do dependency resolution, build all dependencies and your crate, and link everything together properly.

But what actually happens underneath? How does `cargo build` actually work under the hood?

The answer is that you are invoking the _cargo interpretter_ (`corgi`) to run the `cargo-build` program. This is roughly equivalent to the following command:

```sh
corgi <sysroot>/cargo/bin/build/main.rs -- /path/to/your/crate/main.rs
```

Now suppose that you want to do something complicated with your build process, like bootstrapping your program. We can do this by writing our own `cargo build` and specicifying it in `Cargo.toml` like this:

```toml
...

[[cargo]]
main = "/path/to/your/crate/cargo.rs"
```

We then create the `cargo.rs` file, which is a normal rust binary that does dependency resolution, builds everything, and links everything. It can run arbitrary rust code. If we exclude this table from the `Cargo.toml`, then the default `cargo-build` program from the standard rust distribution is used.

Obviously, rewriting all of `cargo` would be a bit painful. Luckily, we don't have to. We can import `libcargo` and use the parts of the standard `cargo-build` that we don't want to replace or modify.

Finally, we can do `cargo build` just like normal, except this time, `cargo build` will end up running something like:

```sh
corgi /path/to/your/crate/cargo.rs -- /path/to/your/crate/main.rs
```

One more thing: `corgi` runs everything in `miri`, so it's super slow and finds all of the UB in your build system. Awesome! This is safe, sandboxed, and discourages using too many dependencies while being infinitely customizable.


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## `corgi`

The `corgi` tool is introduced. `corgi` functions as a virtual machine, and the build system runs inside it via `miri`. `miri` is extended to virtualize the execution of system calls, so that we can execute only system calls that won't cause UB.

`corgi` terminates the build with an error if the build system contains any UB.

## Translating `cargo` commands

Under this RFC, all of the `cargo <foo>` subcommands become thin wrappers to call `corgi` on some rust code. Usually, that rust code will be the standard implementations of the `cargo` tools, found in the sysroot of the user's distribution. However, as shown above, the user can customize the actual implementation of their `cargo` tools by specifying the `[[cargo]]` table in their `Cargo.toml`.

Commands are translated as follows:

```sh
cargo foo
```

becomes

```sh
corgi $PATH_TO_FOO -- /path/to/your/crate/main.rs
```

where `$PATH_TO_FOO` is either the path specified in `Cargo.toml` or `<sysroot>/cargo/bin/foo/main.rs` if unspecified.


# Drawbacks
[drawbacks]: #drawbacks

The slowness of `miri` may increase build times mildly. However, this expense is deemed small in comparison to vast might of safe-build-system-yness that will be unleashed on a UB-impoverished world.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

> - Why is this design the best in the space of possible designs?

Seems obvious, right? `miri`.

> - What other designs have been considered and what is the rationale for not choosing them?

- Other virtualization techniques could be used, including virtual machines or containers. Such a design would increase sandboxing of build scripts, but it doesn't check for UB or increase the customizability of builds.
- Custom wrappers around `cargo` can customize the build process. `cargo` "pluggins" are one means of doing this (e.g. `rust-analyzer` uses the `xtask` pluggin system). Other projects opt to write their own build systems (e.g. `rustc` itself). Still others, just wrap `cargo` in `Makefile`s, `bazel`, or other more flexible build systems. These options have many disadvatages:
    - They are clunky and non-standard. They require the user to ignore muscle memory and do something custom.
    - They can execute even more code without a sandbox.
    - They often require reimplementing features of `cargo` outside of `cargo`.

> - What is the impact of not doing this?

It is highly likely that the world will end.

# Prior art
[prior-art]: #prior-art

There is so much prior work, I wouldn't know where to start. A quick search for "virtualization"  on ACM Digital Library returns 126753 results. A search for "interpretter" returns 158217 results. I haven't read all of them, but my guess is that the ones I haven't read all support the ideas of this RFC.

There have been many [discussions on IRLO about sandboxing code](https://internals.rust-lang.org/search?q=sandbox).

Ken Thompson's excellent [Reflections on Trusting Trust](https://dl.acm.org/doi/10.1145/358198.358210) underscores the importance of trusting one's infrastructure. As we all know, if a program is UB-free, it is entirely trustworthy.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Are corgis really the best dog to name this tool after? The name at least sounds like `cargo` a bit...
- What about all the cat-lovers in the rust community?

# Future possibilities
[future-possibilities]: #future-possibilities

The obvious extension would be to get rid of the system call interface of `corgi`, as this represents attack surface and added complexity.
One way to do so would be to reimplement `Linux` (or really any OS) and run it inside of `corgi` with the rest of the build system.

For added security, one can follow the [Defense in Depth](https://csrc.nist.gov/glossary/term/defense_in_depth) strategy: implement `corgi` inside `vim` on Linux inside `corgi`.
