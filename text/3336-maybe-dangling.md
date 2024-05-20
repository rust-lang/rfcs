# `maybe_dangling`

- Feature Name: `maybe_dangling`
- Start Date: 2022-09-30
- RFC PR: [rust-lang/rfcs#3336](https://github.com/rust-lang/rfcs/pull/3336)
- Tracking Issue: [rust-lang/rust#118166](https://github.com/rust-lang/rust/issues/118166)

# Summary
[summary]: #summary

Declare that references and `Box` inside a new `MaybeDangling` type do not need to satisfy any memory-dependent validity properties (such as `dereferenceable` and `noalias`).

# Motivation
[motivation]: #motivation

### Example 1

Sometimes one has to work with references or boxes that either are already deallocated, or might get deallocated too early.
This comes up particularly often with `ManuallyDrop`.
For example, the following code is UB at the time of writing this RFC:

```rust
fn id<T>(x: T) -> T { x }

fn unsound(x: Box<i32>) {
    let mut x = ManuallyDrop::new(x);
    unsafe { x.drop() };
    id(x); // or `let y = x;` or `mem::forget(x);`.
}

unsound(Box::new(42));
```
It is unsound because we are passing a dangling `ManuallyDrop<Box<i32>>` to `id`.
In terms of invariants required by the language ("validity invariants"), `ManuallyDrop` is a regular `struct`, so all its fields have to be valid, but that means the `Box` needs to valid, so in particular it must point to allocated memory -- but when `id` is invoked, the `Box` has already been deallocated.
Given that `ManuallyDrop` is specifically designed to allow dropping the `Box` early, this is a big footgun (that people do [run into in practice](https://github.com/rust-lang/miri/issues/1508)).

### Example 2

There exist more complex versions of this problem, relating to a subtle aspect of the (currently poorly documented) aliasing requirements of Rust:
when a reference is passed to a function as an argument (including nested in a struct), then that reference must remain live throughout the function.
(In LLVM terms: we are annotating that reference with `dereferenceable`, which means "dereferenceable for the entire duration of this function call"). In [issue #101983](https://github.com/rust-lang/rust/issues/101983), this leads to a bug in `scoped_thread`.
There we have a function that invokes a user-supplied `impl FnOnce` closure, roughly like this:
```rust
// Not showing all the `'lifetime` tracking, the point is that
// this closure might live shorter than `thread`.
fn thread(control: ..., closure: impl FnOnce() + 'lifetime) {
    closure();
    control.signal_done();
    // A lot of time can pass here.
}
```
The closure has a non-`'static` lifetime, meaning clients can capture references to on-stack data.
The surrounding code ensure that `'lifetime` lasts at least until `signal_done` is triggered, which ensures that the closure never accesses dangling data.

However, note that `thread` continues to run even after `signal_done`! Now consider what happens if the closure captures a reference of lifetime `'lifetime`:
- The type of `closure` is a struct (the implicit unnameable closure type) with a `&'lifetime mut T` field.
  References passed to a function must be live for the entire duration of the call.
- The closure runs, `signal_done` runs.
  Then -- potentially -- this thread gets scheduled away and the main thread runs, seeing the signal and returning to the user.
    Now `'lifetime` ends and the memory the reference points to might be deallocated.
- Now we have UB! The reference that as passed to `thread` with the promise of remaining live for the entire duration of the function, actually got deallocated while the function still runs. Oops.

### Example 3

As a third example, consider a type that wants to store a "pointer together with some data borrowed from that pointer", like the `owning_ref` crate. This will usually boil down to something like this:

```rust
unsafe trait StableDeref: Deref {}

struct OwningRef<U, T: StableDeref<Target=U>> {
    buffer: T,
    ref_: NonNull<U>, // conceptually borrows from `buffer`.
}
```

Such a type is unsound when `T` is `&mut U` or `Box<U>` because those types are assumed by the compiler to be unique, so any time `OwningRef` is passed around, the compiler can assume that `buffer` is a unique pointer -- an assumption that this code breaks because `ref_` points to the same memory!

### Goal of this RFC

The goal of this RFC is to
- make the first example UB-free without code changes
- make the second example UB-free without needing to add `unsafe` code
- make it possible to define a type like the third example

(Making the 2nd example UB-free without code changes would incur cost across the ecosystem, see the alternatives discussed below.)

The examples described above are far from artificial, here are some real-world crates that need `MaybeDangling` to ensure their soundness (some currently crudely work-around that problem with `MaybeUninit` but that is really not satisfying):
- [Yoke](https://github.com/unicode-org/icu4x/issues/3696) and [Yoke again](https://github.com/unicode-org/icu4x/issues/2095) (the first needs opting-out of `dereferenceable` for the yoke, the latter needs opting-out of `noalias` for both yoke and cart)
- [ouroboros](https://github.com/joshua-maros/ouroboros/issues/88)

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

To handle situations like this, Rust has a special type called `MaybeDangling<P>`:
references and boxes in `P` do *not* have to be dereferenceable or follow aliasing guarantees.
This applies inside nested references/boxes inside `P` as well.
They still have to be non-null and aligned, and it has to at least be *possible* that there exists valid data behind that reference (i.e., `MaybeDangling<&!>` is still invalid).
Also note that safe code can still generally assume that every `MaybeDangling<P>` it encounters is a valid `P`, but within unsafe code this makes it possible to store data of arbitrary type without making reference guarantees (this is similar to `ManuallyDrop`).
In other words, `MaybeDangling<P>` is entirely like `P`, except that the rules that relate to the contents of memory that pointers in `P` point to (dereferencability and aliasing restrictions) are suspended when the pointers are not being actively used.
You can think of the `P` as being "suspended" or "inert".

The `ManuallyDrop<T>` type internally wraps `T` in a `MaybeDangling`.

This means that the first example is actually fine:
the dangling `Box` was passed inside a `ManuallyDrop`, so there is no UB.

The 2nd example can be fixed by passing the closure in a `MaybeDangling`:
```rust
// Argument is passed as `MaybeDangling` since we might actually keep
// it around after its lifetime ends (at which point the caller can
// start dropping memory it points to).
fn thread(control: ..., closure: MaybeDangling<impl FnOnce() + 'lifetime>) {
    closure.into_inner()();
    control.signal_done();
    // A lot of time can pass here.
}
```

The 3rd example can be fixed by storing the `buffer` inside a `MaybeDangling`, which disables its aliasing requirements:

```rust
struct OwningRef<U, T: StableDeref<Target=U>> {
    buffer: MaybeDangling<T>,
    ref_: NonNull<U>, // conceptually borrows from `buffer`.
}
```

As long as the `buffer` field is not used, the pointer stored in `ref_` will remain valid.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The standard library contains a type `MaybeDangling<P>` that is safely convertible with `P` (i.e., the safety invariant is the same), and that has all the same niches as `P`, but that does allow passing around dangling boxes and references within unsafe code.
`MaybeDangling<P>` propagates auto traits, drops the `P` when it is dropped, and has (at least) `derive(Copy, Clone, Debug)`.

"Behavior considered undefined" is adjusted as follows:

```diff
  * Breaking the [pointer aliasing rules]. `Box<T>`, `&mut T` and `&T` follow LLVM’s
    scoped noalias model, except if the `&T` contains an [`UnsafeCell<U>`].
    References must not be dangling while they are live. (The exact liveness
    duration is not specified, but it is certainly upper-bounded by the syntactic
    lifetime assigned by the borrow checker. When a reference is passed to a
    function, it is live at least as long as that function call, again except if
    the `&T` contains an [`UnsafeCell<U>`].) All this also applies when values of
    these types are passed in a (nested) field of a compound type, but not behind
-   pointer indirections.
+   pointer indirections and also not for values inside a `MaybeDangling<_>`.
[...]
   * Producing an invalid value, even in private fields and locals.
     "Producing" a value happens any time a value is assigned to or
     read from a place, passed to a function/primitive operation or
     returned from a function/primitive operation. The following
     values are invalid (at their respective type):
[...]
-  * A reference or Box<T> that is dangling, unaligned, or points to an
-    invalid value.
+  * A reference or `Box<T>` that is unaligned or null, or whose pointee
+    type `T` is uninhabited. Furthermore, except when this value occurs
+    inside a `MaybeDangling`, if the reference/`Box<T>` is dangling or points
+    to an invalid value, it is itself invalid.
```

*Note: this diff is based on [an updated version of the reference](https://github.com/rust-lang/reference/pull/1290).*

Another way to think about this is: most types only have "by-value" requirements for their validity, i.e., they only require that the bit pattern be of a certain shape.
References and boxes are the sole exception, they also require some properties of the memory they point to (e.g., they need to be dereferenceable).
`MaybeDangling<T>` is a way to "truncate" `T` to its by-value invariant, which changes nothing for most types, but means that references and boxes are allowed as long as their bit patterns are fine (aligned and non-null) and as long as there *conceivably could be* a state of memory that makes them valid (`T` is inhabited).

codegen is adjusted as follows:

- When computing LLVM attributes, we traverse through newtypes such that `Newtype<&mut i32>` is marked as `dereferenceable(4) noalias aligned(4)`.
  When traversing below `MaybeDangling`, no memory-related attributes such as `dereferenceable` or `noalias` are emitted. Other value-related attributes such as `aligned` are still emitted. (Really this happens as part of computing the `ArgAttributes` in the function ABI, and that is the code that needs to be adjusted.)

Miri is adjusted as follows:

- During Stacked Borrows retagging, when recursively traversing the value to search for references and boxes to retag, we stop the traversal when encountering a `MaybeDangling`.
  (Note that by default, Miri will not do any such recursion, and only retag bare references.
  But that is not sound, given that we do emit `noalias` for newtyped references and boxes.
  The `-Zmiri-retag-fields` flag makes retagging "peer into" compound types to retag all references it can find.
  This flag needs to become the default to make Miri actually detect all UB in the LLVM IR we generate. This RFC says that that traversal stops at `MaybeDangling`.)

### Comparison with some other types that affect aliasing

- `UnsafeCell`: disables aliasing (and affects but does not fully disable dereferenceable) behind shared refs, i.e. `&UnsafeCell<T>` is special. `UnsafeCell<&T>` (by-val, fully owned) is not special at all and basically like `&T`; `&mut UnsafeCell<T>` is also not special.
- [`UnsafePinned`](https://github.com/rust-lang/rfcs/pull/3467): disables aliasing (and affects but does not fully disable dereferenceable) behind mutable refs, i.e. `&mut UnsafePinned<T>` is special. `UnsafePinned<&mut T>` (by-val, fully owned) is not special at all and basically like `&mut T`; `&UnsafePinned<T>` is also not special.
- `MaybeDangling`: disables aliasing and dereferencable *of all references (and boxes) directly inside it*, i.e. `MaybeDangling<&[mut] T>` is special. `&[mut] MaybeDangling<T>` is not special at all and basically like `&[mut] T`.


# Drawbacks
[drawbacks]: #drawbacks

- For users of `ManuallyDrop` that don't need this exceptions, we might miss optimizations if we start allowing example 1.
- We are accumulating quite a few of these marker types to control various aspect of Rust's validity and aliasing rules:
  we already have `UnsafeCell` and `MaybeUninit`, and we are likely going to need a "mutable reference version" of `UnsafeCell` to properly treat self-referential types.
  It's easy to get lost in this sea of types and mix up what exactly they are acting on and how.
  In particular, it is easy to think that one should do `&mut MaybeDangling<T>` (which is useless, it should be `MaybeDangling<&mut T>`) -- this type applies in the exact opposite way compared to `UnsafeCell` (where one uses `&UnsafeCell<T>`, and `UnsafeCell<&T>` is useless).

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- The most obvious alternative is to declare `ManuallyDrop` to be that magic type with the memory model exception.
  This has the disadvantage that one risks memory leaks when all one wants to do is pass around data of some `T` without upholding reference liveness.
    For instance, the third example would have to remember to call `drop` on the `buffer`.
    This alternative has the advantage that we avoid introducing another type, and it is future-compatible with factoring that aspect of `ManuallyDrop` into a dedicated type in the future.
- Another alternative is to change the memory model such that the example code is fine as-is.
  There are several variants of this:
    - [Make all examples legal] All newtype wrappers behave the way `MaybeDangling` is specified in this RFC.
      This means it is impossible to do zero-cost newtype-wrapping of references and boxes, which is against the Rust value of zero-cost abstractions.
      It is also a non-compositional surprise for type semantics to be altered through a newtype wrapper.
    - [Make examples 1+2 legal] Or we leave newtype wrappers untouched, but rule that boxes (and references) don't actually have to be dereferenceable.
      This is just listed for completeness' sake, removing all those optimizations is unlikely to make our codegen folks happy. It is also insufficient for example 3, which is about aliasing, not dereferencability.
    - [Make only the 2nd example legal] We could remove the part about references always being live for at least as long as the functions they are passed to.
      This corresponds to replacing the LLVM `dereferenceable` attribute by a (planned by not yet implemented) `dereferenceable-on-entry`, which matches the semantics of references in C++.
      But that does not solve the problem of the `MaybeUninit<Box<_>>` footgun, i.e., the first example.
      (We would have to change the rules for `Box` for that, saying it does not need to be dereferenceable at all.)
      Nor does it help the 3rd example.
      Also this loses some very desirable optimizations, such as
        ```rust
        fn foo(x: &i32) -> i32 {
            let val = *x;
            bar();
            return val; // optimize to `*x`, avoid saving `val` across the call.
        }
        ```
        Under the adjusted rules, `x` could stop being live in the middle of the execution of `foo`, so it might not be live any more when the `return` is executed.
        Therefore the compiler is not allowed to insert a new use of `x` there.
- We could more directly expose ways to manipulate the underlying LLVM attributes (`dereferenceable`, `noalias`) using by-value wrappers.
  (When adjusting the pointee type, such as in `&UnsafeCell<T>`, we already provide a bunch of fine-grained control.)
  However there exist other backends, and LLVM attributes were designed for C/C++/Swift, not Rust. The author would argue that we should first think of the semantics we want, and then find ways to best express them in LLVM, not the other way around.
  And while situations are conceivable where one wants to disable only `noalias` or only `dereferenceable`, it is unclear whether they are worth the extra complexity.
  (On the pointee side, Rust used to have a `Unique` type, that still exists internally in the standard library, which was intended to provide `noalias` without any form of `dereferenceable`. It was deemed better to not expose this.)
- Instead of saying that all fields of all compound types still must abide by the aliasing rules, we could restrict this to fields of `repr(transparent)` types.
  That would solve the 2nd and 3rd example without any code changes.
  It would make it impossible to package up multiple references (in a struct with multiple reference-typed fields) in a way that their aliasing guarantees are still in full force.
  Right now, we actually *do* emit `noalias` for the 2nd and 3rd example, so codegen of existing types would have to be changed under this alternative.
  It would not help for the first example.
- Finally we could do nothing and declare all examples as intentional UB.
  The 2nd and 3rd example could use `MaybeUninit` to pass around the closure / the buffer in a UB-free way.
  That will however require `unsafe` code, and leaves `ManuallyDrop<Box<T>>` with its footgun (1st example).

# Prior art
[prior-art]: #prior-art

The author cannot think of prior art in other languages; the issue arises because of Rust's unique combination of strong safety guarantees with low-level types such as `ManuallyDrop` that manage memory allocation in a very precise way.

Inside Rust, we do have precedent for wrapper types altering language semantics; most prominently, there are `UnsafeCell` and `MaybeUninit`.
Notice that `UnsafeCell` acts "behind references" while `MaybeDangling`, like `MaybeUninit`, acts "around references": `MaybeDangling<&T>` vs `&UnsafeCell<T>`.

There is a [crate](https://docs.rs/maybe-dangling) offering these semantics on stable Rust via `MaybeUninit`.
(This is not "prior" art, it was published after this RFC came out. "Related work" would be more apt. Alas, the RFC template forces this structure on us.)

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- What should the type be called?
  `MaybeDangling` is somewhat misleading since the *safety* invariant still requires everything to be dereferenceable, only the *validity* requirement of dereferenceability and noalias is relaxed.
  This is a bit like `ManuallyDrop` which supports dropping via an `unsafe` function but its safety invariant says that the data is not dropped (so that it can implement `Deref` and `DerefMut` and a safe `into_inner`).
  Furthermore, the type also allows maybe-aliasing references, not just maybe-dangling references.
  Other possible names might be things like `InertPointers` or `SuspendedPointers`.
- Should `MaybeDangling` implement `Deref` and `DerefMut` like `ManuallyDrop` does, or should accessing the inner data be more explicit since that is when the aliasing and dereferencability requirements do come back in full force?

# Future possibilities
[future-possibilities]: #future-possibilities

- One issue with this proposal is the "yet another wrapper type" syndrome, which leads to lots of syntactic salt and also means one loses the special `Box` magic (such as moving out of fields).
  This could be mitigated by either providing an attribute that attaches `MaybeDangling` semantics to an arbitrary type, or by making `Box` magic more widely available (`DerefMove`/`DerefPure`-style traits).
  Both of these are largely orthogonal to `MaybeDangling` though, and we'd probably want the `MaybeDangling` type as the "canonical" type for this even if the attribute existed (e.g., for cases like example 2).
