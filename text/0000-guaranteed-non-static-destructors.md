- Feature Name: Guaranteed non-static destructors
- Start Date: 2015-04-27
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Guarantee that the destructor for data that contains a borrow is run before any
code after the borrow’s lifetime is executed.

# Motivation

Rust is currently not guaranteed to run destructors on an object before
execution moves beyond its bounding-lifetime. This is surprising,
unintuitive, and leads to soundness mistakes. This is evidenced by two recently
approved API designs: `thread::scopped` and `drain_range`, both of which were
found to be unsound because they mistakenly assumed that their destructor would
run before any code outside of their bounding lifetimes could be executed.
While possible solutions have been proposed for both of them, it shows how easy
the mistake is to make. It is very likely that similar mistakes will continue
to be made in the future.

While we can try to continuously hammer home that leaks are not considered
`unsafe`, we’ll never be able to prevent all such mistakes, especially as Rust
grows and more third-party libraries are developed. It would be better if we
could actually provide the intuitive guarantee, which is what this RFC attempts
to accomplish.

Note that this RFC is explicitly not attempting to solve all leaks, nor even to
ensure that all stack-anchored objects are destructed. Rust is primarily
concerned with memory safety, so this RFC tries to formulate a general rule
to allow destructors to be relied upon for safety-related clean-up without
complicating or limiting situations without memory-safety implications. It does
not attempt to prevent the leaking of other resources, as leaking a `'static`
object is never memory-unsafe. (`'static` objects are conceptually safe to keep around “forever”, so any unsoundness that can be obtained by a leak could also
be obtained through other means, such as storing the value in a long-lived
hashmap).

# Detailed design

In addition to current guarantees, the following property can be relied upon:

Given an object `A` implementing `Drop` whose lifetime is restricted to `'a`
(that is, the object may not live longer than `'a`), in absence of unsafe code,
the destructor for `A` is guaranteed to run before any code after the end of
`'a` is executed.

This is already true at the language level. However, certain library constructs
currently allow safe code to break it.

This has the following implications:

 * It is perfectly acceptable to forget, leak, et cetera any object as long as
   that object is valid for the duration of `'static`. Intuitively, this makes
   sense, as forgetting an object is the same as having it live forever.
 * A resource that is never freed due to an endless loop, a deadlock, or
   program termination is valid because any code that comes after the end of
   `'a` is never executed.
 * Code would be allowed to rely on the guarantee for soundness. This means
   that patterns such as `thread::scoped` `Vec::drain_range` would be sound as
   initially proposed.
 * Unsafe code is free to forget objects as needed in cases where the
   programmer guarantees it is sound. However, it is not allowed to expose a
   safe interface that would allow the above guarantee to be violated.

As noted, this guarantee is already true at the core language level. However,
it can be violated in safe code when using the standard library. There are
two known ways in which this can happen: when a destructor panics in certain
situations (e.g., in a `Vec`), and when a reference cycle is created using `Rc`
and friends.

This RFC proposes the following solutions:

 * Specify that any panic that occurs while a destructor is in progress results
   in an abort. Panicking in a destructor is generally a bad idea with many
   edge cases, so this is probably desirable, anyway. It should be possible to
   implement this efficiently in a similar manner to C++’s `noexcept`.
 * Replace the existing `Rc` and `Arc` types with variants that this RFC will
   call `ScopedRc` and `ScopedArc`. These types will have a lifetime parameter
   added, and the contained `RcBox` will have an extra field to facilitate
   cycle collection. `ScopedRc` is described in detail, below, and `ScopedArc`
   will function analogously.

## `ScopedRc<'a, T>` design

 * `ScopedRc` has an added invariant lifetime parameter specifying how long the
   reference is valid. This may be shorter than the lifetime of `T`, but will
   never be longer.
 * A `ScopedRc` can either be guarded or unguarded.
 * A guarded `ScopedRc` works as follows:
    * There is a `ScopedRcGuard` type that can be instantiated with a given
      lifetime `'a`.
    * `ScopedRcGuard` can be used to create a new `ScopedRc`, which cannot
      outlive the creating guard.
    * Data stored in a `ScopedRc` created with a given `ScopedRcGuard` must
      live at least as long as the guard.
    * When the `ScopedRcGuard` is destroyed, it cleans up any cycles that may
      remain among the `ScopedRc` objects created by it.
    * During cycle collection, an attempt by an object in a cycle to
      dereference a `ScopedRc` that has already been collected will result in
      an abort.
 * An unguarded `ScopedRc` does not have an associated cycle collector, and can
   be created in the following ways:
    * Through an `unguarded` constructor on the `ScopedRc`. This constructor
      requires `T: 'static`, and would thus is considered safe to call. This
      simplifies common `Rc` usages such as reference-counted strings.
    * Using a global STATIC_GUARD object. This can be passed anywhere a
      `&ScopedRcGuard` is expected, only accepts `'static` data, and creates
      unguarded `ScopedRc`s. This allows APIs using cycles to provide a single
      interface for creating both guarded and safe unguarded `ScopedRc`s.
    * Using a `NoCycleMarker` type. This is used similarly to a `ScopedRcGuard`
      but requires `T` to strictly outlive the marker. This makes cycles
      impossible, so the created `ScopedRc`s does not need a cycle collector.
    * Through an `unsafe_unguarded` constructor. This allows the creation of
      unguarded `ScopedRc`s of any lifetime, but is `unsafe` because it could
      be used to create uncollected, non-`'static` cycles, violating the
      guarantee provided by this RFC. The caller of this method is responsible
      for ensuring that such cycles are impossible, or guaranteed to be cleaned
      up through some means before the end of the relevant lifetime.
 * A `ScopedRcCreator` trait is also be provided, implemented by both
   `ScopedRcGuard` and `NoCycleMarker`. This allows `ScopedRc` creation APIs to
   be generic over either approach.

See [this gist](https://gist.github.com/rkjnsn/791ee9cc3b6d2961cf33) for a
working `ScopedRc` implementation, including [sample usage](https://gist.github.com/rkjnsn/791ee9cc3b6d2961cf33#file-scopedrctest-rs).

Other interfaces that use `Rc`s or `Arc`s internally (such as channels) have a
few options available. If they only use the `Rc`s to hold `'static` data, or
if they can be sure that no cycles are possible, they can continue to offer the
same API as today. If the objects containing `Rc`s all borrow from a main
object of some sort, the interface can internally use a `ScopedRcGuard` or a
`NoCycleMarker` depending on its needs. Finally, if the created `Rc`s hold a
user-specified type, the interface can generically accept a ScopedRcCreator` to
allow the user to statically eliminate or dynamically clean up cycles at their
discretion.

# Drawbacks

 * Using `ScopedRc`s for non-`'static` types will be slightly less convenient.
   The user will either have to use a `ScopedRcGuard` or a `NoCycleMarker`, or
   use `unsafe_unguarded` and manually verify that no leaks are possible.
 * The internal `RcBox` allocation for unguarded `ScopedRc`s will be one
   pointer larger than that of today’s `Rc`s. However, the author of this RFC
   believes this will be insignificant in practice. Increasing the size of the
   `RcBox` in a test did not noticably affect the memory usage of the Rust
   compiler.
 * Dereferencing a `ScopedRc` requires an extra check compared to today’s `Rc`.
   Again, the author does not expect this to be significant in practice. Adding
   this check to `Rc` did not noticably impact the Rust compiler’s compile
   time. However, the sample implementation does incude an unsafe
   `uncheck_deref` for the rare case where many different `ScopedRc`s need to
   be dereferenced in a tight, performance-crital loop.
 * Additional overhead for guarded `ScopedRc`s. In the sample implementation,
   this overhead is three extra pointer-sized values per `RcBox` allocation
   versus an unguarded `ScopedRc`, and some additional O(1) bookkeeping when
   the underlying allocation is created or destroyed. (All other operations are
   the same for guarded and unguarded `ScopedRc`s.)
 * There may not enough time to finalize the `ScopedRc` design before 1.0. This
   can be mitigated by implementing the minimum necessary to make this
   backward-compatible. Since the behavior of panicking destructors are already
   unspecified and subject to change, doing so would be a matter of restricting
   the safe creation of reference-counted objects to `'static` values. This
   would cause the creation of non-`'static` `Rc`s require unsafe code until
   `ScopedRc` could be introduced.

# Alternatives

 * Don’t consider failing to destruct non-`'static` data unsafe.

   This is more or less the status quo, barring certain adjustments such as
   removing `unsafe` from `mem::forget`.

   This would be unfortunate, as it makes a very common RAII pattern unusable
   for anything memory-safety related. Furthermore, it breaks expectations. It
   seems like using an RAII guard should be memory safe, to the point that two
   new APIs were recently designed that relied on it for soundness, despite the
   fact that leaks have not been considered unsafe for a long time.

   It is very likely that people will continue to make this mistake (even if
   the core team doesn’t, third party developers almost certainly will). Rust’s
   strong lifetime ownership semantics make it seem like something that should
   be reliable. This is compounded by the fact that it is true of the core
   language, and “true-enough” in practice that developers will continue to
   rely on it, even if it’s not technically safe to do so.

   Even if the programmer is aware of the limitation, taking it into account
   can lead to more difficult and convoluted API design to ensure soundness.
   Given that they are *usually* reliable, a programmer my opt to go with an
   RAII design anyway to save time, reducing the value of Rust’s safety
   guarantees.

 * Leave `Rc` and friends. Add a `Leak` trait or similar. All APIs that can
   potentially leak (such as `Rc`) would have a `Leak` bound, and programmers
   who want to rely on their destructor being called before their lifetime ends
   would have to add a `Leak` bound to opt out of being usable with `Rc`s,
   channels, et cetera.

   While `!Leak` would technically only be needed for types can that lead to
   unsoundness if their destructor were skipped, many would probably use it for
   any guard-like type whose destructor “should” run, preventing their use in
   certain contexts unnecessarily.

   Additionally, it is likely that there will be types with *do* need the
   guarantees for memory safety that otherwise would be useful to keep in an
   `Rc` or send over a channel, which would not be possible.

   Instead of defining a single, simple rule for all types, the `Leak`
   trait would have to be specifically dealt with all over, e.g., by adding
   `+Leak` to the bounds of anything that you want to put in an `Rc`. Also, if
   leaking `drain_range` or something similar is made to leak the referenced
   values instead of leading to unsoundness, it needs to be `Leak` if and only
   if the contained type is.  This adds complexity and mental burden.

   Finally, while some of the solutions proposed in this RFC for handling
   non-`'static` data could be applied to `!Leak` data, the added complexity
   added by a `Leak` trait would be even less worth it if the complexity to
   work around it had to be added to `Rc`, channels, et cetera, anyway.

 * Guarantee that all stack-anchored objects have their destructor run if the
   program exits normally.

   This is much more challenging, would require sweeping changes in many areas,
   and is not even that useful: you can always move a `'static` object off the
   stack into a static location. Also, since `'static` objects can conceptually
   live forever, there seems little benefit to attempting to enforce otherwise.

   In addition, there several benefits to being able to being able to safely
   forget static data, such as forgetting a `File` to keep the file from being
   closed once you have transferred the underlying file descriptor.

 * Leave `Rc` as it stands for 1.0, and spend more time testing and refining
   `ScopedRc`. Deprecate `Rc` when `ScopedRc` is ready. This would allow more
   time to develop `ScopedRc` without limiting `Rc` in the meantime, but would
   mean postponing the guarantee set forth by this RFC until Rust 2.0.

 * A slight modification of this proposal, suggested by @pythonesque, would be
   to have `ScopedRcGuard` use `dropck` to statically prevent the possibility
   of dereferencing collected `ScopedRc`s. The idea would be to allow cycles if
   and only if no destructors could observe them. This would eliminate the need
   for a zero-check when dereferencing, potentially speeding up code, slightly.

   Unfortunately, the only way known to the author to accomplish this today
   would require `ScopedRcGuard` to take a type parameter, and thus require a
   distinct `ScopedRcGuard` for every type the programmer wanted to put in a
   `ScopedRc`. For programs like the Rust compiler that construct `Rc`s with
   many different types, this would be untenabled. Depending on usage patterns,
   constructing and passing around references to tens of `ScopedRcGuards` may
   cost more than it saves, performancewise.

   If it becomes possible in the future to impose `dropck` requirements where
   the compiler would not normally infer them, and thus to allow this solution
   to work without many different `ScopedRcGuard`s, it may be worth persuing.

# Unresolved questions

Should we also add an `UnguardedRc` type that can be created with static data
or using `NoCycleMarker`? Like `ScopedRc`, this would have a lifetime parameter
and use the same `RcBox` format. However, it would be statically guaranteed to
be unguarded, so no zero check would be needed to dereference it. Because they
use the same RcBox format, an `UnguardedRc` could be freely converted to a
`ScopedRc` and an unguarded `ScopedRc` could be converted to an `UnguardedRc`
with a quick runtime check. Even though they can be easily converted back and
forth, it’s still another type, and it’s unclear how often it would be needed.
