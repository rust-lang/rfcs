- Feature Name: `unsafe_fields`
- Start Date: 2023-07-13
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary

This RFC proposes extending Rust's tooling support for safety hygiene to named fields that carry
additive library safety invariants. Consequently, Rust programmers will be able to use the `unsafe`
keyword to denote when a named field carries an additive library safety invariant; e.g.:

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

(Note, the `unsafe` field modifier is only applicable to named fields. You should avoid attaching
library safety invariants to unnamed fields.)

Rust provides tooling to help you maintain good field safety hygiene. Clippy's
[`missing_safety_doc`] lint checks that `unsafe` fields have accompanying safety documentation. The
Rust compiler itself enforces that ues of `unsafe` fields that could violate its invariant — i.e.,
initializations, writes, references, and reads — must occur within the context of an `unsafe`
block.; e.g.:

```rust
impl<'a, T> UnalignedRef<'a, T> {
    pub fn from_ref(ptr: &'a T) -> Self {
        // SAFETY: By invariant on `&T`, `ptr` is a valid and well-aligned instance of `T`.
        unsafe {
            Self { ptr, _lifetime: PhantomData, }
        }
    }
}
```

...and Clippy's [`undocumented_unsafe_blocks`] lint enforces that the `unsafe` block has a `//
SAFETY` comment.

[`undocumented_unsafe_blocks`]: https://rust-lang.github.io/rust-clippy/stable/index.html#undocumented_unsafe_blocks

Fields marked `unsafe` may be safely moved and copied; e.g.:

```rust
struct Alignment {
    /// SAFETY: `pow` must be between 0 and 29.
    pub unsafe pow: u8,
}

impl Alignment {
    pub fn as_log(self) -> u8 {
        self.pow
    }
}
```

...but all other uses, including initialization and referencing, require `unsafe`; e.g., this:

```rust
struct CacheArcCount<T> {
    /// SAFETY: This `Arc`'s `ref_count` must equal the
    /// value of the `ref_count` field.
    unsafe arc: Arc<T>,
    /// SAFETY: See [`CacheArcCount::arc`].
    unsafe ref_count: usize,
}

impl<T> CacheArcCount<T> {
    fn new(value: T) -> Self {
        Self {
            arc: Arc::new(value),
            ref_count: 1,
        }
    }
}

impl<T> core::ops::Deref for CacheArcCount<T> {
    type Target = Arc<T>;

    fn deref(&self) -> &Self::Target {
        &self.arc
    }
}
```

...produces the errors:

```
error[E0133]: initializing type with an unsafe field is unsafe and requires unsafe block
  --> src/lib.rs:11:9
   |
11 | /         Self {
12 | |             arc: Arc::new(value),
13 | |             ref_count: 1,
14 | |         }
   | |_________^ initialization of struct with unsafe field
   |
   = note: unsafe fields may carry library invariants

error[E0133]: use of unsafe field is unsafe and requires unsafe block
  --> src/lib.rs:22:10
   |
22 |         &self.arc
   |          ^^^^^^^^ use of unsafe field
   |
   = note: unsafe fields may carry library invariants
```

## When To Use Unsafe Fields

You should use the `unsafe` keyword on any field declaration that carries an invariant
that is assumed to be true by `unsafe` code.

### Example: Field with Local Invariant

In the simplest case, a field's safety invariant is a restriction of the invariants imposed by the
field type, and concern only the immediate value of the field; e.g.:

```rust
struct Alignment {
    /// SAFETY: `pow` must be between 0 and 29.
    pub unsafe pow: u8,
}
```

### Example: Field with Remote Invariant

A field might carry an invariant with respect to data beyond its immediate bytes; e.g. beyond a reference:

```rust
struct CacheArcCount<T> {
    /// SAFETY: This `Arc`'s `ref_count` must equal the value of the `ref_count` field.
    unsafe arc: Arc<T>,
    /// SAFETY: See [`CacheArcCount::arc`].
    unsafe ref_count: usize,
}
```

...or even beyond the Rust abstract machine; e.g.:

```rust
struct Zeroator {
    /// SAFETY: The fd points to a uniquely-owned file, and the bytes from the start of the file to
    /// the offset `cursor` (exclusive) are zero.
    unsafe fd: OwnedFd,
    /// SAFETY: See [`Zeroator::fd`].
    unsafe cursor: usize,
}
```

## When *Not* To Use Unsafe Fields

### Example: Field with Suspended Invariant

A field might be unsafe because it *relaxes* the library safety invariants imposed by its type. For
example, a `str` is bound by both the language safety invariant that it is initialized bytes, and by
the library safety invariant that it contains valid UTF-8. It is sound to temporarily violate the
library invariant of `str`, so long as the invalid `str` is not safely exposed to code that assumes
`str` validity.

Below, `MaybeInvalidStr` encapsulates an initialized-but-potentially-invalid `str` as an unsafe
field:

```rust
struct MaybeInvalidStr<'a> {
    /// SAFETY: `maybe_invalid` may not contain valid UTF-8. Nonetheless, it MUST always contain
    /// initialized bytes (per language safety invariant on `str`).
    unsafe maybe_invalid: &'a str
}
```

However, the `unsafe` modifier, above, is insufficient to prevent invalid use in safe contexts; e.g.
Rust accepts this, without error:

```rust
impl<'a> core::ops::Deref for MaybeInvalidStr<'a> {
    type Target = str;

    fn deref(&self) -> &str {
        self.maybe_invalid
    }
}
```

The `unsafe` keyword, alone, cannot be relied upon to ensure that invalid use of such fields cannot
occur in safe contexts.

### Example: Field with Correctness Invariant

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

We might also imagine a variant of the above example where `alignment_pow`, like `size` doesn't
carry a safety invariant. Ultimately, whether or not it makes sense for a field to be `unsafe` is a
function of programmer preference and API requirements.

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

### Suspended Invariants Are Supported

To provide sound safety tooling for fields with subtractive invariants, Rust must enforce that such
fields have trivial destructors, à la union fields. We initially considered this requirement to be
merely inconvenient — if we followed the same enforcement regime as unions, `unsafe` fields would
either need to be `Copy` or wrapped in `ManuallyDrop`.

In fact, we discovered, adopting this approach would contradict our design tenets and place library
authors in an impossible dilemma. To illustrate, let's say a library author presently provides an
API this this shape:

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
downstream code will no longer compile.

The library author cannot use `unsafe` to denote that this field carries a safety invariant;
consequently, a one-size-fits-all design violates our first design tenet: *a field must be marked
`unsafe` if it carries a safety invariant*.

# Drawbacks

## Alarm Fatigue

Although the `unsafe` keyword gives Rust's safety hygiene tooling insight into whether a field
carries safety invariants, it does not give Rust deeper insight into the semantics of those
invariants. Consequently, Rust must err on the side caution, requiring `unsafe` for most uses of
unsafe field — including uses that the programmer can see are conceptually harmless.

In these cases, Rust's safety hygiene tooling will suggest that the harmless operation is wrapped
in an `unsafe` block, and the programmer will either:

- comply and provide a trivial safety proof, or
- opt out of Rust's field safety tooling by removing the `unsafe` modifier from their field.

The former is a private annoyance; the latter is a rebuttal of Rust's safety hygiene conventions
and tooling. Such rebuttals are not unprecedented in the Rust ecosystem. Even among prominent
projects, it is not rare to find a conceptually unsafe function that is not marked unsafe. The
discovery of such functions by the broader Rust community has, occasionally, provoked controversy.

This RFC takes care not to fuel such flames; e.g., [**Tenet: Unsafe Fields Denote Safety
Invariants**](#tenet-unsafe-fields-denote-safety-invariants) admonishes that programmers *should* —
but **not** *must* — denote field safety invariants with the `unsafe` keyword. It is neither a
soundness nor security issue to continue to adhere to the convention of using visibility to enforce
field safety invariants.

This RFC does not, itself, attempt to address alarm fatigue. Instead, we propose a simple extension
to Rust's safety tooling that is, by virtue of its simplicity, particularly amenable to future
iterative refinement. We imagine empowering Rust to reason about safety invariants, either via
[type-directed analyses](#safe-reads-for-fields-with-local-non-suspended-invariants), or via
syntactic extensions. The design of these refinements will be guided by the valuable usage data
produced by implementing this RFC.

# Prior art

Some items in the Rust standard library have `#[rustc_layout_scalar_valid_range_start]`,
`#[rustc_layout_scalar_valid_range_end]`, or both. These items have identical behavior to that of
unsafe fields described here. It is likely (though not required by this RFC) that these items will
be required to use unsafe fields, which would reduce special-casing of the standard library.

# Unresolved questions

- If the syntax for restrictions does not change, what is the ordering of keywords on a field that
  is both unsafe and mut-restricted?
- Are there any interactions or edge cases with other language features that need to be considered?

# Future possibilities

## Tooling for Fields with Suspended Safety Invariants

We will additionally explore supporting fields with suspended safety invariants. Speculatively, this
might take the form of an additional modifier; e.g.:

```rust
struct MaybeInvalidStr<'a> {
    /// SAFETY: `maybe_invalid` may not contain valid UTF-8. Nonetheless, it MUST always contain
    /// initialized bytes (per language safety invariant on `str`).
    unsafe(relaxed) maybe_invalid: &'a str
}
```

...which would convey that Rust must also treat moves and copies of such fields as unsafe.

Alternatively, this use-case might be addressed by introducing a designated `MaybeInvalid<T>`
wrapper in the standard library, which encapsulates `T` and provides only `unsafe` accessor
functions.

## Safe Unions

Unsafe struct and enum fields behave very similarly to union fields — unsafe fields differ only in
that they additionally make initialization and mutation unsafe. Given this closeness, it may be
viable to migrate — across an edition boundary — today's implicitly unsafe unions into *explicitly*
unsafe unions that leverage the unsafe field syntax.

For example, the 2027 edition could require that all unions leverage the `unsafe` keyword to define
their fields. The 2024-to-2027 migration script would wrap existing initializations and mutations
in `unsafe` blocks annotated with the comment `// SAFETY: No obligations`. In doing so, we would
create syntactic space for *safe* unions in 2030.
