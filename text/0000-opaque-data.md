- Feature Name: opaque_data
- Start Date: 2017-05-04
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Add a new lang item and corresponding struct declaration to `core` which
represents an "Opaque Struct" of unknown size and layout. This type would serve
a similar purpose to C and C++'s incomplete types, allowing for nominal types
with unknown size and layout to be defined in Rust code.

# Motivation
[motivation]: #motivation

When binding to C or C++ libraries, we often have to deal with references to
data types with an unknown size and/or layout. These types are usually declared
in a header using a forward declaration. This type can then be used behind a
pointer in code which imports that header without the full definition ever being
avaliable, and it is a compilation error to use the type as a value or in such a
way that the compiler would need to use its layout.

Rust currently has no way to define a type which does not have a known layout,
and so instead we have accumulated a series of problematic hacks for defining
such types for use in our type system.

## Current Approach: The Empty Type

Currently when representing a type for which the definition is opaque to Rust code,
the option of choice is to use an empty enum behind a raw pointer. For example,
bindings may look something like the following:

```rust
enum Foo {}

extern "C" {
    fn GetAFoo() -> *const Foo;
}
```

This type has the advantage of being unable to be created in rust, but has a
serious disadvantage: namely that references to it cannot be easily managed
through rust's lifetime system. A reference `&'a Foo` is very unsafe, because
it can be dereferenced by safe code to get a value of type `Foo`, which is
equivalent to `!`, generating the `unreachable` llvm intrinsic when matched
against.

This could also be highly unsafe when the empty type is a private member of an
public type, such as the following:

```rust
mod f {
    enum FooInner {}
    pub struct Foo(FooInner);
}
use f::Foo;

extern "C" {
    fn GetAFoo() -> *const Foo;
}
```

As the type `Foo` is still empty as far as rustc is concerned, so any code which
has access to a valid value of that type can be considered to be dead code.

## Current Approach: Dummy Struct

Other options, such as providing a dummy struct representation, also fall down
in front of methods such as `std::mem::swap`. For example:

```rust
mod f { pub struct Foo(u8); /* ... */ }
use f::Foo;

// ...

fn foo_user(a: &mut Foo, b: &mut Foo) {
    std::mem::swap(a, b);
    // The first byte in the representations of a and b were just swapped. UB heyo!
}
```

Zero-sized types don't have this problem, and are probably our best solution at
the moment, but they also have problems.

1. ZSTs are `Sized`, meaning that code can and will assume that they know the
   size of the value, and may act undesirably when working with the type. For
   example, the above code assumes that the values of `a` and `b` have swapped,
   despite nothing having had happened.

2. Code in the module which defines the type can construct a value of the type,
   despite the type being unconstructable.

3. The `improper_ctypes` lint currently complains about ZSTs appearing in FFI
   function signatures, including behind raw `*const` and `*mut` pointers. This
   lint can currently be avoided using the ZST `[u8; 0]`, and could also be
   removed entirely (see Alternatives).

## Current Approach: Custom Reference Types

Another approach which can be taken is to never place the dummy type behind a
reference, and instead always use a custom reference type. This is safe, but is
very verbose, error prone, and doesn't allow you to take advantage of many of
the benefits of using Rust's types and type system for lifetime management of
opaque data types. For example, instead of writing:

```rust
fn use_some_foos<'a, 'b>(a: &'a Foo, b: &'b mut Foo) { /* ... */ }
```

The user would have to write:

```rust
fn use_some_foos<'a, 'b>(a: FooRef<'a>, b: FooRefMut<'b>) { /* ... */ }
```

Which are custom wrapper types which act like the usual `&` and `&mut` types.
This also means that much generic rust code will not work on these types, as it
expects normal references. As an explicit example, `Deref` cannot produce one
of these references.

## Goals

An ideal solution to this problem would provide a way to define types which
explicitly represent opaque data structures. It should have these properties:

1. The type should be `!Sized` so that operations such as `swap` cannot be
   called on it and code will not assume that it can perform such operations on
   it.

2. The type should not be able to be constructed from safe rust code. The only
   way to obtain a reference to a value of this type should be through FFI or
   unsafe logic like `::mem::transmute()`.

3. The type should be non-empty, meaning that the optimizer cannot assume that
   code is dead because it has a reference to it.

4. Stack allocated values of the type should be statically prohibited.

5. The type should act intuitively and not feel like a hack.

This type also has the advantage of not affecting unsize coercion, which is a
thorny subject, as there is no base type which can unsize coerce to this type.
References to objects which contain `Opaque` must be created manually by unsafe
code.

## Improvements to `CStr`

If the `DynSized` and `impl DynSized` features discussed in
the [Detailed Design](#detailed-design) section are adopted, the `&CStr` type
could be modified to have the same representation as a `*const c_char` for FFI
purposes, by implementing it as an `Opaque` type and implementing `DynSized`
with the `impl Dynsized` feature. An implementation of the minimal starting
features might look something like this:

```rust
use std::marker::Opaque;
use std::mem;

pub struct CStr(Opaque);

impl CStr {
    pub fn from_ptr(p: *const c_char) -> &CStr {
        unsafe {
            &*(p as *const CStr)
        }
    }

    pub fn len(&self) {
        unsafe { strlen(self.as_ptr()) }
    }

    pub fn as_ptr(&self) -> *const c_char {
        self as *const CStr as *const c_char
    }
}

impl DynSized for CStr {
    fn size_of_val(&self) -> usize {
        self.len() * mem::size_of::<c_char>()
    }

    fn align_of_val(&self) -> usize {
        mem::align_of::<c_char>()
    }
}
```

# Detailed design
[design]: #detailed-design

## Lang Item

We would introduce a new lang item, `opaque_data`, and a struct implementation
in `core::marker` to match it:

```rust
#[lang = "opaque_struct"]
pub struct Opaque(());
```

This type would be used within structs to transfer its properties to that
struct, similarialy to other types in `core::marker` such as `PhantomData`.
A crate defining FFI bindings to an opaque C type `Foo` would define the type
as:

```rust
use std::marker; // or core::marker in #![no_std]

#[repr(C)]
pub struct Foo(marker::Opaque);
```

This type would have the following properties:

1. The type would be `!Sized`.

2. The type would be unconstructable, as it has a private constructor, (although
   code inside `core::marker` should also not be able to construct a value of
   the type).

3. References and pointers to values of this type would be thin, having only a
   single pointer width.

4. A `Box<>` of this value would not free memory when dropped, like with ZSTs,
   but would invoke the type's `drop(&mut self)` method if present.

### Traits

`Opaque` should implement the `Debug` trait, which should produce the string
'Opaque'. It shouldn't implement any other traits from the standard library.

## `size_of_val` and `align_of_val`

Rust provides 2 intrinsics which are problematic to the implementation of
`Opaque`, `size_of_val` and `align_of_val`. These two methods take an arbitrary
unbounded reference, and return a `usize` representing the size of the value. We
cannot implement these functions for `Opaque`, as it has a completely unknown
layout, which in some cases may not be determinable even at runtime.

There are multiple options for solving this problem, but this section will only
cover one of them. The others are covered in the [Alternatives](#alternatives)
section.

## The `DynSized` trait

A new built-in, unsafe, autoderived trait, `DynSized`, would be added. This type
would be implemented on all existing types, except for `Opaque` and types which
contain a `Opaque` object.

Like `Sized`, the `DynSized` will be implied in all generic bounds, meaning that
the generic declaration `<T>` would require the `DynSized` bound. Relaxing the
`DynSized` bound would be done with `<T: ?DynSized>`.

In addition, trait objects would be modified. Normal trait objects (`Trait`)
would imply the `DynSized` bound, and contain the vtable entries required to
implement the `size_of_val` and `align_of_val` intrinsics. Types which do not
implement `DynSized` would not be able to be converted to a trait object of this
kind. The syntax `Trait + ?DynSized` could also be written which would relax
this `DynSized` requirement, and not include the vtable entries for
`size_of_val` and `align_of_val`. `Sized` trait object should be able to
implicitly coerce to `DynSized` trait objects by offseting their vtable pointer
past the size and align vtable members.

The `Sized` trait would be modified to inherit from the `DynSized` trait, meaning
that `?DynSized` implies `?Sized`.

Finally, many trait parmaeters would be updated in the standard library to relax
their trait bounds from `?Sized` to `?DynSized` which should be a
backwards-compatible change.

`size_of_val` and `align_of_val` would require the `DynSized` trait.

If we decide that we want to allow types which contain `Opaque` to specify a
method for determining `size_of_val` and `align_of_val`, this could easily be
done by allowing `DynSized` to be explicitly implemented:

```rust
import std::marker::DynSized;

pub struct MyStruct(Opaque);

unsafe impl DynSized for MyStruct {
    fn size_of_val(&self) -> usize {
        somehow_determine_size()
    }

    fn align_of_val(&self) -> usize {
        somehow_determine_alignment()
    }
}
```

## Lints

This type is very easy to use incorrectly by placing it in a non-`#[repr(C)]`
struct. A lint, defaulting to `deny`, which warns if `Opaque` is used as a field
of a non `#[repr(C)]` struct should be added, as the layout of the struct cannot
be known for non-`#[repr(C)]` structs.

Alternatively, this could become a strict compile-time error.

Other `#[repr]` annotations which provide a known layout (such as
`#[repr(packed)]` and `#[repr(transparent)]`) should also be OK to use in this
situation.

It is already an error to include an unsized type in an enum variant.

## Feature Gate

The `Opaque` type, and `DynSized` trait would be gated behind the `opaque_data`
feature gate. If implementing `DynSized` is desired, it will be separately gated
behind the `impl_dynsized` feature gate, as it will likely be harder to
stabilize.

# How We Teach This
[how-we-teach-this]: #how-we-teach-this

## The `Opaque` Struct

This type should be taught alongside other FFI tools. In _The Rust Programming
Language_ this should be mentioned as a mechanism for modeling forward declared
C or C++ types which do not have a declared body and will only be manipulated
behind a reference.

It should also be demonstrated how this type can be used to represent foreign
structs with a known prefix, which is a fairly common C pattern, by writing
a type like the following:

```rust
use std::marker::Opaque;

#[repr(C)]
struct KnownPrefix {
    field1: u32,
    field2: u32,
    __opaque: Opaque,
}
```

Existing rust users will have this type introduced to them much the same way
as an existing user, as a superior way to define forward-declared C/C++ types
in rust code to `enum T {}` or `struct T([u8; 0])`.

## The `DynSized` Trait

The `DynSized` trait would be introduced alongside the `Sized` trait as a trait
bound which can be relaxed when a value only needs to be manipulated behind a
reference. It should be suggested that `DynSized` is usually more appropriate,
unless your consumer requires access to `size_of_val` or `align_of_val`, which
should be a fairly small number of users of the `?Sized` trait bound.

If we decide to support the `impl DynSized` feature, it should be introduced
alongside `Opaque` in the `FFI` chapter as a way to specify the size and
alignment of dynamically sized opaque types.

Existing users will have this trait introduced to them as a weaker version of
`Sized`, which only requires that rust is able to determine the size of the type
dynamically at runtime, and not at compile time.

# Drawbacks
[drawbacks]: #drawbacks

This type adds complexity to the rust compiler in the firm of an additional lang
item and built in trait. This complexity is exchanged for what some may consider
to be a "small" improvement over the existing solution of using a zero-sized
type to represent forward-declared C/C++ types in rust code. See
the [Alternatives](#alternatives) section for more information on this
alternative.

This type also changes the layout for unsized pointers, introducing a pointer
which is both unsized, and thin. This is something we likely want to do anyway
as part of a DST or thin traits RFC, but it may break some unsafe code
assumptions.

# Alternatives
[alternatives]: #alternatives

The alternatives section is written in two parts. The first part talks about
alternatives to the use of the `Opaque` type alltogether, while the second talks
about alternative solutions to the `size_of_val` and `align_of_val` issues.

## The `Opaque` Type

### Encourage Zero Sized Types in FFI

Currently the `improper_ctypes` lint complains when ZSTs (zero sized types) are
used in `extern "C"` function signatures. This is done because C doesn't have
the concept of a zero sized type, and thus passing one by value will likely not
do what you would expect.

Zero sized types with a private member have many of the properties which you
would want from a type to represent opaque data types.

1. They cannot be constructed from outside the module they are defined in
   without unsafe code, meaning that safe rust should not be able to obtain a
   reference to a valid value of that type.

2. They are considered non-empty types, meaning that rust will not consider code
   which contains a reference to them to be dead code.

3. Attempts to perform operations such as `std::mem::swap` on two values will
   be compiled to a no-op, avoiding any undefined behavior which may result.

Unfortunately, they don't have some of the features of the `Opaque` data type,
such as:

1. They are `Sized`, which means that unsafe code may incorrectly assume that
   they may be allocated on the stack, or passed around by value.

2. `size_of::<T>` and `align_of::<T>` lie when passed the zero sized type,
   stating that the type has size 0. Same with `size_of_val` and `align_of_val`.

3. Type declarations written using this style are not clearly forward declared
   types from C++, and documentation will be needed in places where they are
   declared to clarify the intent of the type.

4. The type is constructable within the module where it is declared.

5. It is legal to write a type signature which contains this type passed by
   value.

If this choice is taken, the `improper_ctypes` lint should be modified to allow
zero sized types in `extern "C"` function declarations behind pointers or
references, and _The Rust Programming Language_ should mention this as a best
practice in the FFI chapter.

### Custom `extern type` declaration syntax

Instead of using a marker type `Opaque`, a new `extern type` syntax could be
added for defining forward declared
types. [RFC 1861](https://github.com/rust-lang/rfcs/pull/1861) proposes this
idea, and discusses it in more detail. This has the advantage of being
syntactically closer to the C/C++ definition of these types, but also adds more
syntactic complexity to the language.

### Custom `opaque` attribute

Instead of using the marker type `Opaque`, an attribute `#[opaque]` or
`#[repr(opaque)]` could be added that marks the type which it is placed on as
opaque. This would be very similar to the `Opaque` object, however it does not
follow the common system for unsized objects, which is that an object becomes
unsized by having another unsized object being placed inside of it, or by being
one of the built in unsized types.

### Custom Dynamically Sized Types

The `Opaque` type could almost entirely be implemented by an advanced system for
defining custon dynamically sized types, such as the system proposed
in [RFC 1524](https://github.com/rust-lang/rfcs/pull/1524). This feature has
the benefit of being a subset of the features of that RFC, and if something like
it was to be implemented in the future, the `Opaque` type could be changed from
a lang item to instead being implemented on top of it backwards-compatibly.

Unfortunately, while this feature will be much more powerful than `Opaque`,
Merging an RFC for custom dynamically sized types will be a much bigger job than
implementing the `Opaque` type.

## Dynamic Size and Alignment

The addition of the `DynSized` trait is probably the most significant change in
this RFC, as it adds a new built in trait which then would need to be adopted
over `?Sized` in community crates which could support it to allow `Opaque` types
to be used everywhere where unsized types are used today.

A possible alternative to this built in trait would be to avoid having the trait
alltogether, and define an alternate solution for what the value of
`size_of_val(&Opaque)` and `align_of_val(&Opaque)` should return.

### Panicing

The simplest solution would be for the `size_of_val` and `align_of_val` methods to
panic when called on `Opaque` types, with a message like:

```rust
fn size_of_val(_: &Opaque) -> usize {
    panic!("size_of_val called on opaque type");
}
```

This has the unfortunate consequence of making `size_of_val` and `align_of_val`
much less useful types. In this situation we would likely want to deprecate
these functions, adding a comment explaining this unfortunate behavior, and
introduce new functions which return `Option<usize>` instead of `usize` so that
code which depends on the size or alignment of a value can recover correctly.

This also involves a lot of work teaching the community and new programmers of
unsafe rust that `size_of_val` and `align_of_val` can panic in this weird edge
case.

### Faking It

Another simple solution would be to fake the values which we return from this
function to be ones which are likely to not do harm in unsafe code. For example,
we could make the function `size_of_val` unconditionally return `0` (which means
that unsafe code shouldn't ever read off the end of the backing allocation), and
`align_of_val` return the same alignment that `malloc` returns in C or the
maximum alignment which the type could require based on its pointer, whichever
is smaller, as it should be accurate for most types allocated in C.

This has the problem again of needing to teach unsafe programmers that when they
encounter this specific type they can be lied to by these functions, and if we
take this approach we will, again, likely want to deprecate these functions and
replace them with alternatives which return `Option<usize>` so that programmers
can handle this edge case better.

# Unresolved questions
[unresolved]: #unresolved-questions

The biggest unresolved questions are related to what decision to make with
regard to the Dynamic Size and Alignment issue mentioned in
the [Alternatives](#alternatives) section, and issues around the specifics of
implementation in rustc.
