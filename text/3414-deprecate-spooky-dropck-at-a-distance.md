- Feature Name: `deprecate_spooky_dropck_at_a_distance`
- Start Date: 2023-02-13
- RFC PR: [rust-lang/rfcs#3414](https://github.com/rust-lang/rfcs/pull/3414)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Never add outlives requirements for a non-`needs_drop` type. Adjust
`#[may_dangle]` for the required semantics.

# Motivation
[motivation]: #motivation

`PhantomData` having dropck behaviour leads to "spooky-dropck-at-a-distance":

This fails to compile:

```rust
use core::marker::PhantomData;

struct PrintOnDrop<'s>(&'s str);
impl<'s> Drop for PrintOnDrop<'s> {
    fn drop(&mut self) {
        println!("{}", self.0);
    }
}

fn to_pd<T>(_: T) -> PhantomData<T> {
    PhantomData
}

pub fn foo() {
    let mut x;
    {
        let s = String::from("temporary");
        let p = PrintOnDrop(&s);
        x = (to_pd(p), String::new());
    }
}
```

And yet, this compiles:

```rust
use core::marker::PhantomData;

struct PrintOnDrop<'s>(&'s str);
impl<'s> Drop for PrintOnDrop<'s> {
    fn drop(&mut self) {
        println!("{}", self.0);
    }
}

fn to_pd<T>(_: T) -> PhantomData<T> {
    PhantomData
}

pub fn foo() {
    let mut x;
    {
        let s = String::from("temporary");
        let p = PrintOnDrop(&s);
        x = (to_pd(p), ());
    }
}
```

`PhantomData`'s dropck behaviour is only checked if the type (in this case, the
tuple) marks `needs_drop`, which is confusing. Unrelated tuple elements
shouldn't affect `PhantomData` behaviour, but the above example shows that they
do. This RFC makes it so they don't.

Likewise, `[T; 0]` produces the same effect: it is not `needs_drop`, but adds
outlive requirements, thus also exhibiting "spooky-dropck-at-a-distance".

Simply defining every non-`needs_drop` type as being pure w.r.t. drop would,
however, break `#[may_dangle]`, so we need to adapt it.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## Changes to dropck (stable)

Types like `fn(T)`, `ManuallyDrop<T>`, `PhantomData<T>`, `[T; 0]` and `&'a T`,
that don't need to be dropped, place no liveness requirements on `'a` or `T`
when they go out of scope - even if `T` would otherwise introduce liveness
requirements.

In other words, this compiles:

```rust
fn main() {
    let mut x;
    {
        x = &String::new();
        // String implicitly dropped here
    }
    // x implicitly dropped here
}
```

But a type which does need to be dropped, introduces liveness requirements:

```rust
struct PrintOnDrop<'s>(&'s str);
impl<'s> Drop for PrintOnDrop<'s> {
    fn drop(&mut self) {
        println!("{}", self.0);
    }
}

fn main() {
    let mut x;
    {
        x = PrintOnDrop(&*String::new());
        // String implicitly dropped here
    }
    // x implicitly dropped here
    // ERROR: temporary may not live long enough
}
```

Unless it's in one of the above types:

```rust
struct PrintOnDrop<'s>(&'s str);
impl<'s> Drop for PrintOnDrop<'s> {
    fn drop(&mut self) {
        println!("{}", self.0);
    }
}

fn main() {
    let mut x;
    {
        x = &PrintOnDrop(&*String::new());
        // PrintOnDrop implicitly dropped here
        // String implicitly dropped here
    }
    // x implicitly dropped here
}
```

As a special case, some built-in types, like `Vec`, `Box`, `Rc`, `Arc`,
`HashMap`, among others, despite needing to be dropped, do not place liveness
requirements unless `T` demands liveness requirements. This is okay:

```rust
fn main() {
    let mut x = vec![];
    {
        x.push(&String::new());
        // String implicitly dropped here
    }
    // x implicitly dropped here
}
```

But this isn't:

```rust
struct PrintOnDrop<'s>(&'s str);
impl<'s> Drop for PrintOnDrop<'s> {
    fn drop(&mut self) {
        println!("{}", self.0);
    }
}

fn main() {
    let mut x = vec![];
    {
        x.push(PrintOnDrop(&*String::new()));
        // String implicitly dropped here
    }
    // x implicitly dropped here
    // ERROR: temporary may not live long enough
}
```

## Changes to `#[may_dangle]` (unstable)

A type marked `#[may_dangle(drop)]` gets checked for liveness at drop. This is
necessary for `Vec`:

```rust
struct Vec<T> {
  ...
}

unsafe impl<#[may_dangle(drop)] T> Drop for Vec<T> {
  fn drop(&mut self) {
    ...
  }
}
```

So that this compiles:

```rust
fn main() {
  let mut v = vec![];
  {
    v.push(&String::from("temporary"));
  }
}
```

But this cannot compile, as it would be unsound:

```rust
struct PrintOnDrop<'s>(&'s str);
impl<'s> Drop for PrintOnDrop<'s> {
    fn drop(&mut self) {
        println!("{}", self.0);
    }
}

fn main() {
  let mut v = vec![];
  {
    v.push(PrintOnDrop(&*String::from("temporary")));
  }
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This RFC removes the dropck/outlives constraints from `PhantomData` and `[T; 0]`
and moves them into the relevant `Drop` impls instead.

Instead of relying on `PhantomData`, there are now 3 forms of `may_dangle`:

For lifetime parameters: `#[may_dangle] 'a`, places no constraints on `'a`. This
is unchanged from the current form.

For type parameters that are dropped, they need to be annotated with
`#[may_dangle(drop)]`. These type parameters will be checked as if in a struct
like so:

```rust
struct Foo<T>(T);

struct PrintOnDrop<'s>(&'s str);
impl<'s> Drop for PrintOnDrop<'s> {
    fn drop(&mut self) {
        println!("{}", self.0);
    }
}

fn main() {
  let mut foo;
  {
    foo = Foo(PrintOnDrop(&*String::from("temporary")));
  }
  // ERROR: Foo dropped here, runs destructor for PrintOnDrop
}
```

For type parameters that are not dropped, `#[may_dangle(borrow)]` can be used.
These are checked as if in a struct like so:

```rust
struct Foo<T>(*const T);

struct PrintOnDrop<'s>(&'s str);
impl<'s> Drop for PrintOnDrop<'s> {
    fn drop(&mut self) {
        println!("{}", self.0);
    }
}

fn main() {
  let mut foo;
  {
    foo = Foo(&PrintOnDrop(&*String::from("temporary")));
  }
  // no error here
}
```

Effectively, a type is checked to be *safe to drop* by the following procedure:

- If the type has no type or lifetime parameters, it is *safe to drop*.
- If the type is any of the below, it is *safe to drop*:
    References, raw pointers, function pointers, function items, `PhantomData`,
    `ManuallyDrop` and empty arrays. In other words, all (at the time of
    writing) non-`needs_drop` types.
- If the type has a `Drop` impl, or is a trait object:
    - For every lifetime parameter:
        - If the lifetime parameter is marked `#[may_dangle]`, continue.
        - If the lifetime is dead, the type is not *safe to drop*.
    - For every type parameter:
        - If the type parameter is marked `#[may_dangle(borrow)]`, continue.
        - If the type parameter is marked `#[may_dangle(drop)]`:
            - If the type parameter is not *safe to drop*, then the type is not
                *safe to drop*.
            - Continue.
        - If the type parameter contains lifetimes which are dead, the type is
            not *safe to drop*.
- If the type does not have a `Drop` impl:
    - For every field:
        - If the field type is not *safe to drop*, then the type is not *safe to
            drop*.

(N.B. you cannot add `#[may_dangle]` to a trait object's parameters.)

This is different from the status quo in that 1. we skip checking fields
entirely in the `Drop` case, and rely only on the `#[may_dangle]`, and 2. we
always treat `PhantomData` as *safe to drop*.

# Drawbacks
[drawbacks]: #drawbacks

Due to `#[may_dangle]` changes, this requires mild churn to update things to the
new way.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

A type which doesn't need drop should never have dropck/outlives contraints,
but due to the rushed way in which `may_dangle` was implemented, `PhantomData`
ended up having this unfortunate "spooky-dropck-at-a-distance" behaviour. This
RFC removes this behaviour and allows strictly more code to compile.

Lifetimes cannot be dropped, so it wouldn't make sense to have
`#[may_dangle(drop)]` for lifetimes. We adopt a single form for them.

This proposal attempts to be as minimal as possible and focuses entirely on the
"spooky-dropck-at-a-distance" behaviour. It also distinguishes between stable
behaviour and unstable behaviour, opting *not* to document unstable behaviour,
which is subject to change, as part of stable behaviour. While the unstable
behaviour *is* relevant to dropck, particularly where collection types (`Vec`,
`HashMap`, etc) expose some details about it to stable code, we opt to document
it separately instead.

# Prior art
[prior-art]: #prior-art

- Compiler MCP 563: Mostly deals with checking the `Drop` impl's correctness,
    but involves some aspects of this RFC. A full RFC seems appropriate due to
    the observable changes to stable, namely the `PhantomData` behaviour.
- `may_dangle`: RFC 1238, RFC 1327
- This is effectively split from [RFC PR 3390] and is not intended for
    stabilization.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

## `#[may_dangle] T` as alias for `#[may_dangle(drop)] T`.

Most existing uses of `#[may_dangle] T` are actually `#[may_dangle(drop)] T`, so
to avoid churn we could just make them an alias. This is relevant for e.g.
`hashbrown`, a crate which is used by `std` to provide `HashMap`.

# Future possibilities
[future-possibilities]: #future-possibilities

The full [RFC PR 3390], and stabilization.

[RFC PR 3390]: https://github.com/rust-lang/rfcs/pull/3390
