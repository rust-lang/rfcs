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

Currently, we create a new proc macro as so:
1. Create a new package
2. In its cargo.toml, specify that it is a proc macro package
3. In the main project, add the package as a dependency
4. Implement the proc macro in the new package

While this doesn't seem like a lot, with reasons explained later, could actually be making code worse.

It doesn't have to be this way though, because we already have this mechanism of compiling one thing before another – for example, the `tests` directory. It relies on the `src` directory being built first, and likewise we could introduce a `proc-macro` target that would compile before `src`.

**To be absolutely clear**, this is not a proposal for same-*crate* proc macros (unlike previous proposals), but same-*package* proc macros: a much simpler problem. 

The motivation of this new target comes down to just convenience. This may sound crude at first, but convenience is a key part of any feature in software. It is known in UX design that every feature has an *interaction cost*: how much effort do I need to put in to use the feature? For example, a feature with low interaction cost, with text editor support, is renaming a variable. Just press F2 and type in a new name. What this provides is incredibly useful – without it, having a subpar variable/function name needed a high interaction cost, especially if it is used across multiple files, and as a result, we are discouraged to change variable names to make it better, when we have retrospect. With a lower interaction cost, the renaming operation is greatly promoted, and leads to better code.

This proposal aims smooth out the user experience when it comes to creating new proc macro, and achieve a similar effect to the F2 operation. It is important to emphasise that proc macros can dramatically simplify code, especially derive macros, but they a lot of the times aren't used because of all the extra hoops one has to get through. This would make proc macros (more of) "yet another feature", rather than a daunting one.

An objection to this one might raise is "How much harder is typing in `cargo new` than `mkdir proc-macro`?" But we should consider if we would still use as much integration tests if the `tests` directory if it is required to be in a seperate package. The answer is most likely less. This is because (1) having a new package requires ceremony, like putting in a new dependency in cargo.toml, and (2) requires adding to the project structure. A *tiny* bit in lowering the interaction cost, even from 2 steps to 1, can greatly improve the user experience. 

Another benefit is that a library developer don't have to manage two packages if one requires proc macros, and make them be in sync with each other.

In summary (TL;DR), the effort one needs to put in to use a feature is extremely important. Proc macros currently has a higher ceiling, needing one to create a whole new package in order to use it, and lowering the ceiling, even just a little bit, could massively improve user experience. This proposal can lower it.

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
To use the proc macro, simply import it via `macros::*`.
```rust
use macros::my_macro;
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
name = "macros"
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
- `cargo build --proc-macro` – Compile `proc-macro` only
- `cargo build --all-targets` – Equivalent to specifying `--lib --bins --tests --benches --examples --proc-macro`
- `cargo test --proc-macro` – Test `proc-macro` only

## Documentation

There would be a new item listed under "Crates" of the sidebar, for the new crate. This should only display "macros" — or whatever the name of the `proc-macro` crate happened to be called — of the current package. 

# Drawbacks
[drawbacks]: #drawbacks

1. Added complexity - Somewhat increases maintainance cost of cargo
2. Migrations - Existing crates now need to migrate to the new system, taking time, and it may cause some exisiting code that's always using the latest version of libraries to break.
3. Build systems that aren't Cargo needs to update to keep up with this feature

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