- Feature Name: `manuallydrop_deref_move`
- Start Date: 2023-07-30
- RFC PR: [rust-lang/rfcs#3466](https://github.com/rust-lang/rfcs/pull/3466)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary

Extend the special‐case move‐out‐of‐deref behavior of `Box<T>` to
`ManuallyDrop<T>`. Additionally, allow partial moves out of a `T` stored inside
`ManuallyDrop<T>` even when there is a `Drop` impl for `T`.

# Motivation

Often, instead of dropping a struct, one wants to move out of one of its fields.
However, this is impossible to do in safe code when the struct implements
`Drop`, requiring use of `unsafe` APIs like `std::ptr::read`, or
runtime‐expensive workarounds like wrapping the field in an `Option` and using
`take()`.

```rust
pub struct Foo(String);

impl Drop for Foo {
    fn drop(&mut self) { /* ... */ }
}

pub struct Bar(String);

impl Foo {
    pub fn into_bar(self) -> Bar {
        let m = core::mem::ManuallyDrop::new(self);
        Bar(unsafe { core::ptr::read(&m.0) }) // Need to use `unsafe`
    }
}
```

I ran into this limitation while working on the [`async-lock` library](https://github.com/smol-rs/async-lock/blob/8045684f996b15b3dd9bfd621cfc3864d3760923/src/rwlock.rs#L879-L883).
It has also been discussed elsewhere:

- [Blog post by @withoutboats](https://without.boats/blog/ownership/#e0509)
- [Pre‐RFC from 2024](https://internals.rust-lang.org/t/destructuring-droppable-structs/20993)
- [Pre‐RFC from 2019](https://internals.rust-lang.org/t/pre-rfc-destructuring-values-that-impl-drop/10450).
- [Blog post from 2018](https://phaazon.net/blog/rust-no-drop) (recommends
  `mem::uninitialized` as a workaround!)
- [RFC #1180](https://github.com/rust-lang/rfcs/pull/1180)
- [Internals thread from 2014](https://internals.rust-lang.org/t/destructuring-structs-which-implement-drop/137)

# Explanation

In today’s Rust, `Box<T>` has the unique capability that it is possible to move
out of a dereference of it.

```rust
let b: Box<String> = Box::new("hello world".to_owned());
let s: String = *b; // `b`’s backing allocation is dropped here
```

Partial moves are also permitted.

```rust
let s: String;
{
    let b: Box<(String, String)> = Box::new(("hello".to_owned(), "world".to_owned()));
    s = b.1;
    // `b.0` and `b`’s backing allocation dropped here
}
```

This RFC extends this capability to `ManuallyDrop`.

```rust
use core::mem::ManuallyDrop;

let m: ManuallyDrop<(String, String)> = ManuallyDrop::new(("hello".to_owned(), "world".to_owned()));
drop(m.1); // `m.1` moved out of here
// `m.0` is never dropped
```

In addition, partial moves out of a `ManuallyDrop<T>`’s contents are allowed
even when there is a `Drop` impl for `T`.

```rust
struct Foo(String, String);

impl Drop for Foo {
    fn drop(&mut self) {
        println!("down to the ground");
    }
}

let m: ManuallyDrop<Foo> = ManuallyDrop::new(Foo("hello".to_owned(), "world".to_owned()));
let s: String = m.1; // `m.1` moved out of here
// `m.0` is never dropped, and nothing is printed.
```

The example from the motivation section would be rewritten as:

```rust
impl Foo {
    fn into_bar(self) -> Bar {
        let m = core::mem::ManuallyDrop::new(foo);
        Bar(m.0)
    }
}
```

# Drawbacks

- Adds more “magic” to the language.
- This change would give safe Rust code a new capability (moving out of fields
  of `Drop`-implementing structs). It’s currently possible for the soundess of
  `unsafe` code to rely on this capability not existing (though any such API is
  also unsound if combined with [`replace_with`](https://docs.rs/replace_with)).
  For example:

```rust
use core::hint::unreachable_unchecked;

/// If the bomb is dropped while armed,
/// it explodes and triggers undefined behavior.
pub struct Bomb {
    is_armed: bool,
}

impl Drop for Bomb {
    fn drop(&mut self) {
        if self.is_armed {
            println!("💥 BOOM 💥");

            // SAFETY: the only way for arbitrary safe code to obtain a `Bomb`
            // is via `DefuseWrapper::new()`. Because `DefuseWrapper` implements `Drop`,
            // it is impossible to move the `Bomb` out of it.
            // `DefuseWrapper`’s destructor ensures that
            // `is_armed` is set to `false` before the `Bomb` is dropped,
            // so this branch is unreachable.
            unsafe { unreachable_unchecked(); }
        }
    }
}

/// Disarms the bomb before dropping it.
pub struct DefuseWrapper {
    pub bomb: Bomb,
}

impl Drop for DefuseWrapper {
    fn drop(&mut self) {
        self.bomb.is_armed = false;
    }
}

impl DefuseWrapper {
    pub fn new() -> Self {
        DefuseWrapper {
            bomb: Bomb {
                is_armed: true,
            }
        }
    }
}
```

# Rationale and alternatives

## Versus `DerefMove`

A more general mechanism for move‐out‐of‐deref, which would subsume `Box`’s
special‐case support, has long been desired. There have been [three](https://github.com/rust-lang/rfcs/pull/178)
[different](https://github.com/rust-lang/rfcs/pull/1646) [RFCs](https://github.com/rust-lang/rfcs/pull/2439)
attempting it, and extensive discussion going back to 2014. However, these
proposals have all gone nowhere; finding a good design for this API seems to be
a hard problem. Also, such an API would not subsume this RFC, as partial moves
out of structs with `Drop` impls would still need hard‐coded compiler support.
In light of these facts, I think adding an existing lang‐item type to an
existing special case is justified while we wait for a more general `DerefMove`.

## Versus a different API

It’s possible that that partial moves out of `Drop` types scould be supported
via a different API, such as an attribute, macro, or even silently omitting the
`Drop` call (possibly with a lint). However, the design presented by this RFC
hase several desirable properties:

- **Familiarity:** move‐out‐of‐deref is already familiar to Rust developers who
  have worked with `Box`, and `ManuallyDrop` is the recommended API for avoiding
  drop. So, combining these behaviors should be intuitive to users.
- **Explicitness**: There is a prominent indication in the source code
  (`ManuallyDrop`) that a `drop` impl is being skipped.
- **Soundness:** with `ManuallyDrop`, fields that are not moved out of will be
  leaked. This includes fields that are inaccessible due to privacy. Therefore,
  if a type temporarily violates the safety invariant of a private field,
  relying on its `Drop` impl to restore the invariant before the field is
  dropped, this RFC will preserve the soundness of that API.
  - APIs that expose broken safety invariants in *public* fields *will* become
    unsound, as explained [above](#drawbacks). However, these are already
    incompatible with [`replace_with`](https://docs.rs/replace_with).
  - The tradeoff for preserving soundness is that it is possible to accidentally
    leak memory by forgetting to move out of a field. One strategy programmers
    can use to mitigate this is to make the `Drop`‐impementing type a newtype
    struct with a single field.
- **Interaction with `Copy`:** the presence or absence of a `Copy`
  implementation can determine whether a particular segment of code performs a
  copy or a move. Under a model where the `Drop::drop()` call is simply omitted
  following a partial move, whether `drop()` is called can therefore depend on
  the presence of a `Copy` impl—which could be added in a semver‐compatible
  dependency upgrade. The design proposed by this RFC avoids this pitfall.

# Prior art

This RFC addresses a problem unique to Rust’s move and destructor semantics, so
there is no analogue in other languages.

# Unresolved questions

None, as far as I am aware.

# Future possibilities

A more general `DerefMove` mechanism is the natural next step, though it would
not subsume this RFC, as explained in the [Rationale](#rationale-and-alternatives)
section.
