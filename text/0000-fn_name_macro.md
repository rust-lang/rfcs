- Feature Name: fn_name_macro
- Start Date: 2019-11-14
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

<!-- https://github.com/rust-lang/rfcs/issues/1743 -->

# Summary
[summary]: #summary

This RFC adds an additional macro, `function!`, to the `core` crate. When invoked, the macro expands to the name of the function that contains the call site.

# Motivation
[motivation]: #motivation

This is a useful extension of Rust's existing debug reporting: `file!`, `line!`, `column!` and `module_path!`.

For many people, the name of the function is a much more immediate way to understand the context of a message than file name and line number. Often when discussing a log message or panic message with others (in a chat, forum, github issue, etc) the source code might not be immediately available.

As an example, it's a lot easier to understand what's going on when a message comes from "f32::floor" instead of from "src/libcore/f32.rs:45".

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

For debug information about what's happening within the program there are several useful macros that you might use. One of them is the `function!` macro, which expands to the name of the current function.

```rust
// this prints "entering my_func" before doing the work
fn my_func() {
  println!("entering {}", function!());
  // work here
}
```

If used outside of a function it causes a compilation error.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Use of the `function!` macro expands to the compiler's internal name for the function. This will generally be the name that the user wrote into the file but in the case of a closure or similar it will be something like the function's name with a unique suffix.

The exact text is considered "debug" information and not subject to Rust's stability guarantee.

# Drawbacks
[drawbacks]: #drawbacks

* Doing this adds another macro to `core`.
* This macro in particular cannot be implemented in Rust itself, it requires special support from `rustc`, as well as from all potential alternative compilers for the Rust language.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Previous discussion was within an issue of the RFCs repo: https://github.com/rust-lang/rfcs/issues/1743

In summary, this is useful for debug purposes, and has been desired for nearly two years.

Alternative: we could call it `fn_name!` instead.

Alternative (centril): We could introduce a much more general `item_path!()` macro that is useful in non-function contexts.

```rust
mod foo { // item_path!() ==> foo
    const X: &str = item_path!(); // foo::X
    struct A;
    impl A {
        const Y: &str = item_path!(); // foo::A::Y
        fn fun() {
            // item_path!() ==> foo::A::fun
        }
    }
     type T = Stuff<{item_path!()}> // Stuff<"foo::T">
}
```

# Prior art
[prior-art]: #prior-art

* C99 has a `__func__` pre-processor macro that expands to the current funciton name. [link](http://www.open-std.org/jtc1/sc22/wg21/docs/papers/2004/n1642.html)

# Unresolved questions
[unresolved-questions]: #unresolved-questions

* Do type methods and type associated functions include their type in the printed name? eg: `Vec::new`

# Future possibilities
[future-possibilities]: #future-possibilities

None that are specifically improvements of this macro.

We could of course add additional debugging macros.
