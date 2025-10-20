- Feature Name: `unsafe_fields`
- Start Date: 2023-07-13
- RFC PR: [rust-lang/rfcs#3458](https://github.com/rust-lang/rfcs/pull/3458)
- Rust Issue: [rust-lang/rust#132922](https://github.com/rust-lang/rust/issues/132922)

# Summary

This RFC proposes extending Rust's tooling support for safety hygiene to named fields that carry
library safety invariants. Consequently, Rust programmers will be able to use the `unsafe` keyword
to denote when a named field carries a library safety invariant; e.g.:

```rust
struct UnalignedRef<'a, T> {
    /// # Safety
    /// 
    /// `ptr` is a shared reference to a valid-but-unaligned instance of `T`.
    unsafe ptr: *const T,
    _lifetime: PhantomData<&'a T>,
}
```

Rust will enforce that potentially-invalidating uses of such fields only occur in the context of an
`unsafe` block, and Clippy's [`missing_safety_doc`] lint will check that such fields have
accompanying safety documentation.

[`missing_safety_doc`]: https://rust-lang.github.io/rust-clippy/master/index.html#missing_safety_doc

# Motivation

Safety hygiene is the practice of denoting and documenting where memory safety obligations arise
and where they are discharged. Rust provides some tooling support for this practice. For example,
if a function has safety obligations that must be discharged by its callers, that function *should*
be marked `unsafe` and documentation about its invariants *should* be provided (this is optionally
enforced by Clippy via the [`missing_safety_doc`] lint). Consumers, then, *must* use the `unsafe`
keyword to call it (this is enforced by rustc), and *should* explain why its safety obligations are
discharged (again, optionally enforced by Clippy).

Functions are often marked `unsafe` because they concern the safety invariants of fields. For
example, [`Vec::set_len`] is `unsafe`, because it directly manipulates its `Vec`'s length field,
which carries the invariants that it is less than the capacity of the `Vec` and that all elements
in the `Vec<T>` between 0 and `len` are valid `T`. It is critical that these invariants are upheld;
if they are violated, invoking most of `Vec`'s other methods will induce undefined behavior.

[`Vec::set_len`]: https://doc.rust-lang.org/std/vec/struct.Vec.html#method.set_len

To help ensure such invariants are upheld, programmers may apply safety hygiene techniques to
fields, denoting when they carry invariants and documenting why their uses satisfy their
invariants. For example, the `zerocopy` crate maintains the policy that fields with safety
invariants have `# Safety` documentation, and that uses of those fields occur in the lexical
context of an `unsafe` block with a suitable `// SAFETY` comment.

Unfortunately, Rust does not yet provide tooling support for field safety hygiene. Since the
`unsafe` keyword cannot be applied to field definitions, Rust cannot enforce that
potentially-invalidating uses of fields occur in the context of `unsafe` blocks, and Clippy cannot
enforce that safety comments are present either at definition or use sites. This RFC is motivated
by the benefits of closing this tooling gap.

### Benefit: Improving Field Safety Hygiene

The absence of tooling support for field safety hygiene makes its practice entirely a matter of
programmer discipline, and, consequently, rare in the Rust ecosystem. Field safety invariants
within the standard library are sparingly and inconsistently documented; for example, at the time
of writing, `Vec`'s capacity invariant is internally documented, but its length invariant is not.

The practice of using `unsafe` blocks to denote dangerous uses of fields with safety invariants is
exceedingly rare, since Rust actively lints against the practice with the `unused_unsafe` lint.

Alternatively, Rust's visibility mechanisms can be (ab)used to help enforce that dangerous uses
occur in `unsafe` blocks, by wrapping type definitions in an enclosing `def` module that mediates
construction and access through `unsafe` functions; e.g.:

```rust
/// Used to mediate access to `UnalignedRef`'s conceptually-unsafe fields.
///
/// No additional items should be placed in this module. Impl's outside of this module should
/// construct and destruct `UnalignedRef` solely through `from_raw` and `into_raw`.
mod def {
    pub struct UnalignedRef<'a, T> {
        /// # Safety
        /// 
        /// `ptr` is a shared reference to a valid-but-unaligned instance of `T`.
        pub(self) unsafe ptr: *const T,
        pub(self) _lifetime: PhantomData<&'a T>,
    }

    impl<'a, T> UnalignedRef<'a, T> {
        /// # Safety
        ///
        /// `ptr` is a shared reference to a valid-but-unaligned instance of `T`.
        pub(super) unsafe fn from_raw(ptr: *const T) -> Self {
            Self { ptr, _lifetime: PhantomData }
        }

        pub(super) fn into_raw(self) -> *const T {
            self.ptr
        }
    }
}

pub use def::UnalignedRef;
```

This technique poses significant linguistic friction and may be untenable when split borrows are
required. Consequently, this approach is uncommon in the Rust ecosystem.

We hope that tooling that supports and rewards good field safety hygiene will make the practice
more common in the Rust ecosystem.

### Benefit: Improving Function Safety Hygiene

Rust's safety tooling ensures that `unsafe` operations may only occur in the lexical context of an
`unsafe` block or `unsafe` function. When the safety obligations of an operation cannot be
discharged entirely prior to entering the `unsafe` block, the surrounding function must, itself, be
`unsafe`. This tooling cue nudges programmers towards good function safety hygiene.

The absence of tooling for field safety hygiene undermines this cue. The [`Vec::set_len`] method
*must* be marked `unsafe` because it delegates the responsibility of maintaining `Vec`'s safety
invariants  to its callers. However, the implementation of [`Vec::set_len`] does not contain any
explicitly `unsafe` operations. Consequently, there is no tooling cue that suggests this function
should be unsafe — doing so is entirely a matter of programmer discipline.

Providing tooling support for field safety hygiene will close this gap in the tooling for function
safety hygiene.

### Benefit: Making Unsafe Rust Easier to Audit

As a consequence of improving function and field safety hygiene, the process of auditing internally
`unsafe` abstractions will be made easier in at least two ways. First, as previously discussed, we
anticipate that tooling support for field safety hygiene will encourage programmers to document
when their fields carry safety invariants.

Second, we anticipate that good field safety hygiene will narrow the scope of safety audits.
Presently, to evaluate the soundness of an `unsafe` block, it is not enough for reviewers to *only*
examine `unsafe` code; the invariants upon which `unsafe` code depends may also be violated in safe
code. If `unsafe` code depends upon field safety invariants, those invariants may presently be
violated in any safe (or unsafe) context in which those fields are visible. So long as Rust permits
safety invariants to be violated at-a-distance in safe code, audits of unsafe code must necessarily
consider distant safe code. (See [*The Scope of Unsafe*].)

[*The Scope of Unsafe*]: https://www.ralfj.de/blog/2016/01/09/the-scope-of-unsafe.html

For crates that practice good safety hygiene, reviewers will mostly be able to limit their review
of distant routines to only `unsafe` code.

# Guide-level explanation

A safety invariant is any boolean statement about the computer at a time *t*, which should remain
true or else undefined behavior may arise. Language safety invariants are imposed by the Rust
itself and must never be violated; e.g., a `NonZeroU8` must *never* be 0.

Library safety invariants, by contrast, are imposed by an API. For example, `str` encapsulates
valid UTF-8 bytes, and much of its API assumes this to be true. This invariant may be temporarily
violated, so long as no code that assumes this safety invariant holds is invoked.

Safety hygiene is the practice of denoting and documenting where memory safety obligations arise
and where they are discharged. To denote that a field carries a library safety invariant, use the
`unsafe` keyword in its declaration and document its invariant; e.g.:

```rust
pub struct UnalignedRef<'a, T> {
    /// # Safety
    /// 
    /// `ptr` is a shared reference to a valid-but-unaligned instance of `T`.
    unsafe ptr: *const T,
    _lifetime: PhantomData<&'a T>,
}
```

You should use the `unsafe` keyword on any field that carries a library safety invariant which
differs from the invariant provided by its type.

The `unsafe` field modifier is only applicable to named fields. You should avoid attaching library
safety invariants to unnamed fields.

Rust provides tooling to help you maintain good field safety hygiene. Clippy's
[`missing_safety_doc`] lint checks that `unsafe` fields have accompanying safety documentation.

The Rust compiler enforces that uses of `unsafe` fields that could violate its invariant — i.e.,
initializations, writes, references, and copies — must occur within the context of an `unsafe`
block. For example, compiling this program:

```rust
#![forbid(unsafe_op_in_unsafe_fn)]

pub struct Alignment {
    /// SAFETY: `pow` must be between 0 and 29 (inclusive).
    pub unsafe pow: u8,
}

impl Alignment {
    pub fn new(pow: u8) -> Option<Self> {
        if pow > 29 {
            return None;
        }

        Some(Self { pow })
    }

    pub fn as_log(self) -> u8 {
        self.pow
    }

    /// # Safety
    ///
    /// The caller promises to not write a value greater than 29 into the returned reference.
    pub unsafe fn as_mut_log(&mut self) -> &mut u8 {
        &mut self.pow
    }
}
```

...emits the errors:

```
error[E0133]: initializing type with an unsafe field is unsafe and requires unsafe block
  --> src/lib.rs:14:14
   |
14 |         Some(Self { pow })
   |              ^^^^^^^^^^^^ initialization of struct with unsafe field
   |
   = note: unsafe fields may carry library invariants

error[E0133]: use of unsafe field is unsafe and requires unsafe block
  --> src/lib.rs:18:9
   |
18 |         self.pow
   |         ^^^^^^^^ use of unsafe field
   |
   = note: unsafe fields may carry library invariants

error[E0133]: use of unsafe field is unsafe and requires unsafe block
  --> src/lib.rs:25:14
   |
25 |         &mut self.pow
   |              ^^^^^^^^ use of unsafe field
   |
   = note: for more information, see <https://doc.rust-lang.org/nightly/edition-guide/rust-2024/unsafe-op-in-unsafe-fn.html>
   = note: unsafe fields may carry library invariants
note: an unsafe function restricts its caller, but its body is safe by default
  --> src/lib.rs:24:5
   |
24 |     pub unsafe fn as_mut_lug(&mut self) -> &mut u8 {
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
note: the lint level is defined here
  --> src/lib.rs:1:38
   |
1  | #![forbid(unsafe_op_in_unsafe_fn)]
   |           ^^^^^^^^^^^^^^^^^^^^^^

For more information about this error, try `rustc --explain E0133`.
```

...which may be resolved by wrapping the use-sites in `unsafe { ... }` blocks; e.g.:

```diff
  #![forbid(unsafe_op_in_unsafe_fn)]

  pub struct Alignment {
      /// SAFETY: `pow` must be between 0 and 29 (inclusive).
      pub unsafe pow: u8,
  }

  impl Alignment {
      pub fn new(pow: u8) -> Option<Self> {
          if pow > 29 {
              return None;
          }
-         Some(Self { pow })
+         // SAFETY: We have ensured that `pow <= 29`.
+         Some(unsafe { Self { pow } })
      }

      pub fn as_log(self) -> u8 {
-         self.pow
+         // SAFETY: Copying `pow` does not violate its invariant.
+         unsafe { self.pow }
      }

      /// # Safety
      ///
      /// The caller promises to not write a value greater than 29 into the returned reference.
      pub unsafe fn as_mut_lug(&mut self) -> &mut u8 {
-         &mut self.pow
+         // SAFETY: The caller promises not to violate `pow`'s invariant.
+         unsafe { &mut self.pow }
      }
  }
```

You may use `unsafe` to denote that a type relaxes its type's library safety invariant; e.g.:

```rust
struct MaybeInvalidStr {
    /// SAFETY: `maybe_invalid` may not contain valid UTF-8. Nonetheless, it MUST always contain
    /// initialized bytes (per language safety invariant on `str`).
    unsafe maybe_invalid: str
}
```

...but you *must* ensure that the field is soundly droppable before it is dropped. A `str` is bound
by the library safety invariant that it contains valid UTF-8, but because it is trivially
destructible, no special action needs to be taken to ensure it is in a safe-to-drop state.

By contrast, `Box` has a non-trivial destructor which requires that its referent has the same size
and alignment that the referent was allocated with. Adding the `unsafe` modifier to a `Box` field
that violates this invariant; e.g.:

```rust
struct BoxedErased {
    /// SAFETY: `data`'s logical type has `type_id`.
    unsafe data: Box<[MaybeUninit<u8>]>,
    /// SAFETY: See [`BoxErased::data`].
    unsafe type_id: TypeId,
}

impl BoxedErased {
    fn new<T: 'static>(src: Box<T>) -> Self {
        let data = …; // cast `Box<T>` to `Box<[MaybeUninit<u8>]>`
        let type_id = TypeId::of::<T>;
        // SAFETY: …
        unsafe {
            BoxedErased {
                data,
                type_id,
            }
        }
    }
}
```

...does not ensure that using `BoxedErased` or its `data` field in safe contexts cannot lead to
undefined behavior: namely, if `BoxErased` or its `data` field is dropped, its destructor may induce
UB.

In such situations, you may avert the potential for undefined behavior by wrapping the problematic
field in `ManuallyDrop`; e.g.:

```diff
  struct BoxedErased {
      /// SAFETY: `data`'s logical type has `type_id`.
-     unsafe data: Box<[MaybeUninit<u8>]>,
      /// SAFETY: See [`BoxErased::data`].
+     unsafe data: ManuallyDrop<Box<[MaybeUninit<u8>]>>,
      unsafe type_id: TypeId,
  }
```

## When *Not* To Use Unsafe Fields

### Relaxing a Language Invariant

The `unsafe` modifier is appropriate only for denoting *library* safety invariants. It has no impact
on *language* safety invariants, which must *never* be violated. This, for example, is an unsound
API:

```rust
struct Zeroed<T> {
    // SAFETY: The value of `zeroed` consists only of bytes initialized to `0`.
    unsafe zeroed: T,
}

impl<T> Zeroed<T> {
    pub fn zeroed() -> Self {
        unsafe { Self { zeroed: core::mem::zeroed() }}
    }
}
```

...because `Zeroed::<NonZeroU8>::zeroed()` induces undefined behavior.

### Denoting a Correctness Invariant

A library *correctness* invariant is an invariant imposed by an API whose violation must not result
in undefined behavior. In the below example, unsafe code may rely upon `alignment_pow`s invariant,
but not `size`'s invariant:

```rust
struct Layout {
    /// The size of a type.
    ///
    /// # Invariants
    ///
    /// For well-formed layouts, this value is less than `isize::MAX` and is a multiple of the alignment.
    /// To accomodate incomplete layouts (i.e., those missing trailing padding), this is not a safety invariant.
    pub size: usize,
    /// The log₂(alignment) of a type.
    ///
    /// # Safety
    ///
    /// `alignment_pow` must be between 0 and 29.
    pub unsafe alignment_pow: u8,
}
```

The `unsafe` modifier should only be used on fields with *safety* invariants, not merely correctness
invariants.

We might also imagine a variant of the above example where `alignment_pow`, like `size`, doesn't
carry a safety invariant. Ultimately, whether or not it makes sense for a field to be `unsafe` is a
function of programmer preference and API requirements.

## Complete Example

The below example demonstrates how field safety support can be applied to build a practical
abstraction with small safety boundaries
([playground](https://play.rust-lang.org/?version=nightly&mode=debug&edition=2024&gist=e8aa2af933f5bf4892d1be951062538d)):

```rust
#![deny(
    unfulfilled_lint_expectations,
    clippy::missing_safety_doc,
    clippy::undocumented_unsafe_blocks,
)]

use std::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::Arc,
};

/// An `Arc` that provides exclusive access to its referent.
///
/// A `UniqueArc` may have any number of `KeepAlive` handles which ensure that
/// the inner value is not dropped. These handles only control dropping, and do
/// not provide read or write access to the value.
pub struct UniqueArc<T: 'static> {
    /// # Safety
    ///
    /// While this `UniqueArc` exists, the value pointed by this `arc` may not
    /// be accessed (read or written) other than via this `UniqueArc`.
    unsafe arc: Arc<UnsafeCell<T>>,
}

/// Keeps the parent [`UniqueArc`] alive without providing read or write access
/// to its value.
pub struct KeepAlive<T> {
    /// # Safety
    ///
    /// `T` may not be accessed (read or written) via this `Arc`.
    #[expect(unused)]
    unsafe arc: Arc<UnsafeCell<T>>,
}

impl<T> UniqueArc<T> {
    /// Constructs a new `UniqueArc` from a value.
    pub fn new(val: T) -> Self {
        let arc = Arc::new(UnsafeCell::new(val));
        // SAFETY: Since we have just created `arc` and have neither cloned it
        // nor leaked a reference to it, we can be sure `T` cannot be read or
        // accessed other than via this particular `arc`.
        unsafe { Self { arc } }
    }

    /// Releases ownership of the enclosed value.
    ///
    /// Returns `None` if any `KeepAlive`s were created but not destroyed.
    pub fn into_inner(self) -> Option<T> {
        // SAFETY: Moving `arc` out of `Self` releases it from its safety
        // invariant.
        let arc = unsafe { self.arc };
        Arc::into_inner(arc).map(UnsafeCell::into_inner)
    }

    /// Produces a `KeepAlive` handle, which defers the destruction
    /// of the enclosed value.
    pub fn keep_alive(&self) -> KeepAlive<T> {
        // SAFETY: By invariant on `KeepAlive::arc`, this clone will never be
        // used for accessing `T`, as required by `UniqueArc::arc`. The one
        // exception is that, if a `KeepAlive` is the last reference to be
        // dropped, then it will drop the inner `T`. However, if this happens,
        // it means that the `UniqueArc` has already been dropped, and so its
        // invariant will not be violated.
        unsafe {
            KeepAlive {
                arc: self.arc.clone(),
            }
        }
    }
}

impl<T> Deref for UniqueArc<T> {
    type Target = T;

    fn deref(&self) -> &T {
        // SAFETY: We do not create any other owning references to `arc` - we
        // only dereference it below, but do not clone it.
        let arc = unsafe { &self.arc };
        let ptr = UnsafeCell::get(arc);
        // SAFETY: We satisfy all requirements for pointer-to-reference
        // conversions [1]:
        // - By invariant on `&UnsafeCell<T>`, `ptr` is well-aligned, non-null,
        //   dereferenceable, and points to a valid `T`.
        // - By invariant on `Self::arc`, no other `Arc` references exist to
        //   this value which will be used for reading or writing. Thus, we
        //   satisfy the aliasing invariant of `&` references.
        //
        // [1] https://doc.rust-lang.org/1.85.0/std/ptr/index.html#pointer-to-reference-conversion
        unsafe { &*ptr }
    }
}

impl<T> DerefMut for UniqueArc<T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: We do not create any other owning references to `arc` - we
        // only dereference it below, but do not clone it.
        let arc = unsafe { &mut self.arc };
        let val = UnsafeCell::get(arc);
        // SAFETY: We satisfy all requirements for pointer-to-reference
        // conversions [1]:
        // - By invariant on `&mut UnsafeCell<T>`, `ptr` is well-aligned,
        //   non-null, dereferenceable, and points to a valid `T`.
        // - By invariant on `Self::arc`, no other `Arc` references exist to
        //   this value which will be used for reading or writing. Thus, we
        //   satisfy the aliasing invariant of `&mut` references.
        //
        // [1] https://doc.rust-lang.org/1.85.0/std/ptr/index.html#pointer-to-reference-conversion
        unsafe { &mut *val }
    }
}
```

# Reference-level explanation

## Syntax

The [`StructField` syntax][struct syntax], used for the named fields of structs, enums, and unions,
shall be updated to accommodate an optional `unsafe` keyword just before the field `IDENTIFIER`:

```diff
StructField :
   OuterAttribute*
   Visibility?
+  unsafe?
   IDENTIFIER : Type
```

[struct syntax]: https://doc.rust-lang.org/stable/reference/items/structs.html#structs

The use of unsafe fields on unions shall remain forbidden while the [impact of this feature on
unions](#safe-unions) is decided.

# Rationale and Alternatives

The design of this proposal is primarily guided by three tenets:

1. [**Unsafe Fields Denote Safety Invariants**](#tenet-unsafe-fields-denote-safety-invariants)   
   A field *should* be marked `unsafe` if it carries arbitrary library safety invariants with
   respect to its enclosing type.
2. [**Unsafe Usage is Always Unsafe**](#tenet-unsafe-usage-is-always-unsafe)   
   Uses of `unsafe` fields which could violate their invariants *must* occur in the scope of an
   `unsafe` block.
3. [**Safe Usage is Usually Safe**](#tenet-safe-usage-is-usually-safe)   
   Uses of `unsafe` fields which cannot violate their invariants *should not* require an unsafe
   block.

This RFC prioritizes the first two tenets before the third. We believe that the benefits doing so —
broader utility, more consistent tooling, and a simplified safety hygiene story — outweigh its
cost, [alarm fatigue](#alarm-fatigue). The third tenet implores us to weigh this cost.

## Tenet: Unsafe Fields Denote Safety Invariants

> A field *should* be marked `unsafe` if it carries library safety invariants.

We adopt this tenet because it is consistent with the purpose of the `unsafe` keyword in other
declaration positions, where it signals to consumers of the `unsafe` item that their use is
conditional on upholding safety invariants; for example:

- An `unsafe` trait denotes that it carries safety invariants which must be upheld by implementors.
- An `unsafe` function denotes that it carries safety invariants which must be upheld by callers.

## Tenet: Unsafe Usage is Always Unsafe

> Uses of `unsafe` fields which could violate their invariants *must* occur in the scope of an
> `unsafe` block.

We adopt this tenet because it is consistent with the requirements imposed by the `unsafe` keyword
imposes when applied to other declarations;  for example:

- An `unsafe` trait may only be implemented with an `unsafe impl`.
- An `unsafe` function is only callable in the scope of an `unsafe` block.

## Tenet: Safe Usage is Usually Safe

> Uses of `unsafe` fields which cannot violate their invariants *should not* require an unsafe block.

Good safety hygiene is a social contract and adherence to that contract will depend on the user
experience of practicing it. We adopt this tenet as a forcing function between designs that satisfy
our first two tenets. All else being equal, we give priority to designs that minimize the needless
use of `unsafe`.

## Alternatives

These tenets effectively constrain the design space of tooling for field safety hygiene; the
alternatives we have considered conflict with one or more of these tenets.

### Unsafe Variants

We propose that the `unsafe` keyword be applicable on a per-field basis. Alternatively, we can
imagine it being applied on a per-constructor basis; e.g.:

```rust
// SAFETY: ...
unsafe struct Example {
    foo: X,
    bar: Y,
    baz: Z,
}

enum Example {
    Foo,
    // SAFETY: ...
    unsafe Bar(baz)
}
```

For structs and enum variants with multiple unsafe fields, this alternative has a syntactic
advantage: the `unsafe` keyword need only be typed once per enum variant or struct with safety
invariant.

However, in structs and enum variants with mixed safe and unsafe fields, this alternative denies
programmers a mechanism for distinguishing between conceptually safe and unsafe fields.
Consequently, any safety tooling built upon this mechanism must presume that *all* fields of such
variants are conceptually unsafe, requiring the programmer to use `unsafe` even for the consumption
of 'safe' fields. This violates [*Tenet: Safe Usage is Usually
Safe*](#tenet-safe-usage-is-usually-safe).

### Field Moving is Safe

We propose that all uses of `unsafe` fields require `unsafe`, including reading. Alternatively, we
might consider making reads safe. However, a field may carry an invariant that would be violated by
a read. In the [*Complete Example*](#complete-example), `KeepAlive<T>::arc` is marked `unsafe`
because it carries such an invariant:

```rust
/// Keeps the parent [`UniqueArc`] alive without providing read or write access
/// to its value.
pub struct KeepAlive<T> {
    /// # Safety
    ///
    /// `T` may not be accessed (read or written) via this `Arc`.
    unsafe arc: Arc<UnsafeCell<T>>,
}
```

Allowing `arc` to be safely moved out of `KeepAlive<T>` would create the false impression that it is
safe to use `arc` — it is not. By requiring `unsafe` to read `arc`, Rust's safety tooling ensures a
narrow safety boundary: the user is forced to justify their actions when accessing `arc` (which
documents its safety conditions as they relate to `KeepAlive`), rather than in downstream
interactions with `UnsafeCell<T>` (whose methods necessarily provide only general guidance).
Consequently, we require that moving unsafe fields out of their enclosing type requires `unsafe`. 

### Field Copying is Safe

We propose that all uses of unsafe fields require `unsafe`, including copying. Alternatively, we
might consider making field copies safe. However, a field may carry an invariant that could be
violated as consequence a copy. For example, consider a field of type `&'static RefCell<T>` that
imposes an invariant on the value of `T`. In this alternative proposal, such a field could be safely
copiable out of its enclosing type, then safely mutated via the API of `RefCell`. Consequently, we
require that copying unsafe fields out of their enclosing type requires `unsafe`.

### Copy Is Safe To Implement

We propose that `Copy` is conditionally unsafe to implement; i.e., that the `unsafe` modifier is
required to implement `Copy` for types that have unsafe fields. Alternatively, we can imagine
permitting retaining Rust's present behavior that `Copy` is unconditionally safe to implement for
all types; e.g.:

```rust
struct UnalignedMut<'a, T> {
    /// # Safety
    ///
    /// `ptr` is an exclusive reference to a valid-but-unaligned instance of `T`.
    unsafe ptr: *mut T,
    _lifetime: PhantomData<&'a T>,
}

impl<'a, T> Copy for UnalignedMut<'a, T> {}

impl<'a, T> Clone for UnalignedMut<'a, T> {
    fn clone(&self) -> Self {
        *self
    }
}
```

However, the `ptr` field introduces a declaration-site safety obligation that is not discharged
with `unsafe` at any use site; this violates [**Tenet: Unsafe Usage is Always
Unsafe**](#tenet-unsafe-usage-is-always-unsafe).

### Non-Trivial Destructors are Prohibited

If a programmer applies the `unsafe` modifier to a field with a non-trivial destructor and relaxes
its invariant beyond that which is required by the field's destructor, Rust cannot prevent the
unsound use of that field in safe contexts. This is, seemingly, a soft violation of [**Tenet: Unsafe
Usage is Always Unsafe**](#tenet-unsafe-usage-is-always-unsafe). We resolve this by documenting that
such fields are a serious violation of good safety hygiene, and accept the risk that this
documentation is ignored. This risk is minimized by prevalence: we feel that relaxing a field's
invariant beyond that of its destructor is a rare subset of the cases in which a field carries a
relaxed variant, which itself a rare subset of the cases in which a field carries a safety
invariant.

Alternatively, we previously considered that this risk might be averted by requiring that `unsafe`
fields have trivial destructors, à la union fields, by requiring that `unsafe` field types be either
`Copy` or `ManuallyDrop`.

Unfortunately, we discovered that adopting this approach would contradict our design tenets and
place library authors in an impossible dilemma. To illustrate, let's say a library author presently
provides an API this this shape:

```rust
pub struct SafeAbstraction {
    pub safe_field: NotCopy,
    // SAFETY: [some additive invariant]
    unsafe_field: Box<NotCopy>,
}
```

...and a downstream user presently consumes this API like so:

```rust
let val = SafeAbstraction::default();
let SafeAbstraction { safe_field, .. } = val;
```

Then, `unsafe` fields are stabilized and the library author attempts to refactor their crate to use
them. They mark `unsafe_field` as `unsafe` and — dutifully following the advice of a rustc
diagnostic — wrap the field in `ManuallyDrop`:

```rust
pub struct SafeAbstraction {
    pub safe_field: NotCopy,
    // SAFETY: [some additive invariant]
    unsafe unsafe_field: ManuallyDrop<Box<NotCopy>>,
}
```

But, to avoid a memory leak, they must also now provide a `Drop` impl; e.g.:

```rust
impl Drop for SafeAbstraction {
    fn drop(&mut self) {
        // SAFETY: `unsafe_field` is in a library-valid
        // state for its type.
        unsafe { ManuallyDrop::drop(&mut self.unsafe_field) }
    }
}
```

This is a SemVer-breaking change. If the library author goes though with this, the aforementioned
downstream code will no longer compile. In this scenario, the library author cannot use `unsafe` to
denote that this field carries a safety invariant; this is *both* a hard violation of [**Tenet:
Unsafe Fields Denote Safety Invariants**](#tenet-unsafe-fields-denote-safety-invariants), and (in
requiring trivially `unsafe` drop glue), a violation of [**Tenet: Safe Usage is Usually
Safe**](#tenet-safe-usage-is-usually-safe).

### Unsafe Wrapper Type

This RFC proposes extending the Rust language with first-class support for field (un)safety.
Alternatively, we could attempt to achieve the same effects by leveraging Rust's existing visibility
and safety affordances. At first blush, this seems plausible; it's trivial to define a wrapper that
only provides unsafe initialization and access to its value:

```rust
#[repr(transparent)]
pub struct Unsafe<T: ?Sized>(T);

impl<T: ?Sized> Unsafe<T> {

    pub fn new(val: T) -> Self
    where
        T: Sized
    {
        Self(val)
    }

    pub unsafe fn as_ref(&self) -> &T {
        &self.0
    }

    pub unsafe fn as_mut(&mut self) -> &mut T {
        &mut self.0
    }

    pub unsafe fn into_inner(self) -> T
    where
        T: Sized
    {
        self.0
    }
}
```

However, this falls short of the assurances provided by first-class support for field safety.
The safety conditions of its accessors inherit the safety conditions of the field that the `Unsafe`
was read or referenced from. Consequently, what safety proofs one must write when using such a
wrapper depend on the dataflow of the program.

And worse, certain dangerous flows do not require `unsafe` at all. For instance, unsafe fields of
the same type can be laundered between fields with different invariants; safe code could exchange
`Even` and `Odd`s' `val`s:

```rust
struct Even {
    val: Unsafe<usize>,
}

struct Odd {
    val: Unsafe<usize>,
}
```

We can plug this particular hole by adding a type parameter to `Unsafe` that encodes the type of the
outer datatype, `O`; e.g.:

```rust
#[repr(transparent)]
pub struct Unsafe<O: ?Sized, T: ?Sized>(PhantomData<O>, T);
```

However, it remains possible to exchange unsafe fields within the same type; for example, safe code
can freely exchange the values of `len` and `cap` of this hypothetical vector:

```rust
struct Vec<T> {
    alloc: Unsafe<Self, *mut T>,
    len: Unsafe<Self, usize>,
    cap: Unsafe<Self, usize>,
}
```

The [`unsafe-fields`](https://crates.io/crates/unsafe-fields) crate plugs this hole by extending
`Unsafe` with a const generic that holds a hash of the field name. Even so, it remains possible for
safe code to exchange the same unsafe field between different instances of the same type (e.g.,
exchanging the `len`s of two instances of the aforementioned `Vec`).

These challenges motivate first-class support for field safety tooling.

### More Syntactic Granularity

This RFC proposes the rule that *a field marked `unsafe` is unsafe to use*. This rule is flexible
enough to handle arbitrary field invariants, but — in some scenarios — requires that the user write
trivial safety comments. For example, in some scenarios, an unsafe is trivially sound to read:

```rust
struct Even {
    /// # Safety
    ///
    /// `val` is an even number.
    val: u8,
}

impl Into<u8> for Even {
    fn into(self) -> u8 {
        // SAFETY: Reading this `val` cannot
        // violate its invariant.
        unsafe { self.val }
    }
}
```

In other scenarios, an unsafe field is trivially sound to `&`-reference (but not `&mut`-reference).

Since it is impossible for the compiler to precisely determine the safety requirements of an unsafe
field from a type-directed analysis, we must *either* choose a usage rule that fits all scenarios
(i.e., the approach adopted by this RFC) *or* provide the user with a mechanism to signal their
requirements to the compiler. Here, we explore this alternative.

The design space of syntactic knobs is vast. For instance, we could require that the user enumerate
the operations that require `unsafe`; e.g.:

- `unsafe(init,&mut,&,read)` (everything is unsafe)
- `unsafe(init,&mut,&)` (everything except reading unsafe)
- `unsafe(init,&mut)` (everything except reading and `&`-referencing unsafe)
- etc.

Besides the unclear semantics of an unparameterized `unsafe()`, this design has the disadvantage
that the most permissive (and thus dangerous) semantics are the cheapest to type. To mitigate this,
we might instead imagine reversing the polarity of the modifier:

- `safe(read)` all operations except reading are safe
- `safe(read,&)` all operations except reading and `&`-referencing are safe
- etc.

...but using `safe` to denote the presence of a safety invariant is probably too surprising in the
context of Rust's existing safety tooling.

Alternatively, if we are confident that a hierarchy of operations exists, the brevity of the API can
be improved by having the presence of one modifier imply others (e.g., `unsafe(&mut)` could denote
that initialization, mutation and `&mut`-referencing) are unsafe. However, this requires that the
user internalize this hierarchy, or else risk selecting the wrong modifier for their invariant.

Although we cannot explore the entire design space of syntactic modifiers here, we broadly feel that
their additional complexity exceeds that of our proposed design. Our proposed rule that *a field
marked `unsafe` is unsafe to use* is both pedagogically simple and fail safe; i.e., so long as a
field is marked `unsafe`, it cannot be misused in such a way that its invariant is violated in safe
code.

### Mixing Syntactic Knobs with a Wrapper Type

One alternative proposed in this RFC's discussion recommends a combination of syntactic knobs and a
wrapper type. In brief, a simple [`Unsafe` wrapper type](#unsafe-wrapper-type) would be provided,
along with two field safety modifiers:

- `unsafe`
  All uses except reading are `unsafe`.
- `unsafe(mut)`
  All uses except reading and `&`-referencing are `unsafe`.

Under this proposal, a programmer would use some combination of `unsafe`, `unsafe(mut)` and `Unsafe`
to precisely tune Rust's safety tooling protections, depending on the hazards of their invariant.

The primary advantage of this approach is that it results in comparatively fewer instances in which
[the programmer must write a 'trivial' safety proof](#trivial-safety-proofs). However, it achieves
this by front-loading the requirement that the programmer imagine all possible safety hazards of
their field. A mistake, here, may lead to a false sense of security if Rust fails to require
`unsafe` for uses that are, in fact, dangerous. By contrast, this RFC requires that programmers
resolve these questions only on an as-needed basis; e.g., until you need to `&`-reference a field,
you do not need to confront whether doing so is *always* a safe operation.

This alternative also inherits some of the disadvantages of [`Unsafe` wrapper
types](#unsafe-wrapper-type); namely that the safety proofs needed to operate on an `Unsafe` wrapper
value depend on the dataflow of the program; the wrapper value must be traced to its originating
field so that field's safety documentation may be examined.

Comparatively, we believe that this RFC's proposal is both pedagogically simpler and less prone to
misuse, and that these benefits outweigh its [drawbacks](#drawbacks).

# Drawbacks

## Trivial Safety Proofs

The primary drawback of this proposal is that it — in some scenarios — necessitates writing
'trivial' safety proofs. For example, merely reading `Vec`'s `len` field obviously cannot invalidate
its invariant; nonetheless, this field, if marked `unsafe`, would be `unsafe` to read. An `unsafe`
block and attendant `SAFETY` comment is required. In most cases, this is a one-time chore: the
maintainer can define a *safe* accessor (i.e.,
[`Vec::len`](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.len)) that encapsulates this
proof. However, in cases where multiple, partial field borrows are required, such an accessor cannot
be invoked. [Future language extensions that permit partial borrows may resolve this
drawback.](#partial-borrows).

At the extreme, a programmer frustrated with field safety tooling might opt to continue with the
status quo approach for maintaining field invariants. Such rebuttals of safety tooling are not
unprecedented in the Rust ecosystem. Even among prominent projects, it is not rare to find a
conceptually unsafe function or impl that is not marked unsafe. The discovery of such functions by
the broader Rust community has, occasionally, provoked controversy.

This RFC takes care not to fuel such flames; e.g., [**Tenet: Unsafe Fields Denote Safety
Invariants**](#tenet-unsafe-fields-denote-safety-invariants) admonishes that programmers *should* —
but **not** *must* — denote field safety invariants with the `unsafe` keyword. It is neither a
soundness nor security issue to continue to adhere to the current convention of using visibility to
enforce field safety invariants.

# Prior art

Some items in the Rust standard library have `#[rustc_layout_scalar_valid_range_start]`,
`#[rustc_layout_scalar_valid_range_end]`, or both. These items have identical behavior to that of
unsafe fields described here. It is likely (though not required by this RFC) that these items will
be required to use unsafe fields, which would reduce special-casing of the standard library.

# Unresolved questions

- If the syntax for restrictions does not change, what is the ordering of keywords on a field that
  is both unsafe and mut-restricted?

## Terminology

This RFC defines three terms of art: *safety invariant*, *library safety invariant*, and *language
safety invariant*. The meanings of these terms are not original to this RFC, and the question of
which terms should be assigned to these meanings [is being hotly
debated](https://github.com/rust-lang/unsafe-code-guidelines/issues/539). This RFC does not
prescribe its terminology. Documentation of the unsafe fields tooling should reflect broader
consensus, once that consensus is reached.

# Future possibilities

## Partial Borrows

The primary drawback of this proposal is that it — in some scenarios — necessitates writing
'trivial' safety proofs. For example, merely reading `Vec`'s `len` field obviously cannot invalidate
its invariant; nonetheless, this field, if marked `unsafe`, would be `unsafe` to read. An `unsafe`
block and attendant `SAFETY` comment is required. In most cases, this is a one-time chore: the
maintainer can define a *safe* accessor (i.e.,
[`Vec::len`](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.len)) that encapsulates this
proof. However, in cases where multiple, partial field borrows are required, such an accessor cannot
be invoked. Future language extensions that permit partial borrows will resolve this drawback.

## Syntactic Knobs and Wrapper Types

While we are confident that this RFC has the best tradeoffs among the alternatives in the design
space, it is not a one-way door. Changes to the default semantics of `unsafe` could be realized over
an edition boundary. This RFC is also forwards-compatible with some future additions of some
[combinations](#mixing-syntactic-knobs-with-a-wrapper-type) of [syntactic
knobs](#more-syntactic-granularity) and [wrapper types](#unsafe-wrapper-type). For example, in
addition to this RFC's `unsafe` modifier, additional variants in the form `unsafe(<modifiers>)`
(e.g., `unsafe(mut)`) could be added to denote that some subset of uses is always safe.

## Safe Unions

Today, unions provide language support for fields with subtractive *language* invariants. Unions may
be safely defined, constructed and mutated — but require unsafe to read. Consequently, it is
possible to place an union into a state where its fields cannot be soundly read, using only safe
code; e.g.
([playground](https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=1d816399559950ccae810c4a41fab4e9)):

```rust
#[derive(Copy, Clone)] #[repr(u8)] enum Zero { V = 0 }
#[derive(Copy, Clone)] #[repr(u8)] enum One  { V = 1 }

union Tricky {
    a: (Zero, One),
    b: (One, Zero),
}

let mut tricky = Tricky { a: (Zero::V, One::V) };
tricky.b.0 = One::V;

// Now, neither `tricky.a` nor `tricky.b` are in a valid state.
```

The possibility of such unions makes it tricky to retrofit a mechanism for safe access: Because
unsafe was not required to define or mutate this union, the invariant that makes reading sound is
entirely implicit.

Speculatively, it might be possible to make the subtractive language invariant of union fields
*explicit*; e.g.:

```rust
union MaybeUninit<T> {
    uninit: (),
    unsafe(invalid) value: ManuallyDrop<T>,
}
```

Migrating today's implicitly-unsafe unions to tomorrow's explicitly-unsafe unions over an edition
boundary would free up the syntactic space for safe unions.
