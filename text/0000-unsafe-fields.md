- Feature Name: `unsafe_fields`
- Start Date: 2023-07-13
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

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

Rust will enforce that potentially-invalidating uses of `unsafe` fields only occur in the context
of an `unsafe` block, and Clippy's [`missing_safety_doc`] lint will check that `unsafe` fields have
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

Fields may be declared `unsafe`. Unsafe fields may only be initialized or accessed mutably in an
unsafe context. Reading the value of an unsafe field may occur in either safe or unsafe contexts. An
unsafe field may be relied upon as a safety invariant in other unsafe code.

Here is an example to illustrate usage:

```rust
struct Foo {
    safe_field: u32,
    /// Safety: Value must be an odd number.
    unsafe unsafe_field: u32,
}

// Unsafe field initialization requires an `unsafe` block.
// Safety: `unsafe_field` is odd.
let mut foo = unsafe {
    Foo {
        safe_field: 0,
        unsafe_field: 1,
    }
};

// Safe field: no unsafe block.
foo.safe_field = 1;

// Unsafe field with mutation: unsafe block is required.
// Safety: The value is odd.
unsafe { foo.unsafe_field = 3; }

// Unsafe field without mutation: no unsafe block.
println!("{}", foo.unsafe_field);
```

For a full description of where a mutable access is considered to have occurred (and why), see
[RFC 3323]. Keep in mind that due to reborrowing, a mutable access of an unsafe field is not
necessarily explicit.

[RFC 3323]: https://rust-lang.github.io/rfcs/3323-restrictions.html#where-does-a-mutation-occur

```rust
fn change_unsafe_field(foo: &mut Foo) {
    // Safety: An odd number plus two remains an odd number.
    unsafe { foo.unsafe_field += 2; }
}
```

# Reference-level explanation

## Syntax

Using the syntax from [the reference for structs][struct syntax], the change needed to support
unsafe fields is minimal.

[struct syntax]: https://doc.rust-lang.org/stable/reference/items/structs.html#structs

```diff
StructField :
   OuterAttribute*
   Visibility?
+  unsafe?
   IDENTIFIER : Type

TupleField :
   OuterAttribute*
   Visibility?
+  unsafe?
   Type
```

## Behavior

An unsafe field may only be mutated or initialized in an unsafe context. Failure to do so is a compile error.

## "Mutable use" in the compiler

The concept of a "mutable use" [already exists][mutating use] within the compiler. This catches all
situations that are relevant here, including `ptr::addr_of_mut!`, `&mut`, and direct assignment to a
field, while excluding interior mutability. As such, formal semantics of what constitutes a "mutable
use" are not stated here.

[mutating use]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/mir/visit/enum.PlaceContext.html#method.is_mutating_use

# Drawbacks

- Additional syntax for macros to handle
- More syntax to learn

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

??
