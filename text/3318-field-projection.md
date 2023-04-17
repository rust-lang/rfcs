- Feature Name: `field_projection`
- Start Date: 2022-09-10
- RFC PR: [rust-lang/rfcs#3318](https://github.com/rust-lang/rfcs/pull/3318)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Introduce ways to refer to fields of structs via the type system.

# Motivation
[motivation]: #motivation

Accessing field information is at the moment only possible for macros. Allowing the type system to also access some information about fields enables writing code that generalizes over fields.
One important application is field projection. Rust often employs the use of wrapper types, for example `Pin<P>`, `NonNull<T>`, `Cell<T>`, `UnsafeCell<T>`, `MaybeUninit<T>` and more. These types provide additional properties for the wrapped type and often also logically affect their fields. For example, if a struct is uninitialized, its fields are also uninitialized. Giving the type system access to field information allows creating safe projection functions.

Current projection functions cannot be safe, since they take a projection closure that might execute arbitrary code. They also cannot automatically uphold type invariants of the projected struct. A prime example is `Pin`, the projection functions are `unsafe` and accessing fields is natural and often required. This leads to code littered with `unsafe` projections:
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


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## Field Information

When defining a struct, the compiler automatically creates types for each field. This allows referencing fields via the type system. For example when we define the following struct:
```rust
struct Problem {
    info: String,
    count: usize,
}
```
Then the compiler creates two types, one for `info` and one for `count`. We cannot name these types normally, instead we use the `field_of!` macro:
```rust
type ProblemInfo<'a> = field_of!(Problem, info);
```
This field type also implements the `Field` trait that cannot be manually implemented. This trait provides some information about the field:
- which struct it belongs to,
- its own type,
- the offset at which the field can be found inside of the struct.

Since the trait cannot be implemented manually, you can be sure that a type implementing it actually refers to a field:
```rust
fn get_field<F: Field<Struct = Problem>>(problem: &Problem) -> &T::Type {
    let ptr: *const Problem = problem;
    // SAFETY: `F` implements the `Field` trait and thus we find `F::Type` at `F::OFFSET` inside
    // of `ptr` that was derived from a reference.
    unsafe { &*ptr.cast::<u8>().add(F::OFFSET).cast::<F::Type>() }
}
```
There are a lot more powerful things that one can do using this type. For example field projections can be expressed safely. If we are often working with memory that has to be accessed volatile, then we might write the following wrapper type:
```rust
/// A pointer to memory that enforces volatile access.
pub struct VolatileMem<T> {
    ptr: NonNull<T>,
}

impl<T: Copy> VolatileMem<T> {
    pub fn get(&self) -> T {
        // SAFETY: `ptr` is always valid for volatile reads.
        unsafe { ptr::read_volatile(self.ptr.as_ptr()) }
    }

    pub fn put(&mut self, val: T) {
        // SAFETY: `ptr` is always valid for volatile writes.
        unsafe { ptr::write_volatile(self.ptr.as_mut_ptr()) }
    }
}
```
Now consider the following struct that we would like to put into our `VolatileMem<T>`:
```rust
#[repr(C)]
pub struct Config {
    mode: u8,
    reserved: [u8; 128],
}
```
If we want to write a new config, then we always have to write the whole struct, including the `reserved` field that is comparatively big. We can avoid this by providing a field projection:
```rust
impl<T> VolatileMem<T> {
    pub fn map<F: Field<Struct = T>>(self) -> VolatileMem<F::Type> {
        Self {
            // SAFETY: `F` implements the `Field` trait and thus we find `F::Type` at `F::OFFSET`
            // inside of `ptr` that is always valid.
            ptr: unsafe {
                NonNull::new_unchecked(
                    self.ptr.as_ptr().cast::<u8>().add(F::OFFSET).cast::<F::Type>(),
                )
            },
        }
    }
}
```
Now in the scenario from above we can do:
```rust
let mut config: VolatileMem<Config> = ...;
config.put(Config::default());
let mut mode: VolatileMem<u8> = config.map::<field_of!(Config, mode)>();
mode.put(1);
```
And we will not have to always overwrite `reserved` with the same data.


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

For every field of every non-packed struct, the compiler creates a unique, unnameable type that represent that field. These generated types will:
- have meaningful names in error messages (e.g. `Struct::field`), 
- implement the `Field` with accurate associated types and constants.

The `Field` trait will reside in `core::marker` and is:
```rust
/// A type representing a field on a struct.
pub trait Field {
    /// The type of the struct containing this field.
    type Struct;
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

Users will be able to name this type by invoking the compiler built-in macro `field_of!` residing in `core`. This macro takes a struct type and an identifier/number for the accessed field:
```rust
macro_rules! field_of {
    ($struct:ty, $field:tt) => { /* compiler built-in */ }
}
```
Generics of the struct have to be specified and the field has to be accessible by the calling scope:
```rust
pub mod inner {
    pub struct Foo<T> {
        a: usize,
        pub b: T,
    }
    type Ty = field_of!(Foo, a); // Compile error: expected 1 generic argument 
    type Ty = field_of!(Foo<()>, a); // OK
    type Ty = field_of!(Foo::<()>, b); // OK
}
type Ty = field_of!(Foo<()>, a); // Compile error: private field
type Ty = field_of!(Foo<()>, b); // OK
type Ty<T> = field_of!(Foo<T>, b); // OK
```

# Drawbacks
[drawbacks]: #drawbacks

Adds additional complexity.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The presented approach is designed to be minimal and extendable. The `Field` trait can be extended and additional information such as the projection output can be added.

The `field_of!` macro avoids adding special syntax to refer to a field of a type and while it is not ergonomic, this can be changed by adding syntax later.


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

Java has reflection, which gives access to type information at runtime.

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

None.

# Future possibilities
[future-possibilities]: #future-possibilities

## Use closures to improve ergonomics

One has to spell out the projected type for every projection. Closures could be used to make use of type inference where possible. We introduce a new closure type in `core::marker`:
```rust
pub trait FieldClosure<T>: Fn<T> {
    type Field: Field<Struct = T>;
}
```
This trait is only implementable by the compiler. It is implemented for closures that
- do not capture variables
- only do a single field access on the singular parameter they have

Positive example: `|foo| foo.bar`
Negative examples:
- `|_| foo.bar`, captures `foo`
- `|foo| foo.bar.baz`, does two field accesses
- `|foo| foo.bar()`, calls a function
- `|foo| &mut foo.bar`, creates a reference to the field
- `|foo, bar| bar.baz`, takes two parameters

This trait makes calling a projection function a lot more ergonomic:
```rust
wrapper.project(|i| i.field)
// Instead of:
wrapper.project::<field_of!(Struct, field)>()
```

## Limited negative reasoning

There is the need to make the output type of `map` functions depend on properties of the projected field. In the case of `Pin`, this is whether the field is structurally pinned or not. If it is, then the return type should be `Pin<&mut F::Type>`, if it is not, then it should be `&mut F::Type` instead.

Negative reasoning would allow implementing the projection function with the correct type.


## Operator syntax

Introduce a `Project` trait and a binary operator that is syntactic sugar for `Project::project($left, |f| f.$right)`. This would make projections even more ergonomic:
```rust
wrapper->field
// Instead of:
wrapper.project(|i| i.field)
// or
wrapper.project::<field_of!(Struct, field)>()
```

## Support misaligned fields and `packed` structs

Create the `MaybeUnalignedField` trait as a supertrait of `Field` with the constant `WELL_ALIGNED: bool`. This trait is also automatically implemented by the compiler even for packed structs.

## `enum` and `union` support

Both enums and unions cannot be treated like structs, since some variants might not be currently valid. This makes these fundamentally incompatible with the code that this RFC tries to enable. They could be handled using similar traits, but these would not guarantee the same things. For example, union fields are always allowed to be uninitialized.

## Field macro attributes

To make things easier for implementing custom projections, we could create a new proc-macro kind that is placed on fields.
