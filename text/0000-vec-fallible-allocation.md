- Feature Name: `vec_fallible_allocation`
- Start Date: 2022-20-04
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

`Vec` has many methods that may allocate memory (to expand the underlying storage, or shrink it down). Currently each of these method rely on "infallible"
allocation, that is any failure to allocate will call the global OOM handler, which will (typically) panic. Even if the global OOM handler does not panic, the
return type of these method don't provide a way to indicate the failure.

Currently `Vec` does have a `try_reserve` method that uses "fallible" allocation: if `try_reserve` attempts to allocate, and that allocation fails, then the
return value of `try_reserve` will indicate that there was a failure, and the `Vec` is left unchanged (i.e., in a valid, usable state). We propose adding
more of these `try_` methods to `Vec`, specifically for any method that can allocate.

Unlike most RFCs, we are not suggesting that this proposal is the best among many alternatives (in fact we know that adding more `try_` methods is undesirable
in the long term: see the "Drawbacks" section below), instead we are suggesting this as a way forward to unblock important work (see the "Motivations" section
below) while we explore other alternatives. We have no plans to stabilize this feature in its current form and fully expect that these methods will be removed
in the future.

# Motivation
[motivation]: #motivation

The motivation for this change is well documented by other RFCs, such as the proposed [fallible_allocation feature](https://github.com/rust-lang/rfcs/pull/3140)
and the accepted [fallible_collection_alloc feature](https://github.com/rust-lang/rfcs/blob/master/text/2116-alloc-me-maybe.md).

As a brief summary, there are environments where dynamic allocation is required and failing to allocate is both expected and able to be handled. For example,
in OS Kernel environments (such as [Linux](https://lore.kernel.org/lkml/CAHk-=wh_sNLoz84AUUzuqXEsYH35u=8HV3vK-jbRbJ_B-JjGrg@mail.gmail.com/)), some embedded
systems, high-reliability systems (such as databases) and multi-user services (where a single request may fail without the entire service halting, such as a
web server).

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

One often unconsidered failure in a software system is running out of memory (commonly referred to as "Out-of-memory" or OOM):
if code attempts to dynamically allocate memory, and there is an insufficient amount of available memory on the system, how should this be handled?

In most applications the failure to allocate means that the code cannot make any progress, and so the appropriate response is to exit immediately indicating that there was a failure (i.e., to `panic`).
Consider the Rust Compiler itself, if the compiler cannot allocate memory then it cannot fail that part of the compilation and attempt to continue the rest of the compilation.
The only appropriate action is to fail the entire compilation and emit an error message.

Since it is appropriate for most circumstances, the Rust standard library defaults to panicking on allocation failure.
This approach is referred to as "infallible allocation": from the perspective of a function caller the allocation can never fail, because if it does then the called function will `panic` and the caller won't live to observe the failure.

There are, however, environments where allocation failures are both expected and required to be handled more gracefully.
Consider a multi-user service such as a database or a web server: if a request comes in to the service that requires allocating more memory than is available on the system, then the service can respond with a failure for just that request, and it can continue to service other requests.
This approach is referred to as "fallible allocation": each function that may allocate needs to indicate to its caller if an attempt to allocate has failed, and that caller must explicitly handle that possibility.

Within the Rust standard library, one can identify "fallible allocation" functions by their names being prefixed with `try_` and their return types being an instance of `Result`.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Currently any function in `alloc` that may call the global OOM handler (which panics) rather than let its caller handle all allocation failures is marked with `#[cfg(not(no_global_oom_handling))]` (see <https://github.com/rust-lang/rust/pull/84266>),
which the [fallible_allocation feature](https://github.com/rust-lang/rfcs/pull/3140) proposes to change to a check of the `infallible_allocation` feature
(i.e., `#[cfg(feature = "infallible_allocation")]`). Any such method in `Vec` will have a corresponding "fallible allocation" method prefixed with `try_` that
returns a `Result<..., TryReserveError>` and so is usable if `no_global_oom_handling` is enabled (or `infallible_allocation` is disabled).

Under the covers, both the fallible and infallible methods call into the same implementation function which is generic on the error type to return (either
`!` for the infallible version or `TryReserveError` for the fallible version) - this allows to maximum code reuse, while also avoiding any performance overhead
for error handling in the infallible version.

List of APIs to add:

```rust
try_vec!

impl<T> Vec<T> {
    pub fn try_with_capacity(capacity: usize) -> Result<Self, TryReserveError>;
    pub fn try_from_iter<I: IntoIterator<Item = T>>(iter: I) -> Result<Vec<T>, TryReserveError>;
}

impl<T, A: Allocator> Vec<T, A> {
    pub fn try_append(&mut self, other: &mut Self) -> Result<(), TryReserveError>;
    pub fn try_extend<I: IntoIterator<Item = T>>(&mut self, iter: I, ) -> Result<(), TryReserveError>;
    pub fn try_extend_from_slice(&mut self, other: &[T]) -> Result<(), TryReserveError>;
    pub fn try_extend_from_within<R>(&mut self, src: R) -> Result<(), TryReserveError> where R: RangeBounds<usize>; // NOTE: still panics if given an invalid range
    pub fn try_insert(&mut self, index: usize, element: T) -> Result<(), TryReserveError>; // NOTE: still panics if given an invalid index
    pub fn try_into_boxed_slice(self) -> Result<Box<[T], A>, TryReserveError>;
    pub fn try_push(&mut self, value: T) -> Result<(), TryReserveError>;
    pub fn try_resize(&mut self, new_len: usize, value: T) -> Result<(), TryReserveError>;
    pub fn try_resize_with<F>(&mut self, new_len: usize, f: F) -> Result<(), TryReserveError> where F: FnMut() -> T;
    pub fn try_shrink_to(&mut self, min_capacity: usize) -> Result<(), TryReserveError>;
    pub fn try_shrink_to_fit(&mut self) -> Result<(), TryReserveError>;
    pub fn try_split_off(&mut self, at: usize) -> Result<Self, TryReserveError> where A: Clone; // NOTE: still panics if given an invalid index
    pub fn try_with_capacity_in(capacity: usize, alloc: A) -> Result<Self, TryReserveError>;
}

#[doc(hidden)]
pub fn try_from_elem<T: Clone>(elem: T, n: usize) -> Result<Vec<T>, TryReserveError>;

#[doc(hidden)]
pub fn try_from_elem_in<T: Clone, A: Allocator>(elem: T, n: usize, alloc: A) -> Result<Vec<T, A>, TryReserveError>;

```

## Stability

These methods are added without any presumption that they will be stabilized in their current form, and this will be documented.
This is a compromise between those that need this functionality now, and those that are wary of the proliferation of prefixes and suffixes (like `try_`) being used to add variations of functions.

# Drawbacks
[drawbacks]: #drawbacks

Bifurcating the API surface like this is undesirable: it adds yet another decision point for developers using a fundamental type, and requires the developer to
stop and understand what could possibly fail in the `try_` version. Additionally, this bifurcation can't stop at just `Vec`: What about the rest of the
collection types? What about the traits that `Vec` implements (e.g., `IntoIter`)? What about other types that implement those traits? What about functions that
use those traits? This bifurcation becomes viral both within the standard library and to other crates as well.

Implementing both functions using a generic implementation method also complicates the code within `Vec`, making it harder to understand and maintain. While
there is some complex code within `RawVec`, the code in `Vec` itself tends to be fairly straightforward and within reason for an inexperienced developer to
understand.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Rationale

We believe this the best option available at this time, and worthwhile to do rather than wait for a better option.

- **Conforms to existing pattern**: Adding methods with the `try_` prefix follows the existing pattern within the standard library (as used by `Box::try_new` and `Vec::try_reserve`).
  Following this pattern to allow new functions to be added in the short term is preferable than delaying those functions in the hope that a better mechanism will be added in the future.
- **Works in Rust today**: It doesn't require any changes to the language or the compiler.
  Even if we knew exactly what change we would want (which we don't), coordinating the language/compiler and library modification afterwards is a long process.
- **No new types splitting the ecosystem**: Although this design does bifurcate the functions, it doesn't bifurcate the type.
  `Vec` is still `Vec` and so works with any crate expecting a `Vec`, whereas having a completely separate type would preclude interop with the existing ecosystem.
  More functions might be aesthetically displeasing, but doesn't levy any of these harsh forking penalties.
- **Gives users flexibility**: Users of `Vec` can choose which function they wish to call depending on if they are in a mode where allocation failures are recoverable (e.g., handling an incoming request), or non-recoverable (e.g., during application startup).
- **Usual beloved `Result` ergonomics**: Rust developers are already used to using `Result` and the `?` operator, and rich tooling exists to assist developers to use these correctly.
  Methods that require catching allocations failures more coarsely, or handling out-of-band errors, are more analogous to exception handling and `errno`/`GetLastError`, which may be foreign to Rust developers and unsupported by the existing tooling.
- **Avoids forks, encourages interop** Conversely, if these methods are not added to the standard library, then it is highly likely that the projects that require fallible allocation will take one of these approaches themselves, forking the standard library if required.
  Once these forks occur they will continue to diverge from the official standard library and from each other, and it will be increasingly difficult to ever reconcile them together.
- **Provides an upgrade path to the long-term solution**: While we don't know what the long-term solution to handling the proliferation of function variations will be, we can be reasonably confident that the replacement for these `try_` methods will involve the same concepts of calling a method and dealing with a `Result` afterwards.
  By implementing fallible allocation as methods that are prefixed with `try_`, return a `Result` and are gated behind an unstable feature flag, developers will be able to easily identify the use of these `try_` methods and be able to substitute them with their long-term, stable replacements without massive churn to their code (and, possibly, with the help of automation or tooling).

The essential insight of the current implementation is that we can share code between the infallible and fallible approaches by using a polymorphic error type `Result<_, E>`.
By making `E = !`, we move the panicking as close to the original failure as possible, avoiding `Result`-propagating overhead when the code is guaranteed to succeed and improving diagnosability of failures.
While we don't know the ideal form of this looks like *statically* in terms of types, coercions, and other compile-time features, we are confident that the *dynamic* behavior of these `try_` methods is correct and desirable.

## Alternatives

### Rely on `panic=unwind` and `catch_unwind`

The `panic::catch_unwind` function allows a `panic` to be "caught" by a previous function on the stack: that is, each function between the `panic` and the
`catch_unwind` call will immediately return (including running `drop` methods) and then the `panic` is, effectively, canceled. This can be leveraged to handle
OOM errors without any change to the existing APIs by ensuring that any calls to APIs that allocate are wrapped in `catch_unwind`.

Advantages:

- Requires no new or modified APIs.
- The `catch_unwind` call could be placed directly where the error is handled (e.g., at the function handling an incoming request, or the dispatcher for an
  event loop) without having to apply the `?` operator throughout the code.

Disadvantages:

- It is not obvious if an API allocates or not (i.e., if `catch_unwind` is required or not): making it easy to either miss function calls, or to be too
  pessimistic. This could be worked around via static analysis.
- `catch_unwind` requires that the function to be called is `UnwindSafe`, which precludes using a closure that captures a mutable reference.
- This requires `panic=unwind`, which might not be possible in the environment that requires fallible allocation (e.g., embedded systems or OS kernels).
- The standard library and 3rd party crates may not expect this pattern, and so have objects in invalid states at the point of OOM.

### Create a new "fallible allocation" `Vec` type

Instead of bifurcating the `Vec` methods, we could instead bifurcate the `Vec` type itself: thus we would keep `Vec` as the infallible version and introduce a
new fallible version (for the purposes of this RFC, let's call it `FallibleVec`). Under the covers, these two types could rely on the same implementation
type/functions to minimize code duplication.

Advantages:

- Avoids cluttering `Vec` with new APIs.
- Reduces decision points for developers: they only need to decide when choosing the type, rather than for each method call.

Disadvantages:

- Still requires bifurcating traits, and the functions that call those crates.
- `FallibleVec` cannot be used where a something explicitly asks for a `Vec` or any of the infallible allocation traits.
- Requires a developer to choose at the type level if they need fallible or infallible allocation (unless there's a way to convert between the types).

### Always return `Result` (with the error based on the allocator), but allow implicit unwrapping for "infallible allocation"

If the `Allocator` trait is updated to indicate what error is return if the allocation fails (with infallible allocation returning `!`), then the allocating
methods on `Vec` can be changed to return a `Result` using that error type. Normally this would be a breaking change, but we could also change the Rust
compiler to permit an implicit conversion from `Result<T, !>` to `T`, thus any existing code using an infallible allocator will continue to compile.

Advantages:

- Avoids cluttering `Vec` with new APIs.
- It may be potentially useful for allocators to indicate if they are fallible or not?

Disadvantages:

- Still a breaking change: we are adding an associated type to the `Allocator` trait, and any existing code that takes a `Vec` and allows a custom allocator
  will need to either handle the new return types, or restrict the error type for allocators to `!`.
- Still requires bifurcating traits, and the functions that call those crates.
- A `Vec` with a fallible allocator cannot be used where a function asks for the infallible allocation trait types.
- Makes `Vec` confusing for new developers: introduces the concept of `Result` and `!`, but then adds a weird exception where `Result` can be ignored.
- Requires a developer to choose at the type level if they need fallible or infallible allocation (unless there's a way to convert between the types).

### Create a "fallible allocation" fork of the `alloc` crate

Instead of bifurcating individual methods or types, we create a new "fallible allocation" version of the `alloc` crate (perhaps keyed off the
`infallible_allocation` feature). To enable code sharing, we can still have a single implementation method and then have the public wrapper method switch
depending on how the `alloc` crate is built.

Advantages:

- Avoids cluttering `Vec` with new APIs and bifurcating traits (and functions calling those traits).
- It would be possible to use the fallible `Vec` in an API that asks for an infallible `Vec` IF it does not use any of the allocating methods.

Disadvantages:

- Forces developers to choose, at a crate level, whether they want fallible or infallible allocation. It might be possible to mix-and-match by referencing both
  the fallible or infallible versions of the `alloc` crate, but the conflicting names would make that messy and there would be no way to convert a type between
  the two.
- Using a fallible `Vec` in crates that are unaware of the new methods would be hit-or-miss (it's hard to know if a 3rd party crate does or doesn't use an
  allocating method) and very fragile (using an allocating method where none were used previously would become a breaking change).

### Add a side-channel for checking allocation failures

Instead of reporting allocating failures via the return type, it could instead be exposed via a separate method on the allocator or `Vec` (e.g.,
`Allocator::last_alloc_failed` or `Vec::last_alloc_failed`).

Advantages:

- Avoids cluttering `Vec` with new APIs and bifurcating traits (and functions calling those traits).

Disadvantages:

- Very easy to forget to call the side-channel method, and not doing so is highly likely to lead to hard-to-diagnose bugs (e.g., code that thinks it has pushed
  an item but actually hasn't).
  - Static analysis could be added to assist with detecting this.
  - Existing code is unaware of the side-channel, and so will never call it.
  - Subsequent calls to the `Vec` could panic if the last allocation failed, but this would be a performance hit and might not help diagnosability (the
    developer would see a crash at the subsequent call, not the call that failed to allocate).
- Requires that developers add a lot of boilerplate code, slightly less if the side-channel returns a `Result` that the developer can use `?` on.
- It is not obvious if an API allocates or not, so even if a developer is aware of the side-channel method they may not think it needs to be called, or may call
  it too often.

### Add a trait for the "fallible allocation" functions (to effectively hide them)

We could create a new trait specifically for the `try_` methods in `Vec` - this will effectively hide them unless a developer is looking through the list of
trait implementations for `Vec` or adds a `use` declaration for the trait.

Advantages:

- Although we are still adding new methods to `Vec`, they will be hidden from most developers and so avoids the cognitive burden that comes from them being
  directly on the type.
- The trait and the additional methods could be put in a standalone crate (it would require duplicate quite a bit of code and relying on `unsafe` functions
  like `set_len`).

Disadvantages:

- Still requires bifurcating traits, and the functions that call those traits.
- Unusual use for a trait, and would require additional documentation to explain why these methods have been implemented this way.
- Any future allocating methods will require a new trait to implement them, since adding functions to a trait is a breaking change.


### Use `#[doc(hidden)]` to hide the methods

We could mark all of the new methods with `#[doc(hidden)]` to hide them in the documentation.

Advantages:

- Although we are still adding new methods to `Vec`, they will be hidden from most developers and so avoids the cognitive burden that comes from them being
  immediately visible in the docs.

Disadvantages:

- Still requires bifurcating the methods, traits, and the functions that call those traits.
- Hiding methods within an IDE can be frustrating for developers who are actually using those methods.

# Prior art
[prior-art]: #prior-art

`Vec` already has a `try_reserve` method, and `Box` has a number of `try_new*` methods (which, when combined with the `_in`, `_uninit`, `_zeroed` and `_slice`
suffixes, highlights how these variants can combine into an explosion of combinations).

The ["Prior Art" section of the fallible_allocation feature](https://github.com/maurer/rust-rfcs/blob/fallible-allocation/text/0000-fallible-allocation.md#c-oom-handling)
has a good discussion of C++ OOM Handling.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Is bifurcating the API in this way the correct approach? Or can something else can be done (perhaps with some compiler assistance)?
- `try_insert`, `try_extend_from_within` and `try_split_off` will still `panic` if provided an invalid index/range: is this ok, or should they never `panic`?

# Future possibilities
[future-possibilities]: #future-possibilities

## Function variants

The numerous variants of `Box::new` highlight the difficulty with handling variations of an API, especially when those variations have different return types:
`_uninit` and `_zeroed` return a `MaybeUninit<T>` and `try_` returns a `Result<T, AllocError>`. This issue is not unique to Rust, but one wonders if Rust's
rich support for generics and type inference could be leveraged to provide a solution?

For example, consider some theoretical `Box::new` method like:

```rust
pub fn new<Variant>() -> Variant::ReturnType<T>
```

Where `Variant` could be substituted with some sort of marker type like `Zeroed`, `AllocIn<SomeAllocator>`, `Fallible`, or a combination (like
`Zeroed + Fallible`). Each of these markers could then change (or just wrap?) the return type as appropriate. If the compiler knew the set of markers that are
in scope and could be applied, then it might also be able to use inference based on the usage of the returned value to detect which markers to use, for example
seeing the `?` operator implies `Fallible`, and seeing the value used as `Box<T, SomeAllocator>` implies `AllocIn<SomeAllocator>`.

This could also be done without a breaking change if we leveraged editions: if Edition N-1 disabled inference for variations then writing `Box::new()` would
always produce the old behavior, and then Edition N could enable the new inference behavior (and the tooling could detect potential breaking changes by seeing
where the inference did not produce the old behavior, thus allowing automatic fixes by rewriting those calls as `Box::new::<>()`). Alternatively, if none of
the markers are in scope then the compiler will be forced to infer the old behavior, thus we can enable the behavior in Edition N by adding the markers to the
prelude.

A function will also need to be able to declare what markers it supports, either by marker types being "local" to the function (making the marker more like
enum items on an enum declared by the function), or by listing the marker types that it supports.

## Documentation sections

Rather than simply hiding the methods in the documentation, we could add a new feature to be able to move these methods into their own "section". They could
still be effectively hidden by collapsing that section by default, but it would give us the ability to still present documentation for these methods to
developers, and to preface the methods with an explanation of why they exist and when a developer would choose to use them.

Consider the following code:

```rust
//! SECTION=fallible,COLLAPSED=true
//! # Fallible allocation methods
//! Text to describe what fallible allocation is, and why you'd use these methods

impl<T, A: Allocator> Vec<T, A> {
    #[doc(section="fallible")]
    pub fn try_push(&mut self, value: T) -> Result<(), TryReserveError> { ... }
}
```

This would result in a separate section in the `Vec` method docs called "Fallible allocation methods" that is collapsed by default, and which contains the
description text listed (i.e., "Text to describe what fallible allocation is, and why you'd use these methods") and then all of the methods marked with
`#[doc(section="fallible")]` (in this example, `try_push`).

We need to be cautious that a feature like this doesn't become an excuse to add new APIs without careful consideration ("just hide them in a section!") or to
avoid tackling the larger problem of combinatorial explosions of function variants (see "Function variants" above).
