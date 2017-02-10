- Feature Name: uninitialized-uninhabited
- Start Date: 2017-02-09
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Deprecate the usage of `mem::uninitialized::<T>` for possibly-uninhabited `T`.
Specifically:
  * Add a built-in `Inhabited` trait which is automatically implemented for
    types known to have at least 1 possible value.
  * Require an `Inhabited` bound on `T` for `uninitialized::<T>`.
  * Add a `MaybeUninit<T>` union to the standard library for representing
    possibly-initialized data in a more type-system-aware way.

# Motivation
[motivation]: #motivation

The concept of "uninitialized data" is extremely problematic when it comes into
contact with uninhabited types.

For any given type, there may be valid and invalid bit-representations. For
example, the type `u8` consists of a single byte and all possible bytes can be
sensibly interpreted as a value of type `u8`. By contrast, a `bool` also
consists of a single byte but not all bytes represent a `bool`: the
bit vectors `[00000000]` (`false`) and `[00000001]` (`true`) are valid `bool`s
whereas `[00101010]` is not. By further contrast, the type `!` has no valid
bit-representations at all. Even though it's treated as a zero-sized type, the
empty bit vector `[]` is not a valid representation and has no interpretation
as a `!`.

As `bool` has both valid and invalid bit-representations, an uninitialized
`bool` cannot be known to be invalid until it is inspected. At this point, if
it is invalid, the compiler is free to invoke undefined behaviour. By contrast,
an uninitialized `!` can only possibly be invalid. Without even inspecting such
a value the compiler can assume that it's working in an impossible
state-of-affairs whenever such a value is in scope. This is the logical basis
for using a return type of `!` to represent diverging functions.  If we call a
function which returns `bool`, we can't assume that the returned value is
invalid and we have to handle the possibility that the function returns.
However if a function call returns `!`, we know that the function cannot
sensibly return. Therefore we can treat everything after the call as dead code
and we can write-off the scenario where the function *does* return as being
undefined behaviour.

The issue then is what to do about `uninitialized::<T>()` where `T = !`?
`uninitialized::<T>` is meaningless for uninhabited `T` and is currently
instant undefined behaviour when `T = !` - even if the "value of type `!`" is
never read. The type signature of `uninitialized::<!>` is, after all, that of a
diverging function:

```rust
fn mem::uninitialized::<!>() -> !
```

Yet calling this function does not diverge! It just breaks everything then eats
your laundry instead.

This RFC proposes restricting the use of `uninitialized` to types that are
known to be inhabited. I also propose an addition to the standard library of a
`MaybeUninit` type which offers a much more principled way of handling
uninitialized data and can be used to hold uninitialized uninhabited types
without lying to the type system.

# Detailed design
[design]: #detailed-design

Add the following trait as a lang item:

```rust
#[lang="inhabited"]
trait Inhabited {}
```

This trait is automatically implemented for all inhabited types.

Change the type of `uninitialized` to:

```rust
pub unsafe fn uninitialized<T: Inhabited>() -> T
```

Before enforcing this change we should have a future-compatibility warning
cycle and urge people to switch to `MaybeUninit` where possible.

Add a new type to the standard library:

```rust
union MaybeUninit<T> {
    uninit: (),
    value: T,
}
```

For an example of how this type can replace `uninitialized`, consider the
following code:

```rust
fn catch_an_unwind<T, F: FnOnce() -> T>(f: F) -> Option<T> {
    let mut foo = unsafe {
        mem::uninitialized::<T>()
    };
    let mut foo_ref = &mut foo as *mut T;

    match std::panic::catch_unwind(|| {
        let val = f();
        unsafe {
            ptr::write(foo_ref, val);
        }
    }) {
        Ok(()) => Some(foo);
        Err(_) => None
    }
}
```

The problem here is, by the time we get to the third line we're already saying
we have a value of type `T`; which we don't, and which for `T = !` is
impossible. We can use `MaybeUninit` instead like this:

```rust
fn catch_an_unwind<T, F: FnOnce() -> T>(f: F) -> Option<T> {
    let mut foo: MaybeUninit<T> = MaybeUninit {
        uninit: (),
    };
    let mut foo_ref = &mut foo as *mut MaybeUninit<T>;

    match std::panic::catch_unwind(|| {
        let val = f();
        unsafe {
            ptr::write(&mut (*foo_ref).value, val);
        }
    }) {
        Ok(()) => {
            unsafe {
                Some(foo.value)
            }
        },
        Err(_) => None
    }
}
```

Here, we've moved the `unsafe` block to the return position where we actually
know we have a `T`.  This is fine for use with `!` because we can only reach
the line that extracts `value` from `foo` if `f` returned normally (which it
couldn't have).

# How We Teach This
[how-we-teach-this]: #how-we-teach-this

Correct handling of uninitialized data is an advanced topic and should probably be
left to The Rustonomicon.

The documentation for `uninitialized` should explain the motivation for these
changes and direct people to the `MaybeUninit` type.

# Drawbacks
[drawbacks]: #drawbacks

This could be a rather large breaking change depending on how many people are
currently calling `uninitialized::<T>` with a generic `T`. However all such
code is already somewhat future-incompatible as it will malfunction (or panic)
if used with `!`.

Another drawback is that the `Inhabited` trait leaks private information about
types. Consider a type with the following definition:

```rust
pub struct AmIInhabited {
    _priv: (),
}
```

If this type is exported from its crate or module it will also export an impl
for `Inhabited`. Now suppose the definition is changed to:

```rust
pub struct AmIInhabited {
    _priv: !,
}
```

The author of the crate may expect this change to be private and its effects
contained to within the crate. But in making this change they've also stopped
exporting the `Inhabited` impl, causing potential breakages for downstream
users.

Although this is a problem in principal it's unlikely to come up much in
practice. I doubt it's common for someone to change an inhabited exported type
to being uninhabited; and any library consumers would already be unable to use
`uninitialized` with a generic `T`, they'd have to be using it with the
exported type specifically to hit a regression.

# Alternatives
[alternatives]: #alternatives

* Not do this.
* Just make `uninitialized::<!>` panic instead (making `!`'s behaviour
  surprisingly inconsistent with all the other types).
* Adopt these rules but not the `Inhabited` trait. Instead make `uninitialized`
  behave like `transmute` does today by having the restrictions on its type
  arguments enforced outside the trait system.
* Not add the `Inhabited` trait. Instead add `MaybeUninit` as a lang item,
  adopt the `Transmute` trait RFC, and change the signature of `uninitialized` to

    ```rust
    pub unsafe fn uninitialized<T>() -> T
        where MaybeUninit<T>: Transmute<T>
    ```
  to get the same effect.
* Rename `Inhabited` to `Uninitialized` and move the `uninitialized` function
  to being an unsafe method on the trait.

# Unresolved questions
[unresolved]: #unresolved-questions

None known.

# Future directions

Ideally, Rust's type system should have a way of talking about initializedness
statically. In the past there have been proposals for new pointer types which
could safely handle uninitialized data. We should seriously consider pursuing
one of these proposals.

This RFC could be a possible stepping-stone to completely deprecating
`uninitialized`. We would need to see how `MaybeUninit` is used in practice
beforehand, and it would be nice (though not strictly necessary) to also
implement the additional pointer types beforehand so that users of
`uninitialized` have the best possible options available to migrate to.

