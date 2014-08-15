- Start Date: 2014-07-23
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

Summary
=======

Add a new lang-item-defined trait, `DerefMove`, which allows moves out of custom
pointer types.

Motivation
==========

There has been some effort to remove special casing from `Box`—[RFC PR
130](https://github.com/rust-lang/rfcs/pull/130) and RFC PRs
[139](https://github.com/rust-lang/rfcs/pull/139) and
[112](https://github.com/rust-lang/rfcs/pull/112) are examples of this trend.
However, there is still one more problem with `Box` that would have to be fixed
for `Box` to be properly special-case-free (bar the construction syntax and
patterns, which is mostly acceptable because of the plan to support `box`
for other pointer types): `Box` pointers can be moved out of, even though
none of the `Deref*` traits support moves.

Detailed design
===============

Add a new trait, `DerefMove`, with a corresponding lang item, `deref_move`, and
define it in `core::ops`:

```rust
#[lang="deref_move"]
pub trait DerefMove<Result> {
    /// The method called to move out of a dereference
    fn deref_move(self) -> Result;
}
```

In order to determine which method to use when dereferencing with `*`, the
following rules are used:

1. If the type implements `DerefMove` but not `Deref` or `DerefMut`, all
   dereferences are done using `deref_move`.
2. If a reference is being taken to a dereference (e.g. `&*ptr`), call `deref`
   or `deref_mut`, depending on the mutability of the reference.
3. If a value is being dereferenced and implements `DerefMove` and `Deref`:
  1. If the type is `Copy`, call `deref_move`.
  2. Otherwise:
    1. If the type implements `Deref<T>` where `T: Copy`, call `*deref`.
    2. Otherwise, call `deref_move`.
4. If a value is being dereferenced and does not implement `DerefMove` but does
   implement `Deref`, use `deref`.

This applies to implicit dereferences as well.

This means that (at least with DST) it is now possible remove all special
treatment of `Box` by the borrow checker. To do this, we would have add an
implementation of `DerefMove` for `Box` in the standard library roughly as
follows:

```rust
impl<T> DerefMove<T> for Box<T> {
    fn deref_move(self) -> T {
        let Box(ptr) = self;
        std::ptr::read(ptr)
    }
}
```

and also add similar implementations of `Deref` and `DerefMut` to the standard
library. With these changes, it’s now possible to finally remove all previously
built-in dereference functionality for `Box` from the language, because all
dereference functionality is now provided by the standard library.

Drawbacks
=========

Adds yet another `Deref*` trait and another lang item, adding complexity to the
language.

Alternatives
============

* Do nothing, and blame `Box`’s special-casing on the fact that it is a lang
  item anyway.
* Add a `DerefSet` trait as well, for assignments of the form `*ptr = val`.

Unresolved questions
====================

None.
