- Feature Name: `direct_enum_discriminant`
- Start Date: 2024-03-16
- RFC PR: [rust-lang/rfcs#3607](https://github.com/rust-lang/rfcs/pull/3607)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Enable using **`.enum#discriminant`** on values of enum type from safe code in the same module
to get the numeric value of the variant's discriminant in the numeric type of its `repr`.

# Motivation
[motivation]: #motivation

Today in Rust you can use `as` casts on *field-less* `enum`s to get their discriminants,
but as soon as any variant has fields, that's no longer available.

Rust 1.66 stabilized custom discriminants on variants with fields, but as
[the release post][rust 1.66 blog] said,

> Rust provides no language-level way to access the raw discriminant of an enum with fields.
> Instead, currently unsafe code must be used to inspect the discriminant of an enum with fields.

[rust 1.66 blog]: https://blog.rust-lang.org/2022/12/15/Rust-1.66.0.html#explicit-discriminants-on-enums-with-fields

As a result, the [documentation for `mem::Discriminant`][discriminant docs] has a section
about how to write that `unsafe` code, and a bunch of warnings about the different
*incorrect* ways that must not be used.

[discriminant docs]: https://doc.rust-lang.org/std/mem/fn.discriminant.html#accessing-the-numeric-value-of-the-discriminant

It's technically [possible](https://github.com/rust-lang/rust/pull/106418#issuecomment-1700399884)
to write a clever enough safe `match` that compiles down to a no-op in order to get at the discriminant,
but doing so is annoying and fragile.

And accessing the discriminant is quite useful in various places, so it'd be nice for it to be easy.

For example, `#[derive(PartialOrd)]` on an `enum` today uses internal compiler magic to look at discriminants.
It would be nice for other derives in the ecosystem -- there's a whole bunch of things on `enum`s --
to be able to look at the discriminants directly too.

With this RFC, the built-in derives and third-party derives can both use the same stable feature
to implement `PartialOrd::parial_cmp` for the cases where the arguments have different discriminants.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

[Rust 1.66][rust 1.66 blog] stabilized custom discriminants on enum variants,
but didn't give a nice way to actually read them.

In this release, you can use **`.enum#discriminant`** to read them.

For example, if you have the following enum,

```rust
#[repr(u8)]
enum Enum {
    Unit = 7,
    Tuple(bool) = 13,
    Struct { a: i8 } = 42,
}
```

Then the following examples pass:

```rust
let a = Enum::Unit;
assert_eq!(a.enum#discriminant, 7);
let b = Enum::Tuple(true);
assert_eq!(b.enum#discriminant, 13);
let c = Enum::Struct { a: 1 };
assert_eq!(c.enum#discriminant, 42);
```

That's entirely safe code, and the value comes out as the type from the `repr`,
avoiding the change to accidentally use a mismatched type.

To avoid making implicit semver promises, this is only available for `enum`s
that are defined in the current module.  If you want to expose it to others,
feel free to define a method like

```rust
impl Enum {
	pub fn discriminant(&self) -> u8 {
		self.enum#discriminant
	}
}
```

for others to use, or use one of the many derive macros on crates.io
to expose it through a trait implementation.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Lexing

In edition 2021 and later, `enum#discrimant` becomes a legal token,
using part of the syntax space previously reserved
in [RFC#3101](https://rust-lang.github.io/rfcs/3101-reserved_prefixes.html).

This means that
```rust
macro_rules! single_tt {
	($x:tt) => {}
}
single_tt!(enum#discrimant);
```
now matches, instead of being a lexical error.

In editions 2015 and 2018, this feature is not available.

## Parsing

A new form of expression is added,

> *DiscriminantExpression* :
> > *Expression* `.` `enum#discriminant`

Like `.await`, this is *not* a place expression, and as such is invalid on the
left-hand side of an assignment, giving an error like the following:

```text
error[E0070]: invalid left-hand side of assignment
 --> src/lib.rs:5:29
  |
5 |         x.enum#discriminant = 4;
  |         ------------------- ^
  |         |
  |         cannot assign to this expression
```

## Visibility

This acts as though it were a `pub(in self)` field on a type.

As such, it's an error to use `.enum#discriminant` on types from sub-modules or other crates.

```rust
mod inner {
	pub enum Foo { Bar }
}
inner::Foo::Bar.enum#discriminant // ERROR: enum discriminant is private

```

## Type

The LHS is auto-deref'd until it finds something known to be an `enum`.

*Note: this is different from `mem::discriminant`.  For example,*
```rust
#![allow(enum_intrinsics_non_enums)]
enum MyEnum { A, B }
let a = Box::new(MyEnum::A);
let b = Box::new(MyEnum::B);
assert_eq!(std::mem::discriminant(&a), std::mem::discriminant(&b));
assert_ne!(a.enum#discriminant, b.enum#discriminant);
```
<!-- https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=76293c2f83cb7719c22c15bcebeaeb13 -->

For this, a generic parameter is never considered to be an `enum`,
although a generic enum where some of the generic parameters to the
enum constructor are not yet known is fine.

It's an error if, despite deref'ing, the LHS is still not an `enum`.

If the enum has `repr(uN)` or `repr(iM)`, the `.enum#discriminant` expression
returns a value of type `uN` or `iM` respectively.

If the enum does not specify an integer `repr`, then it returns `isize`.

*Note: `isize` is rarely the desired type for discriminants, and indeed custom
discriminants on types with fields are disallowed without explicit `repr` types.
Returning `isize` is fine here, though, thanks to privacy because the code
inside the module can be updated should it change to specify a specific type.*

## Semantics

When the LHS of a discriminant expression is a *place*, that place is read but not consumed.

*Note: this can be thought of as if it read a field of `Copy` type from the LHS.*

This lowers to [`Rvalue::Discriminant`][MIR discr] in MIR.

[MIR discr]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/mir/enum.Rvalue.html#variant.Discriminant

As this expression is an *r-value*, not a *place*, `&foo.enum#discriminant` returns
a reference to a temporary, aka is the same as `&{foo.enum#discriminant}`.
It does *not* return a reference to the memory in which the discriminant is
stored -- not even for types that do store the discriminant directly.

This expression is allowed in `const` contexts, but is not promotable.

*Note: the behaviour of this expression is independent of whether the type gets
layout-optimized.  For example, the following holds even if `x` is `2_i8` in memory.*
```rust
enum MyOption<T> { MyNone, MySome(T) }
let x = MyOption::<std::cmp::Ordering>::MyNone;
assert_eq!(x.enum#discriminant, 0_isize);
```

# Drawbacks
[drawbacks]: #drawbacks

This isn't strictly necessary, we could continue to get along just fine without it.

- For the FFI cases the layout guarantees mean it's already possible to write a
  sound and reliable function that reads the discriminant.
- For cases without `repr(int)`, custom discriminants aren't even allowed,
  so those discriminants much not be all that important.
- It's always possible to write a `match` in safe code that optimizes away
  and produces exactly the same thing that this new expression would.
- A pseudo-field with `#` in the name looks kinda weird.
- There might be a nicer way to do this in the future.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Why have a `#` in the name?

By not being an identifier, `.enum#discriminant` can't conflict with anything.

While today there are no fields directly accessible from values of enum type,
there are lots of plausible-enough proposals that would allow some.

For example, *enum variant types* have come up repeatedly, which would represent a single
variant and thus would allow accessing the fields on that type, but plausibly would
still offer access to the discriminant.  Similarly, a *pattern type* that restricts
the enum to a single variant would plausibly allow access to its fields.  And one
of those fields might be named `discriminant`.

Other requests have come in too, like allowing field access if every variant has
a field with the same name & type or allowing field access if there's only a
single inhabited variant.

By being clearly different it means it can't conflict with any field or method.
That also helps resolve any concerns about it *looking* like field access -- as
existed for `.await` -- since it's visibly lexically different.

And the lexical space is already reserved,

## Why have `enum` in the name?

Well, it seemed short and evocative enough to be fine.
Doing something like `e#` isn't shorter enough to matter, and
I'd rather save very-short prefixes for higher-prevalence things.

And since it's a pre-existing keyword, it means that
```rust
let d = foo().bar.enum#discriminant;
```
already gets highlighting on the `enum` in my editor without needing any updates.

## Isn't this kinda long?

Not really, compared to the existing possibilities.

For example, in a macro expansion even the internal magic today ends up being
```rust
let __self_tag = ::core::intrinsics::discriminant_value(self);
let __arg1_tag = ::core::intrinsics::discriminant_value(other);
::core::cmp::PartialOrd::partial_cmp(&__self_tag, &__arg1_tag)
```
to avoid any accidental shadowing.

In comparison,
```rust
let __self_tag = self.enum#discriminant;
let __arg1_tag = other.enum#discriminant;
::core::cmp::PartialOrd::partial_cmp(&__self_tag, &__arg1_tag)
```
is much easier.

Outside of macros, something like
```rust
discriminant(&foo)
```
(which requires a `use std::mem::discriminant;`)
isn't that different from
```rust
foo.enum#discriminant
```

And of course you can always make a function to give it a shorter name -- or write
a proc macro to generate that function -- if you so wish.

## Why just `pub(in self)`?

The primary use case that led to this RFC is using it in `derive` macros, where
`pub(in self)` is entirely sufficient.

And by being only private, it avoids forcing any semver promises on library authors.

Today, as a library author, you can reorder the variants in an enum should you so wish,
or in a `#[non_exhausive]` enum add new ones in the middle.  There's no way for
the users of your library to care about the order in which you defined the variants
(unless you make other documented promises) -- especially if you never `derive(PartialOrd)`.

Any library author who wishes to provide discriminant stability can always write
a function to expose those discriminants, trivially implemented using this feature.

## Why expose it via `.`?

I like it behaving kinda like a field.  For example, having auto-deref like a field
means you don't need to worry about whether you actually have a `&&Enum` in a `filter`
or you actually have a `Box<Enum>` or whatever.

Of course, if the `enum` is `repr(C)`, then the discriminant [is a field][RFC2195]
in the guaranteed FFI layout, so thinking of it kinda like a field isn't too weird.

There has also been talk of *compressed* or *move-only* fields where getting the
address is disallowed so that Rust can run arbitrary logic whenever they're accessed
and thus have the freedom to do more layout optimizations than are otherwise possible.
Should we have something like that, then it's again not unreasonable to think of it
as a field that sometimes has particularly fancy layout optimization.

[RFC2195]: https://rust-lang.github.io/rfcs/2195-really-tagged-unions.html

## What about if it was a magic method instead?

It could be.  But it would still need to be something that doesn't cause name
resolution failures for other methods that people might already have written.

So I don't think that the extra `()` on it would really improve things.

## Why not allow writing to the discriminant?

The semantics for that get really complicated, especially for `enum`s in `repr(Rust)`
that don't have a guaranteed layout, and even more so those that get layout-optimized.

Maybe one day it could be allowed, but for now this RFC sticks only things that
can be allowed in safe code without worries.

## Couldn't this be a magic macro?

Sure, it could, like `offset_of!`.

I don't think `enum_discriminant!(foo)` is really better than `foo.enum#discriminant`, though.

It doesn't deal in tokens, and there's no special logic to apply to the scope in which
the argument is computed.

It works on a value or place, not on anything dealing tokens, nor does it affect a scope.

## Why not do *\<more complex feature\>*?

Privacy is the problem.

If we wanted to just expose everything's discriminant to everyone, it'd be easy
to have a trait in core that's auto-implemented for every `enum`.

But to do things in a way that doesn't add a new category of major breaking change,
that gets harder.

It'd be great if we had scoped trait impls, for example, so we could do that
in a way where it's up to the trait author how visible things get.  But that's
a *massive* feature, so it would be nice not to block on it.

Or libs-api could create a new trait and a new `derive` that's implemented using
the same magic that today's `derive(PartialOrd)` uses.  But that's another big
bikeshed, and doesn't even work very well for the "I'm writing my own customized
derive" cases that just want to use the discriminant internally.

The goal here is to do something easy using syntactic space that's not particularly
valuable anyway -- if people end up almost never using this directly because there's
a popular community `derive`, that's great.

## What about `as`?

While `as` *works* on field-less enums, it's not that great there either.

It has the fundamental problem that you have to write out the target type that you want,
and the wrong one will silently truncate.  This hits the same general "`as` is error-prone"
theme that is pushing people away from using `as` to using more-specific things
instead that are either lossless or clearer, to help avoid mistakes.

If this exists, I wouldn't be surprised to see people using `foo.enum#discriminant`
even in places where `foo as u8` works and is shorter since you don't have to think
"what was the `repr` of this, again?" and you just get the right thing.

Should the enum's declared `repr` not be the type you actually want, you can always
use `.enum#discriminant` and *then* `as` cast it -- or hopefully `.into()` or
something else with clearer intent -- into the type you need.

# Prior art
[prior-art]: #prior-art

C++'s `std::variant` has an [`index`](https://en.cppreference.com/w/cpp/utility/variant/index)
method, which always returns `std::size_t` since there's no custom discriminants.
(It's more like what rustc calls a *variant index* internally.)

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Is auto-deref worth it?  I would propose leaving it in the RFC for merging,
  as wanting to use this on `&Enum` will be common, but if in the course of
  implementing it's particularly annoying then stabilizing without it would
  be tolerable, since error messages could suggest the correct thing.

# Future possibilities
[future-possibilities]: #future-possibilities

If this turns out to work well, there's a variety of related properties of things
which could be added in ways similar to this.

For example, you could imagine `MyEnum::enum#VARIANT_COUNT` saying how many variants
are declared, `MyEnum::enum#ReprType` to get the type of the discriminant, or
`my_enum.enum#variant_index` to get the declaration-order index of the variant
(as opposed to its *discriminant* value).

Those are *much* easier to generate with a proc macro, however, so are not included
in this RFC.  They would need separate motivation from what's done here.
