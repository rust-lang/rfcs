- Feature Name: `field_projection`
- Start Date: 2022-09-10
- RFC PR: [rust-lang/rfcs#3318](https://github.com/rust-lang/rfcs/pull/3318)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

The std-lib has wrapper types that impose some restrictions/additional features
on the types that are wrapped. For example: `MaybeUninit<T>` allows `T` to be
partially initialized. These wrapper types also affect the fields of the types.
At the moment there is no easy access to these fields.
This RFC proposes to add field projection to certain wrapper types from the
std-lib:

|           from                                        |            to                                        |
|-------------------------------------------------------|------------------------------------------------------|
|`&`[`MaybeUninit`][maybeuninit]`<Struct>`              |`&`[`MaybeUninit`][maybeuninit]`<Field>`              |
|`&`[`Cell`][cell]`<Struct>`                            |`&`[`Cell`][cell]`<Field>`                            |
|`&`[`UnsafeCell`][unsafecell]`<Struct>`                |`&`[`UnsafeCell`][unsafecell]`<Field>`                |
|[`Pin`][pin]`<&Struct>`                                |[`Pin`][pin]`<&Field>`                                |
|[`Pin`][pin]`<&`[`MaybeUninit`][maybeuninit]`<Struct>>`|[`Pin`][pin]`<&`[`MaybeUninit`][maybeuninit]`<Field>>`|

Other pointers are also supported, for a list, see [here][supported-pointers].

Projection is facilitated by an operator and the syntax looks similar to normal field access. This
operator can be a new one (e.g. `~` or `->`) or it could be overloading `.`. This RFC will use `->`
as a placeholder for future bikeshedding.
```rust
struct MyStruct {
    foo: Foo,
    bar: usize,
}
struct Foo {
    count: usize,
}
```
when `mystruct` is of type `MyStruct`/`&mut MyStruct` field access works like this:

| expression              | type        |
|-------------------------|-------------|
|`mystruct.foo`           | `Foo`       |
|`&mystruct.foo`          | `&Foo`      |
|`&mut mystruct.foo.count`|`&mut usize` |

when `mystruct` is of type `&mut MaybeUninit<MyStruct>` this proposal allows this:

| expression              | type                     |
|-------------------------|--------------------------|
|`mystruct->foo`           | `MaybeUninit<Foo>`       |
|`mystruct->foo`          | `&MaybeUninit<Foo>`      |
|`mystruct->foo->count`|`&mut MaybeUninit<usize>` |


[maybeuninit]: https://doc.rust-lang.org/core/mem/union.MaybeUninit.html
[cell]: https://doc.rust-lang.org/core/cell/struct.Cell.html
[unsafecell]: https://doc.rust-lang.org/core/cell/struct.UnsafeCell.html
[option]: https://doc.rust-lang.org/core/option/enum.Option.html
[pin]: https://doc.rust-lang.org/core/pin/struct.Pin.html
[ref]: https://doc.rust-lang.org/core/cell/struct.Ref.html
[refmut]: https://doc.rust-lang.org/core/cell/struct.RefMut.html

# Motivation
[motivation]: #motivation
There are situations that necessitate heavy usage of wrapper types instead of their underlying
pointers. In the Linux kernel for example many types need to be pinned, because they contain self
referential datastructures. This results in `Pin` being present almost everywhere. Thus pin
projections are required instead of normal accesses.

Currently, there are mapping functions that provide this functionality. These
functions are not as ergonomic as a normal field access would be. Accessing the fields can also be a
totally safe operation, but the wrapper mapping functions need to be marked `unsafe`. This results
in poor API ergonomics:
```rust
struct Count {
    inner: usize,
    outer: usize,
}
fn init_count(mut count: Box<MaybeUninit<Count>>) -> Box<Count> {
    let inner: &mut MaybeUninit<usize> =
        unsafe { &mut *addr_of_mut!((*count.as_mut_ptr()).inner).cast::<MaybeUninit<usize>>() };
    inner.write(42);
    unsafe { &mut *addr_of_mut!((*count.as_mut_ptr()).outer).cast::<MaybeUninit<usize>>() }.write(63);
    unsafe {
        // SAFETY: all fields have been initialized
        count.assume_init() // #![feature(new_uninit)]
    }
}
```
Using the proposal from this RFC, the code simplifies to this:
```rust
struct Count {
    inner: usize,
    outer: usize,
}
fn init_count(mut count: Box<MaybeUninit<Count>>) -> Box<Count> {
    let inner: &mut MaybeUninit<usize> = count->inner;
    inner.write(42);
    count->outer.write(63);
    unsafe {
        // SAFETY: all fields have been initialized
        count.assume_init() // #![feature(new_uninit)]
    }
}
```
[`Pin`][pin]`<P>` has a similar story:
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

    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        match unsafe { self.map_unchecked_mut(|t| &mut t.fut1) }.poll(ctx) {
            Poll::Pending => unsafe { self.map_unchecked_mut(|t| &mut t.fut2) }.poll(ctx),
            rdy => rdy,
        }
    }
}
```
It gets a lot simpler:
```rust
struct RaceFutures<F1, F2> {
    // Pin is somewhat special, it needs some way to specify
    // structurally pinned fields, because `Pin<&mut T>` might
    // not affect the whole of `T`.
    #[pin]
    fut1: F1,
    #[pin]
    fut2: F2,
}
impl<F1, F2> Future for RaceFutures<F1, F2>
where
    F1: Future,
    F2: Future<Output = F1::Output>,
{
    type Output = F1::Output;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        match self->fut1.poll(ctx) {
            Poll::Pending => self->fut2.poll(ctx),
            rdy => rdy,
        }
    }
}
```
It is the most important goal of this RFC to make field projection an ergonomic and safe operation.

While there exist macro solutions like `pin-project` and `pin-project-lite`, there are situations
where developers want to avoid the use of 3rd party libraries. In the case of the Rust-for-Linux
project, any proc macro solutions with `syn` is problematic. At the moment (as of 28.11.2022),
there are in total 10k lines of Rust code in the Linux kernel. Compare that with the over 50k lines
that `syn` has. It is very difficult to vendor all of this into the kernel any time soon.
`pin-project-lite` does not have this dependency problem. However, the macro itself is very
convoluted and difficult to understand. When this is proposed to the Linux maintainers, they have to
be able to understand the code, or at the very least comprehend the problem and the solution.
This is why a language level solution that additionally future proofs any needs for projections is
necessary.

Additionally this RFC allows custom projections, these could be used in the Linux kernel to improve
interactions of RCU with other locking mechanisms.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## [`MaybeUninit`][maybeuninit]`<T>`

When working with certain wrapper types in rust, one often want to access fields
of the wrapped types. When interfacing with C one often has to deal with
uninitialized data. In Rust, uninitialized data is represented by
[`MaybeUninit`][maybeuninit]`<T>`. In the following example we demonstrate
how one can initialize fields using [`MaybeUninit`][maybeuninit]`<T>`.
```rust
#[repr(C)]
pub struct MachineData {
    incident_count: u32,
    device_id: usize,
    device_specific: *const core::ffi::c_void,
}

extern "C" {
    // provided by the C code
    /// Initializes the `device_specific` pointer based on the value of `device_id`.
    /// Returns -1 on error (unknown id) and 0 on success.
    fn lookup_device_ptr(data: *mut MachineData) -> i32;
}

pub struct UnknownId;

impl MachineData {
    pub fn new(id: usize) -> Result<Self, UnknownId> {
        let mut this = MaybeUninit::<Self>::uninit();
        this->device_id.write(id);
        this->incident_count.write(0);
        // SAFETY: ffi-call, `device_id` has been initialized
        if unsafe { lookup_device_ptr(this.as_mut_ptr()) } != 0 {
            Err(UnknownId)
        } else {
            // SAFETY: all fields have been initialized
            Ok(unsafe { this.assume_init() })
        }
    }
}
```
So to access a field of [`MaybeUninit`][maybeuninit]`<MachineData>` we can use similar syntax to
accessing a field of `MachineData`. The difference is that the type of the expression
`this->device_id` is now [`MaybeUninit`][maybeuninit]`<usize>`.

These *field projections* are also available on other types.

## [`Pin`][pin]`<P>` projections

Our second example is going to focus on [`Pin`][pin]`<P>`. This type is a little
special, as it allows unwrapping while projecting, but only for specific fields.
This information is expressed via the `#[pin]` attribute on each structurally pinned field.
Untagged fields are *unwrapped* when projected. So the projection is `Pin<&mut Struct> -> &mut
Field`.
```rust
use core::pin::pin;
struct RaceFutures<F1, F2> {
    #[pin]
    fut1: F1,
    #[pin]
    fut2: F2,
    // this will be used to fairly poll the futures
    first: bool,
}
impl<F1, F2> Future for RaceFutures<F1, F2>
where
    F1: Future,
    F2: Future<Output = F1::Output>,
{
    type Output = F1::Output;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        // we can access self.first mutably, because it is not `#[pin]`
        *self->first = !*self->first;
        if *self->first {
            // `self->fut1` has the type `Pin<&mut F1>` because `fut1` is a pinned field.
            // if it was not pinned, the type would be `&mut F1`.
            match self->fut1.poll(ctx) {
                Poll::Pending => self->fut2.poll(ctx),
                rdy => rdy,
            }
        } else {
            match self->fut2.poll(ctx) {
                Poll::Pending => self->fut1.poll(ctx),
                rdy => rdy,
            }
        }
    }
}
```

## Defining your own wrapper type

If you want to add field projection to a wrapper type, you will need to implement the `Project`
trait for the type that you want to project:
```rust
#[repr(transparent)]
pub struct Wrapper<T>(T);

impl<'a, T> Project<'a> for &'a mut Wrapper<T>
where
    T: HasFields // this trait ensures that T actually has fields to project.
{
    // This type is the type that will be projected.
    type Inner = T;
    // This is the type resulting from a projection to a field of type U.
    type Output<U: 'a> = &'a mut Wrapper<U>;
    // This is the type resulting from unwrapping a field of type U.
    type Unwrap<U: 'a> = &'a mut U;

    unsafe fn project_true<U, const N: usize>(self, field: Field<Self::Inner, U, N>) -> Self::Output<U> {
        // SAFETY: because Wrapper is repr(transparent), we can do the cast
        // and because of field's invariants this results in the correct field pointer.
        unsafe {
            // we can project `*mut T` to `*mut U` using `project` (this is provided by the
            // `Project` trait).
            &mut *(&mut self.0 as *mut T).project(field).cast::<Wrapper<U>>()
        }
    }

    unsafe fn unwrap_true<U, const N: usize>(self, field: Field<Self::Inner, U, N>) -> Self::Unwrap<U> {
        // SAFETY: because Wrapper is repr(transparent), we can do the cast
        // and because of field's invariants this results in the correct field pointer.
        unsafe {
            // we can project `*mut T` to `*mut U` using `project` (this is provided by the
            // `Project` trait).
            &mut *(&mut self.0 as *mut T).project(field)
        }
    }
}
```
Now anyone can project `&mut Wrapper<Struct> -> &mut Wrapper<MyField>` via `struct->field` if the
field is visible. Additionally the field needs to be projectable. This is expressed by the
`Projectable` trait. So the projection is only allowed, if `Field<Struct, MyField, N>: Projectable<&mut Wrapper<Struct>>`.
`N` is a compiler generated identifier of the specific field. This implementation needs to be
provided by the author of `Struct`.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The following types will be added to `core`:
```rust
pub unsafe trait Project<'a>: 'a + Sized {
    type Inner: 'a + HasFields;
    type Projected<U: 'a>: 'a
    where
        Self: 'a;
    type Unwrapped<U: 'a>: 'a
    where
        Self: 'a;

    fn project<U: 'a, const N: usize>(
        self,
        field: Field<Self::Inner, U, N>,
    ) -> <<Field<Self::Inner, U, N> as Projectable<'a, Self>>::ProjKind as ProjSelector<'a, Self>>::Output<U>
    where
        Field<Self::Inner, U, N>: Projectable<'a, Self>,
        <Field<Self::Inner, U, N> as Projectable<'a, Self>>::ProjKind: ProjSelector<'a, Self>,
    {
        unsafe {
            <Field<Self::Inner, U, N> as Projectable<'a, Self>>::ProjKind::select_proj(self, field)
        }
    }

    unsafe fn project_field<U: 'a, const N: usize>(
        self,
        field: Field<Self::Inner, U, N>,
    ) -> Self::Projected<U>
    where
        Field<Self::Inner, U, N>: Projectable<'a, Self>;

    unsafe fn unwrap_field<U: 'a, const N: usize>(
        self,
        field: Field<Self::Inner, U, N>,
    ) -> Self::Unwrapped<U>
    where
        Field<Self::Inner, U, N>: Projectable<'a, Self>;
}

pub struct Field<T: HasFields, U, const N: usize> {
    offset: usize,
    phantom: PhantomData<fn(T, U) -> (T, U)>,
}

impl<T: HasFields, U, const N: usize> Field<T, U, N> {
    pub const unsafe fn new(offset: usize) -> Self {
        Self {
            offset,
            phantom: PhantomData,
        }
    }

    pub fn offset(&self) -> usize {
        self.offset
    }
}

pub struct Projected;
pub struct Unwrapped;

mod sealed {
    pub unsafe trait IsField {}
    unsafe impl<T: super::HasFields, U, const N: usize> IsField for super::Field<T, U, N> {}

    pub unsafe trait IsProjKind {}
    unsafe impl IsProjKind for super::Projected {}
    unsafe impl IsProjKind for super::Unwrapped {}
}

pub unsafe trait ProjSelector<'a, P: Project<'a>>: sealed::IsProjKind {
    type Output<U: 'a>: 'a
    where
        Self: 'a;
    unsafe fn select_proj<U, const N: usize>(
        proj: P,
        field: Field<P::Inner, U, N>,
    ) -> Self::Output<U>
    where
        Field<P::Inner, U, N>: Projectable<'a, P>;
}

unsafe impl<'a, P: Project<'a>> ProjSelector<'a, P> for Projected {
    type Output<U: 'a> = P::Projected<U>;
    unsafe fn select_proj<U, const N: usize>(
        proj: P,
        field: Field<P::Inner, U, N>,
    ) -> Self::Output<U>
    where
        Field<P::Inner, U, N>: Projectable<'a, P>,
    {
        P::project_field(proj, field)
    }
}

unsafe impl<'a, P: Project<'a>> ProjSelector<'a, P> for Unwrapped {
    type Output<U: 'a> = P::Unwrapped<U>;
    unsafe fn select_proj<U, const N: usize>(
        proj: P,
        field: Field<P::Inner, U, N>,
    ) -> Self::Output<U>
    where
        Field<P::Inner, U, N>: Projectable<'a, P>,
    {
        P::unwrap_field(proj, field)
    }
}

pub trait Projectable<'a, P: Project<'a>>: sealed::IsField {
    type ProjKind: sealed::IsProjKind;
}
```
This design is very flexible for authors writing wrapper types. There is almost no restriction on
what can be projected. And this design includes direct support for unwrapping.

Here is the list of types from `core` that will have a projection implementation:

- `&mut T`, `&T`
- `*mut T`, `*const T`
- [`&mut MaybeUninit<T>`][maybeuninit]
- [`&Cell<T>`][cell], [`&UnsafeCell<T>`][unsafecell]
- [`Pin<&mut T>`][pin], `Pin<&mut MaybeUninit<T>>`

## Pin projections
[pin-projections-section]: #pin-projections

Because [`Pin<P>`][pin] permits unwrapping when the user specifies so. There are multiple ways to
implement this:

- nothing, users can add their own implementations.
- introduce an attribute `#[pin]` and/or `#[unpin]` on fields that automatically add the respective
    implementation.
- a derive/attribute/function-like macro that adds the implementations.

This RFC is going to use `#[pin]` in the next section to refer to the method of marking a field
pin-projected.

### `PinnedDrop`

An additional challenge is that if a `!Unpin` field is marked `#[pin]`, then
one cannot implement the normal `Drop` trait on the struct, as it would give access to
`&mut self` even if `self` is pinned. Before this did not pose a problem, because
users would have to use `unsafe` to project `!Unpin` fields. But as this
proposal makes this possible, we have to account for this.

The solution is similar to how [pin-project] solves this issue: Users are not
allowed to implement `Drop` manually, but instead can implement `PinnedDrop`:
```rust
pub trait PinnedDrop {
    fn drop(self: Pin<&mut Self>);
}
```
similar to `Drop::drop`, `PinnedDrop::drop` would not be callable by normal code.
The compiler would emit the following `Drop` stub for types that had `#[pin]`ned
fields and a user specified `PinnedDrop` impl:
```rust
impl Drop for $ty {
    fn drop(&mut self) {
        // SAFETY: because `self` is being dropped, there exists no other reference
        // to it. Thus it will never move, if this function never moves it.
        let this = unsafe { ::core::pin::Pin::new_unchecked(self) };
        <Self as ::core::ops::PinnedDrop>::drop(this)
    }
}
```

An error is emitted if a `Drop` impl is found when at least one field is marked `#[pin]`:
```rust
struct MyStruct {
    buf: [u8; 64],
    ptr: *const u8,
    #[pin]
    _pin: PhantomPinned,
}

impl Drop for MyStruct {
//   ^^^^ error: cannot implement `Drop` for `MyStruct` because it has pinned fields.
//   help: implement `PinnedDrop` instead
    fn drop(&mut self) {
        println!("Dropping MyStruct");
    }
}
```

`PinnedDrop` would also reside in `core::ops` and should be added to the prelude in the next
edition.

# Drawbacks
[drawbacks]: #drawbacks

- Users currently relying on crates that facilitate field projections (see
[prior art][prior-art]) will have to refactor their code.
- Increased compiler complexity:
    - longer compile times
    - potential worse type inference
    - `Pin` projection support might be confusing to new users


# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This RFC consciously chose the presented design, because it addresses the
following core issues:
- ergonomic field projection for a wide variety of types with user accessible
  ways of implementing it for their own types.


## Create a field projection operator

In [Gankra's blog post][faultlore] about overhauling raw pointers she talks about using `~` as field
projection for raw pointers.

We could implement field projection for raw pointers with the current approach, but that would
result in `x.y`. If we instead adopt Gankra's idea for field projection in general, it would also
more clearly convey the intent.

### Advantages:

- we can imbue `~` with new meaning, users will not assume anything about what the operator does
  from other languages
- it clearly differentiates it from normal field access (and will not have auto-deref like `.`)

### Disadvantages:

- `.` is less confusing to beginners compared to `~`. Other languages use it primarily as a unary
  binary negation operator
- migrating from `pin-project` is going to be a *lot* tougher. users will have to change every
  access via `.` to `~` compared to just having to remove `.project`.


## What other designs have been considered and what is the rationale for not choosing them?

This proposal was initially only designed to enable projecting
[`Pin`][pin]`<&mut T>`, because that would remove the need for `unsafe` when
pin projecting.

It seems beneficial to also provide this functionality for a wider range of types.

### Using solely `#[pin]`/`#[unpin]` for specifying structurally pinned fields

Because it makes existing code unsound this option has not been chosen.

## What is the impact of not doing this?

Users of these wrapper types need to rely on crates listed in [prior art][prior-art]
to provide sensible projections. Otherwise they can use the mapping functions
provided by some of the wrapper types. These are however, rather unergonomic
and wrappers like [`Pin`][pin]`<P>` require `unsafe`.

# Prior art
[prior-art]: #prior-art

## Crates

There are some crates that enable field projections via (proc-)macros:

- [pin-project] provides pin projections via a proc macro on the type specifying
the structurally pinned fields. At the projection-site the user calls a projection
function `.project()` and then receives a type with each field replaced with
the respective projected field.
- [field-project] provides pin/uninit projection via a macro at the projection-site:
the user writes `proj!($var.$field)` to project to `$field`. It works by
internally using `unsafe` and thus cannot pin-project `!Unpin` fields, because
that would be unsound due to the `Drop` impl a user could write.
- [cell-project] provides cell projection via a macro at the projection-site:
the user writes `cell_project!($ty, $val.$field)` where `$ty` is the type of `$val`.
Internally, it uses unsafe to facilitate the projection.
- [pin-projections] provides pin projections, it differs from [pin-project] by
providing explicit projection functions for each field. It also can generate
other types of getters for fields. [pin-project] seems like a more mature solution.
- [project-uninit] provides uninit projections via macros at the projection-site
uses `unsafe` internally.

All of these crates have in common that their users have to use macros
when they want to perform a field projection.

## Other languages

Other languages generally do not have this feature in the same extend. C++ has
`shared_ptr` which allows the creation of another `shared_ptr` pointing at a field of
a `shared_ptr`'s pointee. This is possible, because `shared_ptr` is made up of
two pointers, one pointing to the data and another pointing at the ref count.
While this is not possible to add to `Arc` without introducing a new field, it
could be possible to add another `Arc` pointer that allowed field projections.
See [the future possibilities section][arc-projection] for more. 

## RFCs

- [`ptr-to-field`](https://github.com/rust-lang/rfcs/pull/2708)


## Further discussion
- https://internals.rust-lang.org/t/cell-references-and-struct-layout/11564

[pin-project]: https://crates.io/crates/pin-project
[field-project]: https://crates.io/crates/field-project
[cell-project]: https://crates.io/crates/cell-project
[pin-projections]: https://crates.io/crates/pin-projections
[project-uninit]: https://crates.io/crates/project-uninit

# Unresolved questions
[unresolved-questions]: #unresolved-questions

You can find the direction I am currently biased towards at the end of each Question. `(Y) = Yes`,
`(N) = No`, `(-) = no bias`. Questions that have been crossed out are no longer relevant/have been
answered with "No".

- [ ] should the warning when using `#[pin]` on an `Unpin` field be an error? `(Y)`
- [ ] the current proposal requires explicit reborrowing, so `pinned.as_mut()->field`. Because it
    would otherwise move `pinned`. Which might be used later.
- [ ] how should the `const N: usize` parameter of `Field<T, U, N>` be calculated and exposed to the
    user? There needs to be a way to refer to `Field<T, U, N>` using only the base type `T` and the
    name of the field.
- [ ] `HasFields` needs to be implemented automatically, how should this be done?

# Future possibilities
[future-possibilities]: #future-possibilities

## Types that could benefit from projections

- [`Option`][option], allowing projecting from `Option<&mut Struct>` to `Option<&mut Field>`
- [`PhantomData`], it could allow macros to better access the type of a field.

## Arrays

Even more generalized projections e.g. slices: At the moment

- [`as_array_of_cells`](https://doc.rust-lang.org/core/cell/struct.Cell.html#method.as_array_of_cells)
- [`as_slice_of_cells`](https://doc.rust-lang.org/core/cell/struct.Cell.html#method.as_slice_of_cells)

exist, maybe there is room for generalization there as well.

## `Arc` projection
[arc-projection]: #arc-projection

With the current design of `Arc` it is not possible to add field projection,
because the ref-count lives directly adjacent to the data. Instead the std-lib could
include a new type of `Arc` (or `ProjectedArc<Field, Struct>`) that allows
projection via a `map` function:
```rust
pub struct ProjectedArc<T, S> {
    backing: Arc<S>,
    ptr: NonNull<T>,
}

impl<T> Arc<T> {
    pub fn project<U>(&self, map: impl FnOnce(&T) -> &U) -> ProjectedArc<U, T> {
        ProjectedArc {
            backing: self.clone(),
            ptr: NonNull::from(map(&**self)),
        }
    }
}
```

## `enum` and `union` support

When destructuring an enum, the discriminant needs to be read. The `MaybeUninit` wrapper type makes
this impossible, as it permits uninitialized data, making the read UB. There could be an unsafe way
of projecting the enum, with the assumption that the discriminant is initialized.

Unions are probably simpler to implement, as they are much more similar to structs compared to
enums. But this is also left to a future RFC.

[`Rc`]: https://doc.rust-lang.org/alloc/sync/struct.Rc.html
[`Arc`]: https://doc.rust-lang.org/alloc/sync/struct.Arc.html
[`PhantomData`]: https://doc.rust-lang.org/core/marker/struct.PhantomData.html
[faultlore]: https://faultlore.com/blah/fix-rust-pointers/
