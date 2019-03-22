- Feature Name: `pointer-match`
- Start Date: 2019-03-21
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Extend match syntax and patterns by support for a limited set of operations for
pointers, which involve only address calculation and not actually reading
through the pointer value. Make it possible to use these matches to calculate
addresses of fields even for `repr(packed)` structs and possibly unaligned
pointers where an intermediate reference must not be created.

# Motivation
[motivation]: #motivation

To create a pointer to a field of a struct, there is currently no way in Rust
that avoids creating a temporary reference. Since reference semantics are
stricter, this may lead to silent undefined behaviour where that reference
should not be valid. Depending on the resolution of reference semantics this
affects:

* Creating a pointer to a field of a packed struct, where the reference may be
  unaligned (depending on .
* Pointing to fields of an uninitialized type, where the reference points to
  uninitialized data. This may be complicated by unions, where it could be
  possible that not a single variant is currently completely initialized, yet
  one wants to access some subfield. See
  <https://github.com/rust-lang/unsafe-code-guidelines/issues/73#issuecomment-460634637>.
* Doing pointer offset calculations where the references does not refer to the
  same, or any, allocation. This is because reference calculations are
  performed with `getelementptr inbounds`.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Match expression are extended from support for a reference binding mode, to a
pointer binding mode. Furthermore, a new pattern binds to a pointer, and
identifiers are extended to allow a new mode similar to `ref` and `ref mut`
binding to a reference. These patterns are called pointer pattern and raw
identifier for the remainder of the document.

```
#[repr(packed)]
struct Foo {
    a: u16,
    b: u32,
}

fn ptr_b(foo: &mut Foo) -> *mut u32 {
    let Foo { raw mut b, .. } = foo;
    b
}
```

Note that pointer binding mode and pointer pattern requires `unsafe`, even when
it will never dereference the pointer. But the arithmetic on the pointer may
implicitely overflow. Furthermore, not all patterns are (yet) allowed, to avoid
implicitely performing an unintended, unsafe read through the pointer. Pointer
binding mode will at first only permit ultimately binding with `raw` and `ref`
and not actually reading the contained memory.

The raw identifier pattern does not require `unsafe` on its own.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The calculation of the value from a pointer pattern will not use an `inbounds`
qualifier when passed to llvm.

There is no restriction on matching enum variants and slices, such that this is
possible:

```
#[repr(packed)]
Foo {
    field: Enum,
}

enum Enum {
    A(usize),
}

fn overwrite_packed_field(foo: &mut Foo ) {
    // Actually safe!
    let Foo { field: Enum::A(raw mut x), } = foo;

    // Write itself not safe, as we write to a pointer.
    unsafe { ptr::write_unaligned(x, 0) };
}
```

The new pattern forms a new kind of field binding, and should be inserted into
the grammar as an option for identifier and StructPatternField, next to 
`id: pattern`, `tuple_index: pattern`, `ref? mut? identifier`. Pointer pattern
uses the obvious `*pattern` and is only allowed in unsafe blocks.

Allowed [patterns](https://doc.rust-lang.org/reference/patterns.html) within
pointer patterns (and thus in the sugar of pointer binding mode) are: wildcard
patttern, path patterns that don't refer to enum variants or constants, struct
patterns, tuple patterns, fixed size array patterns, where the last three are
only not allowed to bind their fields with the new pointer pattern and with
`..`, potentially also with `ref mut? identifier`, but not `mut?  identifier`.
Some further notes on (dis-)allowed patterns:

* The restrictions don't apply to matching the pointer value itself, as that
  is not inside a pointer pattern.
* enum variants and constants obviously read their memory.
* literal, identifier, and reference patterns also constitute a read of the
  pointed-to place, and implicitely assert their type's invariants. Better to
  keep those operations separate.
* no pointer patterns within pointer patterns, must also actually read memory.
* slice patterns would require size information not present in the pointer.
* `ref mut? identifier` may be useful, but may be too tempting sometimes.

# Drawbacks
[drawbacks]: #drawbacks

Match syntax is 'more heavy' than a place based syntax in some or many cases.
On the other side of the coin, initializing a struct often involves grabbing
pointers to all fields, where matching is much terser than each indivdual
expression.

The additional pointer binding mode for match expressions may be confusing due
to the non-explicit pointer nature of its argument.

The pointer retrieved from `raw mut` binding while matching a `&mut _` value
upholds more guarantees than aparent, as it is known to be writable with 
`ptr::write_unaligned`. Some yet-to-be-proposed encapsulation could thus make
this completely safe to the programmer. This is a drawback because of the next
argument.

Assigning semantics to the pattern matching of `*` and `raw` has the risk of
being too restricted for future operations but too constrained to allow
backwards compatible extension. Specifically, the type of `id` in a `raw id`
pattern may be hard to change but a pointer upholds almost no invariants on its
own.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

`&raw <place>` was also proposed to achieve getting a pointer to a field. The
pattern/match syntax has several advantages over place syntax:

* Place expressions are overloaded with auto-deref, custom indexing
  (`core::ops::Index`/`core::ops::IndexMut`), invoking arbitrary user code. A
  solution with place syntax needs to explicitely forbid these forms of place
  statements, both to disallow user code and avoid accidental reference
  intermediates. The new statements thus resembles a very different other
  statement.
* The initial dereferencing of the pointer necessary for a place expression
  (`struct.field` is implicitely `(*struct).field` for a reference argument
  `struct`) will not work with pointer arguments, which do no automatically
  dereference even in unsafe code (and arguably should not, outside `&raw`).
* `raw` feels more natural when paralleling `ref` instead of appearing as yet
  an *additional* qualifier on `&` that is not associated with pointers
  in the first place and confusingly also requires `const` in spite of `&`
  suggesting the opposite.
* It provides a clear pattern that extends to enum fields in packed structs,
  which are not absolutely not expressible in place syntax.

In contrast, patterns fully follow the structural nature of algebraic data
types without customization points in the form of `core::ops`. This makes them
a perfect match when the possibilities should be restricted to exactly those
options.

Not doing this would keep surface level code for creating pointers error prone
or impossible, independent of the underlying MIR changes.

# Prior art
[prior-art]: #prior-art

C++ state-of-the-art, to my best knowledge, also uses the usual lvalue
expression for a pointer to a field. This has several pitfalls: Classes may
overwrite the pointer dereference operator `->`, and the pointer creation
operator `&`. Actually conformant generic code thus requires additional
artificial constructs and a syntax that does not resemble lvalue syntax.
Additionally, most of the operator are not defined while their target object is
not life, making them unfit for initialization of uninitialized objects.

C (and C++ to an extent) also have `offsetof`, a macro based solution to get
the byte offset of a field. This only works reliably for [a very restricted set
of types](https://en.cppreference.com/w/cpp/named_req/StandardLayoutType). This
essentially is the analogue of `#[repr(C)]` in Rust. A `static_assert` based
solution can help unwittingly triggering undefined behaviour on other types.

No other algebraic language with the memory model of Rust is known to the
author, thus comparisons in this way are sparse.

The PR [#2582](https://github.com/rust-lang/rfcs/pull/2582) contains the
necessary MIR operations to perform the address calculations themselves.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

The exact syntax for pointer patterns, while `raw` as a contextual keyword has
already some association with pointer to place it need not be the final answer.

The restrictions on pointer binding mode that are only based on not implicitely
reading memory (enum variants, constants, references, bindings) do not add real
safety, as the matching must occur within an `unsafe` block in any case.
However, they likely do protect against accidental usage similar to auto-deref
in a place expression. They may arguable be more a nuisance than a safety help
nonetheless.

Address calculation will likely depend on not overflow the pointer, i.e. behave
like `pointer::add` but could also utilize `pointer::wrapping_add` instead.
That would make the code safer but provide fewer optimization opportunities.
Also, wrapping addition could promote use to get (specific) field offsets,
within the limits of layout guarantees offered by rust. Since it occurs in an
unsafe block, the burden of fulfilling necessary preconditions ultimately
relies on the programmer.

`ref mut? identifier` within pointer patterns may be disallowed or not.  `raw
identifier` pattern. For half-initialized structs where validity and alignment
of the underlying struct has been checked but `&mut` referencing the complete
struct is not safe due to uninitialized fields this is also useful.
Alternatively, this could be disallowed if not useful enough or it seems to
promote undefined behaviour.

# Future possibilities
[future-possibilities]: #future-possibilities

Some pointer binding matches may be safer than the required `unsafe` suggests:
For example the pointer retrieved from `MaybeUninit` guarantees that the memory
is actually backed by some allocation and thus the offset calculations can both
utilize `inbounds` and will never overflow. It could be possible to remove the
need for an `unsafe` block around such matches if they don't use any of the
memory-reading-patterns discussed in unresolved questions. 
