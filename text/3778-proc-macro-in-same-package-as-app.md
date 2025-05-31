- Feature Name: `proc-macro-in-same-package-as-app`
- Start Date: 2025-05-30
- RFC PR: [rust-lang/rfcs#3826](https://github.com/rust-lang/rfcs/pull/3826)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000) tbd

# Summary
[summary]: #summary

Have a new target in a cargo project, called `proc-macro`. Its default location is  in `src/macros.rs`. This would be like the `lib.rs` in that it is alongside the source code. It would eliminate the need to create an extra package for proc macros.

# Motivation
[motivation]: #motivation

A common thing to ask about proc macros when first learning them is: "Why on earth does it have to be in a separate package?!" Of course, we eventually get to know that the reason is that proc macros are basically *compiler plugins*, meaning that they have to be compiled first, before the main code is compiled. So in summary, one needs to be compiled before the other.

It doesn't have to be this way though, because we already have this mechanism of compiling one thing before another – for example, the `tests` directory. It relies on the `src` directory being built first, and likewise we could introduce a `proc-macro` target that would compile before `src`.

**To be absolutely clear**, this is not a proposal for same-*crate* proc macros (unlike previous proposals), but same-*package* proc macros: a much simpler problem. 

The motivation of this new target comes down to just convenience. This may sound crude at first, but convenience is a key part of any feature in software. It is known in UX design that every feature has an *interaction cost*: how much effort do I need to put in to use the feature? For example, a feature with low interaction cost, with text editor support, is renaming a variable. Just press F2 and type in a new name. What this provides is incredibly useful – without it, having a subpar variable/function name needed a high interaction cost, especially if it is used across multiple files, and as a result, we are discouraged to change variable names to make it better, when we have retrospect. With a lower interaction cost, the renaming operation is greatly promoted, and leads to better code.

This proposal aims smooth out the user experience when it comes to creating new proc macro, and achieve a similar effect to the F2 operation. It is important to emphasise that proc macros can dramatically simplify code, especially derive macros, but they a lot of the times aren't used because of all the extra hoops one has to get through. This would make proc macros (more of) "yet another feature", rather than a daunting one.

An objection to this one might raise is "How much harder is typing in `cargo new` than `touch macros.rs`?" But we should consider if we would still use as much integration tests if the `tests` directory if it is required to be in a seperate package. The answer is most likely less. This is because (1) having a new package requires ceremony, like putting in a new dependency in cargo.toml, and (2) requires adding to the project structure. A *tiny* bit in lowering the interaction cost, even from 2 steps to 1, can greatly improve the user experience. 

Another benefit is that a library developer don't have to manage two packages if one requires proc macros, and make them be in sync with each other.

In summary (TL;DR), the effort one needs to put in to use a feature is extremely important. Proc macros currently has a higher ceiling, needing one to create a whole new package in order to use it, and lowering the ceiling, even just a little bit, could massively improve user experience. This proposal can lower it.

# Explanation
[explanation]: #explanation

Currently, we create a new proc macro as so:
1. Create a new package
2. In its cargo.toml, specify that it is a proc macro package
3. In the main project, add the package as a dependency
4. Implement the proc macro in the new package

After this change, we create a new proc macro like this:
1. Implement the proc macro in a new `macros.rs` in `proc-macro`.

To use the proc macro, simply import it via `macros::*`.
```rust
use macros::my_macro;
```

## An example
Suppose you are developing a library that would have normal functions as well as proc macros. The file structure would look like this:
```
My-Amazing-Library
|---src
|   |---lib.rs
|   |---macros.rs
|   |---common.rs
|---cargo.toml
```
`common.rs` is a normal file that declares common data structures and functions. `macros.rs` defines macros, which will be made available to `lib.rs`. `lib.rs` can use the macros defined, and/or reexport the macros.

Using code in `common.rs` in `macros.rs` is like how you would normally:
```rust
mod common;
use common::*;
```

## Cargo.toml configs
Libraries like `syn`, `quote`, and `proc-macro2`, would be included under `[build-dependecies]` in the cargo.toml. (Perhaps we should put it in a new dependency section for proc macros?)

Like `tests` or `lib`, this would have its own `[proc-macro]` section in cargo.toml.

Here are all the options available under it, the values set are its default.
```toml
[proc-macro]
name = "macros"
path = "src/macros.rs"
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

## How it would work in the implementation
Then pass it into rustc with `--extern=macros=target/_profile_/deps/lib_____`. The process would occur after the compilation of `build.rs` to make metadata and files generated in OUT_DIR available, but before `lib.rs` to make macros available. This means `macros.rs` cannot use code in `lib.rs`, so the code would have to be factored out into a seperate file.

# Drawbacks
[drawbacks]: #drawbacks

None at the moment

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

1. Use `crate::macros::*` as the import path

This would require changes to rustc, when there is a simpler solution.

2. Have it within a `proc_macros` directory

This was the original idea; but upon further consideration it turns out to be worse than the current. The justification of it over a file was:
> *Since it would compile first, proc macro files cannot import functions in the main code. The problem is having it side-by-side to the rest of your code makes it seem like you could just import it, when you cannot. Having it as a seperate directory makes clear of this.*

While it was true that proc macro files cannot import functions in the main code, it can import other modules, making the statement's merits false.

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
2. ~~Should we import like `crate::proc_macro::file::macro`, or via a new keyword, like `crate_macros::file::macro`? The latter would avoid name collisions, but might be more confusing.~~

# Future possibilities
[future-possibilities]: #future-possibilities

1. As described in the [motivation] section, this proposal is aimed to make the process of creating proc macros easier. So a natural extension of this is to remove the need of third-party libraries like syn and proc-macro2. There is already an effort to implement quote, so they might be a possibility.

2. This might enable for some sort of `$crate` metavariable.

3. Enabling multiple lib targets.