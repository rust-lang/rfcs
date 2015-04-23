- Start Date: April 22, 2015
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Add a new default unsafe trait, `Leak`. If a type does not implement `Leak` it can be
memory unsafe to fail to run the destructor of this type and continue execution of the
program past this types lifetime.

Additionally, cause all panics in destructors to immediately abort the process,
solving two other notable bugs that allow leaking arbitrary data.

Possibly add a safe variant of `mem::forget` (e.g. `mem::leak`) which requires `Leak`.
The existing `mem::forget` remains unbounded, but `unsafe`.

This proposal also requires a slight breaking change to a few `std` APIs to add
`Leak` bounds where none exist currently. This is unfortunate so close to 1.0,
but in the author's opinion is better than dedicating to a safe unbounded `mem::forget`
forever.

This RFC is largely an alternative to RFC PR 1066, which makes an unbounded `mem::forget`
safe.

# Motivation

From RFC 1066:

> It was [recently discovered][scoped-bug] by @arielb1 that the `thread::scoped`
> API was unsound. To recap, this API previously allowed spawning a child thread
> sharing the parent's stack, returning an RAII guard which `join`'d the child
> thread when it fell out of scope. The join-on-drop behavior here is critical to
> the safety of the API to ensure that the parent does not pop the stack frames
> the child is referencing. Put another way, the safety of `thread::scoped` relied
> on the fact that the `Drop` implementation for `JoinGuard` was *always* run.
>
> The [underlying issue][forget-bug] for this safety hole was that it is possible
> to write a version of `mem::forget` without using `unsafe` code (which drops a
> value without running its destructor). This is done by creating a cycle of `Rc`
> pointers, leaking the actual contents. It [has been pointed out][dtor-comment]
> that `Rc` is not the only vector of leaking contents today as there are
> [known][dtor-bug1] [bugs][dtor-bug2] where `panic!` may fail to run
> destructors. Furthermore, it has [also been pointed out][drain-bug] that not
> running destructors can affect the safety of APIs like `Vec::drain_range` in
> addition to `thread::scoped`.

[scoped-bug]: https://github.com/rust-lang/rust/issues/24292
[forget-bug]: https://github.com/rust-lang/rust/issues/24456
[dtor-comment]: https://github.com/rust-lang/rust/issues/24292#issuecomment-93505374
[dtor-bug1]: https://github.com/rust-lang/rust/issues/14875
[dtor-bug2]: https://github.com/rust-lang/rust/issues/16135
[drain-bug]: https://github.com/rust-lang/rust/issues/24292#issuecomment-93513451

Previously, Rust provided no guarantee that any destructors for any types will
run. However, some of the current APIs (namely `thread::scoped`) were designed
without keeping this in mind, and can be made memory unsafe through leaking.

This RFC proposes a small, orthogonal feature to allow wondrous RAII-based APIs
like `thread::scoped` to exist in safe Rust, with minimal breaking changes in
the pre-1.0 release.

## Narrowing Goals and Dispelling Fear, Uncertainty, and Doubt

As this issue touched a fairly popular and touted API (`thread::scoped`) there
has naturally been a gigantic amount of discussion and ideation surrounding it.

Some of the proposed solutions to this problem have been far too restrictive,
such as banning `Rc` cycles entirely, making `Rc` unsafe, or having `Rc`
require `'static`. This had lead to a certain amount of FUD about the validity
of all non-safe-`mem::forget` solutions. This RFC is here to dispel this illusion.

### This Proposal Will Not:

 - Prevent *all* destructors from not running.
 - Prevent destructors from not running in the face of process aborts.
 - Prevent destructors from not running for `'static` data.
 - Prevent `Rc` or `Arc` cycles, or ban non-`'static` data from them.

### This Proposal Will:

 - Allow types to manually opt-in to not being leaked.
 - Allow leaks of `'static` data in a large variety of ways, even if they do
   not implement `Leak`.
 - Prevent types which have opted-in to not being leaked from being used in
   contexts where they might leak.
 - Prevent panics in destructors from causing leaks or other memory unsafety.
 - Allow `Rc` and `Arc` cycles, including when they contain non-`'static` data.

# Detailed design

Introduce a new default trait to `core`, `Leak`:

```rust
pub unsafe trait Leak { }
unsafe impl Leak for .. { }
```

Change several `std` APIs to adjust for the guarantees now provided to types
which do not implement `Leak`:

 - `impl<'a, T> !Leak for ::std::thread::JoinGuard<'a, T> { }`
   - If we gain some form of specialization, implement `Leak` for `JoinGuard<'static, T>`
 - implement `!Leak` for the guards returned by collection `drain` operations
 - Add a `Leak` bound to the type parameter of:
   - `rc::Rc`
   - `sync::Arc`
   - `sync::mpsc::Sender` (possibly not, see unresolved questions)
   - Possibly other APIs. Please point any others out if you think of them.

_Cause all panics in destructors to immediately abort the process._

Add a new function, `leak`, to `std::mem`:

```rust
/// Moves a value out of scope without running its destructor.
///
/// `leak` is safe since the data must implement `Leak` and therefore it is
/// safe to avoid its destructor.
pub fn leak<T: Leak>(x: T) {
    unsafe { forget(x) }
}
```

There are no changes to `mem::forget`, which remains an `unsafe` way to bypass
this system.

## When do you need a Leak bound?

If your API can delay or prevent the destructor of non-`'static` data past the
lifetime of that data, you need a `Leak` bound. If your API requires `'static`
already you do not need a `Leak` bound.

# Drawbacks

Introduces additional complexity in the form of the `Leak` trait, which, like
`Reflect`, applies to nearly all types and therefore carries very little
information.

It is possible there are ways to leak arbitrary, non-`'static`, data in the
safe subset of rust (including std) that we may not catch by 1.0, which would
force us to make a breaking fix to those APIs to fix soundness holes if they
are discovered post-1.0 (by adding a `Leak` bound where there was none
previously).

If a way to leak in the entirely safe subset of rust + `mem::swap`, then this
proposal is basically dead on arrival. We should investigate if this is
possible, but the author's slightly educated guess is that it is not possible.

# Alternatives

RFC PR 1066 is partially an alternative to this RFC, and vice versa.

Other possible names for `Leak`: `MayLeak`, `Leakable`, `MaybeDrop`.

Introduce `Leak` post-1.0 and add it to bounds by default, the same way `Sized`
works today.

## Drawbacks of Introducing Leak in 1.X (Why should we rush?)

Unlike with `Sized`, the vast majority of code actually does want what would be
`T: ?Leak`, meaning that over time we would either see lots of unnecessarily
restrictive bounds or the proliferation of `T: ?Leak` all over pretty much all
generic code.

# Unresolved questions

The exact mechanism by which panics are caught in destructors and turned into
process aborts. The author of this RFC is not entirely familiar with the
internals of panic, so leaves this up to the implementor.

Does `mpsc::Sender` actually require `Leak`? The only way to leak non-`'static`
data with it would be if we already had a safe `thread::scoped` abstraction and
the `scoped` thread is deadlocked and never receives. In this case, the
`scoped` API should also cause the spawning thread to deadlock forever, thereby
preventing the "leak" from passing the lifetime of the sent data.

