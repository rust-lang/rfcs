- Feature Name: `next_gen_transmute`
- Start Date: 2025-08-01
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Change `mem::transmute` from having a magic size check to having an ordinary
`const { … }`-enforced check plus some normal lints.

Add a `mem::union_transmute` for an even-less-restricted transmute where size
mismatches are both allowed and not (necessarily) UB.


# Motivation
[motivation]: #motivation

From [#106281](https://github.com/rust-lang/rust/pull/106281#issuecomment-1496648190):

> Many valid (but not provably so) transmute calls are currently rejected by the compiler's checks, pushing folks to less ergonomic options like transmute_copy or pointer casts.

Today, the one-liner when the compiler doesn't let you `transmute` is to instead do

```rust
mem::transmute_copy(&mem::ManuallyDrop(other))
```

But that's not great.  It doesn't communicate that the programmer *expected* the size
to match, and thus there's no opportunity for the compiler to help catch a mistaken
expectation.  Plus it obfuscates other locations that really do want `transmute_copy`,
perhaps because they're intentionally reading a prefix out of something.

It's also a safety footgun because it'll *compile* if you instead were to write
```rust
mem::transmute_copy(&other)
```
but is highly likely to result in use-after-free UB.

It would be nice to move `mem::transmute` to being a normal function -- not the one
intrinsic we let people call directly -- in a way that it can be more flexible for
users as well as easier to update in the compiler without semver worries.


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## `union_transmute`

The `union_transmute` function is a general way to reinterpret the byte representation
of one type as a different type.  This is equivalent to writing and reading through
a `union`, like

```rust
const unsafe fn union_transmute<T, U>(t: T) -> U {
    #[repr(C)]
    union Transmute<A, B> {
        a: ManuallyDrop<A>,
        b: ManuallyDrop<B>,
    }
    let u = unsafe {
        Transmute { a: ManuallyDrop::new(t) }.b
    };
    ManuallyDrop::into_inner(u)
}
```

or to copying over the common prefix length like

```rust
const unsafe fn union_transmute<T, U>(t: T) -> U {
    let mut u = MaybeUninit::<U>::uninit();
    unsafe {
        let bytes = Ord::min(size_of::<T>(), size_of::<U>());
        ptr::copy_nonoverlapping::<u8>((&raw const t).cast(), u.as_mut_ptr().cast(), bytes);
        u.assume_init()
    }
}
```

You might have heard this referred to as "type punning" or similar as well.
It's also a very similar operation to what you can get by casting pointers,
though because it's by *value* you don't need to worry about the alignments of
`T` and `U` the way you would when doing something like
`(&raw const x).cast::<U>().read()`.

This is incredibly `unsafe` because nearly all combinations of types are going
to be immediately UB.  For example, `union_transmute::<u32, u64>(x)` is always
UB because half of the value being read is uninitialized.

However, it's still useful to have this available as the fully-general operation
for those cases where it's useful.  For example, it's sound to use it for
`union_transmute::<[T; BIG], [T; SMALL]>(…)` to read a prefix of an array.
Or a SIMD type like `#[repr(C, align(16))] struct AlignedF32x3(Simd<f32, 3>);`
can (at least based on the current plans for `Simd`) soundly then `union_transmute`
back and forth between `AlignedF32x3` and `[f32; 3]`, despite their different sizes.

And of course some things are trivially sound, like `union_transmute::<T, ()>`
as that would read zero bytes, which of course works.  (There's no need to use
`union_transmute` for that, though, since it's better spelled `mem::forget`.)

## `transmute`

The `transmute` function does the same thing as `union_transmute` when it compiles,
but adds the restriction that the input and output types must have the same size.

It's essentially this:
```rust
const unsafe fn transmute<T, U>(t: T) -> U {
    const { assert!(size_of::<T>() == size_of::<U>()) };
    union_transmute<T, U>(t)
}
```

This has its own name because it's particularly common that when transmuting
you're *expecting* the two types to be the same size, and it's helpful both to
communicate that to the reader and let the compiler help double-check it.

For example, `transmute::<[u32; N], [u64; M]>` is only going to be sound when
the sizes match (aka when M = 2×N), so might as well have that checked at
compile-time instead of potentially letting something unsound sneak in.

Using a const-assert this way does mean that some calls to `transmute` that can
never actually work will not be rejected at declaration time, only sometime later
when the function in question is actually used by something else.

To mitigate that, there's a number of lints:

- `deny`-by-default lints for things where the compiler knows the sizes are
  different, such as `transmute::<u32, u64>`.
- `warn`-by-default lints for things where it's *possible* that the size will
  match, but it's still suspicious, such as `transmute::<[u32; N], u32>` or
  `transmute::<[u64; N], [u32; N]>` where only one monomorphization can work.

The full complement of such lints is not listed here as they're regularly updated
to catch more definitely-wrong cases and be smart enough to prove more things
as *not* being suspicious.

> 📜 Historical Note 📜
>
> In previous versions of rust, `transmute` was actually a hard error when the
> compiler couldn't *prove* that the types were the same size.  This was limiting
> in practice, as humans are smarter than the rules we're willing to run during
> type checking -- this is why `unsafe` code exists at all, really -- and meant
> that people needed workarounds.
>
> All those cases that were previously caught produce lints instead, now, with
> the possible exception of things that were errors before only from the compiler
> being insufficiently smart.  For example, `transmute::<[[u32; N]; 2], [u64; N]>`
> was previously rejected despite those two types always having the same size
> for any monomorphization, so it might not lint now.


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Because lint details are non-normative, in some sense the implementation is trivial.
Just add `union_transmute` and change `transmute` as above in `core`.

## Possible implementation approach

Today we already have a `transmute_unchecked` intrinsic in rustc which doesn't
have a compile-time size check, but *is* still documented as UB if the sizes
don't match.  That intrinsic can be changed to be defined as the union version,
perhaps with fallback MIR using that exact definition, and used to implement
both of the library functions.  Those library functions would probably also
add some UbChecks (which aren't possible today as `mem::transmute` is a
re-exported intrinsic, not an actual function).

The change to make `mem::transmute` an ordinary function would need to update
the existing `check_transmutes` in typeck to instead be a lint that looks for
calls to `#[rustc_diagnostic_item = "transmute"]` instead.  (That diagnostic
item already exists.)  For starters the same logic could be used as a
deny-by-default lint, as the most similar diagnostics, with any splitting done
separately over time at the discretion of the diagnostics experts.

This should be straight-forward in CTFE and codegen as well.  Once lowered such
that we have locals for the source and destination, this can be implemented by
just copying over the number of bytes in the shorter value (plus uninitializing
the upper bytes in the target, if it's bigger).

For example, in cg_clif the general-case copy currently uses the destination size
<https://github.com/rust-lang/rust/blob/6d091b2baa33698682453c7bb72809554204e434/compiler/rustc_codegen_cranelift/src/value_and_place.rs#L641>
but it could use the min of the source and destination size.

In cg_ssa the general case for SSA values actually already supports this
<https://github.com/rust-lang/rust/blob/6d091b2baa33698682453c7bb72809554204e434/compiler/rustc_codegen_ssa/src/mir/rvalue.rs#L309-L316>
and thus would just need the earlier "just emit `unreachable` if the sizes don't
match" check removed to reach it.

There are some other cases that will need more care, like transmuting a larger
`BackendRepr::Scalar` to a smaller `BackendRepr::Memory` where the current code
would do a OOB write if unchanged, but nothing particularly troublesome is expected.

The internal changes (to codegen and similar) would probably happen first so they
could be implemented and tested before doing the publicly-visible switchover.


# Drawbacks
[drawbacks]: #drawbacks

- The more transmute-related functions we add the more people might feel encouraged
  to use them, even if we'd rather not.
- Lots of people don't like post-mono errors, and would rather Rust never have them.
- This is still massively-unsound, so doesn't solve the biggest problems.
- Weird crimes using transmute to check sizes without ever actually running the
  transmute might not get caught by the linting or post-mono check.


# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Aren't the hard errors better than post-mono ones, on transmute?

Well, there's two big reasons to prefer post-mono here:

1. By being post-mono, it eliminates all "the compiler isn't smart enough" cases.
   If you get an error from it, then the two types are *definitely* of different
   sizes, *period*.  If you find a way to encode Fermat's Last Theorem in the
   type system, it's ok, the compiler doesn't have to know how to prove it to let
   you do the transmute.  It would be *nice* if we could be that accurate earlier
   in the compilation pipeline, but for anything layout-based that's extremely
   difficult -- especially for futures.  There's still the potential for "false"
   warnings in code that's only conditionally run, but that's also true of trait
   checks, and is thus left for a different RFC to think about.
2. By being *hard* errors, rather than lints, there's a bunch more breaking change
   concerns.  Any added smarts that allow something to compile need to be around
   *forever* (as removing them would be breaking), and similarly the exact details
   of those checks need to be updated in the specification.  Those changes also
   impact the MSRV of libraries that depend on them.  Whereas putting those smarts
   into *lints* instead mean that cap-lints applies and we can detect new issues
   without worrying about back-compat.  And the details of lints don't need to be
   written down in the specification either.

## Do we really need the `union_transmute`?

Not strictly, no.  We could continue to say that the only "official" kind of
transmute is one where the sizes are definitely equal at runtime.

That said, the SIMD case was most persuasive to the RFC author.  If we chose to
offer an `AlignedSimd<T, N>` that rounds up to some implementation-chosen multiple
in order to offer more alignment (than a `PackedSimd<T, N>` would), it would be
quite convenient to have a name and primitive operation for the kind of transmute
that would work for both directions of `[T; N]` ⇄ `AlignedSimd<T, N>`.

Plus using `union`s for type punning like this is something that people already
do, so having a name for it helps make what's happening more obvious, plus gives
a place for us to provider better documentation and linting when they do use it.

## Why the name `union_transmute`?

The idea is to lean on the fact that Rust already has `union` as a user-visible
concept, since what this does is *exactly* the same as using an
all-fields-at-the-same-offset `union` to reinterpret the representation.
Similarly, a common way to do this operation in C is to use a union, so people
coming from other languages will recognize it.

Thinking about the `union` hopefully also give people the right intuition about
the requirements that this has, especially in comparison to what the requirements
would be if this had the pointer-cast semantics.  Hopefully seeing the union in
the name helps them *not* think that it's just `(&raw const x).cast().read()`.

There's currently (as an implementation detail) a `transmute_unchecked` intrinsic
in rustc which doesn't have the typeck-time size check, but I leaned away from
that name because it's unprecedented, to my knowledge, to have a `foo_unchecked`
in the stable library where `foo` is *also* an `unsafe fn`.

If we were in a world where `mem::transmute` was actually a *safe* function,
then `transmute_unchecked` for this union-semantic version would make sense,
but we don't currently have such a thing.

## Could we keep the compile-time checks on old editions?

This RFC is written assuming that we'll be able to remove the type-checking-time
special behaviour entirely.  That does mean that some things that used to fail
will start to compile, and it's possible that people were writing code depending
on that kind of trickery to enforce invariants.

However, there's never been a guarantee about what exactly those checks enforce,
and in general we're always allowed to make previously-no-compiling things start
to compile in new versions -- as has happened before with the check getting
smarter.  We're likely fine saying that such approaches were never endorsed and
thus that libraries should move to other mechanisms to check sizes, as
[some ecosystem crates](https://github.com/Lokathor/bytemuck/pull/320) have
already started to do.

If for some reason that's not ok, we could consider approaches like
edition-specific name resolution to have `mem::transmute` on edition ≤ 2024
continue to get the typeck hacks for this, but on future editions resolve to
the version using the interior const-assert instead.

## Is transmuting to something bigger ever *not* UB?

As a simple case, if you have

```rust
#[repr(C, align(4))]
struct AlignedByte(u8);
```

then `union_transmute::<u8, AlignedByte>` and `union_transmute::<AlignedByte, u8>`
are in fact *both* always sound, despite the sizes never matching.

You can easily make other similar examples using `repr(packed)` as well.


# Prior art
[prior-art]: #prior-art

C++ has `reinterpret_cast` which sounds like it'd be similar, but which isn't
defined for aggregates, just between integers and pointers or between pointers
and other pointers.

GCC has a cast-to-union extension, but it only goes from a value to a `union`
with a field of matching type, and doesn't include the part of going from the
`union` back to a different field.


# Unresolved questions
[unresolved-questions]: #unresolved-questions

During implementation:
- Should MIR's `CastKind::Transmute` retain its equal-size precondition?
- What name should the new function get?

For nightly and continuing after stabilization:
- What exactly are the correct lints to have about these functions?


# Future possibilities
[future-possibilities]: #future-possibilities

Nothing new foreseen.  Hopefully the safe-transmute project will continue to
make progress and help people use `mem::(union_)transmute` less going forward.

