- Feature Name: `generic-futures`
- Start Date: 2023-05-17
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC proposes adding a (defaulted) generic parameter to the `core::future::Future` trait to allow more flexibility in `Future::poll`'s second argument (`context`). 

# Motivation
[motivation]: #motivation

With the introduction of the async/await syntax (and one could argue, before that), futures have become a core aspect of Rust. However, the current signature poses a few issues:
- The context (and the types it's built from) is not ABI-stable: this is a big problem for plugin systems that wish to expose asynchronous methods, as the futures need to be wrapped in ABI-safe adapters that often impose allocating a new waker for every call to `poll`.
- Asynchronous frameworks such as `tokio` must resort to side-channels to allow futures to access the executor to perform certain tasks, such as spawning new tasks (the use-case is picked from [`feature(waker_getters)`](https://github.com/rust-lang/rust/issues/96992) which considers using the access to raw vtables to downcast the waker into a specialised waker that can accomplish such tasks). 
- The `core::task::Waker` type is the common denominator to all interactions with futures, making adjusting its API and implementation especially trying, as it affects the whole ecosystem indiscriminately. It also imposes its vtable/pointer pair layout to every executor that would wish to construct wakers, regardless of whether this structure is desirable to them.


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Futures could be re-defined as follows (at this stage of the RFC, I'll prefix newly introduced symbol with `_`, and am highly flexible on renaming these symbols):

```rust
mod future {
	/// This trait is meant to be implemented by all wakers, including `core::task::Waker`.
	/// It mainly differs from `std::task::Wake` by the fact that it is independent from `alloc::sync::Arc`,
	/// and can therefore exist in `core`
	pub trait _WakerTrait: Clone {
		fn wake(self) {self.wake_by_ref()}
		fn wake_by_ref(&self);
	}

	pub trait Future<_W: _WakerTrait = core::task::Waker> {
		pub fn poll(self: Pin<&mut Self>, cx: &mut Context<'_, _W>) -> Poll<Self::Output>;
	}
}
mod task {
	impl _WakerTrait for Waker { /* ... */ }
	pub struct Context<'_, _W: _WakerTrait = Waker> { /* ... */ }
}
```

This means that when implementing futures by hand with `impl core::future::Future for MyFuture`, the implementation would likely be over-specialized, and a lint offering to generalize the implementation to `impl<_W: _WakerTrait> core::future::Future<_W> for MyFuture` could help boost the adoption of this new feature.

Note also that the generic parameter should be spread to `IntoFuture` and any other section of the stdlib that currently uses `Future`.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The core of the proposal is the API change, which is explained in the guide-level explaination.

The current mechanics of constructing the state machine for `async` blocks would still be applicable. Care should be taken to generate implementations of future for the entire intersection of the sets of wakers supported by each awaited future. This should be doable by generating:
```rust
impl<_W> core::future::Future<_W> for StmThatAwaitsF1andF2
where
	_W: _WakerTrait,
	F1: core::future::Future<_W>,
	F2: core::future::Future<_W>, {
	/* I leave this one to the compiler */
}
```

# Drawbacks
[drawbacks]: #drawbacks

- Risk of API break: great care should be taken to ensure that there isn't some weird corner case where type inference would start failing on existing code once the defaulted generic is added. I've seen type inference failures in the wild when dealing with default generics in the wild, but I don't think this would happen for traits.
- Risk of fragmentation and/or added complexity of executor APIs: since executors will be able to specialise their futures for their executors, this may cause additional fragmentation in the async ecosystem. Maintaining support for traditional futures _and_ specialised futures could have an impact on executor APIs and implementations, as these futures would need to be kept distinct.
	- Note that this risk also exists with the runtime specialisation discussed in the `waker_getters` RFC, with the clear advantage for this RFC that fragmentation could be detected and handled at compile time.
- Due to `std::task::Wake` _and_ `Waker` already exisiting, naming the new trait for wakers may be awkward, and the three symbols may become confusing to newcomers. 

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- This design opens up new flexibility in the asynchronous ecosystem, at no extra cost to users that do not care about it.
- The generic parameter could be deeper down the waker rabbit hole (`RawWaker` for privacy, for example), but `Waker` level seems like a good point for the generic to be located. Any deeper than `RawWaker` would over-constrain the design of alternative wakers, as they would be forced into a pointer-vtable design regardless of their need.
- This new flexibility holds especially great value to users that wish to pass futures accross the FFI boundary, as this enables desgins that don't need to allocate to clone wakers coming from objects that aren't trusted to have the same ABI.
- Due to the fact that `Future` is a lang-item, this is both a compiler and library proposal.
- This proposal should have very little effect on legibility, as this additional generic will generally be infered.

# Prior art
[prior-art]: #prior-art

RFC #1398 proposed adding the defaulted generics for allocators to `Vec` and `Box`.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Is defaulting the added generic truly sufficient to guarantee that no existing code would break?
	- I've fooled around in test files to check for edge cases, but haven't found any example of code breaking when simulating the change.
	- I've previously attempted to implement this RFC, and test it on a large repo with extensive use of futures, but the repo failed to compile on nightly to begin with.
	- I've encountered cases where default generics would break existing code on structures in the past, typically where constructors are considered ambiguous despite all ambiguities being defaulted. This issue shouldn't arise with traits, but is the source of my concern here.

# Future possibilities
[future-possibilities]: #future-possibilities

This RFC could become a precedent in extending existing traits through defaulted generics.