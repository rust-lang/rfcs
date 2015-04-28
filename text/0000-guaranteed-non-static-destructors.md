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
run before any code outside of their bounding lifetimes could be executed. Even
if both are fixed, it is very likely that similar errors will arise in the
future.

While we can try to continuously hammer home that leaks are not considered
`unsafe`, we'll never be able to prevent all such mistakes. It would be better
if we could actually provide the intuitive guarantee, which is what this RFC
attempts to accomplish.

Note that this RFC is explicitly not attempting to solve all leaks, nor even to
ensure that all stack-anchored objects are destructed. Rust is primarily
concerned with memory safety, so this RFC tries to formulate a general rule
that addresses the issue of incorrect assumptions about destructors leading to
memory unsafety. It does not attempt to prevent the leaking of other resources,
as leaking a `'static` object is never unsafe. (`'static` objects are
conceptually safe to keep around “forever”, so any unsoundness that can be
obtained by a leak could also be obtained through other means).

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
 * A resource that is never freed due to and endless loop, a deadlock, or
   program termination is valid because any code that comes after the end of
   `'a` is never executed.
 * Code would be allowed to rely on the guarantee for soundness. This means
   that patterns such as `thread::scoped` and the initially-proposed
   `Vec::drain_range` would be sound.
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
 * Restrict the basic `new` operation of existing reference-counted types to
   types valid for `'static`, which are always safe to leak. Additionally,
   introduce a scoped cycle collector that can be used to safely create `Rc`s
   with a shorter lifetime, and research the possibility of a reference-counted
   type that statically disallows cycles.

Specifically, the scoped cycle collector would operate as follows:

 * There is a new `ScopedRc` type that has an added lifetime parameter
   specifying for how long the reference itself is valid.
 * There is also a `ScopedRcGuard` type that can be instantiated with a given
   lifetime `'a`.
 * There is a new `ScopedRc` type that can only be constructed with a
   `ScopedRcGuard`, and cannot outlive the guard.
 * A `ScopedRc` created from a `ScopedRcGuard<'a>` can only contain data that
   is valid for the duration of `'a`.
 * When the `ScopedRcGuard` is dropped, it collects any remaining cycles.
 * During cycle collection, an attempt by one object in the cycle to
   dereference another that has already been collected will result in an abort.

See [this gist](https://gist.github.com/rkjnsn/791ee9cc3b6d2961cf33) for a
working `ScopedRc` implementation, including sample usage.

The guard technique can also be applied to channels to ensure that they are
properly cleaned up even if the user does something like send the receiver and
sender into their own channel.

# Drawbacks

 * Using `Rc`s for non-`'static` types will be slightly less convenient. The
   user will either have to use the cycle guard, an acyclic Rc type, or use
   `unsafe` and manually verify that no leaks are possible. (In the compiler,
   for instance, the guard would need to be somewhere outlived by the type
   arenas (perhaps in the `ctxt` object)).
 * Probably not enough time to implement all of this by 1.0. This can be
   mitigated by implementing the minimum necessary to make this
   backward-compatible. Since the behavior of panicking destructors are already
   unspecified and subject to change, this would mean restricting the safe
   creation of reference-counted objects to containing `'static` values and
   adding an `unsafe` way to use shorter lifetimes. This would make using `Rc`s
   for non-`'static` data impossible without `unsafe` in 1.0, which is not
   ideal.

# Alternatives

 * Don't consider failing to destruct non-`'static` data unsafe.

   This is more or less the status quo, barring certain adjustments such as
   removing `unsafe` from `mem::forget`.

   This would be unfortunate, as it makes a very common RAII pattern unusable
   for anything memory-safety related. Furthermore, it breaks expectations. It
   seems like using an RAII guard should be memory safe, to the point that two
   new APIs were recently designed that relied on it for soundness, despite the
   fact that leaks have not been considered unsafe for a long time.

   It is very likely that people will continue to make this mistake (even if
   the core team doesn't, third party developers almost certainly will). Rust’s
   strong lifetime ownership semantics make it seem like something that should
   be reliable. This is compounded by the fact that it is true of the core
   language, and “true-enough” in practice that developers will continue to
   assume that it can be relied upon.

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

# Unresolved questions

What are the best designs for safely allowing channels and `Rc`s to safely work
with non-`'static` data?

Should the `ScopedRc` functionality outlined above be added as separate types,
or should we add a lifetime parameter to the existing `Rc`, `Arc`, et cetera to
encourage the same types to be used for both `'static` and non-`'static` data?
