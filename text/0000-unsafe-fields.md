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

Emphasis on safety is a key strength of Rust. A major point here is that any code path that can
result in undefined behavior must be explicitly marked with the `unsafe` keyword. However, the
current system is insufficient. While Rust provides the `unsafe` keyword at the function level,
there is currently no mechanism to mark fields as `unsafe`.

For a real-world example, consider the `Vec` type in the standard library. It has a `len` field that
is used to store the number of elements present. Setting this field is exposed publicly in the
`Vec::set_len` method, which has safety requirements:

- `new_len` must be less than or equal to `capacity()`.
- The elements at `old_len..new_len` must be initialized.

This field is safe to read, but unsafe to mutate or initialize due to the invariants. These
invariants cannot be expressed in the type system, so they must be enforced manually. Failure to do
so may result in undefined behavior elsewhere in `Vec`.

By introducing unsafe fields, Rust can improve the situation where a field that is otherwise safe is
used as a safety invariant.

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
