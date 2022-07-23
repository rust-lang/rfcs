- Feature Name: `unsafe_block_in_unsafe_fn`
- Start Date: 2018-11-04
- RFC PR: [rust-lang/rfcs#2585](https://github.com/rust-lang/rfcs/pull/2585)
- Rust Issue: [rust-lang/rust#71688](https://github.com/rust-lang/rust/issues/71668)

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

In fact, this double role of `unsafe` in `unsafe fn` (making it both unsafe to
call and enabling it to call other unsafe operations) conflates the two *dual*
roles that `unsafe` plays in Rust.  On the one hand, there are places that
*define* a proof obligation, these make things "unsafe to call/do" (e.g., the
language definition says that dereferencing a raw pointer requires it not to be
dangling).  On the other hand, there are places that *discharge* the proof
obligation, these are "unsafe blocks of code" (e.g., unsafe code that
dereferences a raw pointer has to locally argue why it cannot be dangling).

`unsafe {}` blocks are about *discharging* obligations, but `unsafe fn` are
about *defining* obligations.  The fact that the body of an `unsafe fn` is also
implicitly treated like a block has made it hard to realize this duality
[even for experienced Rust developers][unsafe-dual].  (Completing the picture,
`unsafe Trait` also defines an obligation, that is discharged by `unsafe impl`.
Curiously, `unsafe trait` does *not* implicitly make all bodies of default
functions defined inside this trait unsafe blocks, which is somewhat
inconsistent with `unsafe fn` when viewed through this lens.)

[unsafe-dual]: https://github.com/rust-lang/rfcs/pull/2585#issuecomment-577852430

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The `unsafe` keyword in Rust serves two related purposes.

When you perform an "unsafe to call" operation, like dereferencing a raw pointer
or calling an `unsafe fn`, you must enclose that code in an `unsafe {}` block.
The purpose of this is to acknowledge that the operation you are performing here
has *not* been checked by the compiler, you are responsible yourself for
upholding Rust's safety guarantees.  Generally, unsafe operations come with
detailed documentation for the conditions that must be met when this operation
is executed; it is up to you to check that all these conditions are indeed met.

When you are writing a function that itself has additional conditions to ensure
safety (say, it accesses some data without making some necessary bounds checks,
or it takes some raw pointers as arguments and performs memory operations based
on them), then you should mark this as an `unsafe fn` and it is up to you to
document the conditions that must be met for the arguments.  This use of the
`unsafe` keyword makes your function itself "unsafe to call".

The same duality can be observed in traits: `unsafe trait` is like `unsafe fn`;
it makes implementing this trait an "unsafe to call" operation and it is up to
whoever defines the trait to precisely document what is unsafe about it.
`unsafe impl` is like `unsafe {}`, it acknowledges that there are extra
requirements here that are not checked by the compiler and that the programmer
is responsible to uphold.

For this reason, "unsafe to call" operations inside an `unsafe fn` must be
contained inside an `unsafe {}` block like everywhere else.  The author of these
functions has to ensure that the requirements of the operation are upheld.  To
this end, the author may of course assume that the caller of the `unsafe fn` in
turn uphold their own requirements.

For backwards compatibility reasons, this unsafety check inside `unsafe fn` is
controlled by a lint, `unsafe_op_in_unsafe_fn`.  By setting
`#[deny(unsafe_op_in_unsafe_fn)]`, the compiler is as strict about unsafe
operations inside `unsafe fn` as it is everywhere else.

This lint is allow-by-default initially, and will be warn-by-default across all
editions eventually.  In future editions, it may become deny-by-default, or even
a hard error.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The new `unsafe_op_in_unsafe_fn` lint triggers when an unsafe operation is used
inside an `unsafe fn` but outside `unsafe {}` blocks.  So, the following will
emit a warning:

```rust
#[warn(unsafe_op_in_unsafe_fn)]
unsafe fn get_unchecked<T>(x: &[T], i: usize) -> &T {
  x.get_unchecked(i)
}
```

Moreover, if and only if the `unsafe_op_in_unsafe_fn` lint is not `allow`ed, we
no longer warn that an `unsafe` block is unnecessary when it is nested
immediately inside an `unsafe fn`.  So, the following compiles without any
warning:

```rust
#[warn(unsafe_op_in_unsafe_fn)]
unsafe fn get_unchecked<T>(x: &[T], i: usize) -> &T {
  unsafe { x.get_unchecked(i) }
}
```

However, nested `unsafe` blocks are still redundant, so this warns:

```rust
#[warn(unsafe_op_in_unsafe_fn)]
unsafe fn get_unchecked<T>(x: &[T], i: usize) -> &T {
  unsafe { unsafe { x.get_unchecked(i) } }
}
```

# Drawbacks
[drawbacks]: #drawbacks

Many `unsafe fn` are actually rather short (no more than 3 lines) and will end
up just being one large `unsafe` block.  This change would make such functions
less ergonomic to write, they would likely become

```rust
unsafe fn foo(...) -> ... { unsafe {
  // Code goes here
} }
```

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

To achieve the goals laid out in the motivation section, the proposed approach
is least invasive in the sense that it avoids introducing new keywords, and
instead relies on the existing lint mechanism to perform the transition.

One alternative always is to not do anything, and live with the current
situation.

We could avoid using `unsafe` for dual purpose, and instead have `unsafe_to_call
fn` for functions that are "unsafe to call" but do not implicitly have an
`unsafe {}` block in their body.  For consistency, we might want `unsafe_to_impl
trait` for traits, though the behavior would be the same as `unsafe trait`.

We could introduce named proof obligations (proposed by @Centril) such that the
compiler can be be told (to some extend) if the assumptions made by the `unsafe
fn` are sufficient to discharge the requirements of the unsafe operations.

We could restrict this requirement to use `unsafe` blocks in `unsafe fn` to
those `unsafe fn` that contain at least one `unsafe` block, meaning short
`unsafe fn` would keep compiling like they do now.

And of course, the lint name is subject to bikeshedding.

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

What is the timeline for adding the lint, and cranking up its default level?
Should the default level depend on the edition?

Should we ever make this deny-by-default or even a hard error, in a future
edition?

Should we require `cargo fix` to be able to do *something* about this warning
before making it even warn-by-default?  (We certainly need to do something
before making it deny-by-default or a hard error in a future edition.)  `cargo
fix` could add big `unsafe {}` blocks around the entire body of every `unsafe
fn`.  That would not improve the amount of care that is taken for unsafety in
the fixed code, but it would provide a way to the incrementally improve the big
functions, and new functions written later would have the appropriate amount of
care applied to them from the start.  Potentially, `rustfmt` could be taught to
format `unsafe` blocks that wrap the entire function body in a way that avoids
double-indent.  "function bodies as expressions" would enable a format like
`unsafe fn foo() = unsafe { body }`.

It is not entirely clear if having the behavior of one lint depend on another
will work (though most likely, it will).  If it does not, we should try to find
some other mechanism to opt-in to the new treatment of `unsafe fn` bodies.
