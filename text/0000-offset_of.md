- Feature Name: `offset_of`
- Start Date: 2022-08-29
- RFC PR: [rust-lang/rfcs#3308](https://github.com/rust-lang/rfcs/pull/3308)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Introduce a new macro `core::mem::offset_of!`, which evaluates to a constant
containing the offset in bytes of a field inside some type.

Specifically, this RFC allows usage like the following:

```rs
use core::mem::offset_of;

const EXAMPLES: &[usize] = &[
    offset_of!(Struct, b),
    offset_of!(TupleStruct, 0),
    offset_of!(Union, y),
    offset_of!((i32, u32), 1),
    offset_of!(inner::SubmodGeneric<i32>, pub_field),
];

struct Struct { a: u64, b: &'static str }
struct TupleStruct(u8, i32);
union Union { x: u8, y: u64 }

mod inner {
    pub struct SubmodAndGeneric<T> {
        private_field: T,
        pub pub_field: u8,
    }
}
```

# Motivation
[motivation]: #motivation

Type layout information is very frequently needed in low level code, especially
if it's performing serialization, FFI, or implementing a data structure.

While often the needed information is limited to the size and required alignment
of a given type, sometimes there is a need to access information about the
fields of a type, most commonly (and most fundamentally) the offset (in bytes),
at which the field may be found in the type which contains it.

Currently, Rust's standard library provides good explicit APIs for providing
information about the size and alignment of a given type (specifically,
`core::mem` has `size_of`, `align_of`, `size_of_val`, and `align_of_val`).
Unfortunately, it provides none for determining field-offset, leaving it to be
computed based on implicitly-provided layout information.

This is an unfortunate gap, one we've seen countless workarounds for, which have
caused no end of trouble in the ecosystem. The problem is that while recovering
layout information in this manner is completely possible in rust (recovering the
size and alignment would even be possible using the same technique), doing it
correctly is very subtle. Most of the implementations which seem obvious are
actually wrong, often because they invoke undefined behavior.

Unfortunately, this also means they often tend to work at first, but have a risk
to be something of a "ticking time-bomb", which may break in a future release of
Rust or LLVM.

This is not a theoretical concern, and widespread breakage of incorrect
`offset_of` implementations has happened in the past (e.g. when `mem::zeroed`
started performing validity checks), and may happen again (e.g. the
`deref_nullptr` lint revealed large bodies of code with incorrect
implementations).

Unfortunately, previously there's not been great alternative. Generally, the
recommendation users are given is to either:

1. Use a crate, for example `memoffset` and `bytemuck` both have `offset_of!`
   implementations.
2. Hardcode the constant.

Both of which have several downsides, but even if the operation can be
flawlessly performed by library code, it's the opinion of the author of this RFC
that this operation is fundamental enough that at a minimum, that the standard
library should provide the implementation.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

In low level code, you may find you need to know the byte offset of a field
within a type. This can be accomplished with the `core::mem::offset_of!` macro.

`core::mem::offset_of!` takes two arguments, the type that holds the field, and
the name of the field. For example, if you have:

```rs
#[repr(C)]
struct Vertex {
    tex: [u16; 2],
    pos: [f32; 3],
}
```

Then you can use `core::mem::offset_of!(Vertex, tex)` to get the offset in bytes
where `tex` begins, and `core::mem::offset_of!(Vertex, pos)` to get the offset
in bytes where `pos` begins.

In this example, we also specified the layout algorithm to use, so we know that
`offset_of!(Vertex, tex)` will be 0, and `offset_of!(Vertex, pos)` will be 4.

However, if a `#[repr(...)]` is not used, the compiler is free to place the
fields of `Vertex` in whatever order it prefers (even if they aren't the same as
the order the fields are written in the struct declaration), so there's no way
to know in advance what the positions of the fields will be.

Thankfully, `offset_of!` is still usable here:

```rs
// No `#[repr()]` needed!
struct Vertex {
    tex: [u16; 2],
    pos: [f32; 3],
}
// This time let's define some constants containing the offset value,
// which can be more readable if you need to use them several times.
const OFFSET_VERTEX_TEX: usize = core::mem::offset_of!(Vertex, tex);
const OFFSET_VERTEX_POS: usize = core::mem::offset_of!(Vertex, pos);
```

As you can see, the usage is the same as before, but because we didn't specify
`#[repr(C)]`, compiler may have changed the order or position, so the values may
be different -- it's completely possible that `pos` is located at offset 0, for
example! Thankfully, by using `core::mem::offset_of!`, this code is correct
either way, and will continute to be correct, even if the layout algorithm
changes in the future.

## `offset_of!` On Other Types

If your type doesn't have named fields, `offset_of!` can still be used. For
tuples and tuple structs, the "name" of the field is the numeral value you use
to access it. For example:

```rust
// Works with a tuple struct
struct KeyVal(&'static str, Vec<u8>);
const OFFSET_KV_KEY: usize = core::mem::offset_of!(KeyVal, 0);
const OFFSET_KV_VAL: usize = core::mem::offset_of!(KeyVal, 1);
// Or with an anonymous tuple.
const OFFSET_ANON_KEY: usize = core::mem::offset_of!((&'static str, Vec<u8>), 0);
const OFFSET_ANON_VAL: usize = core::mem::offset_of!((&'static str, Vec<u8>), 1);
```

Finally, `offset_of!` can be used to compute the offset of fields in unions too.
While this may be surprising, the compiler is allowed to put padding in front of
fields in unions which are not `#[repr(C)]`, which would lead to a non-zero
field offset.

```rs
use core::mem::offset_of;
union Buffer {
    metadata: [u64; 3],
    datadata: [u8; 1024 * 1024 * 32],
}
const METADATA_OFFSET: usize = offset_of!(Buffer, metadata);
```

## Limitations

There are a few limitations worth mentioning. Some of these may be relaxed in
the future, however.

1. Perhaps unsurprisingly, it obeys privacy, so both the type and field you call
   `offset_of!` on must be visible to the code calling `offset_of!`.

2. The type holding the field must be `Sized`, so trying to compute where the
   slice begins in something like `offset_of!((i32, [u32]), 1)` isn't supported.

3. Compared to `offsetof` in C and C++, you can't access nested fields/arrays.
   That is, instead of `offset_of!(Foo, quank.zoop.2.quank[4])`, you'll have to
   compute the offsets of each step manually, and sum them.

4. Finally, types other than tuples, structs, and unions are not currently
   unsupported.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

`offset_of` is a new macro exported from `core::mem` which has an signature
similar to the following:

```rs
pub macro offset_of($Container:ty, $field:tt $(,)?) {
    // ...implementation defined...
}
```

Invoking this macro expands to a constant expression of type `usize`, which
evaluates to the offset in bytes from the beginning of `$Container` where
`$field` is found.

`$Container` must be visible and must be or resolve to one of the following
types:

1. A `struct` or `union` type with either named or anonymous/tuple-style fields.

    In this case, `$field` must share a name or tuple index with a field which:
    - Exists on `$Container`.
    - Is visible at the location where `offset_of!` is invoked (but there is no
      requirement that fields other than than `$field` be visible there)

2. An anonymous tuple type.

    In this case, `$field` must be a tuple index (that is, an integer literal)
    that exists on the tuple type in question.

Use on other types is an error, although this may be relaxed in some cases in
the future (see the [Future possibilities][future-possibilities] section).

As a note: the implementation is strongly encouraged to not have runtime
resource usage dependent on the values of `$Container` or `$field`. In
particular, the implementation should not allocate space for an instance of
`$Container` on the runtime stack.

# Drawbacks
[drawbacks]: #drawbacks

1. This exposes layout information at compile time which is otherwise not
   exposed until runtime. This can cause compatibility hazards similar to
   `mem::size_of` or `mem::align_of`, but plausibly greater as it provides even
   more information.

    That said, this API allows querying information which (if needed at compile
    time) would otherwise be hard-coded, so in some cases it may reduce the risk
    of a compatibility hazard.

2. This is a low level feature that most code won't need to use, so perhaps it
   is better off left out.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The general rationale is that it should remove the need to hardcode, hand-roll,
or pull in a third-party crate in order to compute field offsets. This hopefully
should remove as many barriers

That said, there are several alternatives to this, some of which were even
considered:

1. Do nothing, and tell users to use the [`memoffset`][memoffset] crate, or to
   hard-code constant offsets.

    This was not chosen as this operation seems fundamental enough to provided
    by the standard library, especially given how often it is incorrectly
    implemented in the wild.

2. Add `offset_of!`, but disallow use on `#[repr(Rust)]` types.

    This would make `core::mem::offset_of!` have less functionality than the
    implementation from `memoffset`, or the implementation they could implement
    if they computed it manually.

3. Require that all fields of `$Container` be visible at the invocation site,
   rather than just requiring that `$field` is.

    As above, this would make `core::mem::offset_of!` worse than the version
    they'd have written themselves and/or an off-the-shelf implementation.

4. Add `offset_of!`, but disallow use during constant evaluation.

    This would mean that users which need const access to `offset_of!` must
    continue to hardcode the field offsets as constants, which is undesirable,
    error-prone, and can cause compatibility hazards.

5. Try to make `addr_of!((*null::<$Container>()).$field) as usize` work for this:

    Currently this is UB (due to dereferencing a null pointer) and does not
    support use in const (due to accessing the address of a raw pointer).
    Changing both of these issues would be challenging, but may be possible.

    This was not chosen because seems difficult, and would be harder to teach
    (or read) than `core::mem::offset_of`.

6. Hold off until this can be integrated into some larger language feature, such
   as C++-style pointer-to-field, Swift-style field paths, ...

    Aside from avoiding scope creep, this wasn't pursued as `offset_of!` does
    not prevent these in the future, and may not even be solved by them.

7. Use `offset_of!($Container::$field)` as the syntax instead.

    This wasn't chosen because it doesn't really work with tuples, and seems
    like it may harm the quality of error messages (for example, if a user
    forgets `::$field`, and does `offset_of!(crate::path::to::SomeType)`).

    Additionally, this does not generalize as well to some of the extensions in
    future work.

# Prior art
[prior-art]: #prior-art

There is quite a bit of prior art here, which I've grouped into:

1. Crates: Rust libraries that expose similar or equivalent functionality to
   this proposal.
2. Languages: Other languages that provide access to this information either as
   a language builtin, or via a library.

## Prior Art: Crates

Several crates in the ecosystem have `offset_of!` implementations.
[`memoffset`][memoffset] and [`bytemuck`][bmuckcrate] are probably the two most
popular, and provide this functionality in different ways.

- The [`memoffset`][memoffset] crate provides an `offset_of!` macro very similar
  to this proposal. It is a fairly straightforward implementation that avoids
  most pitfalls, although it does allocate an instance of the type on the stack,
  which can cause stack overflow during debug builds (the compiler removes this
  in release builds).

    On nightly, if the `unstable_const` cargo feature is enabled,
    `memoffset::offset_of!` may be used during constant evaluation.

- The [`bytemuck`][bmuckcrate] crate has an [`offset_of!`][bmuckoffset]
  implementation which differs from the one in `memoffset` in that it takes
  three arguments, where the first is an existing instance of the type (or, due
  to a quirk in how it is implemented, a reference to one).

    This is intended to allow an implementation that does not require `unsafe`
    (as it was added in a time when it was unclear how to provide a sound
    `offset_of!`).

    Somewhat interestingly, this first parameter may be used to avoid a large
    stack allocation by providing a reference to a const/static in this first
    parameter (for example as `bytemuck::offset_of!(&SOME_STATIC, SomeTy,
    field)`).

    It does not support use during constant evaluation.

[memoffset]: https://crates.io/crates/memoffset/0.6.5
[bmuckcrate]: https://crates.io/crates/bytemuck/1.12.1
[bmuckoffset]: https://docs.rs/bytemuck/1.12.1/bytemuck/macro.offset_of.html

## Prior Art: Languages

Many languages which support low level programming have some equivalent to this
functionality.

- The C programming language supports this as an [`offsetof`][coffsetof] macro,
  for example: `offsetof(struct some_struct, some_field)` is morally equivalent
  to this proposal's `offset_of!(SomeStruct, some_field)`. It produces a integer
  constant, so it can be used during C's equivalent of constant evaluation.

    Notably, C's `offsetof` is more powerful than the `offset_of!` proposed in
    this RFC, as it supports access to fields of nested types, and even can
    project through arrays, for example `offsetof(some_type, foo.bar[1].baz)` is
    completely allowed.

    Extending `core::mem::offset_of` to support some of these use-cases could be
    done in the future, as is discussed in the future possibilities section
    below.

- C++ can an [`offsetof`][cppoffsetof] macro which is essentially compatible
  with C's, although it is only "conditionally supported" to use it on types
  which are not "standard layout" (see the linked documentation for information
  on what the quoted text means).

    C++ also has support for getting a pointer to a field via it's
    pointer-to-member feature. This feature is powerful and while it replaces
    some uses of `offsetof`, it does not replace all of them

- Zig supports this via the [`@offsetOf`][zigoffsetof] function, which takes a
  `type` and `u8[]` that contains the field name as a string, for example
  `@offsetOf(SomeType, "some_field")` would be essentially equivalent to this
  proposal's `core::mem::offset_of!(SomeType, some_field)`.

    Zig also supports the [`@bitOffsetOf`][zigbitoffset] function, as Zig allows
    structs to contain fields which are not byte-aligned (e.g. bitfields). The
    syntax and semantics are otherwise equivalent.

    These are all `comptime` functions, which means they may be used in
    situations which are morally equivalent to Rust's constant evaluation.

- The D language allows accessing the offset via a property of each field. For
  example, `SomeType.some_field.offsetof` is essentially equivalent to this
  proposal's `core::mem::offset_of!(SomeType, some_field)`.

- Swift supports this via the [`MemoryLayout.offset(of:)`][swiftoffset] function
  (note: the link contains a good overview of the design). For example,
  `MemoryLayout<SomeType>.offset(of: \.some_field))` would be the equivalent to
  `core::mem::offset_of!(SomeType, some_field)`.

    The `\.some_field` syntax is a partial key path (a Swift language feature).
    This can grant access to fields of nested structs in a manner similar to C's
    `offsetof`, for example: `MemoryLayout<SomeType>.offset(of: \.foo.bar.baz)`.

[coffsetof]: https://en.cppreference.com/w/c/types/offsetof
[cppoffsetof]: https://en.cppreference.com/w/cpp/types/offsetof
[zigoffsetof]: hhttps://ziglang.org/documentation/0.9.1/#offsetOf
[zigbitoffset]: https://ziglang.org/documentation/0.9.1/#bitOffsetOf
[doffsetof]: https://dlang.org/spec/struct.html#struct_field_properties
[swiftoffset]: https://github.com/apple/swift-evolution/blob/ec2028964daeda2600e49aa89fd9e59d2363433b/proposals/0210-key-path-offset.md

# Unresolved questions
[unresolved-questions]: #unresolved-questions

1. Should any of the features listed as "Future possibilities" be supported initially?

# Future possibilities
[future-possibilities]: #future-possibilities

This proposal is intentionally minimal, so there are a number of future
possibilities.

## Enum support (`offset_of!(SomeEnum::StructVariant, field_on_variant)`)

Eventually, it may be desirable to allow `offset_of!` to access the fields
inside the struct and tuple variants of certain enums (possibly limited to enums
with a primitive integer representation, such as `#[repr(C)]`, `#[repr(int)]`,
or `#[repr(C, int)]` -- where `int` is one of Rust's primitive integer types —
u8, isize, u128, etc).

For example, in the future somthing like the following could be allowed:

```rs
use core::mem::offset_of;

#[repr(i8)]
enum Event {
    Key { pressed: bool, code: u32 },
    Resize(u32, u32),
}

const EVENT_KEY_CODE: usize = offset_of!(Event::Key, code);
const EVENT_KEY_PRESSED: usize = offset_of!(Event::Key, pressed);

const EVENT_RESIZE_W: usize = offset_of!(Event::Resize, 0);
const EVENT_RESIZE_H: usize = offset_of!(Event::Resize, 1);
```

In this example, the name/path of the variant is used as the first argument.
While there are use-cases for this in low level FFI code (similar to the use
cases for `#[repr(int)]` and `#[repr(C, int)]` enums), this may need further
design work, and is left to the future.

## Nested Field Access

In C, expressions like `offsetof(struct some_struct, foo.bar.baz[3].quux)` are
allowed, where `foo.bar.baz[3].quux` denotes a path to a derived field. This can
be of somewhat arbitrary complexity, accessing fields of nested structs,
performing array indexing (often this is used to access past the end of the
array even), and so on. Similar functionality is offered by
`MemoryLayout.offset` in Swift, where more complex language features are used to
achieve it.

This was omitted from this proposal because it is not commonly used, and can
generally be replaced (at the cost of convenience) by multiple invocations of
the macro.

Additionally, in the future similar functionality could be added in a fully
backwards-compatible way, either by directly allowing usage like
`offset_of!(SomeStruct, foo.bar.baz[3].quux)`, or by requiring each field be
comma-separated, as in `offset_of!(SomeStruct, foo, bar, baz, [3], quux)`.

Note that while this example shows a combination that supports array indexing,
it's unclear if this is actually desirable for Rust.

## `memoffset::span_of!` Functionality

The `memoffset` crate has support for a [`span_of!`][spanof] macro (used like
`memoffset::span_of!(SomeType, some_field)`), which expands to a `Range<usize>`
indicating which bytes of `SomeType` are from the field `some_field`.

The use case for this is more limited than that of `offset_of!`, so it was
omitted from this proposal. That said, should this prove sufficiently useful, it
would be simple to add a similar macro to `core::mem` in the future.

[spanof]: https://docs.rs/memoffset/0.6.5/memoffset/macro.span_of.html

## Support for types with `?Sized` fields.

Currently, we don't support `offset_of!((u8, [i32]), 1)`, as `(u8, [i32])` does
not implement `Sized`.

This is a mostly artificial restriction, and could be relaxed in the future.
