- Feature Name: unsafe_static
- Start Date: 2020-05-29
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Replace static muts with unsafe statics, which are error prone.

# Motivation
[motivation]: #motivation

Since before 1.0, Rust has had a feature called `static mut`, which allows users to define statics
to which mutable references can be taken. Actually taking a mutable reference is unsafe. However,
this feature is incredibly footgunny and almost impossible to use correctly, since creating a
mutable reference to shared memory is undefined behavior, even if you never dereference that
reference.

Unsafe statics would be a better way to get the same effect, by requiring any mutable access to
still pass through an interior mutability primitive like `UnsafeCell`.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

statics can now be declared `unsafe`:

```rust
unsafe static X: i32 = 0;
```

Statics that are declared unsafe are different from other statics in the following ways:

1. All accesses to them are unsafe.
2. The `Sync` check is disabled for unsafe statics. They are not required to implement `Sync`.

Unlike static muts, it is not possible to take a mutable reference to an unsafe static. However
users can achieve the effect of a static mut using an `UnsafeCell`:

```rust
unsafe static X: UnsafeCell<i32> = UnsafeCell::new(0);
// safer equivalent of:
static mut X: i32 = 0;
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Use cases

Users using this like a static mut are responsible for guaranteeing that they never construct a
mutable reference to the inner value in an unsynchronized manner. This is essentially the same as
static mut's use case, but more sound in the face of the actual current definition off undefined
behavior.

Users could also achieve the effect of an unsafe thread local, which has no synchronization, but
which they guarantee they only access from one thread. For example, they could use it with `Cell`
and `RefCell`. (They can do this with static mut too, but with this API they are guaranteed never to
get a totally unsynchronized mutable reference, even if they are responsible for guaranteeing thread
locality.)

Another interesting use case would be to use a totally Sync, shared primitive, but introduce new
invariants which must be upheld. In other words, unsafe statics could behave like unsafe traits in
creating new invariant abstraction points. It's unclear what use cases this would have, but it seems
like something which could be compelling.

## Deprecating static mut

In coordination with adding this feature, we would also deprecate the `static mut` feature. If an
automatic fix from `static mut` to `unsafe static` could be performed with rustfix, we would
completely remove the static mut feature in an upcoming edition.

# Drawbacks
[drawbacks]: #drawbacks

This churns the ecosystem, though only for a very small feature: static mut. It also adds more
surface area to the language by creating a new kind of unsafe item.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

We could just deprecate static mut without adding any replacement. Users using static mut can get a
similar effect by creating a newtype around UnsafeCell, which implements Sync

```rust
struct RacyUnsafeCell<T>(UnsafeCell<T>);

unsafe impl<T: Sync> Sync for RacyUnsafeCell<T> { }
```

We could also provide this abstraction in the standard library along with deprecating static mut.

Unsafe statics are a preferable option because they introduce a clear and explicit point at which
you identify the additional invariants needed to access this static safely, by making the
declaration point of the static which does not obey the safe static rules unsafe. It is a more
direct expression of user intent than creating a RacyUnsafeCell: you want to create a static that
cannot be proven by the compiler to uphold the requirements of a safe static, so you create an
unsafe static.
