- Feature Name: `export_function_ordinals`
- Start Date: 2024-05-19
- RFC PR: [rust-lang/rfcs#3641](https://github.com/rust-lang/rfcs/pull/3641)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Adding an unsafe attribute, `#[unsafe(export_ordinal(n))]`, that marks the ordinal position of an exported function in a cdylib on windows targets without creating a `lib.def` file.

# Motivation
[motivation]: #motivation

Sometimes when creating DLLs, the ordinal position of an exported function is very important. For example, when creating a DLL for use in [Microsoft Detours](https://github.com/microsoft/Detours/), the [`DetourFinishHelperProcess`](https://github.com/microsoft/Detours/wiki/DetourFinishHelperProcess) function must be Ordinal 1.

Rust currently has a [`link_ordinal`](https://doc.rust-lang.org/reference/items/external-blocks.html#the-link_ordinal-attribute) attribute which allows importing a function by its ordinal, however there is currently no option to do the opposite.

Currently, this would be done by creating a `lib.def` file and linking it in `build.rs`.

```def
; lib.def
LIBRARY
EXPORTS
    DetourFinishHelperProcess @1
```

```rs
// build.rs
pub fn main() {
    let lib_def = "path/to/lib.def";
    println!("cargo:rustc-cdylib-link-arg=/DEF:{}", lib_def);
}
```

The biggest downside of the current method is that once you specify a `.def` file, you will have to specify an ordinal for every function that you want to export from the DLL, or else it won't be present in the generated `.lib` file. This can become very overwhelming if you have a lot of exported functions.

By creating an attribute for specifying function ordinals, we can choose the ordinal position for the functions where it matters, and let Rust choose the ordinal for any other functions where ordinal position is not important.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## Ordinals

Function Ordinals refer to the position of an exported function in a Dynamically Linked Library (DLL). When accessing functions by name, this is not important. However some applications access functions based on their position (ordinal), rather than their name. The Microsoft documentation for this concept is available [here.](https://learn.microsoft.com/en-us/cpp/build/exporting-functions-from-a-dll-by-ordinal-rather-than-by-name)

## Usage

You can specify the ordinality of an exported function using the `export_ordinal` attribute on it. The attribute must be marked as unsafe.

```rs
#[unsafe(export_ordinal(1))]
pub extern "C" fn hello() {
    println!("Hello, World!");
}
```

This example will export `hello` as ordinal 1, and when a program tries to call ordinal 1 in your DLL, it will be executed.

## Behaviour

If other software expects your function to be a specific ordinal, you should be very careful when changing the ordinal or removing the `export_ordinal` attribute, as it could lead to the wrong function being called (or not found at all).

If `export_ordinal` isn't provided, an unused ordinal will be assigned during compilation.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

`export_ordinal` is a new attribute for functions which has a signature similar to the following:

```rs
#[unsafe(export_ordinal(n))]
```

`n` must be:

1. A positive integer >= 1
2. Unique across the entire program.
   - An error should be thrown if the same ordinal is provided in multiple places.

The attribute should only affect windows targets, as ordinals are not a feature of shared libraries on other targets.

The attribute must be marked as unsafe.

The attribute must be placed above an exported function like so:

```rs
#[no_mangle]
#[unsafe(export_ordinal(1))]
pub fn hello() {}

// Also works with extern and unsafe functions

#[no_mangle]
#[unsafe(export_ordinal(2))]
pub unsafe extern "C" fn world() {}
```

# Drawbacks
[drawbacks]: #drawbacks

1. Specifying ordinals in code could add a lot of additional complexity with linking.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This design is consistent with the [`link_ordinal`](https://doc.rust-lang.org/reference/items/external-blocks.html#the-link_ordinal-attribute) attribute already in use.

The attribute is marked as unsafe as it shares the same concerns as `export_name`, which is [unsafe as of Rust 2024 Edition](https://github.com/ehuss/edition-guide/blob/b80cba8af64a9c52d56f7081c764e5396e406f6c/src/rust-2024/unsafe-attributes.md).

Some considered alternatives are:

1. Do nothing; keep using the `.def` files with `cargo:rustc-cdylib-link-arg=/DEF`
    - The main downside of doing nothing and using the `.def` file, is that if you only need one function with a specific ordinal, you have to add every exported function to the `.def` file or they won't be linkable.
2. Use macros to generate a `.def` file
    - A good implementation of this would likely require stateful macros.
3. Implement a way to provide a `.def` file without also having to specify every other exported function inside it.
    - This would be a good alternative, although the implementation could be more complicated.

This proposal should make the workflow of specifying ordinals much easier, while staying consistent with the syntax of the existing `link_ordinal`.

# Prior art
[prior-art]: #prior-art

I am not currently aware of any programming languages that currently implement an equivalent feature.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

Some unresolved questions are:
1. Can ordinals be skipped? If you specify ordinals `1, 3, 4`, should this throw an error as `2` is skipped?
2. If ordinals `1, 3` are specified, and you have another exported function, should it use `2` (the next unused ordinal) or `4` (the next in the sequence)?
3. Instead of implementing this proposal, Could the usage of the `.def` file be changed to allow other functions to stay exported, even if they aren't included in the `.def` file?

# Future possibilities
[future-possibilities]: #future-possibilities

I cannot currently think of any future possibilities.
