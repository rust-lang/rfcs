- Start Date: 2015-01-22
- RFC PR:
- Rust Issue:

# Summary

Further subdivide unsized types into dynamically-sized types, implementing
an intrinsic trait `DynamicSize`, and types of indeterminate size. References
for the latter kind will be thin, while allocating slots or copying values
of such types is not possible outside unsafe code.

# Motivation

There are cases where borrowed references provide a view to a larger object
in memory than the nominal sized type being referenced. Of interest are:

- Token types for references to data structures whose size is not available as
  a stored value, but determined by parsing the content, such as
  null-terminated C strings. An example is `CStr`, a
  [proposed](https://github.com/rust-lang/rfcs/pull/592) 'thin' dereference
  type for `CString`.
- References to structs as public views into larger contiguously allocated
  objects, the size and layout of which is hidden from the client.

When only immutable references are available on such types, it's possible
to prevent copying out the nominal sized value in safe code, but there is
still potential for unintentional misuse due to values under reference having
a bogus size.
If mutable references are available, there is added trouble with
`std::mem::swap` and similar operations.

# Detailed design

The types that do not implement `Sized` are further subdivided into DSTs
and types of indeterminate size.

## Truly unsized types

Any type with sized contents can be opted out of `Sized` by including
a marker `NotSized`, which also implies `NoCopy`:
```rust
struct CStr {
    head: libc::c_char,  // Contains at least one character...
    rest: std::marker::NotSized  // ...but there can be more
}
```

This makes references to values of such types unusable for copying
the value out, `size_of`, `std::mem::swap`, being the source in
`transmute_copy`, etc.

## Dynamically sized types

Plain old (ahem) dynamically sized types will intrinsically implement
a newly introduced trait, `DynamicSize`. Only references to `DynamicSize`
types will be fat.

# Fallout

There may be cases where `!Sized` is taken to mean DSTs. These will have to
switch to using the `DynamicSize` bound.

Specifically, there are generic items where the `Sized` bound is not
lifted only to ensure that a reference is thin so it can be coerced or
transmuted to a raw pointer. These will be unable to use truly unsized types,
and should relax the type bound to `?Sized` and a negative bound (#586)
on `DynamicSize`.

# Drawbacks

Adding further complexity to the type system.

# Alternatives

Keep to the current practice of Irrelevantly Sized Types, learning to avoid
trouble by coding discipline, documentation, and best practices. The problem
of mutable references can be resolved on a case-by-case basis by providing
an accessor facade for fields that can be safely mutated, and disallowing
direct mutable references to the pseudo-sized type. This feels at odds
with the overall philosophy of Rust, though.

# Unresolved questions

For convenience, there could be an intrinsically implemented positive trait
to complement `DynamicSize`.
