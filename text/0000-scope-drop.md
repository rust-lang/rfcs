- Feature Name: `scope_drop`
- Start Date: 2022-02-12
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)
- **Status:** First draft, looking for comments/indication of interest before opening PR to formally submit this as an RFC

# Summary
[summary]: #summary

Another stab at almost-linear types. Draws heavily from http://aidancully.blogspot.com/2021/12/less-painful-linear-types.html

Add a trait `ScopeDrop` that can be used to determine which types can be dropped implicitly. On an unwinding panic, these types are dropped the same way as today (with `Drop::drop` and recursively dropping subvalues). However, during unexceptional control flow, the compiler will give an error if a value of a type which does not implement `ScopeDrop` exits scope without being consumed.

`mem::forget` is still valid for any type: unsafe code can not assume that destructors of any kind are run for safety.

Include in the core library a zero-sized struct `PhantomLinear` which has the negative impl `!ScopeDrop`. Automatically implement `ScopeDrop` when all component types implement it. Make `ScopeDrop` assumed by default on type parameters and associated types, with `?ScopeDrop` syntax to explicitly declare support for these types.

# Motivation
[motivation]: #motivation

Scope based implicit drop is a user-friendly way to make cleaning up resources properly easy, and leaking resources hard. For most cases, this works exceedingly well. However, in some APIs it would be nice to force the API user to call some method rather than allowing the implicit drop to clean up (For example, to force the user to think about errors on cleanup, or to recieve extra data needed for cleanup).

A workaround used today is panicking on drop, but this is a runtime check for what could be checked at compile time. Having a strong type system that catches many errors at compile time is one of Rust's strengths; it makes sense to allow types to opt out of implicit drop when they decide that implicit drop is wrong.

Unwinding complicates this picture: for an unwinding panic, the compiler needs to clean up everything on the stack without a chance for the user to provide or recieve data. `Drop::drop` is a reasonably good fit for this case: it lets types specify custom cleanup behavior and reduces the amount of boilerplate by recursively dropping the subvalues.

In many cases, is also convenient to have cleanup code that is automatically called when a value goes out of scope, and we want to do the same thing on unwinding and end of scope often enough that it makes sense to combine those two facilities. But while every type needs to handle cleanup on unwind, it is reasonable to declare that, for some types, we don't want the convenience of implicit drop on end of scope.

Unwinding panics are an exceptional case. It is important to handle them without breaking Rust's memory safety guarantees, and useful to allow types to customize their behavior when dropped because of unwinding. But silently and implicitly making every variable which is not consumed before the end of its scope to do the same thing (you can check `thread::panicking`, but still) as when handling an unwinding panic is frustrating when the best-effort cleanup can you can do leaks memory, silently ignores errors, or sends a placeholder value. The current use of `drop` combines the exceptional case of unwinding with the extremely common case of ending a scope: we don't necessarily want to do the same thing for both, and don't need to use the API forced by the limitations of unwinding panic in the case of normal control flow.

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
1:  fn main() -> Result<(), IoError> {
2:      let mut file = File::create("foo.txt")?;
3:      file.write("Hello")?;
4:      file.close_catch_errors()?;
        // Omitting this last line causes a compiler error:
        // value file cannot be implicitly dropped; type File does not implement ScopeDrop.
        // value defined on line 2, scope ends on line 5
        // value is not consumed by last use on line 4.
        // Using instead file.close_ignore_errors(), we cannot use the ? to propagate errors up to the main result.
5:  }
```

When you see an error saying that type `T` does not implement `ScopeDrop`, it is a signal that the API expects an explicit action to clean up the referenced variable. Like `Sized`, by default type parameters and associated types are assumed to implement `ScopeDrop`. If you are implementing a generic API and don't need to drop values, consider adding the `?ScopeDrop` bound.

Note that you can still use `mem::forget` to leak an object, so unsafe code still can't rely on destructors being called for safety. ([discussion][memforget]). However, unsafe code can rely on the invariant that if `T: !ScopeDrop` then `Drop::drop::<T>` will only be called while the stack is unwinding. If you are working in an environment where `panic = "abort"`, as is common in some bare-metal or embedded Rust applications, this means you are guaranteed that your type is never dropped! (Though again, it can be forgotten about.)

If there is a `panic!` before the file is closed, `Drop::drop` will be called as normal: in this case you don't have an opportunity to catch any errors reported by file close because you are already panicking.

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

* This does not eliminate the use of `mem::forget`, so APIs like scoped thread guards are still broken: unsafe code cannot rely on destructors of any kind to be run for safety.
* Adds another `?Trait` like `Sized` that is assumed by default. Because `?ScopeDrop` expands the class of possible types, people writing generic code are faced with the extra task of determining whether or not to allow `?ScopDrop` types. This increases the burden on the ecosystem.
* Adds another auto trait, which adds another piece of magic that happens without the user explicitly requesting it.

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

## Unwinding

How to deal with unwinding is one of the issues that has complicated previous proposals for linear types in Rust. When unwinding is possible, at almost any point the stack could need to be cleaned up: what is to be done with linear types in this case? In postponed RFC https://github.com/rust-lang/rfcs/issues/814, a `Finalize` trait was proposed that behaves identically to `Drop` but is only called in the unwinding case. Here we propose simply reusing the `Drop` trait for custom cleanup in both the unwinding and scope drop cases: If you need different behavior you can check `thread::panicking`.

This is a bit of a "punt": we still allow linear types to be scooped up and disposed of by panic at any point. I argue that doing so makes integrating `ScopeDrop` into Rust much less difficult than trying to forbid their use in contexts where panic is possible.

Types can individually decide whether they want to abort, to leak, or to attempt a best-effort cleanup in the exceptional case of unwinding while being confident that users will not accidentally default to this suboptimal behavior.

## `mem::forget`
[memforget]: #memforget

Should `mem::forget::<T>` require `T: ScopeDrop`? I don't believe so.

If the bound `ScopeDrop` is added to all type parameters and associated types in the standard library, I do not see a way to write
`mem::forget` without using `unsafe`. However, I do not think that this is a promise we want to make.

There are many APIs for which obvious generalization to `!ScopeDrop` types allows writing `mem::forget` in safe code. For example, while `Rc` needs to destroy the value it is holding at an unpredictable time, as long as we provide a method `fn destroy(self) -> ()` this is a seemingly sound system. But in the case of a reference cycle, `destroy` never gets called, so choosing `fn destroy(self) { panic!() }` will allow writing `mem::forget` in safe code.

As another example, it seems eminently reasonable to pass `!ScopeDrop` values to a separate thread: the new thread takes ownership which entails responsibility for cleaning up. But if the new thread goes into an infinite loop, then the value never gets cleaned up and we can again safely forget values.

For these reasons, I argue that `mem::forget` should not require `T: ScopeDrop`.

## Why a trait?

Rather than changing the type system, we could consider using a lint to discover when a type we annotate as linear gets implicitly dropped. However, to do this in a compositional way requires computing for generic functions which types they may implicitly drop, and we would want to propagate this information across crates. At this point, it seems clear that adding a lint for unused linear values would require computing and communicating much the same data as using a trait, and using a trait gives a systematic way of integrating the feature into the language.

## Branded completion tokens

Branding with unique lifetimes offers some of the benefits of linear types. (https://plv.mpi-sws.org/rustbelt/ghostcell/paper.pdf) By requiring the production of branded completion tokens we can require a callback to consume a passed in value rather than dropping it. But this also has limitations: it requires use to be confined to a callback with a higher-order lifetime parameter, and they cannot be stored in collections. Branded types do not seem sufficient to replace all potential uses of linear types.

## What are the consequences of not doing this?

If we do not adopt this proposal, types for which implicitly drop is a footgun will remain reliient on runtime checking. The case of accidental async future cancellation by drop has been brought up. Another example of an API that would benefit from `?ScopeDrop` is when you are expected to eventually complete every recieved request with a completion status (the example I am familiar with is Windows Driver Framework requests: https://docs.microsoft.com/en-us/windows-hardware/drivers/wdf/completing-i-o-requests).

The issue of disabling implicit drops seems to come up frequently enough to demonstrate some level of desire for this feature in the Rust community. I believe that this feature does require language support: I do not believe it is possible to faithfully emulate compile-time checked `?ScopeDrop` without compiler support.

# Prior art
[prior-art]: #prior-art

I am not aware of other languages with similar features.

This feature seems to rest heavily on having a substructural type system, and Rust is the only such language I am familiar with.

I would be excited to learn about examples of prior art in other languages.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

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

Go through the standard library and find type parameters and associated types that can be generalized to `?ScopeDrop`. Need to be careful to preserve backwards compatibility, but should be profitable. It may be possible to define lints that detect when the `ScopeDrop` bound is unnecessary, and list those out or even automatically add the `?ScopeDrop` bounds.

However, I believe that the `ScopeDrop` feature is useful even without generalizing the standard library or the ecosystem. Right now, we don't have experience with using this feature, and cannot be expected to know the right abstractions. Adding this feature to the language allows users to start experimenting.

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
