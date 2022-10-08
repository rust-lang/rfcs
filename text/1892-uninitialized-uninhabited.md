- Feature Name: `uninitialized_uninhabited`
- Start Date: 2017-02-09
- RFC PR: [rust-lang/rfcs#1892](https://github.com/rust-lang/rfcs/pull/1892)
- Rust Issue: [rust-lang/rust#53491](https://github.com/rust-lang/rust/issues/53491)

# Summary
[summary]: #summary

Deprecate `mem::uninitialized::<T>` and `mem::zeroed::<T>` and replace them with
a `MaybeUninit<T>` type for safer and more principled handling of uninitialized
data.

# Motivation
[motivation]: #motivation

The problems with `uninitialized` centre around its usage with uninhabited
types, and its interaction with Rust's type layout invariants. The concept of
"uninitialized data" is extremely problematic when it comes into contact with
types like `!` or `Void`.

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

This problem is most prominent with `!` but also applies to other types that
have restrictions on the values they can carry.  For example,
`Some(mem::uninitialized::<bool>()).is_none()` could actually return `true`
because uninitialized memory could violate the invariant that a `bool` is always
`[00000000]` or `[00000001]` -- and Rust relies on this invariant when doing
enum layout. So, `mem::uninitialized::<bool>()` is instantaneous undefined
behavior just like `mem::uninitialized::<!>()`. This also affects `mem::zeroed`
when considering types where the all-`0` bit pattern is not valid, like
references: `mem::zeroed::<&'static i32>()` is instantaneous undefined behavior.

## Tracking uninitializedness in the type

An alternative way of representing uninitialized data is through a union type:

```rust
union MaybeUninit<T> {
    uninit: (),
    value: T,
}
```

Instead of creating an "uninitialized value", we can create a `MaybeUninit`
initialized with `uninit: ()`. Then, once we know that the value in the union
is valid, we can extract it with `my_uninit.value`. This is a better way of
handling uninitialized data because it doesn't involve lying to the type system
and pretending that we have a value when we don't. It also better represents
what's actually going on: we never *really* have a value of type `T` when we're
using `uninitialized::<T>`, what we have is some memory that contains either a
value (`value: T`) or nothing (`uninit: ()`), with it being the programmer's
responsibility to keep track of which state we're in. Notice that creating a
`MaybeUninit<T>` is safe for any `T`! Only when accessing `my_uninit.value`,
we have to be careful to ensure this has been properly initialized.

To see how this can replace `uninitialized` and fix bugs in the process,
consider the following code:

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

Naively, this code might look safe. The problem though is that by the time we
get to `let mut foo_ref` we're already saying we have a value of type `T`. But
we don't, and for `T = !` this is impossible. And so if this function is called
with a diverging callback it will invoke undefined behaviour before it even
gets to `catch_unwind`.

We can fix this by using `MaybeUninit` instead:

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

Note the difference: we've moved the unsafe block to the part of the code which is
actually unsafe - where we have to assert to the compiler that we have a valid
value. And we only ever tell the compiler we have a value of type `T` where we
know we actually do have a value of type `T`. As such, this is fine to use with
any `T`, including `!`. If the callback diverges then it's not possible to get
to the `unsafe` block and try to read the non-existent value.

Given that it's so easy for code using `uninitialized` to hide bugs like this,
and given that there's a better alternative, this RFC proposes deprecating
`uninitialized` and introducing the `MaybeUninit` type into the standard
library as a replacement.

# Detailed design
[design]: #detailed-design

Add the aforementioned `MaybeUninit` type to the standard library:

```rust
pub union MaybeUninit<T> {
    uninit: (),
    value: ManuallyDrop<T>,
}
```

The type should have at least the following interface
([Playground link](https://play.rust-lang.org/?gist=81f5ab9a7e7107c9583de21382ef4333&version=nightly&mode=debug&edition=2015)):

```rust
impl<T> MaybeUninit<T> {
    /// Create a new `MaybeUninit` in an uninitialized state.
    ///
    /// Note that dropping a `MaybeUninit` will never call `T`'s drop code.
    /// It is your responsibility to make sure `T` gets dropped if it got initialized.
    pub fn uninitialized() -> MaybeUninit<T> {
        MaybeUninit {
            uninit: (),
        }
    }

    /// Create a new `MaybeUninit` in an uninitialized state, with the memory being
    /// filled with `0` bytes.  It depends on `T` whether that already makes for
    /// proper initialization. For example, `MaybeUninit<usize>::zeroed()` is initialized,
    /// but `MaybeUninit<&'static i32>::zeroed()` is not because references must not
    /// be null.
    ///
    /// Note that dropping a `MaybeUninit` will never call `T`'s drop code.
    /// It is your responsibility to make sure `T` gets dropped if it got initialized.
    pub fn zeroed() -> MaybeUninit<T> {
        let mut u = MaybeUninit::<T>::uninitialized();
        unsafe { u.as_mut_ptr().write_bytes(0u8, 1); }
        u
    }

    /// Set the value of the `MaybeUninit`. The overwrites any previous value without dropping it.
    pub fn set(&mut self, val: T) {
        unsafe {
            self.value = ManuallyDrop::new(val);
        }
    }

    /// Extract the value from the `MaybeUninit` container.  This is a great way
    /// to ensure that the data will get dropped, because the resulting `T` is
    /// subject to the usual drop handling.
    ///
    /// # Unsafety
    ///
    /// It is up to the caller to guarantee that the the `MaybeUninit` really is in an initialized
    /// state, otherwise this will immediately cause undefined behavior.
    pub unsafe fn into_inner(self) -> T {
        std::ptr::read(&*self.value)
    }

    /// Get a reference to the contained value.
    ///
    /// # Unsafety
    ///
    /// It is up to the caller to guarantee that the the `MaybeUninit` really is in an initialized
    /// state, otherwise this will immediately cause undefined behavior.
    pub unsafe fn get_ref(&self) -> &T {
        &*self.value
    }

    /// Get a mutable reference to the contained value.
    ///
    /// # Unsafety
    ///
    /// It is up to the caller to guarantee that the the `MaybeUninit` really is in an initialized
    /// state, otherwise this will immediately cause undefined behavior.
    pub unsafe fn get_mut(&mut self) -> &mut T {
        &mut *self.value
    }

    /// Get a pointer to the contained value. Reading from this pointer will be undefined
    /// behavior unless the `MaybeUninit` is initialized.
    pub fn as_ptr(&self) -> *const T {
        unsafe { &*self.value as *const T }
    }

    /// Get a mutable pointer to the contained value. Reading from this pointer will be undefined
    /// behavior unless the `MaybeUninit` is initialized.
    pub fn as_mut_ptr(&mut self) -> *mut T {
        unsafe { &mut *self.value as *mut T }
    }
}
```

Deprecate `uninitialized` with a deprecation messages that points people to the
`MaybeUninit` type. Make calling `uninitialized` on an empty type trigger a
runtime panic which also prints the deprecation message.

# How We Teach This
[how-we-teach-this]: #how-we-teach-this

Correct handling of uninitialized data is an advanced topic and should probably
be left to The Rustonomicon. There should be a paragraph somewhere therein
introducing the `MaybeUninit` type.

The documentation for `uninitialized` should explain the motivation for these
changes and direct people to the `MaybeUninit` type.

# Drawbacks
[drawbacks]: #drawbacks

This will be a rather large breaking change as a lot of people are using
`uninitialized`. However, much of this code already likely contains subtle
bugs.

# Alternatives
[alternatives]: #alternatives

* Not do this.
* Just make `uninitialized::<!>` panic instead (making `!`'s behaviour
  surprisingly inconsistent with all the other types).
* Introduce an `Inhabited` auto-trait for inhabited types and add it as a bound
  to the type argument of `uninitialized`.
* Disallow using uninhabited types with `uninitialized` by making it behave
  like `transmute` does today - by having restrictions on its type arguments
  which are enforced outside the trait system.

# Unresolved questions
[unresolved]: #unresolved-questions

None known.

# Future directions

Ideally, Rust's type system should have a way of talking about initializedness
statically. In the past there have been proposals for new pointer types which
could safely handle uninitialized data. We should seriously consider pursuing
one of these proposals.

