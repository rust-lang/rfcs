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

The `deref_move` method would be called in any of the following situations:

* Something is being dereferenced, and the only `Deref*` trait it implements is
  `DerefMove`;
* Something is being dereferenced, and while it *does* implement `Deref` and/or
  `DerefMut`, it also implements `DerefMove` and a value is being moved out of
  the dereference.

This applies to implicit derefences as well.

Remove all special treatment of `Box` by the borrow checker. Instead, `Box`
implements `DerefMove` in the standard library roughly as follows:

```rust
impl<T> DerefMove<T> for Box<T> {
    fn deref_move(self) -> T {
        let Box(ptr) = self;
        std::ptr::read(ptr)
    }
}
```

Drawbacks
=========

Adds yet another `Deref*` trait and another lang item, adding complexity to the
language.

Alternatives
============

* Do nothing, and blame `Box`’s special-casing on the fact that it is a lang item
  anyway.
* Add a `DerefSet` trait as well, for assignments of the form `*ptr = val`.

Unresolved questions
====================

None.
