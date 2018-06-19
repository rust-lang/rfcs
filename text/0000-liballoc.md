- Feature Name: `liballoc`
- Start Date: 2018-06-14
- RFC PR:
- Rust Issue: [rust-lang/rust#27783](https://github.com/rust-lang/rust/issues/27783)

# Summary
[summary]: #summary

Stabilize the `alloc` crate, with a module structure matching `std`.

This crate provides the subset of the standard library’s functionality that requires
a global allocator (unlike the `core` crate) but not other operating system
capabilities (unlike the `std` crate).


# Motivation
[motivation]: #motivation

## Background: `no_std`

In some environments the `std` crate is not available:
micro-controllers that don’t have an operating system at all, kernel-space code, etc.
The `#![no_std]` attribute allows a crate to not link to `std` implicitly,
using `core` instead with only the subset of functionality that doesn’t have a runtime dependency.

## Use case 1: pushing `no_std` programs toward stable Rust

Programs (or `staticlib`s) that do not link `std` at all may still want to use `Vec<T>`
or other functionality that requires a memory allocator.
Blockers for doing so on stable Rust are diminishing:

* [The `#[global_allocator]` attribute][global_allocator] to specify an allocator
  and remove the need for an operating-system-provided one is stable since Rust 1.28.
* [PR #51607] adds a fallback handling for OOM (allocation failure) conditions
  for when `std` is not available,
  removing the need for programs to define an unstable `oom` lang item themselves.

With this, the only allocation-related blocker is being able to import `Vec` in the first place.
This RFC proposes stabilizing the current unstable way to do it: `extern crate alloc;`

[global_allocator]: https://doc.rust-lang.org/nightly/std/alloc/#the-global_allocator-attribute
[PR #51607]: https://github.com/rust-lang/rust/pull/51607

## Use case 2: making stable libraries `no_std`

Even if a `no_std` program might still require other features that are still unstable,
it is very common to use libraries from crates.io that have other users.
Such a library might support stable Rust and use `Vec<T>`
(or something else that requires a memory allocator)
but not other operating-sytem functionality.

Today, making such a library possible to use without `std` without breaking stable users
requires a compile-time flag:

```rust
#![no_std]

#[cfg(feature = "no_std")] extern crate alloc;
#[cfg(not(feature = "no_std"))] extern crate std as alloc;

use alloc::vec::Vec;
```

With this RFC, this can be simplified to:

```rust
#![no_std]

extern crate alloc;

use alloc::vec::Vec;
```

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When using `#![no_std]` in a crate, that crate does not implicitly depend on `std`
but depends on `core` instead. For example:

```diff
-use std::cell::RefCell;
+use core::cell::RefCell;
```

APIs that require a memory allocator are not available in `core`.
In order to use them, `no_std` Rust code must explicitly depend on the `alloc` crate:

```rust
extern crate alloc;

use core::cell::RefCell;
use alloc::rc::Rc;
```

Like `std` and `core`, this dependency does not need to be declared in `Cargo.toml`
since `alloc` is part of the standard library and distributed with Rust.

The `alloc` crate does not have a prelude (items that are implicitly in scope).
So its items that are in the `std` prelude must be imported explicitly
to be used in `no_std` crates:

```rust
use alloc::vec::Vec;
```


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The `alloc` crate already exists (marked unstable),
and every public API in it is already available in `std`.

[PR #51569] moves them around so that the module structure matches that of `std`,
and the public APIs become a subset:
any path that starts with `alloc::` should still be valid and point to the same item
after replacing that prefix with `std::` (assuming both crates are available).

All that remains is stabilizing the `alloc` crate itself and tweaking its doc-comment.
(In particular, removing the “not intended for general usage” sentence
and mention `no_std` instead.)
Since it is the only remaining unstable crate tracked by [tracking issue #27783],
that issue can be closed after this RFC is implemented.

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

[PR #51569]: https://github.com/rust-lang/rust/pull/51569
[tracking issue #27783]: https://github.com/rust-lang/rust/issues/27783


# Drawbacks
[drawbacks]: #drawbacks

[Tracking issue #27783] is the tracking issue for the `alloc` crate and, historically, some other crates.
Although I could not find much discussion of that, I believe it has been kept unstable so far
because of uncertainty of uncertainty of what is the eventual desired crate structure
for the standard library, given infinite time and resources.

In particular, could we have a single crate with some mechanism for selectively disabling
or enabling some of the crate’s components, depending on which runtime dependencies
are available in targetted environments?
In that world, the `no_std` attribute and standard library crates other than `std`
would be unecessary.

By stabilizing the `alloc` crate, we commit to having it − and its public API − exist “forever”.


# Rationale and alternatives
[alternatives]: #alternatives

The `core` and the `no_std` attribute are already stable,
so in a sense it’s already too late for the “pure” version of the vision described above
where `std` really is the only standard library crate that exists.

It may still be [desirable] to regroup the standard library into one crate,
and it is proably still possible.
The `core` crate could be replaced with a set of `pub use` reexport
to maintained compatibility with existing users.
Whatever the eventual status is for `core` is,
we can do the same for `alloc`.
[PR #51569] mentioned above also hopes to make this easier.

While we want to leave the possibility open for it,
at the time of this writing there are no concrete plans
for implementing such a standard library crates unification any time soon.
So the only alternative to this RFC seems to be
leaving heap allocation for `no_std` in unstable limbo for the forseeable future.

[desirable]: https://aturon.github.io/2018/02/06/portability-vision/#the-vision


# Prior art
[prior-art]: #prior-art

I am not aware of a mechanism similar to `no_std` in another programming language.

[Newlib] is a C library for “embedded” systems that typically don’t have an operating system.
It does provide a memory allocator through `malloc` and related functions, unconditionally.

[Newlib]: https://sourceware.org/newlib/


# Unresolved questions
[unresolved]: #unresolved-questions

Did I miss something in [PR #51569] that makes `alloc` not a subset of `std`?
A double-check from someone else would be appreciated.
