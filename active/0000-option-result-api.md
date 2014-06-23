- Start Date: 2014-06-09
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Provide a common API across `Option` and the `Ok` and `Err` variants of
`Result`.

# Motivation

The current `Option` API has a comprehensive set of methods to make dealing
with the wrapped values more convenient:

~~~rust
pub enum Option<T> { None, Some(T) }

impl<T> Option<T> {
    pub fn is_some(&self) -> bool { ... }
    pub fn is_none(&self) -> bool { ... }
    pub fn as_ref<'r>(&'r self) -> Option<&'r T> { ... }
    pub fn as_mut<'r>(&'r mut self) -> Option<&'r mut T> { ... }
    pub fn as_slice<'r>(&'r self) -> &'r [T] { ... }
    pub fn as_mut_slice<'r>(&'r mut self) -> &'r mut [T] { ... }
    pub fn unwrap(self) -> T { ... }
    pub fn unwrap_or(self, def: T) -> T { ... }
    pub fn unwrap_or_else(self, f: || -> T) -> T { ... }
    pub fn expect(self, msg: &str) -> T { ... }
    pub fn map<U>(self, f: |T| -> U) -> Option<U> { ... }
    pub fn map_or<U>(self, def: U, f: |T| -> U) -> U { ... }
    pub fn mutate(&mut self, f: |T| -> T) -> bool { ... }
    pub fn mutate_or_set(&mut self, def: T, f: |T| -> T) -> bool { ... }
    pub fn iter<'r>(&'r self) -> Item<&'r T> { ... }
    pub fn mut_iter<'r>(&'r mut self) -> Item<&'r mut T> { ... }
    pub fn move_iter(self) -> Item<T> { ... }
    pub fn and<U>(self, optb: Option<U>) -> Option<U> { ... }
    pub fn and_then<U>(self, f: |T| -> Option<U>) -> Option<U> { ... }
    pub fn or(self, optb: Option<T>) -> Option<T> { ... }
    pub fn or_else(self, f: || -> Option<T>) -> Option<T> { ... }
    pub fn take(&mut self) -> Option<T> { ... }
    pub fn filtered(self, f: |t: &T| -> bool) -> Option<T> { ... }
    pub fn while_some(self, f: |v: T| -> Option<T>) { ... }
    pub fn take_unwrap(&mut self) -> T { ... }
    pub fn get_ref<'a>(&'a self) -> &'a T { ... }
    pub fn get_mut_ref<'a>(&'a mut self) -> &'a mut T { ... }
}

impl<T: Default> Option<T> 
    pub fn unwrap_or_default(self) -> T { ... }
}
~~~

The `Result` API provides a similar, yet greatly reduced set of methods that
are biased either towards the `Ok` variant or the `Err` variant (denoted with
the `_err` suffix):

~~~rust
// core::result

pub enum Result<T, E> { Ok(T), Err(E) }

impl<T, E> Result<T, E> {
    pub fn is_ok(&self) -> bool { ... }
    pub fn is_err(&self) -> bool { ... }
    pub fn ok(self) -> Option<T> { ... }
    pub fn err(self) -> Option<E> { ... }
    pub fn as_ref<'r>(&'r self) -> Result<&'r T, &'r E> { ... }
    pub fn as_mut<'r>(&'r mut self) -> Result<&'r mut T, &'r mut E> { ... }
    pub fn map<U>(self, op: |T| -> U) -> Result<U,E> { ... }
    pub fn map_err<F>(self, op: |E| -> F) -> Result<T,F> { ... }
    pub fn and<U>(self, res: Result<U, E>) -> Result<U, E> { ... }
    pub fn and_then<U>(self, op: |T| -> Result<U, E>) -> Result<U, E> { ... }
    pub fn or(self, res: Result<T, E>) -> Result<T, E> { ... }
    pub fn or_else<F>(self, op: |E| -> Result<T, F>) -> Result<T, F> { ... }
    pub fn unwrap_or(self, optb: T) -> T { ... }
    pub fn unwrap_or_else(self, op: |E| -> T) -> T { ... }
}

impl<T, E: Show> Result<T, E> {
    pub fn unwrap(self) -> T { ... }
}

impl<T: Show, E> Result<T, E> {
    pub fn unwrap_err(self) -> E { ... }
}
~~~

Which methods that are and are not implemented on `Result` does not seem to be
well thought through. This can make it hard to transition code from `Option`
to `Result` and make it irritating for the client when a standard operation
that is implemented on `Option` is not available when working with `Result`
values. A `Result` *can* be converted to an `Option` using the
`Result::{ok, err}` methods, but this is a lossy transformation, losing the
value stored either in the `Err` or `Ok` variant respectively.

The inconsistency will also make it more challenging to transition to an API
with a single trait that abstracts over Option-style things once higher-kinded
types are implemented post-1.0.

# Detailed design

The goal is to provide a common set of methods across `Option`, and both the
`Ok` and `Err` variants of `Result`. Ideally we would do this by defining a
trait but without higher-kinded types we cannot define such a trait. As an
interim solution we will have separate, matching `impl` blocks in `Option` and
`Result` which can eventually be made into trait implementations once
higher-kinded traits are definable. A `ForErr` adapter struct will also be
added to provide method implementations biased towards the `Err` variant.

## Option API

The `Option` API will be cleaned up, and the methods that are shared with
`Result` will be clearly differentiated in the source code:

~~~rust
// core::option

pub enum Option<T> { None, Some(T) }

// Option-specific predicates
impl<T> Option<T> {
    pub fn is_some(&self) -> bool { ... }
    pub fn is_none(&self) -> bool { ... }
}

// Reference conversion methods
impl<T> Option<T> {
    pub fn as_ref<'r>(&'r self) -> Option<&'r T> { ... }
    pub fn as_mut<'r>(&'r mut self) -> Option<&'r mut T> { ... }
}

// Methods in common with Result
impl<T> Option<T> {
    pub fn as_slice<'r>(&'r self) -> &'r [T] { ... }
    pub fn as_mut_slice<'r>(&'r mut self) -> &'r mut [T] { ... }
    pub fn unwrap(self) -> T { ... }
    pub fn unwrap_or(self, def: T) -> T { ... }
    pub fn unwrap_or_else(self, f: || -> T) -> T { ... }
    pub fn expect(self, msg: &str) -> T { ... }
    pub fn map<U>(self, f: |T| -> U) -> Option<U> { ... }
    pub fn map_or<U>(self, def: U, f: |T| -> U) -> U { ... }
    pub fn mutate(&mut self, f: |T| -> T) -> bool { ... }
    pub fn mutate_or_set(&mut self, def: T, f: |T| -> T) -> bool { ... }
    pub fn iter<'r>(&'r self) -> SomeItem<&'r T> { ... }
    pub fn mut_iter<'r>(&'r mut self) -> SomeItem<&'r mut T> { ... }
    pub fn move_iter(self) -> SomeItem<T> { ... }
    pub fn and<U>(self, optb: Option<U>) -> Option<U> { ... }
    pub fn and_then<U>(self, f: |T| -> Option<U>) -> Option<U> { ... }
    pub fn or(self, optb: Option<T>) -> Option<T> { ... }
    pub fn or_else(self, f: || -> Option<T>) -> Option<T> { ... }
}

// Option-specific methods
impl<T> Option<T> {
    pub fn take(&mut self) -> Option<T> { ... }
    pub fn filtered(self, f: |t: &T| -> bool) -> Option<T> { ... }
    pub fn while_some(self, f: |v: T| -> Option<T>) { ... }
}

impl<T: Default> Option<T> 
    pub fn unwrap_or_default(self) -> T { ... }
}
~~~

### Changes from the old Option API

Old API             | New API
--------------------|--------------------------------------------------
`.get_ref()`        | `.as_ref().unwrap()`
`.get_mut_ref()`    | `.as_mut().unwrap()`
`.take_unwrap()`    | `take().unwrap()`

## Result API

Even though we can't express the abstraction over Option-like things without
higher kinded types, we should aim to make the methods implemented on `Result`
consistent with the `Option` API.

When implemented directly on `Result`, these methods would be biased towards
the `Ok` variant. The `_err`-suffixed methods would be removed, replaced by a
`ForErr` adapter type that would allow for the Option-style methods to be
implemented again - this time being biased towards the `Err` variant.

The `take`, `filtered` and `while_some` methods from `Option` are not included
in the `Result` API because they require a default `None` value.

~~~rust
// core::result

use std::any::Any { ... }

pub enum Result<T, E> { Ok(T), Err(E) }

// Result-specific predicates
impl<T, E> Result<T, E> {
    pub fn is_ok(&self) -> bool { ... }
    pub fn is_err(&self) -> bool { ... }
}

// Reference conversion methods
impl<T, E> Result<T, E> {
    pub fn as_ref<'a>(&'a self) -> Result<&'a T, &'a E> { ... }
    pub fn as_mut<'a>(&'a mut self) -> Result<&'a mut T, &'a E> { ... }
}

// Conversion methods
impl<T, E> Result<T, E> {
    pub fn to_option(self) -> Option<T> { ... }
    pub fn for_err(self) -> ForErr<T, E> { ... }
}

// Methods in common with Option
impl<T, E> Result<T, E> {
    pub fn as_slice<'r>(&'r self) -> &'r [T] { ... }
    pub fn as_mut_slice<'r>(&'r mut self) -> &'r mut [T] { ... }
    pub fn unwrap_or(self, def: T) -> T { ... }
    pub fn unwrap_or_else(self, f: || -> T) -> T { ... }
    pub fn expect(self, msg: &str) -> T { ... }
    pub fn map<U>(self, f: |T| -> U) -> Result<U, E> { ... }
    pub fn map_or<U>(self, def: U, f: |T| -> U) -> U { ... }
    pub fn mutate(&mut self, f: |T| -> T) -> bool { ... }
    pub fn mutate_or_set(&mut self, def: T, f: |T| -> T) -> bool { ... }
    pub fn iter<'r>(&'r self) -> OkItem<&'r T> { ... }
    pub fn mut_iter<'r>(&'r mut self) -> OkItem<&'r mut T> { ... }
    pub fn move_iter(self) -> OkItem<T> { ... }
    pub fn and<U>(self, other: Result<U, E>) -> Result<U, E> { ... }
    pub fn and_then<U>(self, f: |T| -> Result<U, E>) -> Result<U, E> { ... }
    pub fn or(self, other: Result<T, E>) -> Result<T, E> { ... }
    pub fn or_else(self, f: || -> Result<T, E>) -> Result<T, E> { ... }
}

impl<T, E: Show> Result<T, E> {
    pub fn unwrap(self) -> T { ... }
}

pub struct ForErr<T, E>(pub Result<T, E>);

// Conversion methods
impl<T, E> ForErr<T, E> {
    pub fn to_result(self) -> Result<T, E> { ... }
    pub fn to_option(self) -> Option<E> { ... }
}

// Methods in common with Option
impl<T, E> ForErr<T, E> {
    pub fn as_slice<'r>(&'r self) -> &'r [E] { ... }
    pub fn as_mut_slice<'r>(&'r mut self) -> &'r mut [E] { ... }
    pub fn unwrap(self) -> E { ... }
    pub fn unwrap_or(self, def: E) -> E { ... }
    pub fn unwrap_or_else(self, f: || -> E) -> E { ... }
    pub fn expect(self, msg: &str) -> E { ... }
    pub fn map<F>(self, f: |E| -> F) -> Result<T, F> { ... }
    pub fn map_or<F>(self, def: F, f: |E| -> F) -> F { ... }
    pub fn mutate(&mut self, f: |E| -> E) -> bool { ... }
    pub fn mutate_or_set(&mut self, def: E, f: |E| -> E) -> bool { ... }
    pub fn iter<'r>(&'r self) -> ErrItem<&'r E> { ... }
    pub fn mut_iter<'r>(&'r mut self) -> ErrItem<&'r mut E> { ... }
    pub fn move_iter(self) -> ErrItem<E> { ... }
    pub fn and<F>(self, other: Result<T, F>) -> Result<T, F> { ... }
    pub fn and_then<F>(self, f: |E| -> Result<T, F>) -> Result<T, F> { ... }
    pub fn or(self, other: Result<T, E>) -> Result<T, E> { ... }
    pub fn or_else(self, f: || -> Result<T, E>) -> Result<T, E> { ... }
}

impl<T: Show, E> ForErr<T, E> {
    pub fn unwrap(self) -> E { ... }
}
~~~

### Added methods

- `Result::for_err`
- `Result::unwrap`
- `Result::expect`
- `Result::map_or`
- `Result::mutate`
- `Result::mutate_or_set`
- `Result::iter`
- `ForErr::unwrap`
- `ForErr::unwrap_or`
- `ForErr::unwrap_or_else`
- `ForErr::expect`
- `ForErr::map_or`
- `ForErr::mutate`
- `ForErr::mutate_or_set`
- `ForErr::iter`
- `ForErr::mut_iter`
- `ForErr::move_iter`
- `ForErr::and`
- `ForErr::and_then`
- `ForErr::or`
- `ForErr::or_else`

### Changes from the old Result API

Old API             | New API
--------------------|--------------------------------------------------
`.ok()`             | `.to_option()`
`.err()`            | `.for_err().to_option()`
`.map_err(...)`     | `.for_err().map(...)`
`.or_else(...)`     | `.for_err().or(...)`

# Drawbacks

Working with `Err` values is more verbose. It also increases the complexity of
the `Result` API.

# Alternatives

What other designs have been considered? What is the impact of not doing this?

- `Result::for_err` could be renamed to `Result::err`. This would be more
  succinct.
- Instead of using the `ForErr` adapter, we could instead just use the `_err`
  suffix. This would be slightly more convenient, but would incompatible
  with a move to a trait-based API in the future.
- Instead of implementing the `Ok`-biased methods directly on `Result`, a
  `ForOk` adapter and `for_ok` method could be added, mirroring `ForErr`. This
  would be more inconvenient for the common case, but would be more
  symmetrical with the `Err` biased API.
- We could remove the `Err` methods entirely, because they bloat the API
  considerably.

# Unresolved questions

To preserve the current behaviour of `Result::{unwrap, unwrap_err}`,
`Result::unwrap` and `ForErr::unwrap` must be implemented on `E: fmt::Show`
and `T: fmt::Show` respectively. It is unclear how to make this completely
mirror the `Option` API.

It might be a good idea to think more deeply about how the API will be
structured with higher-kinded types, dividing the `impl`s up in a more
fine-grained way, annotating them with comments referring to their future
traits. This would be similar to how the STL uses the idea of 'concepts', even
though they are still not implemented in the language. This kind of thinking
could also extend to the library defined pointer types, which could be tied
together in the future with a higher kinded abstraction.
