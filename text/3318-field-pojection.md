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
|[`Option`][option]`<&Struct>`                          |[`Option`][option]`<&Field>`                          |
|[`Pin`][pin]`<&Struct>`                                |[`Pin`][pin]`<&Field>`                                |
|[`Pin`][pin]`<&`[`MaybeUninit`][maybeuninit]`<Struct>>`|[`Pin`][pin]`<&`[`MaybeUninit`][maybeuninit]`<Field>>`|

Other pointers are also supported, for a list, see [here][supported-pointers].

The projection works exactly like current field access:
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
|`mystruct.foo`           | `MaybeUninit<Foo>`       |
|`&mystruct.foo`          | `&MaybeUninit<Foo>`      |
|`&mut mystruct.foo.count`|`&mut MaybeUninit<usize>` |


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
it is transformative for [`Pin`][pin]`<P>` and [`MaybeUninit`][maybeuninit]`<T>`:
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

[`Pin`][pin]`<P>` has a similar story:
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
This information is expressed via the `#[pin]` attribute on each structurally pinned field.
The `#[unpin]` attribute enables *pin unwrapping*. Fields with no attributes will not be accessible
via field projections (remember that `Pin<&T>` implements `Deref` for *all* types `T`).
```rust
use core::pin::pin;
struct RaceFutures<F1, F2> {
    #[pin]
    fut1: F1,
    #[pin]
    fut2: F2,
    // this will be used to fairly poll the futures
    #[unpin]
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
        self.first = !self.first;
        if self.first {
            // `self.fut1` has the type `Pin<&mut F1>` because `fut1` is a pinned field.
            // if it was not pinned, the type would be `&mut F1`.
            match self.fut1.poll(ctx) {
                Poll::Pending => self.fut2.poll(ctx),
                rdy => rdy,
            }
        } else {
            match self.fut2.poll(ctx) {
                Poll::Pending => self.fut1.poll(ctx),
                rdy => rdy,
            }
        }
    }
}
```

## Defining your own wrapper type

First you need to decide what kind of projection your wrapper type needs:
- field projection: this allows users to project `&mut Wrapper<Struct>` to `&mut Wrapper<Field>`,
  this is only available on types with `#[repr(transparent)]`
- inner projection: this allows users to project `Wrapper<&mut Struct>` to `Wrapper<&mut Field>`,
  this is *not* available for `union`s


### Field projection
Annotate your type with `#[field_projecting($T)]` where `$T` is the
generic type parameter that you want to project.
```rust
#[repr(transparent)]
#[field_projecting(T)]
pub union MaybeUninit<T> {
    uninit: (),
    value: ManuallyDrop<T>,
}
```

### Inner projection
Annotate your type with `#[inner_projecting($T)]` where `$T` is the generic type parameter
that you want to project.
```rust
#[inner_projecting(T)]
pub enum Option<T> {
    Some(T),
    None,
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Here is the list of types from `core` that will be `field_projecting`:

- [`MaybeUninit<T>`][maybeuninit]
- [`Cell<T>`][cell]
- [`UnsafeCell<T>`][unsafecell]

These will be `inner_projecting`:

- [`Option<T>`][option]
- [`Pin<T>`][pin]

## Supported pointers
[supported-pointers]: #supported-pointers

These are the pointer types that can be used as `P` in `P<Wrapper<Struct>> ->
P<Wrapper<Field>>` for `field_projecting` and in `Wrapper<P<Struct>> ->
Wrapper<P<Field>>` for `inner_projecting`:

- `&mut T`, `&T`
- `*mut T`, `*const T`, `NonNull<T>`, `AtomicPtr<T>`
- `Pin<P>` where `P` is from above

Note that all of these pointers have the same size and all can be transmuted to
and from `*mut T`. This is by design and other pointer types suggested should
follow this. There could be an internal trait
```rust
trait NoMetadataPtr<T> {
    fn into_raw(self) -> *mut T;

    /// # Safety
    /// The supplied `ptr` must have its origin from either `Self::into_raw`, or
    /// be directly derived from it via field projection (`ptr::addr_of_mut!((*raw).field)`)
    unsafe fn from_raw(ptr: *mut T) -> Self;
}
```
that then could be used by the compiler to ease the field projection
implementation.


## Implementation

Add two new attributes: `#[field_projecting($T)]` and `#[inner_projecting($T)]`
both taking a generic type parameter as an argument.

### `#[field_projecting($T)]`

#### Restrictions
This attribute is only allowed on `#[repr(transparent)]` types where the only
field has the layout of `$T`. Alternatively the type is a ZST.

```rust
#[field_projecting(T)]
pub struct Twice<T> {
//         ^^^^^^^^ error: field projecting type needs to be `#[repr(transparent)]`
    field: T,
}
```

#### How it works
This is done, because to do a projection, the compiler will
`mem::transmute::<&mut Wrapper<Struct>, *mut Struct>` and then get the field
using `ptr::addr_of_mut!` after which the pointer is then again
`mem::transmute::<*mut Field, &mut Wrapper<Field>>`d to yield the
projected field.

### `#[inner_projecting($T)]`

#### Restrictions
This attribute cannot be added on `union`s, because it is unclear what field
the projection would project. For example:
```rust
#[inner_projecting($T)]
pub union WeirdPair<T> {
    a: (ManuallyDrop<T>, u32),
    b: (u32, ManuallyDrop<T>),
}
```
Each field mentioning `$T` will either need to be a ZST, `#[inner_projecting]` or `$T`.

#### How it works
First each field of type `Pointer<$T>` (remember, we are projecting from
`Wrapper<Pointer<Struct>> -> Wrapper<Pointer<Field>>`) is projected to
`Pointer<$F>` and construct a `Wrapper<Pointer<$F>>` in place (because `Pointer<$F>`
will have the same size as `Pointer<$T>` this will take up the same number of
bytes, although the layout might be different).

The special behavior of `Pin` is explained [below][pin-projections-section].

## Interactions with other language features

### Bindings

Bindings are also be supported:
```rust
struct Foo {
    a: usize,
    b: u64,
}

fn process(x: &Cell<Foo>, y: &Cell<Foo>) {
    let Foo { a: ax, b: bx } = x;
    let Foo { a: ay, b: by } = y;
    // ax, bx, ay and by are all &Cell;
    ax.swap(ay);
    bx.set(bx.get() + by.get());
}
```
Enum bindings cannot be supported with the wrappers [`MaybeUninit`][maybeuninit]`<T>` and
[`Cell`][cell]`<T>`:
```rust
enum FooBar {
    Foo(usize, usize),
    Bar(usize),
}

fn problem(foo: &Cell<FooBar>) {
    match foo {
        Foo(a, b) => {
            foo.set(Bar(0));
            // UB: access to uninhabited field!
            let x = b.get();
        }
        _ => {}
    }
}
```
[`MaybeUninit`][maybeuninit]`<T>` has the problem that we cannot read the discriminant (as it might
not be initialized).

### `Deref` and `DerefMut`

Field projection should have higher priority similar to how field access has a higher priority than
`Deref`:
```rust
struct Foo {
    field: usize,
    inner: Bar,
}

struct Bar {
    field: isize,
}

impl core::ops::Deref for Foo {
    type Target = Bar;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

fn demo(f: &Foo) {
    let _: usize = f.field;
    let _: isize = (**f).field;
}
```

Users will have to explicitly deref the expression/call `deref` explicitly.

This does not introduces code breakage, because `Pin` is the only wrapper type that is `Deref`.
And it special treatment will be explained below.

## Pin projections
[pin-projections-section]: #pin-projections

Because [`Pin`][pin]`<P>` is a bit special, as it is the only Wrapper that permits access to raw
fields when the user specifies so. It needs a mechanism to do so. This proposal has chosen two
attributes named `#[pin]` and `#[unpin]` for this purpose. They are marker attribute and provide no
further functionality. They will be located at `::core::pin::{pin, unpin}`.

In a future edition they can be added into the prelude.

Two attributes are required for backwards compatibility. Suppose there exists this library:
```rust
pub struct BufPtr {
    buf: [u8; 64],
    // null, or points into buf
    ptr: *const u8,
    // for some legacy reasons this is `pub` and cannot be changed
    pub len: u8,
    _pin: PhantomPinned,
}

impl BufPtr {
    pub fn new(buf: [u8; 64]) -> Self {
        Self {
            buf,
            ptr: ptr::null(),
            len: 0,
            _pin: PhantomPinned,
        }
    }

    pub fn next(self: Pin<&mut Self>) -> Option<u8> {
        // SAFETY: we do not move out of `this`
        let this = unsafe { Pin::get_unchecked_mut(self) };
        if this.ptr.is_null() {
            let buf: *const [u8] = &this.buf[1..];
            this.ptr = buf.cast();
            // here we set the len and it cannot be changed, since `self` is `!Unpin` and it is pinned.
            this.len = 1;
            Some(this.buf[0])
        } else if this.len >= 64 {
            None
        } else {
            this.len += 1;
            // SAFETY: `ptr` is not null, so it points into `buf`
            let res = Some(unsafe { *this.ptr });
            this.ptr = this.ptr.wrapping_add(1);
            res
        }
    }
}
```
The code is sound, because after pinning `BufPtr`, users cannot change `len` with safe code.

If this proposal would treat all fields without `#[pin]` as `#[unpin]` then this would be possible:
```rust
fn main() {
    let mut buf = Box::pin(BufPtr::new([0; 64]));
    loop {
        // field projection used here:
        buf.as_mut().len = 0;
        buf.as_mut().next(); // at some point we read some bad address
    }
}
```

That is why for `Pin` we need the default behavior of **no projection at all**.
Users can specify unwrapping/projecting with `#[unpin]`/`#[pin]`.

Marking an `Unpin` field `#[pin]` produces a warning:
```rust
struct MyStruct {
    #[pin]
 // ^^^^^^ warning pinning `u64` is useless, because it implements `Unpin` [read here for more information].
    num: u64,
}
```

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
- this feature integrates well with itself and other parts of the language.
- the field access operator `.` is not imbued with additional meaning: it does
  not introduce overhead to use `.` on `&mut MaybeUninit<T>` compared to `&mut T`.

In particular this feature will not and *should not in the future* support
projecting types that require additional maintenance like `Arc`.
This would change the meaning of `.` allowing implicit creations of potentially
as many `Arc`s as one writes `.`.

## *Out of scope:* `Arc` projection
[out-of-scope-arc-projection]: #out-of-scope-arc-projection

With the current design of `Arc` it is not possible to add field projection,
because the ref-count lives directly adjacent to the data. Instead the std-lib should
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

## What other designs have been considered and what is the rationale for not choosing them?

This proposal was initially only designed to enable projecting
[`Pin`][pin]`<&mut T>`, because that would remove the need for `unsafe` when
pin projecting.

It seems beneficial to also provide this functionality for a wider range of types.

### Using solely `#[pin]`/`#[unpin]` for specifying structurally pinned fields

Because it makes existing code unsound this option has not been chosen.

### Trait instead of attributes to create a wrapper type

The trait could look like this:
```rust
pub trait FieldProjecting<T> {
    type Inner<U>;
    unsafe fn project<U>(self, proj: impl FnOnce(*mut T) -> *mut U) -> Self::Inner<U>;
}
```
Implementing this trait is a footgun. One has to use raw pointers only and already know a good bit
about `unsafe` code to write this correctly. This also opens the door for implementations that do
not abide by the "no additional maintenance" invariant.

It could of course just be a marker trait and fulfill the same purpose as the attributes. That would
enable using the condition "this type has projection" as type bounds. But this marker trait could
also be added later.


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
See [this section][out-of-scope-arc-projection] for more, as this is out of this RFC's
scope.

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

- [ ] Is new syntax for the borrowing necessary (e.g. `&pin mut x.y` or `&uninit mut x.y`)?

## Before stabilization
- [x] How can we enable users to leverage field projection? Maybe there should exist
a public trait that can be implemented to allow this.
- [ ] Should `union`s also be supported?
- [ ] ~~How can `enum` and  [`MaybeUninit`][maybeuninit]`<T>` be made compatible?~~
- [ ] for `Pin`, should we use `#[unpin]` like other `#[inner_projecting]`, or should we stick with `#[pin]` (and maybe introduce a way to switch between the two modes).
- [ ] how special does `PinnedDrop` need to be? This also ties in with the previous point, with `#[pin]` it is very easy to warrant a `PinnedDrop` instead of `Drop` (that will need to be compiler magic). With `#[unpin]` I do not really see a way how it could be implemented.
- [ ] Any new syntax? <small>*I am leaning towards NO (except for the next point).*</small>
- [ ] Disambiguate member access could we do something like `<struct as MaybeUninit>.value`?
- [ ] Should we expose the `NoMetadataPtr` to the user?
- [ ] What types should we also support? I am thinking of `PhantomData<&mut T>`, because this seems helpful in e.g. macro contexts that want to know the type of a field.
- [ ] should the warning when using `#[pin]` on an `Unpin` field be an error?

# Future possibilities
[future-possibilities]: #future-possibilities

## Arrays

Even more generalized projections e.g. slices: At the moment

- [`as_array_of_cells`](https://doc.rust-lang.org/core/cell/struct.Cell.html#method.as_array_of_cells)
- [`as_slice_of_cells`](https://doc.rust-lang.org/core/cell/struct.Cell.html#method.as_slice_of_cells)

exist, maybe there is room for generalization there as well.

## [`Rc`]`<T>` and [`Arc`]`<T>` projections

While out of scope for this RFC, projections for [`Rc`]`<T>` and [`Arc`]`<T>`
could be implemented by adding another field that points to the ref count.
This RFC is designed for low cost projections, modifying an atomic ref count is
too slow to let it happen without explicit opt-in by the programmer and as such
it would be better to implement it via a dedicated `map` function.

[`Rc`]: https://doc.rust-lang.org/alloc/sync/struct.Rc.html
[`Arc`]: https://doc.rust-lang.org/alloc/sync/struct.Arc.html
