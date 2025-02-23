- Feature Name: `inline_intents`
- Start Date: 2025-02-22
- RFC PR: [rust-lang/rfcs#3778](https://github.com/rust-lang/rfcs/pull/3778)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Add `#[inline(trampoline)]` and `#[inline(rarely)]` to give additional control
over the inlining decisions made by the compiler.

# Motivation
[motivation]: #motivation

Right now it's pretty common to just slap `#[inline]` on things without thinking
about it all that hard about it, and the existing controls aren't great.

Often we'll get PRs using `inline(always)` "because it's just calling something
else so of course it should be inlined", for example.  But because of the
bottom-up nature of inlining, that's a bad thing to do because if the callee
happens to get inlined, then it'll "always" inline that callee too, which might
not be what was actually desired.

At the same time, sometimes it's useful to put `inline` on things to make the
definition available to to LLVM, but where it probably shouldn't actually be
inlined in general, only in particular special cases (perhaps when one of the
arguments is a small constant, for example).

It would thus be nice to give additional options to the user to let them
describe *why* they wanted a function inlined, and hopefully be able to make
better decisions in the backend as a result.


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

In most cases, plain `#[inline]` is fine, especially with PGO and LTO.
However, if you've measured and things are making a poor choice, there are some
options you can use to hint the compiler towards what you want.

## `inline(trampoline)`

This is intended for functions which quickly "bounce" the caller off to some
other implementation, after doing some initial checks or transformations.

For example, this is useful in a safe function which does some safety checks,
then calls an `unsafe` version of the function to do the actual work.  Or maybe
it's a function with a common trivial path, but which sometimes needs to call
out to a more complicated version, like how `Vec::push` is usually trivial but
occasionally needs to reallocate.

## `inline(rarely)`

This is intended for functions which normally shouldn't be inlined, but where
exceptions exist so you don't want full `inline(never)`.

For example, maybe this is a vectorized loop that you wouldn't want copied into
every caller, but you know that using it for short arrays is common, and thus
want the back-end to be able to inline and fully unroll it in those cases.

## In combination

These can work particularly well together.

For example, perhaps the public function is `inline(trampoline)` so that the
initial `NonZero::new` check can be inlined into the caller (where it's more
easily optimized out) but that calls a private `inline(rarely)` function which
takes `NonZero` to avoid needing extra checks internally.

Or perhaps you write an `inline(trampoline)` function that picks a strategy
based on the types and arguments, then dispatches to one of many possible
`inline(rarely)` implementations.


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Like `inline`, these "do nothing" in a spec sense.

The only language change is thus in allowing the two new tokens in the attribute
(in addition to `never` and `always`).

## Implementation options

⚠ These are not normative, just to illustrate possibilities. ⚠

In LLVM, `#[inline]` sets the [`inlinehint` function attribute](https://llvm.org/docs/LangRef.html#function-attributes),
so `inline(rarely)` could skip doing that, and thus comparatively slightly
discourage inlining it.

In MIR inlining, we already attempt to deduce trampolines as of [#127113].
This would let people opt-in to that behaviour, even in places it's not obvious.
Then we could allow inlining trampolines into trampolines, but avoid inlining
non-trampolines into a trampoline.  And we have an internal attribute
`rustc_no_mir_inline` which blocks MIR inlining, so `inline(rarely)` could also
do that (or maybe just make the threshold very restrictive).

[#127113]: https://github.com/rust-lang/rust/pull/127113


# Drawbacks
[drawbacks]: #drawbacks

These are still up to the programmer to get right, so
- they might just make analysis paralysis worse
- they might be worse than having something like a PGO-based system instead
- they might turn out to not actually help as hoped
- they might lead to more bugs in the tracker when they don't do what people thought
- they might be the wrong pivots and we'll just end up needing more

However, at worst we just make them allowed but not do anything, so at worst the
cost of having them is very small: a compiler just parses-then-ignores them.


# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The hope here is that by giving *intent* we can tune how these work in a better
way than is possible with the existing knobs.

The existing options are insufficient because `always` and `never` are too strong.
With LLVM's bottom-up inlining logic, `always` isn't the right answer the vast
majority of the time, and if LLVM really doesn't want to inline it, most of the
time there's a good reason for that.  Similarly if it really thinks that
something would be good to inline, the majority of the time blocking that with
`never` isn't really want you want.

For example, the library wants to *discourage* inlining of the UB check
functions, but doesn't want `never` because if all the arguments are constants
inlining it to const-fold away all the checks is good.  (It'd be silly to force
`NonNull::new(ptr::without_provenance(2))` to have an actual call to
`precondition_check(2_ptr)`.)

And the blanket impl of `Into::into` that just calls `From::from` wants to
"always" be inlined, but because of how inlining works it'd be wrong for it to
use `always` since it doesn't necessarily want to inline the whole subtree.

Thus today usually all that one can say is `#[inline]`, and hope that the
compiler does the right thing.  Often it does, but sometimes it also makes poor
decisions that can result in binary bloat or poor performance.  With these to
nudge it one way or the other, hopefully that will avoid the downsides of the
existing really big hammers, while also not over-promising for what will always
just be a hint, not a promise.

`inline(rarely)` could instead be done with something in the body to increase
the cost, perhaps marking it `#[inline]` but then calling a hypothetical magic
`core::hint::discourage_inlining()`.  But to make it an inlining candidate at
all means it still needs the attribute, so it seems nicer to let it just be
mentioned in the attribute without needing to inspect the body, given that
the attribute is already parameterized.


# Prior art
[prior-art]: #prior-art

Various languages have inlining controls, but I'm unaware of any with either of
these specific intents.

[LLVM](https://llvm.org/docs/LangRef.html#function-attributes)
has `alwaysinline` vs `inlinehint` vs `noduplicate` vs `noinline`.

[GCC](https://gcc.gnu.org/onlinedocs/gcc/Inline.html)
has `inline` and `__attribute__((always_inline))` and `extern inline`.

[DotNet](https://learn.microsoft.com/en-us/dotnet/api/system.runtime.compilerservices.methodimploptions)
has `NoInlining` vs `AggressiveInlining` hints.


# Unresolved questions
[unresolved-questions]: #unresolved-questions

The exact behaviour of how these affect program compilation will likely continue
to be tweaked even after they stabilize, assuming this were to be accepted.

The goal of the RFC process is to pick tokens that are sufficiently evocative
of what intent the coder is expressing by using them; the details happen later.


# Future possibilities
[future-possibilities]: #future-possibilities

There are always more possible intents that one could imagine.  For example, one
for "this is small and uninteresting" could make sense, which could even have
more non-semantic effects like emitting less debug info.  But for now it's less
obvious that that's worth distinguishing, since just things being small (like
basic accessor functions often are) is enough to already trigger making it
cross-crate inlinable even without the attribute, and things that are small
and simple are reliably inlined without extra hinting anyway.

