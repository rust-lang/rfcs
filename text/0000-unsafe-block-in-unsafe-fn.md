- Feature Name: unsafe_block_in_unsafe_fn
- Start Date: 2018-11-04
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

No longer treat the body of an `unsafe fn` as being an `unsafe` block.  To avoid
a breaking change, this is a warning now and may become an error in a future
edition.

# Motivation
[motivation]: #motivation

Marking a function as `unsafe` is one of Rust's key protections against
undefined behavior: Even if the programmer does not read the documentation,
calling an `unsafe` function (or performing another unsafe operation) outside an
`unsafe` block will lead to a compile error, hopefully followed by reading the
documentation.

However, we currently entirely lose this protection when writing an `unsafe fn`:
If I, say, accidentally call `offset` instead of `wrapping_offset`, or if I
dereference a raw pointer thinking it is a reference, this happens without any
further notice when I am writing an `unsafe fn` because the body of an `unsafe
fn` is treated as an `unsafe` block.

For example, notice how
[this PR](https://github.com/rust-lang/rust/pull/55043/files) significantly
increased the amount of code in the thread spawning function that is considered
to be inside an `unsafe` block.

The original justification for this behavior (according to my understanding) was
that calling this function is anyway unsafe, so there is no harm done in
allowing *it* to perform unsafe operations.  And indeed the current situation
*does* provide the guarantee that a program without `unsafe` cannot be UB.
However, this neglects the other aspect of `unsafe` that I described above: To
make the programmer aware that they are treading dangerous ground even when they
may not realize they are doing so.

Using some more formal terminology, an `unsafe` block generally comes with a
proof *obligation*: The programmer has to ensure that this code is actually safe
to execute in the current context, because the compiler just trusts the
programmer to get this right.  In contrast, `unsafe fn` represents an
*assumption*: As the author of this function, I make some assumptions that I
expect my callees to uphold.  (In terms of
[introduction and elimination forms of natural deduction](https://en.wikipedia.org/wiki/Natural_deduction#Introduction_and_elimination),
`unsafe` blocks are where you introduce, and `unsafe fn` is where you
eliminate.)

Making `unsafe fn` also implicitly play the role of an `unsafe` block conflates
these two dual aspects of unsafety (one party making an assumption, another
party having the obligation to prove that assumption).  There is no reason to
believe that the assumption made by the `unsafe fn` is the same as the
obligation incurred by unsafe operations inside this function, and hence the
author of the `unsafe fn` should better carefully check that their assumptions
are sufficient to justify the unsafe operations they are performing.  This is
what an `unsafe` block would indicate.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When you perform an unsafe operation, like dereferencing a raw pointer or
calling an `unsafe` function, you must enclose that code in an `unsafe` block.
The purpose of this is to acknowledge that the operation you are performing here
has *not* been checked by the compiler, you are responsible yourself for
upholding Rust's safety guarantees.  Generally, unsafe operations come with
detailed documentation for the conditions that must be met when this operation
is executed; it is up to you to check that all these conditions are indeed met.

When you are writing a function that itself has additional conditions to ensure
safety (say, it accesses some data without making some necessary bounds checks,
or it takes some raw pointers as arguments and performs memory operations based
on them), then you should mark this as an `unsafe fn` and it is up to you to
document the conditions that must be met for the arguments.

Your `unsafe fn` will likely perform unsafe operations; these have to be
enclosed by an `unsafe` block as usual.  This is the place where you have to
check that the requirements you documented for your own function are sufficient
to satisfy the conditions required to perform this unsafe operation.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

1.  First of all, we no longer warn that an `unsafe` block is unnecessary when it is
    nested immediately inside an `unsafe fn`.  So, the following compiles without
    any warning:

    ```rust
    unsafe fn get_unchecked<T>(x: &[T], i: usize) -> &T {
      unsafe { x.get_unchecked(i) }
    }
    ```

    However, nested `unsafe` blocks are still redundant, so this warns:

    ```rust
    unsafe fn get_unchecked<T>(x: &[T], i: usize) -> &T {
      unsafe { unsafe { x.get_unchecked(i) } }
    }
    ```

2.  Optionally, we could add a clippy "correctness" lint to warn about unsafe
    operations inside an `unsafe fn`, but outside an `unsafe` block.  So, this
    would trigger the lint:

    ```rust
    unsafe fn get_unchecked<T>(x: &[T], i: usize) -> &T {
      x.get_unchecked(i)
    }
    ```

3.  In a next step, we move this lint to rustc proper, make it warn-by-default.
    This gets us into a state where programmers are much less likely to
    accidentally perform undesired unsafe operations inside `unsafe fn`.

4.  Even later (in the 2021 edition?), it might be desirable to turn this
    warning into an error.

# Drawbacks
[drawbacks]: #drawbacks

This new warning will likely fire for the vast majority of `unsafe fn` out there.

Many `unsafe fn` are actually rather short (no more than 3 lines) and will
likely end up just being one large `unsafe` block.  This change would make such
functions less ergonomic to write.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

I explained the rationale in the motivation section.

The alternative is to not do anything, and live with the current situation.

We could introduce named proof obligations (proposed by @Centril) such that the
compiler can be be told (to some extend) if the assumptions made by the `unsafe
fn` are sufficient to discharge the requirements of the unsafe operations.

# Prior art
[prior-art]: #prior-art

The only other language that I am aware of that has a notion of `unsafe` blocks
and `unsafe` functions is C#.  It
[looks like](https://docs.microsoft.com/en-us/dotnet/csharp/language-reference/keywords/unsafe)
there, unsafe operations can be freely used inside an `unsafe` function even
without a further `unsafe` block.  However, based on @Ixrec's experience,
`unsafe` plays hardly any role in the C# ecosystem and they do not have a
culture of thinking about this in terms of proof obligations.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

Should this lint be in clippy first before in becomes warn-by-default in rustc,
to avoid a huge flood of warnings showing up at once?  Should the lint ever
become a hard error (on newer editions), or remain a warning indefinitely?

Should we require `cargo fix` to be able to do *something* about this warning
before making it warn-by-default?  `cargo fix` could, for example, wrap the body
of every `unsafe fn` in one big `unsafe` block.  That would not improve the
amount of care that is taken for unsafety in the fixed code, but it would
provide a way to the incrementally improve the big functions, and new functions
written later would have the appropriate amount of care applied to them from the
start.
