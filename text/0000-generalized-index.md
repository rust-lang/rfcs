- Feature Name: generalized_index
- Start Date: 2018-06-13
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

This document changes the definition of the `Index` trait to make it more general so that the return
value is not necessarily a reference. This enables returning more complex kind of borrowing values
and extend the use of the `[]` operator to more use cases.

# Motivation
[motivation]: #motivation

The `Index` trait current definition is the following:

```rust
pub trait Index<Idx> where Idx: ?Sized {
  type Output: ?Sized;

  fn index(&self, index: Idx) -> &Self::Output;
}
```

It’s easy to notice by the definition of the `index` function that the return value must borrow from
`&self` – lifetime elision here might be confusing if you’re not used to it, but it’s the same thing
as:

```rust
fn index<'a>(&'a self, index: Idx) -> &'a Self::Output;
```

Now consider the following code:

```rust
struct Foo<'a> {
  x: &'a X,
  start: usize,
  end: usize,
  k: f32 // whatever
}

struct X;

impl X {
  fn full_range(&self) -> Foo {
    Foo {
      x: self,
      start: 0,
      end: 100,
      k: 0.
    }
  }

  fn from_range(&self, start: usize) -> Foo {
    Foo {
      x: self,
      start,
      end: 100,
      k: 0.
    }
  }

  fn to_range(&self, end: usize) -> Foo {
    Foo {
      x: self,
      start: 0.,
      end,
      k: 0.
    }
  }

  fn range(&self, end: usize) -> Foo {
    Foo {
      x: self,
      start,
      end,
      k: 0.
    }
  }
}
```

It’s natural for us to spot that `full_range`, `from_range`, `to_range` and `range` are very similar
to the `index` function for, respectively, `Index<RangeFull>`, `Index<RangeFrom>` and
`Index<Range>`.

However, the current definition of `Index<_>` forbids us to implement it for `X` for two reasons:

  - The `index` implementation borrows `self` without giving the lifetime to the implementor, so
    it’s impossible to express something like `Foo<'a>`.
  - The return value is a reference to `Self::Output`, which discards us from returning something by
    move semantics.

This RFC suggests to alter the definition of `Index` to this:

```rust
pub trait Index<'a, Idx> where Idx: ?Sized {
  type Output: ?Sized + 'a;

  fn index(&'a self, index: Idx) -> Self::Output;
}
```

So that it’s now possible to return `Foo<'a>`.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This RFC changes the way the `Index` trait is defined. The `Index::Output` associated type is now
returned via a move, instead of borrowed from the input, and the lifetime used for the input
argument is now fully accessible to `Index::Output` so that nested borrowing is possible.

This has a consequence: legacy implementations can be expressed with this new implementation by
replacing the `Index::Output` type with a the same type with a reference bound to the `'a` lifetime.

```rust
pub trait Index<'a, Idx> where Idx: ?Sized {
  type Output: ?Sized + 'a;

  fn index(&'a self, index: Idx) -> Self::Output;
}
```

For instance, the legacy implementation of `impl Index<RangeFull> for String` is the following:

```rust
impl Index<RangeFull> for String {
  type Output = str;

  fn index(&self, _: RangeFull) -> &Self::Output {
    // …
  }
}
```

With this new proposal, the implementation would be:

```rust
impl<'a> Index<'a, RangeFull> for String {
  type Output = &'a str;

  fn index(&'a self, _: RangeFull) -> Self::Output {
    // …
  }
}
```

The real advantage of such a change is to enable moving out values that are not references, but
still borrow something. For instance, if we take the example from the [motivation] section,
implementing the new `Index` trait would look like this:

```rust
impl<'a> Index<'a, RangeFull> for X {
  type Output = Foo<'a>;

  fn index(&'a self, _: RangeFull) -> Self::Output {
    self.full_range()
  }
}
```

That would enable the following code to compile:

```rust
fn main() {
  let range = X[..];
}
```

This change is a *small change* in regard to newcomers – it’s more general than the legacy trait but
it doesn’t change the way people use it. However, for people used to this trait and especially for
developers who’ve already implemented this trait for custom types, code breakage must be addressed
as the following: because the legacy trait’s implementors return only references, we can use
`rustfix` to automatically migrate to the new trait. For people wanting to do it manually, the
following guide should be sufficient:

  1. Change the type of the `impl` signature to add the `'a` lifetime.
  2. Change the `Index::Output` type by prepending a `&'a` in front of the type defined in there.
  3. Change the definition of the `Index::index` function.
  4. That’s all.

This change then only changes the type signatures. The impact on the implementations’ bodies is
null, because the new trait is more general than the legacy one (it creates a superset of accepted
types, so the legacy types are in this bigger set).

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Most of the current implementors must receive the fix described in the end of section
[guide-level-explanation]. This will change the documentation (but not the actual implementations).

# Drawbacks
[drawbacks]: #drawbacks

## Breakage

This change has the huge drawback that it will generate breakage in most codebases that:

 - `impl Index<_> for` any custom types.
 - The std / core codebase will break as well since most common implementors are defined in there.

## More general yet more complex trait

The new trait version is more general yet more complex to wrap your fingers around. The `'a`
lifetime exposed in the trait interface might confuse people and could be seen has a drawback for
most of the common cases – i.e. when people just want to index the **inside of an object**.

# Rationale and alternatives
[alternatives]: #alternatives

This change is small in terms of code modification to operate on codebase and doesn’t introduce any
newkeyword or language construct. Another lead to implement this feature would be to add a new
operator, like the `[[]]` operator or even `[move idx]` construct. This would then require adding a
new trait, like `IndexMove`, to make the whole design work.

# Unresolved questions
[unresolved]: #unresolved-questions
