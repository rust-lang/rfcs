- Feature Name: `scope_drop`
- Start Date: 2022-02-12
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)
- **Status:** Rough first draft, comments welcome
  - Summary, Reference-level explanation, Unresolved questions, Future possibilities sections: pretty good
  - Motivation, Guide-level explanation: need revision
  - Drawbacks, Rationale sections need to be filled in
  - Prior art is bare-bones, but I don't have more information to add

# Summary
[summary]: #summary

Another stab at almost-linear types. Draws heavily from http://aidancully.blogspot.com/2021/12/less-painful-linear-types.html

Add a trait `ScopeDrop` that can be used to determine which types can be dropped implicitly. On an unwinding panic, these types are dropped the same way as today (with `Drop::drop` and recursively dropping subvalues). However, during unexceptional control flow, the compiler will give an error if a value of a type which does not implement `ScopeDrop` exits scope without being consumed. Include in the core library a zero-sized struct `PhantomLinear` which has the negative impl `!ScopeDrop`. Automatically implement `ScopeDrop` when all component types implement it. Make `ScopeDrop` assumed by default on type parameters and associated types, with `?ScopeDrop` syntax to explicitly declare support for these types.

# Motivation
[motivation]: #motivation

Scope based implicit drop is a user-friendly way to make cleaning up resources properly easy, and leaking resources hard. For most cases, this works exceedingly well. However, the interface of `Drop` is quite limited.

* From the library's perspective, `Drop::drop` takes `&mut self`, and thus must leave self in a valid state after running. For related reasons, you cannot partially move out of a type which implements `Drop`, making it difficult to implement other functions which consume a `T: Drop` by breaking it up into pieces. This proposal gives library writers a way to *guarantee* that their types are consumed (and thus closed, finalized, or otherwise cleaned up) in normal control flow, with the flexibility to move away from `fn(self) -> ()` to `fn close(self) -> Result<Ok, Err>` (close which may produce data/error), `fn complete(self, return_value) -> ()` (complete a request with a value), or even `fn try_cleanup(self) -> Option<(Self, Error)>` (cleanup may fail, and you need to try again).

* From the user's perspective, `mem::drop<T>` has type `fn(T) -> ()`, which leaves no room for falliable cleanup, nor for providing extra information (such as a return value) on completion. Furthermore, `mem::drop` may get called implicitly whenever a variable goes out of scope. When the action we want to take when we are done using a value fits the simple pattern `fn(self) -> ()`, this is extremely convenient. We use the value when we need it, and trust it to go away on it's own once we don't. But for some types, it is extremely unlikely that forgetting about them before calling some completion API is correct behavior. This is sometimes handled by implementing `drop` as `panic!`, or slightly less agressivly implementing `drop` as closing with status ``Well, I forgot what I was doing with this, sorry!''. This proposal turns those runtime checks into compile time checks.

I am aware of two relevant factors influencing the design of `Drop::drop` today.

1. In case of an unwinding panic, values may need to be cleaned up at almost any point. I believe this is why `Drop` has no interface for providing extra data to or returning data from the destructor: it doesn't have a place in the exceptional flow case. This proposal adresses panic by essentially giving up. We acknowledge that at almost any time in the program, there may be a need to unwind the stack due to a `panic!`, and in this case we may have to settle for ``best-effort'' cleanup. This might mean leaking memory, it might mean ignoring errors on file close, it might mean aborting the process. We must only preserve the safety guarantees of Rust, and ensure that the value is left in a valid, though perhaps nonsensical, state. The `Drop` trait is very well suited to these scenarios, and this proposal keeps the behavior during unwinding panics exactly the same. As an added bonus, for people working in an environment where `panic = "abort"`, they can be sure that `drop` is *never* called.

2. Writing drop glue code (the stuff that recursively drops all your fields) manually is a lot of busywork. I believe this is part of the reason that `Drop::drop` takes `&mut self`: so that the compiler-generated drop glue can safely run afterwards. This proposal adresses drop glue by having most types implement `ScopeDrop`. If you have a `struct T { a: A, b: B, c: C }`, and `A` is the only type that doesn't implement `ScopeDrop`, then a consumer for `T` can be as simple as
```rust
fn foo(t: T, x: X) {
    let {a, b, c} = t;
    A::close(a, x);
}
```
This lets you write only the essential glue code: if `A` doesn't have a consumer `fn(A) -> ()`, then we don't want the compiler to try and write one for us. But the fields `b` and `c` can be implicitly dropped, if you don't have a use for them. (Caution: this may change the drop order of struct fields)

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## The `ScopeDrop` Trait

Most types take care of their own cleanup when they go out of scope. A `Vec` deallocates its storage, a `MutexGuard` unlocks the mutex, a `Rc` or `Arc` decrements the reference count, and so on, along with types that don't need any cleanup like integers. These types automatically implement the `ScopeDrop` trait, which means you can drop them whenever you want just by letting their scope expire, or calling `mem::drop`.

However, some types would much rather have an explicit step before they go away. This could take the form of a file that might fail to close and wants to tell you about it, or a request that needs your signature as the last step before completion (and you don't want to just forget about it accidentally), among other possibilities.

For example, you might have
```rust
type File // Does not implement ScopeDrop

impl File {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, IoError>;
    fn write(&mut self, buf: &mut [u8]) -> Result<usize, IoError>;
    fn sync_all(&mut self) -> Result<(), IoError>
    
    /// This function tells you about errors resulting from the close of a file.
    fn close_catch_errors(self) -> Result<(), IoError> {
        self.sync_all()
    }
    
    /// Warning: close can run into errors which this function will ignore.
    fn close_ignore_errors(self) {
        let _ = self.close_catch_errors(self);
    }
}
```
(Disclaimer: This RFC does not suggest changing the standard library file handling API, rather if an external crate wanted to provide this API this RFC would give them the tools for good compile-time checking.)

Then to use it, you would need to call `close_catch_errors` or `close_ignore_errors` to close the file: it won't happen automatically when the file goes out of scope. This makes it easy to do the correct thing (handle the errors), and makes it so that ignoring errors on file close is intentional, not an accident.
```rust
fn main() -> Result<(), IoError> {
    let mut file = File::create("foo.txt")?;
    file.write("Hello")?;
    file.close_catch_errors()?;
    // Omitting this last line causes a compiler error:
    // file cannot be implicitly dropped; type File does not implement ScopeDrop.
    // Using instead file.close_ignore_errors(), we cannot use the ? to propagate errors up to the main result.
}
```

When you see an error saying that type `T` does not implement `ScopeDrop`, it is a signal that the API expects an explicit action to clean up the referenced variable. Like `Sized`, by default type parameters and associated types are assumed to implement `ScopeDrop`. If you are implementing a generic API and don't need to drop values, consider adding the `?ScopeDrop` bound.

Note that you can still use `mem::forget` to leak an object, so unsafe code still can't rely on destructors being called for safety.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Invariant
[invariant]: #invariant

`Drop::drop::<T>` may be called while the stack is *not* unwinding (in unexceptional control flow) if and only `T: ScopeDrop + Drop`.

## ScopeDrop

We add a new `unsafe` trait `ScopeDrop`, which is assumed by default on type parameters and associated types [as `Sized` is](https://doc.rust-lang.org/stable/reference/trait-bounds.html#sized) (including syntax `?ScopeDrop` to relax that bound). `ScopeDrop` is an auto trait [like `Sync`](https://doc.rust-lang.org/stable/reference/special-types-and-traits.html#auto-traits).

```
// core/marker.rs
#[lang = "unexceptional_drop"]
pub unsafe auto trait ScopeDrop {}
```

In order to uphold [the invariant][invariant], implementations of `ScopeDrop` have to follow a contract.

### Contract
[contract]: #contract

A value `t: T` is correct to drop if

1. If `T: Drop` then `T: ScopeDrop`. **AND**
2. After calling `Drop::drop(&mut t)` (if `T: Drop`, otherwise skip), all values that are to be recursively droppped are correct to drop.

A type `T` may only implement `ScopeDrop` if all values `t: T` are correct to drop.

## Compiler support

When the compiler cannot determine that a `!ScopeDrop` value is completely consumed before the end of its scope, produce an error.

## Minimal Standard Library Tweaks

### PhantomLinear

In the standard library:

```rust
// core/marker.rs
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct PhantomLinear;

impl !ScopeDrop for PhantomLinear {}
```

This should not need to be a lang item.

### Copy requires ScopeDrop

Add bound `Copy: ScopeDrop` [discussion][copy-scopedrop]
```
// core/marker.rs
pub trait Copy: Clone + ScopeDrop {}
```

### Allow forgetting `!ScopeDrop` values.

We weaken the bounds on `mem::forget` and `ManuallyDrop`: [discussion][memforget]
```rust
// core/mem/mod.rs
pub const fn forget<T: ?ScopeDrop>(t: T) {
    let _ = ManuallyDrop::new(t);
}

// core/mem/manually_drop.rs
#[lang = "manually_drop"]
pub struct ManuallyDrop<T: ?Sized + ?ScopeDrop> {
    value: T,
}

unsafe impl<T: ?ScopeDrop> ScopeDrop for ManuallyDrop<T> {}
```

## Corner cases

### Self type of a trait

[Unresolved question][#default-bound-self-type] if the `Self` type of a trait should have an implicit `ScopeDrop` bound by default, or not like `Sized`.

# Drawbacks
[drawbacks]: #drawbacks

* Adds another `?Trait` like `Sized` that is assumed by default.
* Adds another auto trait.

# Rationale and alternatives

- Why is this design the best in the space of possible designs?
- What other designs have been considered and what is the rationale for not choosing them?
- What is the impact of not doing this?

The blog post that inspired this RFC: http://aidancully.blogspot.com/2021/12/less-painful-linear-types.html

Miscelaneous, non-exhaustive collection of similar prior proposals:
* https://github.com/rust-lang/rfcs/issues/814
* https://github.com/rust-lang/rfcs/issues/523 (this feature request would seem to be resolved by this proposal)
* https://github.com/rust-lang/rfcs/issues/2642 (an approach with a variation on `#[must_use]`)
* https://internals.rust-lang.org/t/pre-pre-rfc-nodrop-marker-trait/15682

# Prior art
[prior-art]: #prior-art

I am not aware of other languages with similar features.

This feature seems to rest heavily on having a substructural type system, and Rust is the only such language I am familiar with.

I would be excited to learn about examples of prior art in other languages.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

## `mem::forget`
[memforget]: #memforget

to resolve: during RFC discussion

Should `mem::forget::<T>` require `T: ScopeDrop`? I don't believe so.

If the bound `ScopeDrop` is added to all type parameters and associated types in the standard library, I do not see a way to write
`mem::forget` without using `unsafe`. However, I do not think that this is a promise we want to make.

## Default Bound Self Type
[copy-scopedrop]: #copy-scopedrop

to resolve: after implementation

Should the `Self` type of a trait should have an implicit `ScopeDrop` bound by default, or not like `Sized`.

Hopefully someone more familiar with object safety can provide input here.

## `Copy: ScopeDrop`

to resolve: after implementation

We propose making `T: Copy` imply `T: ScopeDrop`, but this is not necessary for the desired semantics. My reasoning is that when a type is `Copy`, the compiler makes copies relatively freely, and it may not always be obvious which copies need to be explicitly consumed, so it is best to ensure that `Copy` types can be implicitly dropped as well. This closes off the possibility of having copyable relevant (use at least once) types, but there is no problem with `Clone + !ScopeDrop`. If we can determine that adding this constraint wouldn't help usability at all, we might want to make `Copy` independent of `ScopeDrop`.

## Destructuring `PhantomLinear`

to resolve: after implementation

We have a choice for whether or not `PhantomLinear` should have a private field or not.

If `PhantomLinear` has no private fields, we should be able to consume values without using `mem::forget` by destructuring.

In the case that negative implementations are stablized for use outside of the standard library, users will be able to define their own version that makes the opposite choice.

# Future possibilities
[future-possibilities]: #future-possibilities

## Audit standard library

Go through the standard library and find type parameters and associated types that can be generalized to `?ScopeDrop`. Need to be careful to preserve backwards compatibility, but should be profitable. It may be possible to define lints that detect when the `ScopeDrop` bound is unnecessary, and list those out or even automatically add the `?SizedDrop` bounds.

However, I believe that the `SizedDrop` feature is useful even without generalizing the standard library or the ecosystem. Right now, we don't have experience with using this feature, and cannot be expected to know the right abstractions. Adding this feature to the language allows users to start experimenting.

## Making generic types `?ScopeDrop`

All current Rust types implement `?ScopeDrop` automatically, and `mem::drop<T>` works without bounds on `T`. For backwards compatibility, it is necessary for type parameters and associated types to be `ScopeDrop` by default, the same as how `Sized` works. However, we could consider a future edition where users are expected to add explicit `ScopeDrop` bounds on type parameters and associated types if they need to implicitly drop values of such type. This would make `ScopeDrop` a bit less magical, and a bit more explicit, as well as bringing it in line with the auto traits `Send`, `Sync`, and `Unpin`. This would be an intrusive change in codebases which drop generic types with any frequency, but might be easy to automate.

## Potential additions to standard library

Or, alternatively, an external crate designed for working with `?ScopeDrop`.

### Affine Escape Hatch

When interfacing with code that has not been generalized to support `?ScopeDrop` types, it is useful to have an escape hatch: a way to safely wrap a `?ScopeDrop` type into a type that can be dropped. The type `ManuallyDrop<T>` is the simplest way to do this: essentially saying: "I give you permission to forget about this value".

If you want to add a destructor that runs on drop rather than just leaking the value, this is also possible.

```rust
struct AffineWrapper<T: DefaultDestroy> {
    contents: ManuallyDrop<T>
}

impl<T: ?ScopeDrop, F: FnOnce(T) -> () + ?ScopeDrop> Drop for AffineWrapper<T, F> {
    fn drop(&mut self) {
        unsafe { ManuallyDrop::take(self.contents) }.destroy()
    }
}

unsafe impl<T: ?ScopeDrop> ScopeDrop for AffineWrapper<T> {}
```

### `DefaultDestroy` trait

Just as the `Default` trait gives a default way to create values, `DefaultDestroy` gives give a default way to destroy values. (bikeshedding: maybe `DefaultDrop`? `Consume`?)

```rust
trait DefaultDestroy {
    fn destroy(self);
}

impl<T: ScopeDrop> DefaultDestroy for T {
    fn destroy(self) {}
}
```

Types implement this if they have a good default destructor that they want to protect from implicitly being called.

### Drop bomb

A type that will abort the process if it is ever dropped.

```
pub struct Bomb { _marker: PhantomLinear }

impl Drop for Bomb {
    fn drop(&mut self) {
        panic!();
    }
}
```

### `mem::exceptional_drop`

When `panic = "unwind"`, it should be possible to get a drop function in safe code without the `ScopeDrop` bound by
```
fn exceptional_drop<T: ?ScopeDrop>(t: T) {
    panic::catch_unwind(AssertUnwindSafe(move || {
        let capture: T = t; // Need our closure to capture t by move
        panic!()
    }))
}
```

This is, however, rather silly.

Note that if `panic = "abort"`, `exceptional_drop` would abort, upholding the invariant that `Drop::drop` never gets called on a `!ScopeDrop` type when `panic = "abort"`.

# Documentation Notes
[documentation-notes]: #documentation-notes

A list of places to add documentation for this feature to.

* Add `ScopeDrop` to the list of auto traits in https://doc.rust-lang.org/stable/reference/special-types-and-traits.html#auto-traits
* Document `?ScopeDrop` next to `?Sized` in https://doc.rust-lang.org/stable/reference/trait-bounds.html#sized
* Also document `?ScopeDrop` in the book next to `?Sized` at https://doc.rust-lang.org/book/ch19-04-advanced-types.html#dynamically-sized-types-and-the-sized-trait
