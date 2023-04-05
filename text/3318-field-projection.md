- Feature Name: `field_projection`
- Start Date: 2022-09-10
- RFC PR: [rust-lang/rfcs#3318](https://github.com/rust-lang/rfcs/pull/3318)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Rust often employs the use of wrapper types, for example `Pin<P>`, `NonNull<T>`, `Cell<T>`, `UnsafeCell<T>`, `MaybeUninit<T>` and more. These types provide additional properties for the wrapped type and often also logically affect their fields. For example, if a struct is uninitialized, its fields are also uninitialized. This RFC introduces architecture to make it possible to provide safe projections backed by the type system.

# Motivation
[motivation]: #motivation

Some wrapper types provide projection functions, but these are not ergonomic. They also cannot automatically uphold type invariants of the projected struct. The prime example is `Pin`, the projection functions are `unsafe` and accessing fields is natural and often required. This leads to code littered with `unsafe` projections:
```rust
struct RaceFutures<F1, F2> {
    fut1: F1,
    fut2: F2,
}

impl<F1, F2> Future for RaceFutures<F1, F2>
where
    F1: Future,
    F2: Future<Output = F1::Output>,
{
    type Output = F1::Output;

    fn poll(mut self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        match unsafe { self.as_mut().map_unchecked_mut(|t| &mut t.fut1) }.poll(ctx) {
            Poll::Pending => {
                unsafe { self.map_unchecked_mut(|t| &mut t.fut2) }.poll(ctx)
            }
            rdy => rdy,
        }
    }
}
```
Since the supplied closures are only allowed to do field projections, it would be natural to add `SAFETY` comments, but that gets even more tedious.

Other types would also greatly benefit from projections, for example raw pointers, since projection could be based on `wrapping_add` and they would not dereference anything. It would reduce syntactic clutter of `(*ptr).field`.

Cell types like `Cell`, `UnsafeCell` would similarly enjoy additional ergonomics, since they also propagate their properties to the fields of structs.


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This section will be created when a solution has been agreed upon.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Core design

For every field of every struct, the compiler creates unnameable unit structs that represent that field. These types will have a meaningful name in error messages (e.g. `Struct::field`). These types implement the `Field` trait holding information about the type of the field and its offset within the struct:
```rust
// in core::marker:
/// A type representing a field on a struct.
pub trait Field {
    /// The type of the struct containing this field.
    type Base;
    /// The type of this field.
    type Type;
    /// The offset of this field from the beginning of the `Base` struct in bytes.
    const OFFSET: usize;
}
```
This trait cannot be implemented manually and users are allowed to rely on the associated types/constants to be correct. For example the following code is allowed:
```rust
fn project<F: Field>(base: &F::Base) -> &F::Type {
    let ptr: *const Base = base;
    let ptr: *const u8 = base.cast::<u8>();
    // SAFETY: `ptr` is derived from a reference and the `Field` trait is guaranteed to contain
    // correct values. So `F::OFFSET` is still within the `F::Base` type.
    let ptr: *const u8 = unsafe { ptr.add(F::OFFSET) };
    let ptr: *const F::Type = ptr.cast::<F::Type>();
    // SAFETY: The `Field` trait guarantees that at `F::OFFSET` we find a field of type `F::Type`.
    unsafe { &*ptr }
}
```

Importantly, the `Field` trait should only be implemented on non-`packed` structs, since otherwise the above code would not be sound.

Users will be able to name this type by invoking the compiler built-in macro `field_of!` that takes a struct and an identifier/number for the accessed field:
```rust
// in core:
macro_rules! field_of {
    ($struct:ty, $field:tt) => { /* compiler built-in */ }
}
```
## Improving ergonomics 1: Closures

For improving the ergonomics of getting a field of a specific type, we could leverage specially marked closures:
```rust
// in core::marker:
pub trait FieldClosure<T>: Fn<T> {
    type Field: Field<Base = T>;
}
```
This trait is also only implementable by the compiler. It is implemented for closures that
- do not capture variables
- only do a single field access on the singular parameter they have

Positive example: `|foo| foo.bar`
Negative examples:
- `|_| foo.bar`, captures `foo`
- `|foo| foo.bar.baz`, does two field accesses
- `|foo| foo.bar()`, calls a function
- `|foo| &mut foo.bar`, creates a reference to the field
- `|foo, bar| bar.baz`, takes two parameters

With this trait one could write a function like this:
```rust
pub unsafe fn map<T, F>(pin: Pin<&mut T>, f: F) -> Pin<&mut <F::Field as Field>::Type>
where
    F: FieldClosure<T>,
{
    /* do the offsetting */
}
```

To make this function safe, we require the feature discussed in the next section.

## Limited negative reasoning

There is the need to make the output type of the `map` function above depend on a property of the field. In the case of `Pin`, this is whether the field is structurally pinned or not. If it is, then the return type should be as declared above, if it is not, then it should be `&mut <F::Field as Field>::Type` instead.

A way this could be expressed is by allowing some negative reasoning. Here is the solution discussed on the example of `Pin`:
```rust
// First we create a marker trait to differ structurally pinned fields:
pub trait StructurallyPinnedField: Field {}
// This trait needs to then be implemented for every field that should be structurally pinned.
// Since this is user-decideable at the struct definition, this could be done with a proc-macro akin to `pin-project`.

impl<T> Pin<&mut T> {
    pub fn map<F: FieldClosure<T>>(self, f: F) -> Pin<&mut <F::Field as Field>::Type>
    where
        F::Field: StructurallyPinnedField,
    { /* do the offsetting */ }

    pub fn map<F: FieldClosure<T>>(self, f: F) -> &mut <F::Field as Field>::Type
    where
        F::Field: !StructurallyPinnedField,
    { /* do the offsetting */ }
}
```
The compiler would need to be able to prove that these two functions do not overlap for this to work.

A variation of this feature is `xor` traits, where a type is only ever allowed to implement one from the given set. It could achieve the same thing, while being more flexible when one wants to define more than two different user-specified projections.

## Alternative to negative reasoning

One alternative is to also create an extension trait of `Field`, but then rely on the proc-macro to specify the correct projection-output type in an associated type:
```rust
// We have to mark it `unsafe`, because the `ProjOutput` type could be wrongly specified.
pub unsafe trait PinProjectableField: Field {
    type ProjOutput<'a>; // this is either `&'a mut Self::Type` or `Pin<&'a mut Self::Type>`.
}

impl<'a, T> Pin<&'a, mut T> {
    pub fn map<F: FieldClosure<T>>(self, f: F) -> <F::Field as PinProjectableField>::ProjOutput<'a>
    where
        F::Field: PinProjectableField,
    { /* do the offsetting */ }
}
```
This approach is used by [the field projection example from Gary](https://github.com/nbdd0121/field-projection/).

A big issue that this approach has is that the `map` function cannot have behavior depending on the projection output. This results in practice, that `mem::transmute_copy` has to be used, since the compiler cannot prove that `ProjOutput` always has the same size.

# Drawbacks
[drawbacks]: #drawbacks

Adds considerable complexity.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Why specify projections via proc-macros?

In the design meeting the following alternative was brought up:
```rust
pub struct NotPin<T>(pub T);

impl<T> Unpin for NotPin<T> {}
```
Instead of marking fields with `#[pin]`, they should wrap not structurally pinned fields with the `NotPin` struct. The `map` function of `Pin` would then use `!Unpin`/`Unpin` instead of the `StructurallyPinnedField` field used here.

The problem with this solution is that now users always get reminded that the field is not structurally pinned. They have to wrap a value when they want to assign it and they have to unwrap it when they want to read it. This property is also not something for the type system, rather it is a property of that specific field of the struct.


# Prior art
[prior-art]: #prior-art

## Crates

There are some crates that enable field projections via (proc-)macros:

- [pin-project] provides pin projections via a proc macro on the type specifying the structurally pinned fields. At the projection-site the user calls a projection function `.project()` and then receives a type with each field replaced with the respective projected field.
- [field-project] provides pin/uninit projection via a macro at the projection-site: the user writes `proj!($var.$field)` to project to `$field`. It works by internally using `unsafe` and thus cannot pin-project `!Unpin` fields, because that would be unsound due to the `Drop` impl a user could write.
- [cell-project] provides cell projection via a macro at the projection-site: the user writes `cell_project!($ty, $val.$field)` where `$ty` is the type of `$val`. Internally, it uses unsafe to facilitate the projection.
- [pin-projections] provides pin projections, it differs from [pin-project] by providing explicit projection functions for each field. It also can generate other types of getters for fields. [pin-project] seems like a more mature solution.
- [project-uninit] provides uninit projections via macros at the projection-site uses `unsafe` internally.
- [field-projection] is an experimental crate that implements general field projections via a proc-macro that hashes the name of the field to create unique types for each field that can then implement traits to make different output types for projections.

## Other languages

Other languages generally do not have this feature in the same extend. C++ has `shared_ptr` which allows the creation of another `shared_ptr` pointing at a field of a `shared_ptr`'s pointee. This is possible, because `shared_ptr` is made up of two pointers, one pointing to the data and another pointing at the ref count. While this is not possible to add to `Arc` without introducing a new field, it could be possible to add another `Arc` pointer that allowed field projections. See [the future possibilities section][arc-projection] for more.

## RFCs
- [`ptr-to-field`](https://github.com/rust-lang/rfcs/pull/2708)

## Further discussion
- https://internals.rust-lang.org/t/cell-references-and-struct-layout/11564

[pin-project]: https://crates.io/crates/pin-project
[field-project]: https://crates.io/crates/field-project
[cell-project]: https://crates.io/crates/cell-project
[pin-projections]: https://crates.io/crates/pin-projections
[project-uninit]: https://crates.io/crates/project-uninit
[field-projection]: https://crates.io/crates/field-projection

# Unresolved questions
[unresolved-questions]: #unresolved-questions

The whole design, please look at the PR to see the currently open questions.

# Future possibilities
[future-possibilities]: #future-possibilities

## Operator syntax

Introduce a `Project` trait and a binary operator that is syntactic sugar for `Project::project($left, |f| f.$right)`.

## Project multiple field at once

Introduce a `FieldChainClosure` trait that is implemented for closures that contain only a field access chain `|foo| foo.bar.baz`. The chain should not contain `Box`es or other types with `deref`s, since then we could be leaving the allocation of the base struct.

## Support misaligned fields and `packed` structs

Create the `MaybeUnalignedField` trait that also has a constant `WELL_ALIGNED: bool`. This trait is also automatically implemented by the compiler even for packed structs.

## `enum` and `union` support

Both enums and unions cannot be treated like structs, since some variants might not be currently valid. This makes these fundamentally incompatible with the code that this RFC tries to enable. They could be handled using similar traits, but these would not guarantee the same things. For example, union fields are always allowed to be uninitialized.

## Field marco attributes

To make things easier for implementing custom projections, we could create a new proc-macro kind that is placed on fields.
