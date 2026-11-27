- Feature Name: clarifying_unwind_safety
- Start Date: 2020-02-27
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

The [`UnwindSafe`](https://doc.rust-lang.org/std/panic/trait.UnwindSafe.html) marker trait and associated machinery were stabilized in Rust 1.9 with the intent to help prevent a class of logic errors which might otherwise be introduced by the addition of the [`catch_unwind`](https://doc.rust-lang.org/std/panic/fn.catch_unwind.html) function.

However, the exact requirements for an `UnwindSafe` type have never been formally stated, and there is much confusion on the topic. Many types which probably should be considered `UnwindSafe` do not implement the trait, resulting in many false positives when `catch_unwind` is used. As a result, it is often regarded by Rust developers as simply a nuisance to be bypassed by liberal use of `AssertUnwindSafe`.

# Motivation
[motivation]: #motivation

With this RFC I hope to:
- Attain consensus on what precisely the `UnwindSafe` trait means.
- Agree a direction for improving the current state of `UnwindSafe`.


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

As this is a rather atypical RFC, this section will explain the current situation and propose a set of more formal rules for when a type should be considered `UnwindSafe`. Potential improvements will be discussed in the reference-level explanation.

## Background

The `UnwindSafe` trait was originally introduced under the name `PanicSafe` in [RFC 1236](https://github.com/rust-lang/rfcs/blob/master/text/1236-stabilize-catch-panic.md). That RFC gives many examples of how `UnwindSafe` might be used, but falls short of providing any formal definition of what is required for a type to be `UnwindSafe`. Since then there has been no further clarification of what this trait actually means. As a result, documentation has been written about this trait whose facts do not come from any accepted RFC, and which is (in my view) incorrect.

One key point from the RFC can be seen under the [future extensions](https://github.com/rust-lang/rfcs/blob/master/text/1236-stabilize-catch-panic.md#future-extensions) section, where it is explained that ideally there would be a blanket implementation of `UnwindSafe` for all `Send` types, but this is not possible due to coherence rules. This will become relevant later.

It should be noted that `UnwindSafe` is *not* related to memory safety. Incorrect usage of this trait cannot result in undefined behaviour.

## Pre-requisites

Before we dive in, we should agree on:

1) Do we want a mechanism like `UnwindSafe` at all?
2) If we do want it, do we want to have more formally defined semantics for this trait?

The rest of the RFC will proceed assuming the answer to these two questions is "yes", but that is something that can be further discussed.

## The goal of `UnwindSafe`

The goal of `UnwindSafe` is to reduce logic errors caused by broken invariants as a result of unwinding. Logic errors are avoided by reducing the number of places in the code where it is possible for these kinds of errors to occur. This is similar to memory safety: we don't try to eliminate unsafe code entirely, we merely contain it so that we can be more wary around that code. In the same way as `unsafe`, we use `AssertUnwindSafe` to break these invariants.

## A more formal definition

In order to more formally define what `UnwindSafe` means, we need to be able to list the "escape hatches" (like `AssertUnwindSafe`) which may break its guarantees. If we can list these, then as well as providing programmers a check-list for areas where unwind-safety needs be carefully considered, it also allows us to verify that code that does not make use of these escape hatches cannot suffer from this class of logic error.

In an ideal world, this list would contain *only* `AssertUnwindSafe`.

Current escape hatches:

1) `AssertUnwindSafe`.
2) Destructors - implementations of `Drop` can freely observe broken invariants as a result of a panic.
3) Accesses to thread-local storage - a thread might panic whilst mutating a value stored in TLS.
4) Accesses to `Sync` types - another thread could panic at any time whilst accessing the same value regardless of whether the type is `RefUnwindSafe`.

## `Sync` types

The reason that `Sync` types act as an escape hatch is because there is an implicit panic boundary between threads. We can `Send` a reference to a `Sync` type to another thread, and through this mechanism the type is able to cross a panic boundary (the other thread can panic whilst accessing the value and leave us with a broken invariant).

Given that all `Sync` types break any guarantees made by the `UnwindSafe` mechanism, we should ask whether it ever makes sense to have a type which is `Sync` but *not* `RefUnwindSafe`. This would amount to removing the "panic boundary" between a set of threads: creating a "thread group" where any panic within the group immediately unwinds all threads within the group. In this hypothetical, we could pass data between threads in the same group without requiring an `UnwindSafe` bound, and so there *could potentially* be a case where a type might usefully implement `Sync` but not `RefUnwindSafe`.

However, in practice this setup is untenable. Pre-empting threads to panic them in this way is not possible to do safely without extensive instrumentation by the compiler (see also: Java's `Thread.stop()` debacle). For this reason we can discard the notion of "thread groups" and we are left with the observation that every `Sync` type should also implement `RefUnwindSafe`, and as a result every `Send` type should implement `UnwindSafe`. Referring back to the beginning of this RFC, I believe this logic was clear to the author of the original `PanicSafe` RFC, but has since been lost to time.

## Poisoning

The documentation for `UnwindSafe` makes the [erroneous observation](https://doc.rust-lang.org/std/panic/trait.UnwindSafe.html#who-implements-unwindsafe) that types like `Mutex` implement `RefUnwindSafe` *because* they implement a poisoning mechanism. This is incorrect: as we saw above, the `RefUnwindSafe` trait is meaningless for *all* `Sync` types because of the implicit panic boundary between threads. We can also observe that many existing internally mutable `Sync` types do in fact [already implement `RefUnwindSafe`](https://doc.rust-lang.org/std/sync/atomic/struct.AtomicUsize.html#impl-RefUnwindSafe) despite not being poisonable.

The only time poisoning would affect whether a type is `RefUnwindSafe` is when that type is `!Sync`. For example, a type like `RefCell` could implement a poisoning mechanism and this would allow it to implement `RefUnwindSafe`.

The real value of poisoning is that it allows us to significantly shrink the number of "escape hatches" to the `UnwindSafe` mechanism:

~~4) Accesses to `Sync` types.~~

4) Accesses to `Sync` types which do not implement poisoning.

This is a big improvement: we only need to think about unwind safety when dealing with specific `Sync` types like `AtomicXXX` or the non-poisonable concurrency primitives from `parking_lot`.

## When should a type be `RefUnwindSafe`?

A type should be `RefUnwindSafe` iff any of the following are true:

1) The type is not internally mutable.
2) The type is `Sync`.
3) The type implements poisoning.
4) The type is `AssertUnwindSafe`.

These are very simple and clear rules for developers to follow.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Here we will discuss what we could potentially do to improve the situation. The intent is not for this RFC to actually propose these changes; instead, acceptance of this RFC would imply agreement that these kinds of changes would be in-line with how we expect `UnwindSafe` to work. A future RFC would be needed to actually make the changes, and that future RFC would have to deal with the backwards compatibility concerns that any such change raises.

## Ideal world

If we didn't have to consider backwards compatibility, we might consider making one or more of the following changes:

1) Make `UnwindSafe` and `RefUnwindSafe` super-traits of `Send` and `Sync` respectively.

This would largely eliminate the false positives thrown up by almost all uses of `catch_unwind` today.

2) Require an `UnwindSafe` bound on all types stored in thread-local storage.

This eliminates an escape hatch.

If we made these changes, we would have effectively eliminated all false-positive warnings from `catch_unwind` whilst reducing the number of places the programmer needs to consider unwind safety to solely:

1) `AssertUnwindSafe`.
2) Destructors.
3) Accesses to `Sync` types which do not implement poisoning.

## Real world

In practice, all of these are breaking changes. However, there is a path to integrating some of them which aligns with Rust's backwards compatibility policy:

### Make `UnwindSafe` and `RefUnwindSafe` super-traits of `Send` and `Sync` respectively.

This is the one change which we absolutely cannot make. However, we could:

- Clearly document the relationship between `Send`/`Sync` and `UnwindSafe`/`RefUnwindSafe` so that crate authors can implement these traits appropriately.
- Introduce warnings when types implement `Send` or `Sync` but not the corresponding trait.
- If/when overlapping marker trait implementations are allowed, provide blanket implementations of `UnwindSafe` and `RefUnwindSafe` for `Send` and `Sync` types.

### Require an `UnwindSafe` bound on all types stored in thread-local storage.

This could be introduced as a warning, but it may require the compiler having special knowledge of the thread local types, or else require some general mechanism to warn when a trait bound is missing.

# Drawbacks
[drawbacks]: #drawbacks

This RFC does not actually propose any language changes. I don't believe there are any drawbacks to clarifying an existing part of the language.

# Alternatives
[alternatives]: #alternatives

- Remove the `UnwindSafe` bound on `catch_unwind` altogether.

  This is certainly an option. However, I believe `UnwindSafe` can provide real value if it is used correctly. I believe the current problems are ones of implementation rather than an inherent problem with the concept.

- Leave the `UnwindSafe` trait as an "informal" warning with no specific definition.

  Marker traits impose a significant burden on developers using Rust. I believe if a marker trait does not allow us to make formal statements about a program then it is not worth the cost.

- Find a different (yet consistent) way to define `UnwindSafe`.

  I did not come across any other consistent ways to define this trait, but suggestions are welcome.
