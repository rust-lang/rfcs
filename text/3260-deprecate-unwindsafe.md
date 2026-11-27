- Feature Name: deprecate_unwind_safe
- Start Date: 2022-01-17
- RFC PR: [rust-lang/rfcs#3260](https://github.com/rust-lang/rfcs/pull/3260)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Currently rust has the [UnwindSafe](https://doc.rust-lang.org/std/panic/trait.UnwindSafe.html#) and [RefUnwindSafe](https://doc.rust-lang.org/core/panic/trait.RefUnwindSafe.html#) marker traits. This RFC proposes to deprecate them, and remove the `F: UnwindSafe` bound on [catch_unwind](https://doc.rust-lang.org/std/panic/fn.catch_unwind.html#).

# Motivation
[motivation]: #motivation

Unwind safety is not actually related to safety. It acts as a lint. [AssertUnwindSafe](https://doc.rust-lang.org/std/panic/struct.AssertUnwindSafe.html#) can be used to ignore it, and using it does not require unsafe. If using it results in undefined behaviour or unsoundness, the problem lies elsewhere. The existence of unwind safety makes it seem as if you can rely on it for soundness, which is not true (See discussion in [UnwindSafe docs are unclear](https://github.com/rust-lang/rust/issues/65717).)

It can also be problematic when a type does not implement the marker trait, but it could, notably with trait objects (See discussion in [`UnwindSafe` is unergonomic](https://github.com/rust-lang/rust/issues/40628)). It can also be a pain point for library authors, who are not sure if they should add a bound on them for their generic types to guarantee their types are UnwindSafe, which would make their downstream users sometimes have to use AssertUnwindSafe despite not using catch_unwind just to satisfy the bounds.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

UnwindSafe and RefUnwindSafe are deprecated, and you never need to use them. If you can cause undefined behaviour with catch_unwind, something else is unsound.

The following now compiles:
```rs
    let x = std::cell::UnsafeCell::new(1u8);
    let result = std::panic::catch_unwind(|| {
        println!("{:p}", x.get());
        panic!()
    });
```
Which used to require AssertUnwindSafe:
```rs
    let x = std::panic::AssertUnwindSafe(std::cell::UnsafeCell::new(1u8));
    let result = std::panic::catch_unwind(|| {
        println!("{:p}", x.get());
        panic!()
    });
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

UnwindSafe and RefUnwindSafe are now deprecated, and the UnwindSafe bound on the F generic parameter of catch_unwind is removed.

# Drawbacks
[drawbacks]: #drawbacks

We lose any value that UnwindSafe was actually providing as a lint.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

 - We could keep UnwindSafe as-is without deprecating it.
 - Rename UnwindSafe to something that does not mention "safety".
 - We could make using something !UnwindSafe through catch_unwind a warning via language magic instead of completely removing it. This would probably require a fundamentally new feature of trait resolution, to turn a missing trait implementation into a warning.

# Prior art
[prior-art]: #prior-art

In the pull request where UnwindSafe was moved to core, it was mentioned the libs team may want to deprecate it https://github.com/rust-lang/rust/pull/84662#issuecomment-840010967

I found a comment in this issue mentioning deprecation as far back as 2019: https://github.com/rust-lang/rust/issues/40628#issuecomment-549050573

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- How will this impact the ecosystem? How will libraries with an MSRV deal with this?

# Future possibilities
[future-possibilities]: #future-possibilities

