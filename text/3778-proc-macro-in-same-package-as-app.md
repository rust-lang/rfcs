- Feature Name: `proc-macro-in-same-package-as-app`
- Start Date: 2025-05-30
- RFC PR: [rust-lang/rfcs#3826](https://github.com/rust-lang/rfcs/pull/3826)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000) tbd

# Summary
[summary]: #summary

Have a new target in a cargo project, called `proc-macro`. Its default location is in `proc-macro/lib.rs`. This would be like the `tests` directory in that it is alongside the source code. It would eliminate the need to create an extra package for proc macros.

# Motivation
[motivation]: #motivation

A common thing to ask about proc macros when first learning them is: "Why on earth does it have to be in a separate package?!" Of course, we eventually get to know that the reason is that proc macros are basically *compiler plugins*, meaning that they have to be compiled first, before the main code is compiled. So in summary, one needs to be compiled before the other.

**To be absolutely clear**, this is not a proposal for same-*crate* proc macros (unlike previous proposals), but same-*package* proc macros: a much simpler problem. In a package, we already have mechanisms of compiling one target before another (for example lib before bin).

There are two major benefits:

## For Library <small>(and application, actually)</small> Code
Take the standard relationship between two packages: the main `lib`, and the corresponding `lib-derive`. 

`lib: 1.0.0` depends on `lib-derive: 1.0.0` , but `lib-derive` needs testing, or APIs from `lib`, so it itself depends on `lib` (`1.0.0`). When updates happen, they have be synced up, which is acheived by specifying `=x.x.x` as its version, and could cause problems if improperly handled. **This happens naturally if they are merged into a single package.**

Through having them together, the relationship between the `lib` and its corresponding `lib-derive` is enforced. Having this "official" way of declaring that these two crates are related means (1) the experience would be smoother and (2) there could be more improvements made upon this. One such improvement is opening up the possibility `$crate` (or somethig akin to it) for procedural macros.

## For Application Code
Currently, we create a new proc macro as so:
1. Create a new package
2. In its cargo.toml, specify that it is a proc macro package
3. In the main project, add the package as a dependency
4. Implement the proc macro in the new package

It is known in UX design that every feature has an *interaction cost*: how much effort do I need to put in to use the feature? For example, a feature with low interaction cost, with text editor support, is renaming a variable. Just press F2 and type in a new name. What this provides is incredibly useful – without it, having a subpar variable/function name needed a high interaction cost, especially if it is used across multiple files, and as a result, we are discouraged to change variable names to make it better, when we have retrospect. With a lower interaction cost, the renaming operation is greatly promoted, and leads to better code.

This proposal aims smooth out the user experience when it comes to creating new proc macro, and achieve a similar effect to the F2 operation. It is important to emphasise that proc macros can dramatically simplify code, especially derive macros, but they a lot of the times aren't used because of all the extra hoops one has to get through. This would make proc macros (more of) "yet another feature", rather than a daunting one.

An objection to this one might raise is "How much harder is typing in `cargo new` than `mkdir proc-macro`?" But we should consider if we would still use as much integration tests if the `tests` directory if it is required to be in a seperate package. The answer is most likely less. This is because (1) having a new package requires ceremony, like putting in a new dependency in cargo.toml, and (2) requires adding to the project structure. A *tiny* bit in lowering the interaction cost, even from 2 steps to 1, can greatly improve the user experience. 

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

After this change, we create a new proc macro like this:
1. Create a new folder in the root of the project called `proc-macro`
2. Implement the proc macro in a new `lib.rs` in the new folder.

To build only the macro, use:
```console
$ cargo build --proc-macro
```

## Importing
[importing]: #importing
To use the proc macro, simply import it via `proc_macro::*`.
```rust
use proc_macro::my_macro;
```

Note that macros is only available to inside the package (i.e. bin, lib, examples...). This means that one would have to reexport the macros in `lib.rs` in order for users of a library to use it. It would still be available in `main.rs`, `tests`, `examples`, etc, though.

## An example
Suppose you are developing a library that would have normal functions as well as proc macros. The file structure would look like this:
```
My-Amazing-Library
|---proc-macro
|   |---lib.rs
|---src
|   |---lib.rs
|---cargo.toml
```
`proc-macro/lib.rs` defines macros, which will be made available to `src/lib.rs`. `src/lib.rs` can use the macros defined, and reexport the macros to make it available to anyone using the library.

Now, to make the macros available, reexport them, and if you want gate a macro behind a feature flag, it would be like how you would normally also, with cfg:
```rust
// in src/lib.rs
#[cfg(feature = "my_feature")]
pub use proc_macro::a_niche_macro;
pub use proc_macro::a_common_macro; // (not gated)
```

Finally, testing is also how you would expect it. It would be made available to the `tests` directory. ([importing])

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The package targets would be compiled in the following order:
1. `build`
2. `proc-macro`
3. `lib`
4. `bin`s
5. ...

The macros would be available to all targets built afterwards. Exports of `proc-macro` is only available inside the package, so any publicly available ones need to be reexported in `lib`.

Any libraries to be linked, as specified in `build.rs` via stdout, are not to be available to `proc-macro`. In addition, linker arguments can be passed through `cargo::rustc-link-arg-proc-macro=FLAG` via stdout of `build.rs`.

Any artifacts created during compilation are to be in `target/_profile_/deps`, as usual. The compiled macros crate would be passed into rustc with `--extern=proc_macro=target/_profile_/deps/lib_____` when compiling the other crates. Finally, the compiled lib_____ would be put in `target/proc-macro/`, along with its `.d` file.

## Cfg and Environment Variables
During compilation, it would set the `proc_macro` cfg variable (i.e. `assert!(cfg!(proc_macro))` would be ok in the `proc-macro` crate).

As well as those it, the following environment variables are set. For conciseness, this RFC will not attempt to outline the use of all environment variables. Refer to the [documentation](https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-crates).
- `CARGO`
- `CARGO_MANIFEST_DIR`
- `CARGO_MANIFEST_PATH`
- `CARGO_PKG_VERSION`
- `CARGO_PKG_VERSION_MAJOR`
- `CARGO_PKG_VERSION_MINOR`
- `CARGO_PKG_VERSION_PATCH`
- `CARGO_PKG_VERSION_PRE`
- `CARGO_PKG_AUTHORS`
- `CARGO_PKG_NAME`
- `CARGO_PKG_DESCRIPTION`
- `CARGO_PKG_HOMEPAGE`
- `CARGO_PKG_REPOSITORY`
- `CARGO_PKG_LICENSE`
- `CARGO_PKG_LICENSE_FILE`
- `CARGO_PKG_RUST_VERSION`
- `CARGO_PKG_README`
- `OUT_DIR`
- `CARGO_PRIMARY_PACKAGE`

## Cargo.toml configs
Libraries like `syn`, `quote`, and `proc-macro2`, would be included under `[build-dependecies]` in the cargo.toml. (Perhaps we should put it in a new dependency section for proc macros?)

Like `tests` or `lib`, this would have its own `[proc-macro]` section in cargo.toml.

Here are all the options available under it, the values set are its default.
```toml
[proc-macro]
name = "proc_macro"
path = "proc-macro/lib.rs"
test = true
doctest = true
bench = false
doc = true
proc-macro = true # (cannot be changed)
```

To disable automatic finding, use:
```toml
[package]
autoprocmacro = false
```

## Cargo CLI Additions
For all cargo subcommands that provide options for selecting a specific target, the `--proc-macro` flag is used to refer to this target.

For example:
- `cargo build --proc-macro` – Compile `proc-macro` only
- `cargo build --all-targets` – Equivalent to specifying `--lib --bins --tests --benches --examples --proc-macro`

## Documentation

Running `cargo doc` automatically creates documentation for the macro crate by default. The identifier would be:
- the name of the crate as specified in cargo.toml, if it has been overriden
- otherwise, if the name of the macro crate is on its default value: the name of the package, with `.proc_macro` appended to it. *For example: `my-library.proc_macro`*

This is to prevent name collisions in the documentation, since dependecies may also produce the `proc-macro` target.

In the case that the user has specified `doc = false`, the `--proc-macro` flag can be used to explicitly tell `cargo doc` to generate documentation for this target.

# Drawbacks
[drawbacks]: #drawbacks

1. Added complexity - Somewhat increases maintainance cost of cargo
2. Migrations - Existing crates now need to migrate to the new system, taking time, and it may cause some exisiting code that's always using the latest version of libraries to break.
3. Build systems that aren't Cargo needs to update to keep up with this feature

## 3-Part Libraries
*Not so much of a drawback as it is a limitation, but it felt like this was the most appropriate section to put this in.*

Some crates like serde seperate the 'core' functionality and the macros: `serde-core` and `serde-derive` are two packages, which are re-exported in a façade package `serde`. (Sidenote: This would also encounter syncing issues)

This provides an edge over feature gating (what would be used in the previous example), and can be seen when another package `serde-json` depends on only `serde-core`, and not the entire `serde`. This way, `serde-json` can be compiled as soon as `serde-core` is, and doesn't need to wait for `serde-derive`.

The downside of this RFC is that if there are two crates that depend on a library with both a `lib` component and a `proc-macro` component, one of them may need to pay for more compilation time. 
Say, if this library allows users to choose whether or not they need macros, by providing a `macros` feature gate. If both crates don't need `macros`, then we can save some compilation time. However, when one of them enables `macros`, **then both crates needs to wait for the `proc-macro` target to be compiled, even if the other crate does not need the extra functionality.**

Rendered, libraries such as `serde` would be unlikely to make use of the proposed feature, but other libraries like `tokio`, which finds that only two parts are needed, and application code could be improved.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

1. Use `crate::proc_macro::*` as the import path

This would require changes to rustc, when there is a simpler solution.

2. Have it within `src/proc-macro.rs`

The problem with this is the confusion it creates for users. Someone looking through `src/` aren't able to see which files are part of the library, or part of proc macro. In addition, having it within the same directory have led some users, like with `lib.rs` and `main.rs`, to use `mod` for importing when they meant `use` from the library.

3. Eliminate the need for new proc macro files/folders entirely, have the compiler work out where the proc macros are and separate them.

This would suffer from the same issue as the last alternative, plus being harder to implement.

4. Introspection

Harder to implement, with less payoff relative to the amount of work required. 

5. Naming conventions: `proc-macro` is chosen as the name of the target to align with existing formats. See [the `crate-type` field](https://doc.rust-lang.org/cargo/reference/cargo-targets.html#the-crate-type-field). Furthermore, many projects already have a `macros` module for declarative macros, thus we use `proc_macro` as the crate name to avoid a collision.

# Prior art
[prior-art]: #prior-art

1. Zig comptime: metaprogramming code can sit directly next to application code.
2. Declarative macros: can sit side by side as well, but is less powerful.
3. Lisp macros: same as last two, except more powerful.
4. `tests` directory, and `build.rs`: compiled at a different time as the main code.
5. `Makefiles`, or other build systems: they allow for customisability for when code is built.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

1. Should proc macro dependencies be listed under `[build-dependencies]`, or a new `[proc-macro-dependencies]` section?
2. What case should `proc-macro/` be? No spaces, kebab case, or snake? Having no spaces would be the most agnostic solution
2. ~~Should we import like `crate::proc_macro::file::macro`, or via a new keyword, like `crate_macros::file::macro`? The latter would avoid name collisions, but might be more confusing.~~

# Future possibilities
[future-possibilities]: #future-possibilities

1. As described in the [motivation] section, this proposal is aimed to make the process of creating proc macros easier. So a natural extension of this is to remove the need of third-party libraries like syn and proc-macro2. There is already an effort to implement quote, so they might be a possibility.

2. This might enable for some sort of `$crate` metavariable.