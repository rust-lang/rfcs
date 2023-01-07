- Feature Name: `multi_type_return_position_impl_trait` (MTRPIT)
- Start Date: 2023-01-05
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Table of Contents
- [Table of Contents](#table-of-contents)
- [Summary](#summary)
- [Motivation](#motivation)
  - [Overview](#overview)
  - [Example: Error Handling](#example-error-handling)
- [Desugaring](#desugaring)
  - [Overview](#overview-1)
  - [Step-by-step guide](#step-by-step-guide)
  - [Handling default trait methods](#handling-default-trait-methods)
  - [(TODO) Representing `Self` in traits and types](#todo-representing-self-in-traits-and-types)
  - [(TODO) `Any` trait and `TypeId`](#todo-any-trait-and-typeid)
- [Interaction with lifetimes](#interaction-with-lifetimes)
- [(TODO) Relationship to `dyn`](#todo-relationship-to-dyn)
  - [(TODO) Parity in lifetime rules](#todo-parity-in-lifetime-rules)
  - [(TODO) Sized and Self](#todo-sized-and-self)
- [Drawbacks](#drawbacks)
  - [(TODO) Downcasting](#todo-downcasting)
  - [(TODO) Size and performance](#todo-size-and-performance)
  - [(TODO) Teaching `impl` and `dyn`](#todo-teaching-impl-and-dyn)
- [Alternatives](#alternatives)
  - [(TODO) Introduce new syntax for multi-type RPIT](#todo-introduce-new-syntax-for-multi-type-rpit)
  - [Design a general-purpose delegation language feature first](#design-a-general-purpose-delegation-language-feature-first)
  - [(TODO) Stack-allocated `dyn Trait`.](#todo-stack-allocated-dyn-trait)
- [Prior art](#prior-art)
  - [RFC 1951 and RFC 2515](#rfc-1951-and-rfc-2515)
  - [auto-enums crate](#auto-enums-crate)
- [Future possibilities](#future-possibilities)
  - [Anonymous enums](#anonymous-enums)
  - [Language-level support for delegation/proxies](#language-level-support-for-delegationproxies)
  - [(TODO) Mark unimplementable/sealed trait methods](#todo-mark-unimplementablesealed-trait-methods)
  - [(TODO) Clippy lint tracking big impl traits](#todo-clippy-lint-tracking-big-impl-traits)


# Summary
[summary]: #summary

This RFC enables [Return Position Impl Trait (RPIT)][RPIT] to work in functions
which return more than one type, mostly without any additional restrictions:

[RPIT]: https://doc.rust-lang.org/stable/rust-by-example/trait/impl_trait.html#as-a-return-type

```rust
// Possible already
fn single_iter() -> impl Iterator<Item = i32> {
    1..10 // `std::ops::Range<i32>`
}

// Enabled by this RFC
fn multi_iter(x: i32) -> impl Iterator<Item = i32> {
    match x {
        0 => 1..10,                   // `std::ops::Range<i32>`
        _ => vec![5, 10].into_iter(), // `std::vec::IntoIter<i32>`
    }
}
```

# Motivation
[motivation]: #motivation

## Overview

[Return Position Impl Trait (RPIT)][RPIT] is used when you want to return a
value, but don't want to specify the type. In today's Rust (1.66.0 at the time
of writing) it's only possible to use this when you're returning a single type
from the function. The moment multiple types are returned from the function, the
compiler [will return an error](https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=d651759521a2ba4c84015a4fb89209a9):

```text
error[E0308]: `match` arms have incompatible types
  --> src/lib.rs:10:14
   |
8  | /     match x {
9  | |         0 => 1..10,
   | |              ----- this is found to be of type `std::ops::Range<{integer}>`
10 | |         _ => vec![5, 10].into_iter(),
   | |              ^^^^^^^^^^^^^^^^^^^^^^^ expected struct `std::ops::Range`, found struct `std::vec::IntoIter`
11 | |     }
   | |_____- `match` arms have incompatible types
   |
   = note: expected struct `std::ops::Range<{integer}>`
              found struct `std::vec::IntoIter<{integer}>`
```

Experiencing this can be frustrating, since even if the compiler explains
_what_ is happening, it doesn't explain _why_ it's happening. And that's a
good question: Why is this happening? When we're returning `dyn Trait` we can
return as many types as we want; the compiler handles this for us. But when
we're using `-> impl Trait` we can't? The distinction can be hard to explain.

If you're new to Rust, the first feeling you'll have with this may be one of
frustration. You _know_ you want to return two different types, and only
preserve their interface, but the compiler isn't telling you how to resolve
this. It's as you become more experienced with Rust that you'll learn about how to
work around this: either return `dyn Trait` or unify the types using an enum.

Returning a `Box<dyn Trait>` is a pretty common alternative, but it has some
sharp edges to it too. First of all: it requires access to an allocator. This
is common, but especially on embedded platforms is not a given. Second of all,
it may not be as performant as `impl Trait`: creating an allocation + vtable,
and following pointers during runtime is not zero-cost. And finally: `dyn` has
many quirks which can be hard to reason about. The meaning of `Sized` can
confuse even experienced Rustaceans.

The other alternative is to manually author an enum which has a member for
each unique type in the code. This is an entirely manual process, which can
be labor-intensive, and the resulting code can often obfuscate the original
intent. This is not unlike constructing a manual vtable, minus the unsafe
parts. This is a pattern which is hard to teach via diagnostics, and requires
a thorough understanding of Rust's basics to use effectively.


## Example: Error Handling

A motivating example for this is use in error handling: it's not uncommon to
have a function return more than one error type, but you may not necessarily
care about the exact errors returned. You may either choose to define a `Box<dyn
Error + 'static>` which has the downside that [it itself does not implement
`Error`][no-error]. Or you may choose to define your own enum of errors, which
can be a lot of work and may obfuscate the actual intent of the code. It may
sometimes be preferable to return an `impl Trait` instead:

[no-error]: https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=97894fc907fa2d292cbe909467d4db4b

```rust
use std::error::Error;
use std::fs;

// ❌ Multi-type RPIT does not yet compile (Rust 1.66.0)
// error[E0282]: type annotations needed
fn main() -> Result<(), impl Error> {
    let num = i8::from_str_radix("A", 16)?;       // `Result<_, std::num::ParseIntError>`
    let file = fs::read_to_string("./file.csv")?; // `Result<_, std::io::Error>`
    // ... use values here
    Ok(())
}
```

# Desugaring
[reference-level-explanation]: #reference-level-explanation

This section covers one _possible_ desugaring. It may be the case that, say, for
larger numbers of types using for example a stack-allocated vtable may be more
efficient. The desugaring we're covering here was chosen because we think it's
relatively simple, but as long as the semantics are preserved it may be
substituted with more optimized implementations. This RFC intentionally provides
no guarantees about the layout of the desugaring.

## Overview

Let's take a look again at the code from our motivation section. This function
has two branches which each return a different type which implements the
[`Iterator` trait][`Iterator`]:

[`Iterator`]: https://doc.rust-lang.org/std/iter/trait.Iterator.html

```rust
fn multi_iter(x: i32) -> impl Iterator<Item = i32> {
    match x {
        0 => 1..10,                   // `std::ops::Range<i32>`
        _ => vec![5, 10].into_iter(), // `std::vec::IntoIter<i32>`
    }
}
```

In its simplest form, this code should be desugared by the compiler into
something resembling the following
([playground](https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=af4c0e61df25acaada168449df9838d3)):

```rust
// anonymous enum generated by the compiler
enum Enum {
    A(std::ops::Range<i32>),
    B(std::vec::IntoIter<i32>),
}

// trait implementation generated by the compiler,
// delegates to underlying enum member's values
impl Iterator for Enum {
    type Item = i32;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Enum::A(iter) => iter.next(),
            Enum::B(iter) => iter.next(),
        }
    }

    // ..repeat for the remaining 74 `Iterator` trait methods
}

// the desugared function now returns the generated enum
fn multi_iter(x: i32) -> Enum {
    match x {
        0 => Enum::A(1..10),
        _ => Enum::B(vec![5, 10].into_iter()),
    }
}
```

TODO: Not all trait methods can be delegated, since they would _obviously_ be
wrong. As pointed out in [this
comment](https://github.com/rust-lang/rfcs/pull/3367#discussion_r1063688548), we
need to provide a strategy to keep the right version. 

## Step-by-step guide

What we're proposing we start with is the 
This desugaring can be implemented using the following steps:

1. Find all return calls in the function
2. Define a new enum with a member for each of the function's return types
3. Implement the traits declared in the `-> impl Trait` bound for the new enum,
   matching on `self` and delegating to the enum's members
   1. Check whether the trait methods on the delegates are the _default_ method,
   or a custom implementation.
   2. Keep the default method if all instances of the method on the delegates
   are the default.
   3. Only generate delegating implementations if there are custom implementations
4. Substitute the `-> impl Trait` signature with the concrete enum
5. Wrap each of the function's return calls in the appropriate enum member

The hardest part of implementing this RFC will likely be the actual trait
implementation on the enum, as each of the trait methods will need to be
delegated to the underlying types.

## Handling default trait methods

In complex traits such as `Iterator`, a majority of the methods will not be
replaced on implementations and remain the default. In fact, for a large number
of default methods it's not even possible to replace them with custom
implementations, because they return types whose constructors are effectively
sealed. For example, this is how `Iterator::step_by` is implemented in the
stdlib:

```rust
fn step_by(self, step: usize) -> StepBy<Self>
where
    Self: Sized,
{
    StepBy::new(self, step) // `StepBy::new` is private to `core`
}
```

Say we're returning two different iterators from a function using `-> impl
Iterator`. If we attempted to generate a delegation, the codegen might be
invalid and fail to compile:

```rust
// ❌ This codegen would be invalid
fn step_by(self, step: usize) -> StepBy<Enum<'a>> {
    match self {
        Enum::A(iter) => iter.step_by(step), // : StepBy<Range<i32>>
        Enum::B(iter) => iter.step_by(step), // : StepBy<vec::IntoIter<'a>>
    }
}
```

Instead we should only ever generate a delegation for methods where the default
method has been replaced by a custom implementation. Which tracks with how we
would implement this by hand as well, and is probably more efficient even in
cases where delegating to default implementations was possible:

```rust
// sticking to the default implementation...
impl Iterator for Enum {
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, None) // default impl
    }
}

// ... is going to be more efficient than delegating to two instances of the
// default implementation
impl Iterator for Enum {
    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            A(iter) => iter.size_hint(), // calls default impl: `(0, None)` 
            B(iter) => iter.size_hint(), // calls default impl: `(0, None)`
        }
    }
}
```

This approach is similar to how this would be done when unifying types to an
enum by hand, and will work as expected in all practical cases. The only
special case this needs to account for is custom implementations of methods
which return the never type (`!`).

```rust
impl Iterator for MyType {
    // This would type-check, but would not be practical, as it can't ever
    // generate `StepBy`. This may still register as a "custom implementation"
    // however, which could cause code to be generated which doesn't typecheck
    fn step_by(self, step: usize) -> StepBy<Enum<'a>> {
        loop {} // this would 
    }
}
```

These implementations are not at all practical but would styll type-check, and
would cause the delegation to generate code which wouldn't work. This seems
impractical enough that we could probably ignore it in an initial
implementation. But later on we could detect where the actual returned type
of the function is never, and emit a diagnostic for that. In the even longer
term, we could annotate these types of unimplementable functions. See the "future
possibilities" section for more on that.

## (TODO) Representing `Self` in traits and types

TODO (tldr: when generating types which expect a `Self`, make sure to wrap the
output in the appropriate enum member branch. If anyone later asks what type it
is: just say it's an `impl Trait`.)

## (TODO) `Any` trait and `TypeId`

TODO (tldr: proxy the type id to the inner type, that's what `dyn` does too. You
can't downcast `impl Trait` as-is anyway, so it's likely fine.

# Interaction with lifetimes

`dyn Trait` already supports multi-type _dynamic_ dispatch. The rules we're
proposing for multi-type _static_ dispatch using `impl Trait` should mirror the
existing rules we apply to `dyn Trait.` We should follow the same lifetime rules
for multi-type `impl Trait` as we do for `dyn Trait`:

```rust
fn multi_iter<'a>(x: i32, iter_a: &'a mut std::ops::Range<i32>) -> impl Iterator<Item = i32> + 'a {
    match x {
        0 => iter_a,                  // `&'a std::ops::Range<i32>`
        _ => vec![5, 10].into_iter(), // `std::vec::IntoIter<i32>`
    }
}
```

This code should be desugared by the compiler into something resembling the following
([playground](https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=60ddacbb20c4068a0fff44a5481a7136)):

```rust
enum Enum<'a> {
    A(&'a mut std::ops::Range<i32>),
    B(std::vec::IntoIter<i32>),
}

impl<'a> Iterator for Enum<'a> {
    type Item = i32;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Enum::A(iter) => iter.next(),
            Enum::B(iter) => iter.next(),
        }
    }

    // ..repeat for the remaining 74 `Iterator` trait methods
}

fn multi_iter<'a>(x: i32, iter_a: &'a mut std::ops::Range<i32>) -> Enum<'a> {
    match x {
        0 => Enum::A(iter_a),
        _ => Enum::B(vec![5, 10].into_iter()),
    }
}
```

It should be fine if multiple iterators use the same lifetime. But only a single
lifetime should be permitted on the return type, as is the case today when
using `dyn Trait`:

```rust
// ❌ Fails to compile (Rust 1.66.0)
// error[E0226]: only a single explicit lifetime bound is permitted
fn fails<'a, 'b>() -> Box<dyn Iterator + 'a + 'b> {
    ...
}
```

# (TODO) Relationship to `dyn`

TODO (explain the relationship to dyn traits, returning `Self`, and which traits
can and can't be allowed, and how that differs from single-value TAITs).

## (TODO) Parity in lifetime rules

TODO: Move lifetimes section here

## (TODO) Sized and Self

TODO (tldr: neither is an issue with `impl Trait` today, and it shouldn't have
to be either in the future. That's the main difference in this approach.)

# Drawbacks

## (TODO) Downcasting

TODO (tldr: because we're generating an anonymous enum, you can't downcast to a
concrete type. With single-value RPIT you have a concrete type which you _can_
downcast to. But with MTRPIT we're generating a type, meaning you can't downcast
into it)

## (TODO) Size and performance

TODO (tldr: we already see issues around auto-generated enums in the compiler
already. Boats did a great post on that. The solution is probably not to eschew
ergonomics in favor of more "manual" uses, but instead to provide better tools
to track, diagnose, and mitigate problems as they arise. Both the compiler and
clippy have knowledge of type sizes. They can, and already do provide hints
where needed.)

[boats-segmented-stacks]: https://without.boats/blog/futures-and-segmented-stacks/
[clippy-large-enum]: https://rust-lang.github.io/rust-clippy/master/index.html#large_enum_variant

## (TODO) Teaching `impl` and `dyn`

TODO (tldr: this blurs the line between `impl` and `dyn` somewhat, and we need
to provide a better explanation than "impl for one type, dyn for many types".
But that's okay; we're not fundamentally enabling anything new, so the decision
isn't new either - we just need to better explain the tradeoffs.)

[rbe-dyn]: https://doc.rust-lang.org/stable/rust-by-example/trait/dyn.html
[trpl-trait-objs]: https://doc.rust-lang.org/book/ch17-02-trait-objects.html
[trpl-dyn-size]: https://doc.rust-lang.org/book/ch19-04-advanced-types.html#dynamically-sized-types-and-the-sized-trait

# Alternatives

## (TODO) Introduce new syntax for multi-type RPIT

TODO (Q: why not introduce `impl enum Trait` or the like? A: enums are an
implementation detail, and syntax is expensive. If there are no _semantic_
differences between single and multi `impl` trait, then there should be no
_syntactic_ differences either. Plus parity with the way `dyn Trait` already
works for both single and multi impls.)

## Design a general-purpose delegation language feature first

This RFC has a focus on making `-> impl Trait` work for more than value. As part
of the implementation, we're proposing to create an enum with trait impls which
calls out to types contained within it. This forwarding of method calls is also
known as "delegation" or "proxying".

One question that can be asked here is: _"If we're going to delegate anyway, 
shouldn't we start by designing the delegation feature _first_, so that we can
use this to implement MVRPIT with?"_

MVRPIT and delegation syntax have similarities in that they both call out to
inner types. But the main difference is that delegation syntax carries the
burden of having to define a complete language feature with syntax included.
This is a _far_ greater scope than what this RFC covers.

In the "future possibilities" section we explore what a "delegation syntax"
language feature might look like. We believe that is a promising idea, which
solves actual issues people face. But in terms of ordering we believe it's best
to start with the simpler feature first, which in turn can help make the harder
feature easier to implement later. This same argument applies to "why not start
with anonymous enums" as well.

## (TODO) Stack-allocated `dyn Trait`.

TODO (tldr: `dyn Trait` has object-safety restrictions which apply even if you
stack-allocate. `impl Trait` does not have these restrictions, making it more
flexible in certain cases.)

# Prior art
[prior-art]: #prior-art

## RFC 1951 and RFC 2515

This isn't the first proposal to cover expanding the capabilities of the `impl
Trait` syntax. With the implementation of [RFC 1951: Expand Impl Trait][rfc1951]
questions surrounding lifetimes were resolved, and it became possible to use
`impl Trait` in argument position (also known as [existential types]):

[existential types]: https://blog.rust-lang.org/2018/05/10/Rust-1.26.html#impl-trait

```rust
// Possible since Rust 1.26
fn take_iter(t: impl Iterator) { ... }
```

And more recently [RFC 2515: Type Alias Impl Trait (TAIT)][TAIT] has been
accepted, enabling the use of opaque types satisfying certain bounds. Or put
put more simply: it enables the use of `impl Trait` in type declarations:

```rust
type Foo = impl Bar;
```

[rfc1951]: https://github.com/rust-lang/rfcs/blob/master/text/1951-expand-impl-trait.md
[TAIT]: https://rust-lang.github.io/rfcs/2515-type_alias_impl_trait.html

We can think of the current RFC as "static-multi-type dispatch using anonymous
enums". Which is a way of describing _how_ the RFC is intended to work. But
_what_ we're doing from a user's perspective should feel as yet another
expansion of `impl Trait`. A restriction people might have hit before will now
no longer be there. Which feels like another step in the trend of enabling
`impl Trait` to be usable in more places.

## auto-enums crate

The [`auto-enums` crate][auto-enums] implements a limited variation of what is
proposed in this RFC using procedural macros. It's limited to a predefined set
of traits only, whereas this RFC enables multi-type RPIT to work for _all_
traits. This limitation exists in the proc macro because it doesn't have access
to the same type information as the compiler does, so the trait delegations
have to be authored by hand. Here's an example of the crate being used to
generate an `impl Iterator`:

[auto-enums]: https://docs.rs/auto_enums/latest/auto_enums/

```rust
use auto_enums::auto_enum;

#[auto_enum(Iterator)]
fn foo(x: i32) -> impl Iterator<Item = i32> {
    match x {
        0 => 1..10,
        _ => vec![5, 10].into_iter(),
    }
}
```

# Future possibilities
[future-possibilities]: #future-possibilities

## Anonymous enums

Rust provides a way to declare anonymous structs using tuples. But we don't yet
have a way to declare anonymous enums. A different way of interpreting the
current RFC is as a way to declare anonymous type-erased enums, by expanding what
RPIT can be used for. It stands to reason that there will be cases where people
may want anonymous _non-type-erased_ enums too.

Take for example the iterator code we've been using throughout this RFC. But
instead of `Iterator` yielding `i32`, let's make it yield `i32` or `&'static
str`:

```rust
fn multi_iter(x: i32) -> impl Iterator<Item = /* which type? */> {
    match x {
        0 => 1..10,                              // yields `i32`
        _ => vec!["hello", "world"].into_iter(), // yields `&'static str`
    }
}
```

One solution to make it compile would be to first map it to a type which can
hold *either* `i32` or `String`. The obvious answer would be to use an enum for
this:

```rust
enum Enum {
    A(i32),
    B(&'static str),
}

fn multi_iter(x: i32) -> impl Iterator<Item = Enum> {
    match x {
        0 => 1..10.map(Enum::A),
        _ => vec!["hello", "world"].into_iter().map(Enum::B),
    }
}
```

This code resembles the desugaring for multi-type RPIT we're proposing in this
RFC. In fact: it may very well be that a lot of the internal compiler machinery
used for multi-RPIT could be reused for anonymous enums.

The similarities might become even closer if we consider how "anonymous enums"
could be used for error handling. Sometimes it can be useful to know which error
was returned, so you can decide how to handle it. For this RPIT isn't enough: we
actually want to retain the underlying types so we can match on them. We might
imagine the earlier errror example could instead be written like this:

```rust
use std::{fs, io, num};

// The earlier multi-type RPIT version returned `-> Result<(), impl Error>`.
// This example declares an anonymous enum instead, using made-up syntax
fn main() -> Result<(), num::ParseIntError | io::Error> {
    let num = i8::from_str_radix("A", 16)?;       // `Result<_, std::num::ParseIntError>`
    let file = fs::read_to_string("./file.csv")?; // `Result<_, std::io::Error>`
    // ... use values here
    Ok(())
}
```

There are a lot of questions to be answered here. Which traits should
this implement? What should the declaration syntax be? How could we match on
values? All enough to warrant its own exploration and possible RFC in the
future.

## Language-level support for delegation/proxies

One of the trickiest parts of implementing this RFC will be to delegate from the
generated enum to the individual enum's members. If we implement this
functionality in the compiler, it may be beneficial to generalize this
functionality and create syntax for it. We're already seen [limited support for
delegation codegen][support] in Rust-Analyzer as a source action [^disclaimer], and [various crates]
implementing delegation exist on Crates.io.

[support]: https://github.com/rust-lang/rust-analyzer/issues/5944
[various crates]: https://crates.io/search?q=delegate

[^disclaimer]: Full disclosure: One of the authors of this RFC has filed the issue and
subsequently authored the extension to Rust-Analyzer for this. Which itself was
based on prior art found in the VS Code Java extension.

To provide some sense for what this might look like. Say we were authoring some
[newtype] which wraps an iterator. We could imagine we'd write that in Rust
by hand today like this:

[newtype]: https://doc.rust-lang.org/rust-by-example/generics/new_types.html

```rust
struct NewIterator<T>(iter: std::array::Iterator<T>);

impl<T> Iterator for NewIterator<T> {
    type Item = T;

    #[inline]
    pub fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }

    // ..repeat for the remaining 74 `Iterator` trait methods
}
```

Forwarding a single trait with a single method is doable. But we can imagine
that repeating this for multiple traits and methods quickly becomes a hassle,
and can obfuscate the _intent_ of the code. Instead if we could declare that
`NewIterator` should _delegate_ its `Iterator` implementation to the iterator
contained within. Say we adopted a [Kotlin-like syntax], we could imagine it
could look like this:

[Kotlin-like syntax]: https://kotlinlang.org/docs/delegation.html#overriding-a-member-of-an-interface-implemented-by-delegation

```rust
struct NewIterator<T>(iter: std::array::Iterator<T>);

impl<T> Iterator for NewIterator<T> by Self.0; // Use `Self.0` as the `Iterator` impl
```

There are many open questions here regarding semantics, syntax, and expanding it
to other features such as method delegation. But given the codegen for both
multi-type RPIT and delegation will share similarities, it may be worth
exploring further in the future.

## (TODO) Mark unimplementable/sealed trait methods

TODO (we can't implement `Iterator::step_by` manually because of orphan rules,
but that information is not available from the interface anywhere. Can we
surface or somehow mark which methods can _actually_ be overloaded, versus which
can't?)

## (TODO) Clippy lint tracking big impl traits

TODO (tldr clippy tracks big enums and big errors already, it probably should
track big futures too - and likely big `impl Trait`s as well.)

---
