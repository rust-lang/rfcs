- Feature Name: missing_derefs
- Start Date: 2016-06-09
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Add `&move` pointers, the `DerefMove` trait, and the unsafe
`DerefPure` traits. Allow using `DerefPure` derefs in lvalues.

# Motivation
[motivation]: #motivation

Rust's `Box` has a few features that are not implementable by library
traits: it is possible to match on `Box` with box patterns, and to
move out of it.

User-defined types also want to make use of these features.

Also, it is not possible to use pattern matching on structures that
contain smart pointers. We would want this to be possible.

# Detailed design
[design]: #detailed-design

## DerefPure

Add a `DerefPure` trait:
```Rust
pub unsafe trait DerefPure : Deref {}
```

Implmenenting the `DerefPure` trait tells the compiler that dereferences
of the type it is implemented for behave like dereferences of normal
pointers - as long as the receiver is borrowed, the compiler can merge,
move and remove calls to the `Deref` methods, and the returned pointer
will stay the same.

Also, the methods must not panic and (if `DerefMove` is implemented) may
be called on a partially-initialized value.

If a type implements `DerefPure`, then user-defined dereferences of it
are implemented with a `deref` lvalue projection as if they were a built-in
pointer.

Types implementing `DerefPure` can be used in `box` patterns. This works
like all the other reference patterns. For example, if `Vec` implements
`DerefPure` and `BasicBlockData.statements` is a `Vec`:

```Rust
match self.basic_blocks[*start] {
    BasicBlockData {
        statements: box [],
        terminator: ref mut terminator @ Some(Terminator {
	    kind: TerminatorKind::Goto { .. }, ..
	}), ..
    } => { /* .. */ }
    _ => return
};
```

## &move

Add a new mutability `move`. `&move` references are references that own their
contents, but not the memory they refer to. Of course, `*move` raw pointers
exist too as another family of newtyped integers.

`&move` references are covariant in both their lifetime and type parameters
(and `*move` pointers are covariant in their type parameter) for the same
reason `Box` is - unlike `&mut`, there is nobody to return control to that
can observe the changed type.

When parsing a `move` closure, `&move |..` is parsed as `& (move |..` -
as creating a `move` closure and taking an immutable reference to it, rather
than creating a non-moving closure and taking an `&move` reference to it. Of
course, you can force the other choice by explicit parentheses - `&move (|..`.

Unlike some other proposals, the [RFC1214] rules remain the same - a
`&'a move T` reference requires that `T: 'a`. We may want to relax these
rules.

Dereferences of `&move` references are tracked by the move checker like
local variables. It is possible to move values in and out, both partially
and completely, and the move checker will make sure that when the `&move`
goes out of scope, the contained value is dropped only if it was not moved out.

Dereferences of `*move` pointers behave similarly, except they are not dropped
when they go out of scope, and are always treated by the move checker as fully
initialized.

For example, this is well-behaved code with `&move` but double-drops if
`t` is changed to an `*move`:

```Rust
fn example(t: &move Option<Box<u32>>) {
    drop(*t);
    *t = None;
}
```

Outside of the move checker, `&move` references always have valid contents.
If you want to create a temporary uninitialized `&move` reference, you can
use `mem::forget`:

```Rust
unsafe fn move_val_init_from_closure<T, F: FnOnce() -> T>(p: *mut T, f: F)
{
    let ptr = &move *p;
    mem::forget(*ptr);
    *ptr = f(); // if `f` panics, `*ptr` is not dropped.
}
```

An `&move x.y` borrow, unlike the other borrows, actually moves out of
`x.y`. This applies to all borrows, including implicit reborrows. I think
this would make implicit reborrows useless, but it is the consequence of
the rules.

Of course, it is possible to borrow `&move` references as either `&` or
`&mut`, and not possible to borrow `&` or `&mut` references as `&move`.

## DerefMove

This allows moving out of user-defined types.

Add a `DerefMove` trait:
```Rust
pub trait DerefMove: DerefMut + DerefPure {
    fn deref_move(&mut self) -> &move Self::Target;
}
```

The `DerefMove` trait can't be called directly, in the same manner
as `Drop` and for exactly the same reason - otherwise, this
would be possible:

```Rust
fn example<T>(data: T) -> T {
    let b = Box::new(data);
    drop(b.deref_move());
    *b // would return dropped data
}
```

It is also restricted in the same manner as `Drop` with regards to
implementations and dropck. Of course, a type is allowed to implement
both `Drop` and `DerefMove` - `Box` implements them both.

If a type implements `DerefMove`, then the move checker treats it
as a tree:

x
    - *x

It is not possible to move out of the ordinary fields of such a
type, similarly to types implementing `Drop`.

When such a type is dropped, `*x` (aka `x.deref_move()`) is dropped
first if it was not moved from already, similarly to `Box` today. Then
the normal destructor and the destructors of the fields are called.

This means that `Vec<T>` can be implemented as

```Rust
pub struct Vec<T> {
    buf: RawVec<T>,
    len: usize,
}

impl<T> ops::Deref for Vec<T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        unsafe {
            let p = self.buf.ptr();
            assume(!p.is_null());
            slice::from_raw_parts(p, self.len)
        }
    }
}

impl<T> ops::DerefMut for Vec<T> {
    /* id. */
}

impl<T> ops::DerefMove for Vec<T> {
    #[unsafe_destructor_blind_to_params]
    fn deref_move(&mut self) -> &move [T] {
        unsafe {
            let p = self.buf.ptr();
            assume(!p.is_null());
            slice::from_raw_parts_move(p, self.len)
        }
    }
}

unsafe impl<T> ops::DerefPure for Vec<T> {}

// no `Drop` impl is needed - `RawVec` handles
// that
```

# Drawbacks
[drawbacks]: #drawbacks

The new mutability kind adds a significant amount of complexity to the
middle of the user-visible type-system. I think the move checker already
supports most of that complexity, but there probably will be unexpected
problems.

There may be some way to have the entire thing safe. However, all proposals
that I have seen were very complicated.

# Alternatives
[alternatives]: #alternatives

We may want to relax the [RFC1214] rules to allow `&'static move T` as an
equivalent to `Unique<T>`.

Add more features of the move checker to the type-system, e.g. strongly
linear `&out`. That is quite complex, and requires more considerations
wrt. panics.

# Unresolved questions
[unresolved]: #unresolved-questions

How to formalize the requirements for `DerefPure`?

Are there any issues with implementing `&move` lvalues "just like other lvalues"?

How do we do exhaustiveness checking on `box` patterns if there are also
normal patterns? For example, how do we discover that the box pattern is
useless here:

```Rust
match x: Rc<Option<_>> {
    Rc { .. } => {}
    box None => {},
}
```

[RFC1214]: https://github.com/rust-lang/rfcs/blob/master/text/1214-projections-lifetimes-and-wf.md