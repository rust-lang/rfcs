TODO

#![feature(default_type_parameter_fallback)] - for function generics
don't need import flag
cast references
rvalue stuff
match
unsized enums


- Feature Name: variant_types
- Start Date: 2016-01-07
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

This is something of a two-part RFC, it proposes

* making enum variants first-class types,
* untagged enums (aka unions).

The latter is part of the motivation for the former and relies on the former to
be ergonomic.

In the service of making variant types work, there is some digression into
default type parameters for functions. However, that needs its own RFC to be
spec'ed properly.


# Motivation

Enums are a convenient way of dealing with data which can be in one of many
forms. When dealing with such data, it is typical to match, then perform some
operations on the interior data. However, in many cases there is a large amount
of processing to be done. Ideally we would factor that out into a function,
passing the data to the function. However, currently in Rust, enum variants are
not types and so we must choose an unsatisfactory work around - we pass
each field of the variant separately (leading to unwieldy function signatures
and poor maintainability), we pass the whole variant with enum type (and have to
match again, with `unreachable!` arms in the function), or we embed a struct
within the variant and pass the struct (duplicating data structures for no good
reason). It would be much nicer if we could refer to the variant directly in the
type system.

When working with FFI code, we need to communicate with C programs which may use
union data types. There is no way to represent a union in Rust, and thus working
with such types is awkward and involves bug-prone transmutes. We should provide
some way for Rust to handle such types.

As we'll see below, variant types allow for an elegant solution to the union
problem.


# Detailed design - variant types

Consider the example enum `Foo`:

```rust
pub enum Foo {
    Variant1,
    Variant2(i32, &'static str),
    Variant3 { f1: i32, f2: &'static str },
}
```

We create new instances by constructing one of the variants. The only type
introduced is `Foo`. Variant names can only be used in patterns and for creating
instances. E.g.,

```rust
fn new_foo() -> Foo {
    Foo::Variant2(42, "Hello!")
}
```

This RFC proposes allowing the programmer to use variant names as types, e.g.,

```rust
fn bar(x: Foo::Variant2) {}
struct Baz {
    field: Foo::Variant3,
}
```

Both enums and their variants can currently be imported:

```rust
use Foo;
use Foo::Variant1;
```

Importing an enum imports it into both the value and type namespace. Importing
a variant imports it only into the value namespace. To maintain backwards
compatibility, this will remain the default. In order to import an enum variant
into the type namespace, one must use the `import_variant_type` attribute:

```rust
use Foo;
#[import_variant_type]
use Foo::Variant1;

fn bar(v: Variant1) {
    let _ = Variant1;
}
```

When we release Rust v2.0, we may choose to import variants into both namespaces
by default and remove the attribute.


## Constructors

Consider `let x = Foo::Variant1;`, currently `x` has type `Foo`. In order to
preserve backwards compatibility, this must remain the case. However, it would
be convenient for `let x: Foo::Variant1 = Foo::Variant1;` to also be valid.

The type checker must consider multiple types for an enum construction
expression - both the variant type and the enum type. If there is no further
information to infer one or the other type, then the type checker uses the enum
type by default. This is analogous to the system we use for integer fallback or
default type parameters.

The type of the variants when used as functions must change. Currently they have
a type which maps from the field types to the enum type:

```rust
let x: &Fn(i32, &'static str) -> Foo = &Foo::Variant2;
```

I.e., one could imagine an implicit function definition:

```rust
impl Foo {
    fn Variant2(a: i32, b: &'static str) -> Foo { ... }
}
```

This would change to accommodate inferring either the enum or variant type,
imagine

```rust
impl Foo {
    fn Variant2<T=Foo>(a: i32, b: &'static str) -> T { ... }
}
```

Since we do not allow generic function types, the result type must be chosen
when the function is referenced:

```rust
let x: &Fn(i32, &'static str) -> Foo = &Foo::Variant2::<Foo>;
let x: &Fn(i32, &'static str) -> Foo::Variant2 = &Foo::Variant2::<Foo::Variant2>;
```

Due to the default type parameter, we remain backwards compatible:

```rust
let x: &Fn(i32, &'static str) -> Foo = &Foo::Variant2;
```

Note that this is an innovation. Default type parameters on functions have
[recently](https://github.com/rust-lang/rust/pull/30724) been feature-gated for
more consideration. The compiler has never accepted referencing a generic
function without specifying type parameters, even when there is a default.
However, I think this should be the expected behaviour. This should be discussed
further in a separate RFC.


## Representation

Enum values have the same representation whether they have enum or variant type.
That is, a value with variant type will still include the discriminant and
padding to the size of the largest variant. This is to make sharing
implementations easier (via coercion), see below.

## Conversions

A variant value may be implicitly coerced to its corresponding enum type (an
upcast). An enum value may be explicitly cast to the type of any of its variants
(a downcast). Such a cast includes a dynamic check of the discriminant and will
panic if the cast is to the wrong variant. Variant values may not be converted
to other variant types. E.g.,

```
let a: Foo::Variant1 = Foo::Variant1;
let b: Foo = a; // Ok
let _: Foo::Variant2 = a; // Compile-time error
let _: Foo::Variant2 = b; // Compile-time error
let _ = a as Foo::Variant2; // Compile-time error
let _ = b as Foo::Variant2; // Runtime error
let _ = b as Foo::Variant1; // Ok
```

## impls

`impl`s may exist for both enum and variant types. There is no explicit sharing
of impls, and just because as enum has a trait bound, does not imply that the
variant also has that bound. However, the usual conversion rules apply, so if a
method would apply to the enum type, it can be called on a variant value due to
coercion performed by the dot operator.


# Detailed design - untagged enums

An enum may have `#[repr(union)]` as an attibute. This implies `#[repr(C)]`,
i.e., variants will have the layout expected for C structs. More importantly, it
means that the enum is untagged: there is no discriminant. Matching (and `if
let`, etc.) are not allowed on such enums.

The size of a union value is exactly the size of the largest variant (including
any padding). There is no discriminant, nor is it possible to have drop flags.

There is no restriction on the kind of variants that can be used with
`#[repr(union)]`. Unit-like, tuple-like, and struct-like can all be used. Note
that if all variants are unit-like, then the enum is a zero-sized type. If there
are other variants, then unit-like variant values are all padding. I don't see
the utility of such variants, but I see no reason to ban them.

The only operation that can be performed on a union value is casting. An enum
value can be cast to a variant type. This is not checked (it cannot be, since
there is no discriminant) and thus is *unsafe*. Variants can also be cast
'sideways' to other variant types (also unsafe). Like other enums, a variant
value can be implicitly coerced to the enum type; this is a safe operation.

impls work exactly like regular enums.

## Example

```rust
#[repr(union)]
enum MyUnion {
    MyInt(i64),
    MyBytes(u8, u8, u8, u8),
}

fn foo(m: MyUnion) -> i64 {
    #[import_variant_type]
    use MyUnion::*;

    assert!(size_of::<MyUnion>() == 8);
    assert!(size_of::<MyInt>() == 8);
    assert!(size_of::<MyBytes>() == 8); // 4 bytes of inaccessible padding

    if consult_magic_8_ball() == 42 {
        unsafe {
            process_bytes(m as MyBytes)
        }
    } else {
        unsafe { m as MyInt }.0
    }
}

fn process_bytes(bytes: MyUnion::MyBytes) -> i64 {
    // safe code
    ...
}
```

## Destructors

It would be unsafe for the compiler to assume that a union is a particular
variant, therefore it cannot run destructors for any fields in the union. For
consistency, destructors will not be run even if the union value has a variant
type.

There are two ways to achieve this, either it is forbidden for any field in a
union to implement `Drop`; or, even if a field implements `Drop`, this is
ignored. A compromise solution is that the programmer must opt-in to ignoring
`Drop` on a per-field, per-variant, or per-enum basis, and otherwise fields
which implement `Drop` are forbidden, either with an attribute, or with a
`ManuallyDrop` type (see [RFC PR 197](https://github.com/rust-lang/rfcs/pull/197)).
I prefer this compromise solution.

It will be legal to implement `Drop` for an enum type, but illegal to implement
`Drop` for a variant type (if the variant belongs to an untagged enum). I fear
this must just be an ad-hoc check in the compiler.


# Drawbacks

The variant types proposal is a little bit hairy, in part due to trying to
remain backwards compatible.

One could argue that having both tagged and untagged enums in a language is
confusing. However, I believe the guidance here can be very clear: only use
`#[repr(union)]` for C/FFI interop. The fact that it is an attribute should make
it an obvious second choice.


# Alternatives

An alternative to allowing variants as types is allowing sets of variants as
types, a kind of refinement type. This set could have one member and then would
be equivalent to variant types, or could have all variants as members, making it
equivalent to the enum type. Although more powerful, this approach is more
complex, and I do not believe the complexity is justified.


## Unsafe enums

See [RFC PR 724](https://github.com/rust-lang/rfcs/pull/724) and
[internals dicussion](https://internals.rust-lang.org/t/pre-rfc-unsafe-enums-now-including-a-poll/2873).

Uses `unsafe` rather than an attribute to indicate that an enum is untagged.
Uses an unsafe, irrefutable pattern match (let syntax) to destructure the enum,
giving access to its fields.

Using variant types and unsafe casting as proposed here should be more ergonomic
- it better isolates the operation which is unsafe (discriminating the enum),
from the safe operations (operating on the fields themselves).

## Union structs

See [RFC PR 1444](https://github.com/rust-lang/rfcs/pull/1444).

Annotates structs rather than enums. This has the advantage over RFC 724 that
fields can be accessed directly which is an ergonomic improvement (also true
with this proposal). However, since all field access must be unsafe, it still
requires more unsafe code than you might want.

My preference is for an enum approach (as oppossed to structs) since a union
offers multiple choices of data, like an enum, rather than combining data
together like a struct. That is, enums and unions both 'or' data together,
whereas structs 'and' data together. (In C, structs and unions are syntactically
similar, but semantically very different).

Furthermore, by using enums we allow union variants to have more than one field.
While this is strictly more powerful than is needed for C interop, it is useful
in general. For example, when dealing with binary data, formats will often have
fields which are a given size, but may contain data of different types, an
untagged enum is perfect for this.


# Unresolved questions

There is some potential overlap with some parts of some proposals for efficient
inheritance: if we allow nested enums, then there are many more possible types
for a variant, and generally more complexity. If we allow data bounds (c.f.,
trait bounds, e.g., a struct is a bound on any structs which inherit from it),
then perhaps enum types should be considered bounds on their variant types.
There are also interesting questions around subtyping. However, without a
concrete proposal, it is difficult to deeply consider the issues here.

See destructor question above.
