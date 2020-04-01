- Feature Name: `trustme_keyword`
- Start Date: 2020-04-01
- RFC PR: N/A
- Rust Issue: N/A

# Summary
[summary]: #summary

Add a `trustme` keyword for marking `unsafe` code which guaranteed to be actually safe by the programmer.

# Motivation
[motivation]: #motivation

Although `unsafe` is a critical technique in Rust programming, the term `unsafe` seems much too visually scary and doesn't express the intention of developers well. Proper written `unsafe` code only means safety is guaranteed by the programmer, which doesn't mean the lost of safety.

The `unsafe` keyword discourages programmers to use Unsafe Rust properly and even creates hostility to `unsafe` codes. Some programmers actually developed a phenomenon called 'Unsafe PTSD (post-traumatic stress disorder)', meaning people go insane as long as they see any `unsafe` code displayed on their screens.

New concepts can be introduced into the language to ease tensions of programmers from dealing with `unsafe` code.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

`trustme` is a keyword that defines a block of code that can use all the unsafe superpowers. It's nearly identical to the `unsafe` keyword.

But a `trustme` block requires a doc comment to describe its safety concerns. Lacking of the doc comment may end up with a warning:

```rust
unsafe  { libc::srand(0); } // compiles without warning

trustme { libc::srand(0); } // compiles with warning

trustme {
    //! syscall `srand()` produces no safety problems
    libc::srand(0);
} // compiles without warning
```

`trustme` cannot used to define an `unsafe` function. Following code doesn't work:

```rust
trustme fn dangerous() { } // doesn't compile
```

Here are some suggested ways to use `trustme`:

 - Write `trustme` from the beginning if the safety of code can be obvious ensured.
 - Write an `unsafe` block in the beginning, and change it to `trustme` when the safety is guaranteed by proper procedures (code review, formal verification, etc).

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

`trustme` acts just like an alias of `unsafe`, only adds a warning and some limits to its usage.

Adding a keyword requires a new Rust edition, as it breaks any existing code that uses `trustme` as an identifier.

# Drawbacks
[drawbacks]: #drawbacks

This keyword is not really useful. In fact, leaving a note doesn't require a new keyword at all:

```rust
unsafe {
    //! syscall `srand()` produces no safety problems
    libc::srand(0);
}
```

Even if a project asks all the `unsafe` blocks to be documented, a static analysis tool can be introduced to scan the code. It's not the Rust compiler's job to enforce guidelines in a specific project.

# Future possibilities
[future-possibilities]: #future-possibilities

After all, this is not a serious RFC - see the start date. This RFC is not expected to be accepted.
