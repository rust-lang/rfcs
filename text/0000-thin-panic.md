- Feature Name: thin_panic
- Start Date: 2018-01-18
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Introduce a `core::fmt`less `panic_fmt` alternative

# Motivation
[motivation]: #motivation

Currently for embedded targets `core::fmt` is a lot of program size cost that cannot be afforded. Panic is fundamental to any program in Rust however it requires use of `core::fmt`, which makes even the most basic application not feasible for some embedded platforms.

This RFC proposes a new optional `panic_fmt` implementation which only takes in a `&'static str` as a message (rather than a `core::fmt::Arguments`) without requiring changes to existing Rust internals.

This proposal focuses on the importance of getting some level of messaging in size-restricted targets, without sacrificing existing code or programming styles. To do this some corners have to be cut, but keep in mind even with cut corners it is better than nothing, especially in a world of embedded development.

# Design

## Proposed `panic_thin` prototype

This is the same as a normal `panic_fmt` lang item however it uses a `&'static str` for the `msg` argument.

```
#[lang = "panic_thin"]
#[no_mangle]
pub extern fn panic_thin(msg: &'static str,
                         file: &'static str,
                         line: u32,
                         column: u32) -> ! {}
```

## Getting static strings from existing format strings

It is not reasonable to have multiple panic messages for every panic use, thus it is important that there is backwards compatibility with old panics.

I propose that we should create a static string from the existing `fmt::Arguments` at compile time, simply concatenating any constants and filling in any dynamic argument with a placeholder (like the repr of the formatting expression eg: `"{:04x}"`).

For example something like `panic_bounds_check()` would provide a static str containing the literal string `"index out of bounds: the len is {} but the index is {}"` to `panic_thin`.

Note that this string is not ideal. It doesn't tell you the actual lengths of indexes being accessed. However currently the alternative for size-restricted targets is no message at all.

# Drawbacks
[drawbacks]: #drawbacks

- Increases options for ways to make a `#![no_std]` program to be created, feature creep
- Introduces a new lang item
- Might be a specific solution to a generic problem

# Rationale and alternatives
[alternatives]: #alternatives

This requires no changes to existing Rust codebases, the only difference is to adopt this you must implement the `panic_thin` lang item rather than `panic_fmt`. Further development is required to construct the `&'static str` representing only the constant components of a `fmt::Arguments`.

- A `get_repr()` function could be implemented on `fmt::Arguments` returning a static string of the constant components. This could then be used in a normal `panic_fmt` rather than introducing a new `panic_thin`.
    - This might be ideal if it's easy to expose

- `core::fmt` could undergo a serious rewrite focusing on code size
    - This seems like a lot of work for little reward. It's unlikely much could be reduced anyways.

- Advise size-restricted users of Rust to not use the `fmt::Arguments` argument to `panic_fmt` and build with fat LTO to remove use of the arguments.
    - This is my current workaround for my bootloader which is restricted to 32 KiB
    - This however doesn't allow access to the message in any capacity

# Unresolved questions
[unresolved]: #unresolved-questions

- How hard is it to make a static string out of a `fmt::Arguments`?
- Are small targets worth the hassle for Rust to formally support?
