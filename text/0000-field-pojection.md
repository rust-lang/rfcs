- Feature Name: `field_projection`
- Start Date: 2022-09-10
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

The stdlib has wrapper types that impose some restrictions/additional features
on the types that are wrapped. For example: `MaybeUninit<T>` allows `T` to be
partially initialized. These wrapper types also affect the fields of the types.
At the moment there is no easy access to these fields.
This RFC proposes to add field projection to certain wrapper types from the
stdlib:

|           from                                        |            to                                        |
|-------------------------------------------------------|------------------------------------------------------|
|`&`[`MaybeUninit`][maybeuninit]`<Struct>`              |`&`[`MaybeUninit`][maybeuninit]`<Field>`              |
|`&`[`Cell`][cell]`<Struct>`                            |`&`[`Cell`][cell]`<Field>`                            |
|`&`[`UnsafeCell`][unsafecell]`<Struct>`                |`&`[`UnsafeCell`][unsafecell]`<Field>`                |
|[`Option`][option]`<&Struct>`                          |[`Option`][option]`<&Field>`                          |
|[`Pin`][pin]`<&Struct>`                                |[`Pin`][pin]`<&Field>`                                |
|[`Pin`][pin]`<&`[`MaybeUninit`][maybeuninit]`<Struct>>`|[`Pin`][pin]`<&`[`MaybeUninit`][maybeuninit]`<Field>>`|
|[`Ref`][ref]`<'_, Struct>`                             |[`Ref`][ref]`<'_, Field>`                             |

As well as their mutable versions:

|           from                                            |            to                                            |
|-----------------------------------------------------------|----------------------------------------------------------|
|`&mut` [`MaybeUninit`][maybeuninit]`<Struct>`              |`&mut` [`MaybeUninit`][maybeuninit]`<Field>`              |
|`&mut` [`Cell`][cell]`<Struct>`                            |`&mut` [`Cell`][cell]`<Field>`                            |
|`&mut` [`UnsafeCell`][unsafecell]`<Struct>`                |`&mut` [`UnsafeCell`][unsafecell]`<Field>`                |
|[`Option`][option]`<&mut Struct>`                          |[`Option`][option]`<&mut Field>`                          |
|[`Pin`][pin]`<&mut Struct>`                                |[`Pin`][pin]`<&mut Field>`                                |
|[`Pin`][pin]`<&mut` [`MaybeUninit`][maybeuninit]`<Struct>>`|[`Pin`][pin]`<&mut` [`MaybeUninit`][maybeuninit]`<Field>>`|
|[`RefMut`][refmut]`<'_, Struct>`                           |[`RefMut`][refmut]`<'_, Field>`                           |

[maybeuninit]: https://doc.rust-lang.org/core/mem/union.MaybeUninit.html
[cell]: https://doc.rust-lang.org/core/cell/struct.Cell.html
[unsafecell]: https://doc.rust-lang.org/core/cell/struct.UnsafeCell.html
[option]: https://doc.rust-lang.org/core/option/enum.Option.html
[pin]: https://doc.rust-lang.org/core/pin/struct.Pin.html
[ref]: https://doc.rust-lang.org/core/cell/struct.Ref.html
[refmut]: https://doc.rust-lang.org/core/cell/struct.RefMut.html

# Motivation
[motivation]: #motivation

Currently, there are some map functions that provide this functionality. These
functions are not as ergonomic as a normal field access would be:
```rust
struct Count {
    inner: usize,
    outer: usize,
}
fn do_stuff(debug: Option<&mut Count>) {
    // something that will be tracked by inner
    if let Some(inner) = debug.map(|c| &mut c.inner) {
        *inner += 1;
    }
    // something that will be tracked by outer
    if let Some(outer) = debug.map(|c| &mut c.outer) {
        *inner += 1;
    }
}
```
With this RFC this would become:
```rust
struct Count {
    inner: usize,
    outer: usize,
}
fn do_stuff(debug: Option<&mut Count>) {
    // something that will be tracked by inner
    if let Some(inner) = &mut debug.inner {
        *inner += 1;
    }
    // something that will be tracked by outer
    if let Some(outer) = &mut debug.outer {
        *inner += 1;
    }
}
```
While this might only seem like a minor improvement for [`Option`][option]`<T>`
it is transformative for [`Pin`][pin]`<P>` and
[`MaybeUninit`][maybeuninit]`<T>`:
```rust
struct Count {
    inner: usize,
    outer: usize,
}
fn init_count(mut count: Box<MaybeUninit<Count>>) -> Box<Count> {
    let inner: &mut MaybeUninit<usize> = count.inner;
    inner.write(42);
    count.outer.write(63);
    unsafe {
        // SAFETY: all fields have been initialized
        count.assume_init() // #![feature(new_uninit)]
    }
}
```
Before, this had to be done with raw pointers!
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
        match self.fut1.poll(ctx) {
            Poll::Pending => self.fut2.poll(ctx),
            rdy => rdy,
        }
    }
}
```
Without this proposal, one would have to use `unsafe` with
`Pin::map_unchecked_mut` to project the inner fields.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## [`MaybeUninit`][maybeuninit]`<T>`

When working with certain wrapper types in rust, you often want to access fields
of the wrapped types. When interfacing with C one often has to deal with
uninitialized data. In rust uninitialized data is represented by
[`MaybeUninit`][maybeuninit]`<T>`. In the following example we demonstrate
how one can initialize partial fields using [`MaybeUninit`][maybeuninit]`<T>`.
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
        // the type of `this.device_id` is `MaybeUninit<usize>`
        this.device_id.write(id);
        this.incident_count.write(0);
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
So to access a field of [`MaybeUninit`][maybeuninit]`<MachineData>` we can use
the already familiar syntax of accessing a field of `MachineData`/`&MachineData`
/`&mut MachineData`. The difference is that the type of the expression
`this.device_id` is now [`MaybeUninit`][maybeuninit]`<usize>`.

These *field projections* are also available on other types.

## [`Pin`][pin]`<P>` projections

Our second example is going to focus on [`Pin`][pin]`<P>`. This type is a little
special, as it allows unwrapping while projecting, but only for specific fields.
This information is expressed via the `#[pin]` attribute on the given field.
```rust
struct RaceFutures<F1, F2> {
    // we specify structurally pinned fields like this
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
        // `self.fut1` has the type `Pin<&mut F1>` because `fut1` is a pinned field.
        // if it was not pinned, the type would be `&mut F1`.
        match self.fut1.poll(ctx) {
            Poll::Pending => self.fut2.poll(ctx),
            rdy => rdy,
        }
    }
}
```

## Cells

When using [`Cell`][cell]`<T>` or [`UnsafeCell`][cell]`<T>`, one can use the
same field access syntax as before to get a projected field:
```rust
struct Foo {
    a: usize,
    b: u64,
}

fn process(x: &Cell<Foo>, y: &Cell<Foo>) {
    x.a.swap(y.a);
    x.b.set(x.b.get() + y.b.get());
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

See the tables from the [summary][summary] section for the exact projections that
are part of this RFC.

## Implementation
### Trait-based

It is currently unclear as to how this mechanism should be implemented. One
way would be to create a compiler-internal trait:
```rust
pub trait FieldProject {
    type Wrapper<'a, T>
    where
        T: 'a;
    type WrapperMut<'a, T>
    where
        T: 'a;

    /// Safety: closure must only do a field projection and not access the inner data
    unsafe fn field_project<'a, T, U>(
        this: Self::Wrapper<'a, T>,
        f: impl FnOnce(*const T) -> *const U,
    ) -> Self::Wrapper<'a, U>;

    /// Safety: closure must only do a field projection and not access the inner data
    unsafe fn field_project_mut<'a, T, U>(
        this: Self::WrapperMut<'a, T>,
        f: impl FnOnce(*mut T) -> *mut U,
    ) -> Self::WrapperMut<'a, U>;
}
```

That then would be implemented for the wrapper types. An example implementation
for [`MaybeUninit`][maybeuninit]`<T>`:
```rust
impl FieldProject for MaybeUninit<()> {
    type Wrapper<'a, T> = &'a MaybeUninit<T>where T:'a;
    type WrapperMut<'a, T> = &'a mut MaybeUninit<T>where T:'a;

    unsafe fn field_project<'a, T, U>(
        this: Self::Wrapper<'a, T>,
        f: impl FnOnce(*const T) -> *const U,
    ) -> Self::Wrapper<'a, U> {
        &*f(this.as_ptr()).cast::<MaybeUninit<U>>()
    }

    unsafe fn field_project_mut<'a, T, U>(
        this: Self::WrapperMut<'a, T>,
        f: impl FnOnce(*mut T) -> *mut U,
    ) -> Self::WrapperMut<'a, U> {
        &mut *f(this.as_mut_ptr()).cast::<MaybeUninit<U>>()
    }
}
```
You can find the other implementations in [this](https://github.com/y86-dev/field-project) repository.

### Others
It could also be entirely lang-item based. This would mean all wrapper types
would exhibit special field projection behavior.

## Interactions with other language features

### Bindings

Bindings could also be supported:
```rust
struct Foo {
    a: usize,
    b: u64,
}

fn process(x: &Cell<Foo>, y: &Cell<Foo>) {
    let Foo { a: ax, b: bx } = x;
    let Foo { a: ay, b: by } = y;
    ax.swap(ay);
    bx.set(bx.get() + by.get());
}
```

This also enables support for `enum`s:
```rust
enum FooBar {
    Foo(usize, usize),
    Bar(usize),
}

fn process(x: &Cell<FooBar>, y: &Cell<FooBar>) {
    use FooBar::*;
    match (x, y) {
        (Foo(a, b), Foo(c, d)) => {
            a.swap(c);
            b.set(b.get() + d.get());
        }
        (Bar(x), Bar(y)) => x.swap(y),
        (Foo(a, b), Bar(y)) => a.swap(y),
        (Bar(x), Foo(a, b)) => b.swap(x),
    }
}
```
They however seem not very compatible with [`MaybeUninit`][maybeuninit]`<T>`
(more work needed).

## Pin projections

Because [`Pin`][pin]`<P>` is a bit special, as it is the only Wrapper that
permits access to raw fields when the user specifies so. It needs a mechanism
to do so. This proposal has chosen an attribute named `#[pin]` for this purpose.
It would only be a marker attribute and provide no functionality by itself.

An additional challenge is that if a `!Unpin` field is marked `#[pin]`, then
one cannot implement the normal `Drop` trait, as it would give access to
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

# Drawbacks
[drawbacks]: #drawbacks

- Users currently relying on crates that facilitate field projections (see
[prior art][prior-art]) will have to refactor their code.
- Increased compiler complexity:
    - longer compile times
    - potential worse type inference


# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Why is this design the best in the space of possible designs?
This design is most likely the simplest design for field-projections, as users
will use the same syntax they would use for field access. There is also no
ambiguity, because when fields can be projected, the fields would not be
accessible in the first place.

One remaining issue is: how can users specify their own wrapper types?

## What other designs have been considered and what is the rationale for not choosing them?

This proposal was initially only designed to enable projecting
[`Pin`][pin]`<&mut T>`, because that would remove the need for `unsafe` when
pin projecting.

It seems beneficial to also provide this functionality for a wider range of types.

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

I have done some quick research but have not found similar concepts in other
languages. C and C++ handle uninitialized memory differently by allowing
any memory to be uninitialized and thus a field projection to uninitialized
memory is just normal field access. These languages also do not have the wrapper
types that rust provides.

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

## Before merging

- Is new syntax for the borrowing necessary (e.g. `&pin mut x.y` or `&uninit mut x.y`)?
- Should there be a general mechanism to support nested projections? Currently
there is explicit support planned for [`Pin`][pin]`<&mut` [`MaybeUninit`][maybeuninit]`<T>>`.

## Before stabilization
- How can we enable users to leverage field projection? Maybe there should exist
a public trait that can be implemented to allow this.
- Should `union`s also be supported?
- How can `enum` and  [`MaybeUninit`][maybeuninit]`<T>` be made compatible?

# Future possibilities
[future-possibilities]: #future-possibilities

## Arrays

Even more generalized projections e.g. slices: At the moment

- [`as_array_of_cells`](https://doc.rust-lang.org/core/cell/struct.Cell.html#method.as_array_of_cells)
- [`as_slice_of_cells`](https://doc.rust-lang.org/core/cell/struct.Cell.html#method.as_slice_of_cells)

exist, maybe there is room for generalization here as well.

## [`Rc`]`<T>` and [`Arc`]`<T>` projections

While out of scope for this RFC, projections for [`Rc`]`<T>` and [`Arc`]`<T>`
could be implemented in a similar way. This change seems to be a lot more
involved and will probably require that more information is stored in these
pointers. It seems more likely that this could be implemented for a new type
that explicitly opts in to provide field projections.

[`Rc`]: https://doc.rust-lang.org/alloc/sync/struct.Rc.html
[`Arc`]: https://doc.rust-lang.org/alloc/sync/struct.Arc.html