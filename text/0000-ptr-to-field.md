- Feature Name: ptr-to-field
- Start Date: 2019-06-05
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This feature could serve as the backbone for some pointer to field syntax, and even if no syntax is made, this feature serves as a safe generic way to talk about types and their fields.

# Motivation
[motivation]: #motivation

The motivation for this feature is to allow safe projection through smart pointers, for example `Pin<&mut T>` to `Pin<&mut Field>`. This is a much needed feature to make `Pin<P>` more usable in safe-contexts, without the need to use unsafe to map to a field. This also can allow projection through other smart pointers like `Rc<T>`, `Arc<T>`. This feature cannot be implemented as a library effectively because it depends on the layouts of types, so it requires integration with the Rust compiler until Rust gets a stable layout (which may never happen).

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This RFC introduces the idea of field types, and build projections on top of field types.

## Terminology, types, and traits

### Field Types

A field type is a compiler generated type that represents a field. For example,

```rust
struct Person {
    name: String,
    age: u32,
}
```

A field type for the `name` field of `Person` would be `Person.name`.

<sub>Note about syntax: the syntax `Type.field` is going to be used as a placeholder syntax for the rest of this RFC, but this is *not* the final syntax for field types, that decision will be made after this RFC gets accepted but before it gets stabilized.</sub>

Field types serve a few important purposes
* They are integrated with the new `Project` trait (explained later)
* Becaue they are types, they can implement traits
    * This will allow conditional implementations of `Project`, which is important for `Pin<P>` (also explained later)

Because they are types we can also generalize over field types like so...

### `trait Field` and `type FieldDescriptor`

In order to generalize over field types, we have the `trait Field` 

```rust
trait Field {
    /// The type of the type that the field comes from
    type Parent: ?Sized;

    /// The type of the field itself
    type Type: ?Sized;

    const FIELD_DESCRIPTOR: FieldDescriptor;
}

struct FieldDescriptor {
    ...
}
```
`FieldDescriptor` is an opaque type that will store some metadata about how to convert from a `*const Field::Parent` to a `*const Field::Type`. There is no way to safely construct `FieldDescriptor` from user code on Stable Rust until Rust gets a defined stable type layout for `repr(Rust)` types.

The `Field` trait will allow generalizing over field types, and thus allow other apis to be created, for example...

### `*const T`/`*mut T` methods

We will add the following methods to raw pointers
```rust
// details about why we need both and what they do exactly in Reference-level explanation

impl<T: ?Sized> *const T {
    pub unsafe fn project_unchecked<F: Field<Parent = T>>(self, field: F) -> *const F::Type;
    
    pub fn wrapping_project<F: Field<Parent = T>>(self, field: F) -> *const F::Type;
}

impl<T: ?Sized> *mut T {
    pub unsafe fn project_unchecked<F: Field<Parent = T>>(self, field: F) -> *mut F::Type;
    
    pub fn wrapping_project<F: Field<Parent = T>>(self, field: F) -> *mut F::Type;
}
```

These will allowing projections through raw pointers without dereferencing the raw pointer. This is useful for building projections through other abstractions like smart pointers (`Rc<T>`, `Pin<&T>`)!

This is the extent of the core api of this RFC.

Using this we can do something like this

```rust
struct Foo {
    bar: Bar,
    age: u32
}

struct Bar {
    name: String,
    id: i32
}

let x : Foo = ...;
let y : *const Foo = &x;

let y_bar_name: *const String = unsafe { y.project_unchecked(Foo.bar).project_unchecked(Foo.name) };
```

In the end `y_bar_name` will contain a pointer to `x.bar.name`, all without dereferencing a single pointer! (Given that this is a verbose, we may want some syntax for this, but that is out of scope for this RFC)

But, we can build on this foundation and create a more power abstraction, to generalize this project notion to smart pointers...

### `trait Project`

```rust
trait Project<F: Field> {
    type Projection;

    fn project(self, field: F) -> Self::Projection;
}
```

This trait takes a pointer/smart pointer/reference and gives back a projections that represents a field on the pointee.

i.e.

For raw pointers we could implement this as so

```rust
impl<T: ?Sized, F: Field<Parent = T>> Project<F> for *const T {
    type Projection = F::Type;
    
    fn project(self, field: F) -> Self::Projection {
        self.wrapping_project(field)
    }
}
```

On it's own, this doesn't look like much, but once we have implementations for `&T`, we can also get an implementation for `Pin<&T>`! Meaning we would have safe projections through pins! (See [implementing pin projections](#implementing-pin-projections) for details about this)

But before we can get there we need to discuss...

### `trait PinProjectable`

```rust
unsafe trait PinProjectable {}
```

Due to some safety requirements that will be detailed in the reference-level explanation, we can't just freely hand out pin projections to every type (sad as it is). To enable pin projections to a field, a field type must implement `PinProjectable`.

Like so,
```rust
unsafe impl PinProjectable for Foo.field {}
```

## Examples of usage

Here is a toy example of how to use this api:
Given the implementation of `Project for Pin<&mut T>`
```rust
use std::project::Project;

struct Foo {
    bar: u32,
    qaz: u32,
    hal: u32,
    ptr: *const u32,
    pin: PhantomPinned
}

// These type annotations are unnecessary, but I put them in for clarity 

let foo : Pin<Box<Foo>  = Box::pin(...);

let foo : Pin<&mut Foo> = foo.as_mut();

let bar : Pin<&mut u32> = foo.project(Foo.bar);

*bar = 10;
...
```

In entirely safe code, we are able to set the value of a field inside a pinned type!

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This section will go in detail about the semantics and implementation of the types, traits, and methods introduced by this RFC.

## Field types

Field types are just sugar for unit types (`struct Foo;`) that are declared right next to the `Parent` type.

Like so,
```rust
struct Person {
    pub name: String,
    pub(super) age: u32,
    id: u32,
}

pub struct Person.name;
pub(super) struct Person.age;
struct Person.id;
```

All field types will implement the following traits: `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Hash`, with the same implementation for unit types.

They will also need to interact with the privacy rules for fields, and will have the same privacy as the field that they are derived from.

You can make a field type for the following types of types (some example syntax is shown)
* tuples `(T, U, ...)`
    * `<(T, U)>.0`
* tuple structs `Foo(T, U, ...)`
    * `Foo.0`
* structs `struct Foo { field: Field, ... }`
    * same syntax as tuple struct
* unions `union Foo { field: Field }`
    * same syntax as tuple struct
    * constructing a field type is `unsafe`, this is because accessing fields of `union`s is unsafe

The compiler can decide whether to actual generate a field type, this is to help compile times (if you don't use field types, then the compiler shouldn't slow down too much because of this feature).

## `*const T`/`*mut T`

`project_unchecked` and `wrapping_project` will both live inside of `std::ptr`.

We need both `project_unchecked` and `wrapping_project` because there are some important optimization available inside of LLVM related to aliasing and escape analysis. In particular the LLVM `inbounds` assertion tells LLVM that a pointer offset stays within the same allocation and if the pointer is invalid or the offset does not stay within the same allocation it is considered UB. This behaviour is exposed via `project_unchecked`. This can be used by implementations of the `Project` trait for smart pointers that are always valid, like `&T` to enable better codegen. `wrapping_project` on the other hand will not assert `inbounds`, and will just wrap around if the pointer offset is larger than `usize::max_value()`. This safe defined behaviour, even if it is almost always a bug, unlike `project_unchecked` which is UB on invalid pointers or offsets.

This corresponds with `std::ptr::offset` and `std::ptr::wrapping_offset` in safety and behaviour. `project_unchecked` and `project` need to be implemented as intrinsics because there is no way to assert that the pointer metadata for fat pointers of `Field::Parent` and `Field::Type` will always match in general without some other compiler support. This is necessary to allow unsized types to be used transparently with this scheme.

For example of where `project_unchecked` would be UB.

```rust
struct Foo {
    bar: Bar
}

let x : *const Foo = 2usize as *const Foo;
let y : *const Bar = x.project_unchecked(Foo.bar); // UB, x does not point to a valid instance of Foo
```

With `wrapping_project` trait

```rust
let z : *const Bar = x.wrapping_project(Foo.bar); // not UB, but is still invalid
```

If the raw pointer is valid, then the result of both `project_unchecked` and `wrapping_project` is a raw pointer to the given field.

## Type and Traits

The `Project`, and `Field` traits and the `FieldDescriptor` type will live inside the `std::project` module, and will not be added to `prelude`.
The `PinProjectable` trait will be added to `std::marker`, and will also not be added to `prelude`.

The `Field` trait will only be implemented by the compiler, and it compiler should make sure that no other implementations exist. This allows unsafe code to assume that the implmentors or the `Field` trait always refer to valid fields. The `FieldDescriptor` type may be unnecessary raw pointer projections are implemented via intrinsics, if so we can remove it entirely.

The `Project` trait will be implemented for `*const T`, `*mut T`, `&T`, `&mut T`, `Pin<&T>`, `Pin<&mut T>`. Other smart pointers can get implementations later if they need them. We may also provide the following implementations to allow better documentation of intent

## `PinProjectable`

The safety of `PinProjectable` depends on a few things. One if a field type is marked `PinProjectable`, then the `Drop` on `Parent` may not move that field or otherwise invalidate it. i.e. You must treat that field as if a `Pin<&Field::Type>` has been made on it outside of your code.

Because the following impls conflict,

```rust
unsafe impl<F: Field> PinProjectable for F where F::Parent: Unpin {}

struct Foo(std::marker::PhantomPinned);

unsafe impl PinProjectable for Foo.0 {}
```
[Proof](https://play.rust-lang.org/?version=nightly&mode=debug&edition=2018&gist=b0796a8b631e0fec1804318caef162c7)

I think making `PhantomPinned` a lang-item that is known to always implment `!Unpin` would solve this. This way only those who implment `!Unpin` types need to worry about implementing `PinProjectable`. Another way to solve this would be to somehow make `PinProjectable` a lang-item that allows this one case of conflicting impls. But I am unsure of how to properly handle this, both way s that I showed seem unsatisfactory. The blanket impl is highly desirable, because it enables those who don't write `!Unpin` types to ignore safe pin projections, and still have them available.

Because of this, is `PinProjectable` worth it? Or do we want to punt it to another RFC.

## Implementing pin projections
[impl-pin-projections]: #implementing-pin-projections

First we neeed an implmention of `Project` for `&T` before we can get `Pin<&T>`

```rust
impl<'a, F: Field> Project<F> for &'a F::Parent where F::Type: 'a {
    type Projection = &'a F::Type;

    fn project(self, field: F) -> Self {
        unsafe {
            // This is safe because a reference is always valids
            // So offsetting the reference to a field is always fine
            // The resulting pointer must always be valid
            // because it came from a reference
            let ptr: *const F::Type = (self as *const F::Parent).project_unchecked(field);

            &*ptr
        }
    }
}
```

We can introduce an unsafe projection interface to better document intent (this is optional)

```rust
impl<'a, T> Pin<&'a T> {
    /// This is unsafe because `Drop` code 
    pub unsafe fn project_unchecked<F: Field<Parent = T>>(self, field: F) -> Pin<&'a F::Type> {
        self.map_unchecked(|slf| slf.project(field))
    }
}

impl<'a, T> Pin<&'a mut T> {
    pub unsafe fn project_unchecked<F: Field<Parent = T>>(self, field: F) -> Pin<&'a mut F::Type> {
        self.map_unchecked_mut(|slf| slf.project(field))
    }
}
```

We can then implment safe pin projections like so

```rust
impl<'a, T> Project<F: Field<Parent = T>> for Pin<&'a T> 
where F: PinProjectable {
    type Projection = Pin<&'a F::Type>;

    fn project(self, field: F) -> Self::Projection {
        unsafe { self.project_unchecked(field) }
    }
}
```

# Drawbacks
[drawbacks]: #drawbacks

- This adds quite a bit of complexity to both the compiler and the standard library and could increase dramatically compile times

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- The `&[mut] raw T` could solve some of the problems, but only for raw pointers. It doesn't help with abstractions.
- Somehow expand on `Deref` to allow dereferencing to a smart pointer
    - This would require Generic Associated Types at the very least, and maybe some other features like assocaited traits

# Prior art
[prior-art]: #prior-art

- C++'s pointer to members `Parent::*field`
- Java's `class Field`
    - Similar reflection capabilies in other languages

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Do we want to strip this proposal down to just the [api specified here](https://github.com/rust-lang/rfcs/pull/2708#issuecomment-499578814), where we only have the `Field` trait, `FieldDescriptor` type, and some associated functions on raw pointers.
- Are we going to accept `PinProjectable`?
    - If not, we won't have a safe way to do pin-projections
    - Do we want another way to do safe pin-projections?

- Syntax for the type fields
    - not to be decided before accepting this RFC, but must be decided before stabilization
- Do we want a dedicated syntax to go with the Project trait?
    - If yes, the actual syntax can be decided after accepting this RFC and before stabilization

# Future possibilities
[future-possibilities]: #future-possibilities

- Extend the `Project` trait to implement all smart pointers in the standard library
- [`InitPtr`](https://internals.rust-lang.org/t/idea-pointer-to-field/10061/72), which encapsulates all of the safety requirements of `project_unchecked` into `InitPtr::new` and safely implements `Project`
- Distant Future, we could reformulate `Copy` based on the `Field` trait so that it enforces that all of the fields of a type must be `Copy` in order to be the type to be `Copy`, and thus reduce the amount of magic in the compiler.