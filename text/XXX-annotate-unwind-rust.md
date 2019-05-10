- Feature Name: (fill me in with a unique ident, `my_awesome_feature`)
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: ???
- Rust Issue: ???

# Summary
[summary]: #summary

A new function annotation, `#[unwind(Rust)]`, will be introduced; it will
explicitly permit `extern` functions to unwind (`panic`) without aborting. No
guarantees are made regarding the specific unwinding implementation.

# Motivation
[motivation]: #motivation

This will enable resolving
[rust-lang/rust#58794](https://github.com/rust-lang/rust/issues/58794) without
breaking existing code.

Currently, unwinding through an FFI boundary is always undefined behavior. We
would like to make the behavior safe by aborting when the stack is unwound to
an FFI boundary. However, there are existing Rust crates (notably, wrappers
around the `libpng` and `libjpeg` C libraries) that make use of the current
implementation's behavior. The proposed annotation would act as an "opt out" of
the safety guarantee provided by aborting at an FFI boundary.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Consider an `extern "C"` function that may `panic`:

```rust
extern "C" fn may_panic(i: i32) {
    if i < 0 {
        panic!("Oops, I should have used u32.");
    }
}
```

In a future (TBD) version of Rust, calling this function with an argument less
than zero will cause the program to be aborted. This is the only way the Rust
compiler can ensure that the function is safe, because there is no way for it
to know whether or not the calling code supports the same implementation of
stack-unwinding used for Rust.

As a concrete example, Rust code marked `exern "C"` may be invoked from C code,
but C code on Linux or BSD may be compiled without support for native
stack-unwinding. This means that the runtime would lack the necessary metadata
to properly perform the unwinding operation.

However, there are cases in which a `panic` through an `extern` function can be
used safely. For instance, it is possible to invoke `extern` Rust functions
via Rust code built with the same toolchain, in which case it would be
irrelevant to the unwinding operation that the `panic`ing function is an
`extern` function:

```rust
fn main() {
    let result = panic::catch_unwind(|| {
        may_panic(-1);
    }
    assert!(result.is_err());
}
```

In order to ensure that `may_panic` will not simply abort in a future version
of Rust, it must be marked `#[unwind(Rust)]`. This annotation can only be used
with an `unsafe` function, since the compiler is unable to make guarantees
about the behavior of the caller.

```rust
#[unwind(Rust)]
unsafe extern "C" fn may_panic(i: i32) {
    if i < 0 {
        panic!("Oops, I should have used u32.");
    }
}
```

**PLEASE NOTE:** Using this annotation **does not** provide any guarantees
about the unwinding implementation. Therefore, using this feature to compile
functions intended to be called by C code **does not make the behavior
well-defined**; in particular, **the behavior may change in a future version of
Rust.**

The *only* well-defined behavior specified by this annotation is Rust-to-Rust
unwinding.

It is safe to call such a function from C or C++ code only if that code is
guaranteed to provide the same unwinding implementation as the Rust compiler
used to compile the function.

Since the behavior may be subject to change without notice as the Rust compiler
is updated, it is recommended that all projects that rely on unwinding from
Rust code into C code lock the project's `rustc` version and only update it
after ensuring that the behavior will remain correct.

<!-- TODO: below here is still the template -->

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This is the technical portion of the RFC. Explain the design in sufficient detail that:

- Its interaction with other features is clear.
- It is reasonably clear how the feature would be implemented.
- Corner cases are dissected by example.

The section should return to the examples given in the previous section, and explain more fully how the detailed proposal makes those examples work.

# Drawbacks
[drawbacks]: #drawbacks

Why should we *not* do this?

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- Why is this design the best in the space of possible designs?
- What other designs have been considered and what is the rationale for not choosing them?
- What is the impact of not doing this?

# Prior art
[prior-art]: #prior-art

Discuss prior art, both the good and the bad, in relation to this proposal.
A few examples of what this can include are:

- For language, library, cargo, tools, and compiler proposals: Does this feature exist in other programming languages and what experience have their community had?
- For community proposals: Is this done by some other community and what were their experiences with it?
- For other teams: What lessons can we learn from what other communities have done here?
- Papers: Are there any published papers or great posts that discuss this? If you have some relevant papers to refer to, this can serve as a more detailed theoretical background.

This section is intended to encourage you as an author to think about the lessons from other languages, provide readers of your RFC with a fuller picture.
If there is no prior art, that is fine - your ideas are interesting to us whether they are brand new or if it is an adaptation from other languages.

Note that while precedent set by other languages is some motivation, it does not on its own motivate an RFC.
Please also take into consideration that rust sometimes intentionally diverges from common language features.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- What parts of the design do you expect to resolve through the RFC process before this gets merged?
- What parts of the design do you expect to resolve through the implementation of this feature before stabilization?
- What related issues do you consider out of scope for this RFC that could be addressed in the future independently of the solution that comes out of this RFC?

# Future possibilities
[future-possibilities]: #future-possibilities

Think about what the natural extension and evolution of your proposal would
be and how it would affect the language and project as a whole in a holistic
way. Try to use this section as a tool to more fully consider all possible
interactions with the project and language in your proposal.
Also consider how the this all fits into the roadmap for the project
and of the relevant sub-team.

This is also a good place to "dump ideas", if they are out of scope for the
RFC you are writing but otherwise related.

If you have tried and cannot think of any future possibilities,
you may simply state that you cannot think of anything.

Note that having something written down in the future-possibilities section
is not a reason to accept the current or a future RFC; such notes should be
in the section on motivation or rationale in this or subsequent RFCs.
The section merely provides additional information.

