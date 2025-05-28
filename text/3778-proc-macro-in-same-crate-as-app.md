- Feature Name: `proc-macro-in-same-crate-as-app`
- Start Date: 2025-5-29

tbd:
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Have a new folder in a cargo project, called `proc-macro`. This would be like the `tests` directory in that it is alongside the source code. This would eliminate the need to create an extra crate for proc macros.

# Motivation
[motivation]: #motivation

A common thing to ask about proc macros when one is first learning them is: "Why on earth does it have to be in a separate crate?!" Of course, we eventually get to know that the reason is that proc macros are basically *compiler plugins*, meaning that they have to be compiled first, before the main code is compiled. So in summary, one needs to be compiled before the other.

It doesn't have to be this way though, because we already have this mechanism of compiling orders – the example that come to mind is the `tests` directory. It relies on the `src` directory being built first, and likewise we could introduce a `proc-macro` directory that would be compiled before `src`.

The motivation of having this new directory comes down to just convenience. This may sound crude at first, but convenience is a key part of any feature in software. It is known in UX design that every feature has an *interaction cost*: how much effort do I need to put in to use the feature? For example, a feature with low interaction cost, with text editor support, is renaming a variable. Just press F2 and type in a new name. What this provides is incredibly useful – without it, having a subpar variable/function name needed a high interaction cost, especially if it is used across multiple files, and as a result, we are discouraged to change variable names to make it better, when we have new retrospect. With a lower interaction cost, the renaming operation is greatly promoted, and leads to better code.

This proposal aims smooth out the user experience when it comes to creating new proc macro, and achieve a similar effect to the F2 operation. It is important to emphasise that proc macros can dramatically simplify code, especially derive macros, but they a lot of the times aren't used because of all the extra hoops one has to get through. This would make proc macros (more of) "yet another feature", rather than a daunting one.

An objection to this one might raise is "How much harder is typing in `cargo new` than `mkdir proc-macro`?" But we should consider if we would still use as much integration tests if the `tests` directory if it is required to be in a seperate crate. The answer is most likely less. This is because (1) having a new crate requires ceremony, like putting in a new dependency in cargo.toml, and (2) requires adding to the project structure. A *tiny* bit in lowering the interaction cost, even from 2 steps to 1, can greatly improve the user experience. 

In summary (TL;DR), the effort one needs to put in to use a feature is extremely important. Proc macros currently has a higher ceiling, needing one to create a whole new crate in order to use it, and lowering the ceiling, even just a little bit, could massively improve user experience. This proposal can lower it.

# Explanation
[explanation]: #explanation

Currently, we create a new proc macro as so:
1. Create a new crate
2. In its cargo.toml, specify that it is a proc macro crate
3. In the main project, add the crate as a dependency
4. Implement the proc macro in the new crate

After this change, we create a new proc macro like this:
1. Create a new directory called `proc-macro` alongside your `src` directory
2. Implement the proc macro in a new file in `proc-macro`.

To use the proc macro, simply import it via `crate::proc_macro`.
```rust
use crate::proc_macro::my_file::my_macro;
```
Or, if the file happens to be `mod.rs`, you can access it directly after the `proc_macro` bit.

## Proc Macro Libraries
Crates like `syn`, `quote`, and `proc-macro2`, would be included under `[dev-dependecies]` in the cargo.toml. (Perhaps we should put it in build dependencies? or a new dependency section for proc macros.)

## How it would work in the implementation
Cargo would have to compile the `proc-macro` directory first, as a proc macro type (of course). Then, in compiling the main code, `crate::proc_macro::file_name::my_macro` would resolve the module to the file `/proc-macro/file_name.rs`. Alternatively, if the user uses `mod.rs`, it would be resolved from `crate::proc_macro::my_macro`. This would finally be passed into rustc.

# Drawbacks
[drawbacks]: #drawbacks

1. The proc macro directory cannot use functions from src. (but that was not possible before anyways)

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

> Have proc macro files marked `#![proc_macro_file]` to signal to cargo to compile it first.

Since it would compile first, proc macro files cannot import functions in the main code. The problem is having it side-by-side to the rest of your code makes it seem like you could just import it, when you cannot. Having it as a seperate directory makes clear of this.

> Eliminate the need for new proc macro files/folders entirely, have the compiler work out where the proc macros are and separate them.

This would suffer from the same issue as the last alternative, plus being harder to implement.

> Introspection

Harder to implement, with less payoff. 

# Prior art
[prior-art]: #prior-art

1. Zig comptime: metaprogramming code can sit directly next to application code.
2. Declarative macros: can sit side by side as well, but is less powerful.
3. Lisp macros: same as last two, except more powerful.
4. `tests` directory, and `build.rs`: compiled at a different time as the main code.
5. `Makefiles`, or other build systems: they allow for more customisability for when code is built.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

1. Should proc macro dependencies be listed under `[dev-dependencies]`, `[build-dependencies]`, or a new `[proc-macro-dependencies]` section?
2. Should we import like `crate::proc_macro::file::macro`, or via a new keyword, like `crate_macros::file::macro`? The latter would avoid name collisions, but might be more confusing.

# Future possibilities
[future-possibilities]: #future-possibilities

As described in the [motivation] section, this proposal is aimed to make the process of creating proc macros easier. So a natural extension of this is to remove the need of third-party crates like syn and proc-macro2. There is already an effort to implement quote, so they might be a possibility.