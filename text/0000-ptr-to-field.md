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

First the core traits, type, and functions that need to be added

```rust
/// Contains metadata about how to get to the field from the parent using raw pointers
/// This is an opaque type that never needs to be stablized, and it only an implementation detail
struct FieldDescriptor {
    ...
}

/// The compiler should prevent user implementations for `Field`,
/// i.e. only the compiler is allowed to implement `Field`
/// This is to prevent people from creating fake "fields"
trait Field {
    /// The type that the field is a part of
    type Parent;
    /// The type of the field
    type Type;

    /// The metadata required to get to the field using raw pointers
    const FIELD_DESCRIPTOR: FieldDescriptor;
}

trait Project<F: Field> {
    /// The projected version of Self
    type Projection;

    fn project(self, field: F) -> Self::Projection;
}

impl<T: ?Sized> *const T {
    unsafe fn project_unchecked<F: Field<Parent = T>>(self, field: F) -> *const F::Type {
        // make the field pointer, this code is allowed to assume that
        // self points to a valid instance of T
        ... 
    }
}

impl<T: ?Sized> *mut T {
    unsafe fn project_unchecked<F: Field<Parent = T>>(self, field: F) -> *mut F::Type {
        // make the field pointer, this code is allowed to assume that
        // self points to a valid instance of T
        ...
    }
}
```

Now we need some syntax to refer to the fields of types. Some ideas for the syntax are

* `Parent.field`  // my favorite, as it seems most natural
* `Parent::field` // bad as it conflicts with associated functions
* `Parent~field`  // or any other sigil

We will call these field types, because they will desugar to a unit type that correctly implements `Field`, like so

```rust
struct Foo {
    bar: Bar
}

struct Foo.bar; // or some other name mangle that doesn't conflict with any other name

impl Field for Foo.bar { ... }
```

These are the core parts of this proposal. Every other part of this proposal can be postponed or dropped without affecting this feature's core principles.

Using these core parts we can build as a library projections through `Pin<&T>`, `Rc<_>` and more. We can then use this to safely project through smart pointers like so.

```rust
let foo   : Pin<Box<Foo>    = Box::pin(immovable);
let foo   : Pin<&mut Foo>   = foo.as_mut();
let field : Pin<&mut Field> = foo.project(Foo.field);
```
But to do safe pin projections we will need to introduce a marker trait. Adding this trait is up for debate.
```rust
/// The only people who can implement `PinProjectable` are the creator of the parent type
/// This allows people to opt-in to allowing their fields to be pin projectable.
/// The guarantee is that once you create `Pin<P<Parent>>`, all of the same guarantees that
/// apply to `Pin<P<Parent>>` also apply to `Pin<P<Field>>`
/// For all `Parent: Unpin`, these can be auto implemented for all of their fields.
unsafe trait PinProjectable: Field {}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The field types needs to interact with the privacy rules for fields. A field type has the same privacy as the field it is derived from. Anything else would be too restrictive or unsound.

The `Project`, and `Field` traits and the `FieldDescriptor` type will live inside the `std::project` module, and will not be added to `prelude`.
The `PinProjectable` trait will be added to `std::marker` if it is accepted.

As example of how to implement `Project`, here is the implementation for `&T`.

```rust
impl<'a, F: Field> Project<F> for &'a F::Parent where F::Type: 'a {
    type Projection = &'a F::Type;

    fn project(self, field: F) -> Self {
        unsafe {
            // This is safe because a reference is always valids
            let ptr: *const F::Type = (self as *const F::Parent).project_unchecked(field);

            &*ptr
        }
    }
}
```

The raw pointer implementations will be done via intrinsics or by depending on `F::FieldDescriptor`. If it is done by intrinsics, then `F::FieldDescriptor` can be removed. All other implementations of `Project` must boil down to some raw pointer projection. The raw pointer projections that we will provide include `project_unchecked` and a `Project` impl. The `project_unchecked` will assume that the input raw pointer is valid (i.e. points to a valid instance of `T` given a raw pointer `*[const|mut] T`) and optimize around that. The project impl will make no such guarantee, and if the pointer is not valid, then the behaviour is implementation defined, and may change between editions (but not other smaller version changes).

For example of where `project_unchecked` would be UB.

```rust
struct Foo {
    bar: Bar
}

let x : *const Foo = 2usize as *const Foo;
let y : *const Bar = x.project_unchecked(Foo.bar); // UB, x does not point to a valid instance of Foo
```

With `Project` trait

```rust
use std::project::Project;

let z : *const Bar = x.project(Foo.bar); // not UB, but z's value will be implementation defined
```

If the raw pointer is valid, then the result of both `project_unchecked` and `Project::project` is a raw pointer to the given field.

The `Project` trait will be implemented for `*const T`, `*mut T`, `&T`, `&mut T`. Other smart pointers can get implementations later if they need them. We will also provide the following implementations to allow better documentation of intent

```rust
impl<'a, T> Pin<&'a T> {
    unsafe fn project_unchecked<F: Field<Parent = T>>(self, field: F) -> Pin<&'a F::Type> {
        self.map_unchecked(|slf| slf.project(field))
    }
}

impl<'a, T> Pin<&'a mut T> {
    unsafe fn project_unchecked<F: Field<Parent = T>>(self, field: F) -> Pin<&'a mut F::Type> {
        self.map_unchecked_mut(|slf| slf.project(field))
    }
}
```

If `PinProjectable` is accepted, then `Project` trait will also be implemented for `Pin<&T>`, `Pin<&mut T>` and will be bound by `PinProjectable`.

# Drawbacks
[drawbacks]: #drawbacks

Why should we *not* do this?
- This adds quite a bit of complexity and can increase compile times

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

- Behavior of `<*[const|mut] T as Project>::project` when the raw pointer is invalid (does not point to a valid T)
    - Can this behaviour change across editions? How about smaller version changes?
    - This issue blocks accepting this RFC
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