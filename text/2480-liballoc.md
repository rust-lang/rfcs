- Feature Name: `liballoc`
- Start Date: 2018-06-14
- RFC PR: [rust-lang/rfcs#2480](https://github.com/rust-lang/rfcs/pull/2480)
- Rust Issue: [rust-lang/rust#27783](https://github.com/rust-lang/rust/issues/27783)

# Summary
[summary]: #summary

Stabilize the `alloc` crate.

This crate provides the subset of the standard library’s functionality that requires
a global allocator (unlike the `core` crate) and an allocation error handler,
but not other operating system capabilities (unlike the `std` crate).


# Motivation
[motivation]: #motivation

## Background: `no_std`

In some environments the `std` crate is not available:
micro-controllers that don’t have an operating system at all, kernel-space code, etc.
The `#![no_std]` attribute allows a crate to not link to `std` implicitly,
using `core` instead with only the subset of functionality that doesn’t have a runtime dependency.

## `no_std` with an allocator

The `core` crate does not assume even the presence of heap memory,
and so it excludes standard library types like `Vec<T>`.
However some environments do have a heap memory allocator
(possibly as `malloc` and `free` C functions),
even if they don’t have files or threads
or something that could be called an operating system or kernel.
Or one could be defined [in a Rust library][wee-alloc]
ultimately backed by fixed-size static byte array.

An intermediate subset of the standard library smaller than “all of `std`”
but larger than “only `core`” can serve such environments.

[wee-alloc]: https://github.com/rustwasm/wee_alloc

## Libraries

In 2018 there is a [coordinated push]
toward making `no_std` application compatible with Stable Rust.
As of this writing not all of that work is completed yet.
For example, [`#[panic_implementation]`][panic-impl] is required for `no_std` but still unstable.
So it may seem that this RFC does not unlock anything new,
as `no_std` application still need to be on Nightly anyway.

[coordinated push]: https://github.com/rust-lang-nursery/embedded-wg/issues/42
[panic-impl]: https://github.com/rust-lang/rust/issues/44489

The immediate impact can be found in the library ecosystem.
Many general-purpose libraries today are compatible with Stable Rust
and also have potential users who ask for them to be compatible with `no_std` environments.

For a library that is fundamentally about using for example TCP sockets or threads,
this may not be possible.

For a library that happens to only use parts of `std` that are also in `core`
(and are willing to commit to keep doing so), this is relatively easy:
add `#![no_std]` to the crate, and change `std::` in paths to `core::`.

And here again, there is the intermediate case of a library that needs `Vec<T>`
or something else that involves heap memory, but not other parts of `std` that are not in `core`.
Today, in order to not lose compatibility with Stable,
such a library needs to make compatibility with `no_std` an opt-in feature flag:

```rust
#![no_std]

#[cfg(feature = "no_std")] extern crate alloc;
#[cfg(not(feature = "no_std"))] extern crate std as alloc;

use alloc::vec::Vec;
```

But publishing a library that uses unstable features, even optionally,
comes with the expectation that it will be promptly updated whenever those features change.
Some maintainers are not willing to commit to this.

With this RFC, the library’s code can be simplified to:

```rust
#![no_std]

extern crate alloc;

use alloc::vec::Vec;
```

… and perhaps more importantly,
maintainers can rely on the stability promise made by the Rust project.


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## For libraries

When using `#![no_std]` in a crate, that crate does not implicitly depend on `std`
but depends on `core` instead. For example:

```diff
-use std::cell::RefCell;
+use core::cell::RefCell;
```

APIs that require a memory allocator are not available in `core`.
In order to use them, `no_std` Rust code must explicitly depend on the `alloc` crate:

```rust
#[macro_use] extern crate alloc;

use core::cell::RefCell;
use alloc::rc::Rc;
```

Note: `#[macro_use]` imports the [`vec!`] and [`format!`] macros.

[`vec!`]: https://doc.rust-lang.org/alloc/macro.vec.html
[`format!`]: https://doc.rust-lang.org/alloc/macro.format.html

Like `std` and `core`, this dependency does not need to be declared in `Cargo.toml`
since `alloc` is part of the standard library and distributed with Rust.

The implicit prelude (set of items that are automatically in scope) for `#![no_std]` crates
does not assume the presence of the `alloc` crate, unlike the default prelude.
So such crates may need to import either that prelude or specific items explicitly.
For example:

```rust
use alloc::prelude::*;

// Or

use alloc::string::ToString;
use alloc::vec::Vec;
```

## For programs¹

[¹] … and other roots of a dependency graph, such as `staticlib`s.

Compared to `core`, the `alloc` crate makes two additional requirements:

* A global heap memory allocator.

* An allocation error handler (that is not allowed to return).
  This is called for example by `Vec::push`, whose own API is infallible,
  when the allocator fails to allocate memory.

`std` provides both of these. So as long as it is present in the dependency graph,
nothing else is required even if some crates of the graph use `alloc` without `std`.

If `std` is not present they need to be defined explicitly,
somewhere in the dependency graph (not necessarily in the root crate).

* [The `#[global_allocator]` attribute][global_allocator], on a `static` item
  of a type that implements the `GlobalAlloc` trait,
  defines the global allocator. It is stable in Rust 1.28.

* [Tracking issue #51540] propose the `#[alloc_error_handler]` attribute
  for a function with signature `fn foo(_: Layout) -> !`.
  As of this writing this attribute is implemented but unstable.

[global_allocator]: https://doc.rust-lang.org/nightly/std/alloc/#the-global_allocator-attribute
[Tracking issue #51540]: https://github.com/rust-lang/rust/issues/51540


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The `alloc` crate already exists (marked unstable),
and every public API in it is already available in `std`.

Except for the `alloc::prelude` module, since [PR #51569] the module structure is a subset
of that of `std`: every path that starts with `alloc::` is still valid and point to the same item
after replacing that prefix with `std::` (assuming both crates are available).

The concrete changes proposed by this RFC are:

* Stabilize `extern crate alloc;`
  (that is, change `#![unstable]` to `#![stable]` near the top of `src/liballoc/lib.rs`).

* Stabilize the `alloc::prelude` module and its contents
  (which is only re-exports of items that are themselves already stable).

* Stabilize the fact that the crate makes no more and no less than
  the two requirements/assumptions of a global allocator and an allocation error handler
  being provided for it, as described above.

  The exact mechanism for [providing the allocation error handler][Tracking issue #51540]
  is not stabilized by this RFC.

  In particular, this RFC proposes that the presence of a source of randomness
  is *not* a requirement that the `alloc` crate can make.
  This is contrary to what [PR #51846] proposed,
  and means that `std::collections::hash_map::RandomState` cannot be moved into `alloc`.

[Tracking issue #27783] tracks “the `std` facade”:
crates whose contents are re-exported in `std` but also exist separately.
Other such crates have already been moved, merged, or stabilized,
such that `alloc` is the only remaining unstable one.
Therefore #27783 can serve as the tracking issue for this RFC
and can be closed once it is implemented.

[PR #51569]: https://github.com/rust-lang/rust/pull/51569
[PR #51846]: https://github.com/rust-lang/rust/pull/51846
[Tracking issue #27783]: https://github.com/rust-lang/rust/issues/27783


The structure of the standard library is therefore:

* `core`: has (almost) no runtime dependency, every Rust crate is expected to depend on this.
* `alloc`: requires a global memory allocator,
  either specified through the `#[global_allocator]` attribute
  or provided by the `std` crate.
* `std`: re-exports the contents of `core` and `alloc`
  so that non-`no_std` crate do not need care about what’s in what crate between these three.
  Depends on various operating system features such as files, threads, etc.
* `proc-macro`: depends on parts of the compiler, typically only used at build-time
  (in procedural macro crates or Cargo build scripts).



# Drawbacks
[drawbacks]: #drawbacks

[Tracking issue #27783] is the tracking issue for the `alloc` crate and, historically, some other crates.
Although I could not find much discussion of that, I believe it has been kept unstable so far
because of uncertainty of what the eventual desired crate structure
for the standard library is, given infinite time and resources.

In particular, should we have a single crate with some mechanism for selectively disabling
or enabling some of the crate’s components, depending on which runtime dependencies
are available in targeted environments?
In that world, the `no_std` attribute and standard library crates other than `std`
would be unnecessary.

By stabilizing the `alloc` crate, we commit to having it − and its public API − exist “forever”.


# Rationale and alternatives
[alternatives]: #alternatives

## Single-crate standard library

The `core` and the `no_std` attribute are already stable,
so in a sense it’s already too late for the “pure” version of the vision described above
where `std` really is the only standard library crate that exists.

It may still be [desirable] to regroup the standard library into one crate,
and it is probably still possible.
The `core` crate could be replaced with a set of `pub use` reexport
to maintain compatibility with existing users.
Whatever the eventual status for `core` is,
we can do the same for `alloc`.
[PR #51569] mentioned above also hopes to make this easier.

While we want to leave the possibility open for it,
at the time of this writing there are no concrete plans
for implementing such a standard library crates unification any time soon.
So the only alternative to this RFC seems to be
leaving heap allocation for `no_std` in unstable limbo for the foreseeable future.

[desirable]: https://aturon.github.io/2018/02/06/portability-vision/#the-vision

## Require randomness

[PR #51569] proposed adding a source of randomness to the other requirements
made by the `alloc` crate.
This would allow moving `std::collections::hash_map::RandomState`,
and therefore `HashMap` (which has `RandomState` as a default type parameter),
into `alloc`.

This RFC chooses not to do this because it would make it difficult to use for example `Vec<T>`
in environments where a source of randomness is not easily available.

I hope that the language will eventually make it possible to have `HashMap` in `alloc`
without a default hasher type parameter, and have the same type in `std` with its current default.

Although I am not necessarily in favor
of continuing the increase of the number of crates in the standard library,
another solution for `HashMap` in `no_std` might be another intermediate crate
that depends on `alloc` and adds the randomness source requirement.

Additionally, with this RFC it should be possible to make https://crates.io/crates/hashmap_core
compatible with Stable Rust.
The downside of that crate is that although based on a copy of the same code,
it is a different type incompatible in the type system with `std::collections::HashMap`.


# Prior art
[prior-art]: #prior-art

I am not aware of a mechanism similar to `no_std` in another programming language.

[Newlib] is a C library for “embedded” systems that typically don’t have an operating system.
It does provide a memory allocator through `malloc` and related functions, unconditionally.

[Newlib]: https://sourceware.org/newlib/


# Unresolved questions
[unresolved]: #unresolved-questions

* Did I miss something in [PR #51569] that makes `alloc` not a subset of `std`?
  A double-check from someone else would be appreciated.

* Should the crate be renamed before stabilization?
  It doesn’t have exclusivity for memory-allocation-related APIs,
  since the `core::alloc` module exists.
  What really characterizes it is the assumption that a global allocator is available.
  The name `global_alloc` was proposed.
  (Although the crate doesn’t only contain the global allocator itself.)

* ~Should the `alloc::prelude` module be moved to `alloc::prelude::v1`?
  This would make the `alloc` module structure a subset of `std` without exception.
  However, since this prelude is not inserted automatically,
  it is less likely that we’ll ever have a second version of it.
  In that sense it is closer to `std::io::prelude` than `std::prelude::v1`.~
  Done in [PR #58933].

* In addition to being a subset of `std`, should the `alloc` crate (by itself)
  be a super-set of `core`? That is, should it reexport everything that is defined in `core`?
  See [PR #58175] which proposes reexporting `core::sync::atomic` in `alloc::sync`.

[PR #58933]: https://github.com/rust-lang/rust/pull/58933
[PR #58175]: https://github.com/rust-lang/rust/pull/58175
