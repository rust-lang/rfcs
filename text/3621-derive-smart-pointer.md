- Feature Name: `derive_smart_pointer`
- Start Date: 2024-05-01
- RFC PR: [rust-lang/rfcs#3621](https://github.com/rust-lang/rfcs/pull/3621)
- Rust Issue: [rust-lang/rust#123430](https://github.com/rust-lang/rust/issues/123430)

# Summary
[summary]: #summary

Make it possible to define custom smart pointers that work with trait objects.
For now, it will only be possible to do this using a derive macro, as we do not
stabilize the underlying traits.

This RFC builds on top of the [arbitrary self types v2 RFC][ast]. All
references to the `Receiver` trait are references to the version defined by
that RFC, which is different from the `Receiver` trait in nightly at the time
of writing.

# Motivation
[motivation]: #motivation

Currently, the standard library types `Rc` and `Arc` are special. It's not
possible for third-party libraries to define custom smart pointers that work
with trait objects.

It is generally desireable to make std less special, but this particular RFC is
motived by use-cases in the Linux Kernel. In the Linux Kernel, we need
reference counted objects often, but we are not able to use the standard
library `Arc`. There are several reasons for this:

1. The standard Rust `Arc` will call `abort` on overflow. This is not
   acceptable in the kernel; instead we want to saturate the count when it hits
   `isize::MAX`. This effectively leaks the `Arc`.
2. Using Rust atomics raises various issues with the memory model. We are using
   the LKMM (Linux Kernel Memory Model) rather than the usual C++ model. This
   means that all atomic operations should be implemented with an `asm!` block
   or similar that matches what kernel C does, rather than an LLVM intrinsic
   like we do today.

The Linux Kernel also needs another custom smart pointer called `ListArc`,
which is needed to provide a safe API for the linked list that the kernel uses.
The kernel needs these linked lists to avoid allocating memory during critical
regions on spinlocks.

For more detailed explanations of these use-cases, please refer to:

* [Arc in the Linux Kernel](https://rust-for-linux.com/arc-in-the-linux-kernel).
  * This document was discussed during [the 2024-03-06 meeting with t-lang](https://hackmd.io/OCz8EfzrRXeogXEDcOrL2w).
* The kernel's custom linked list: [Mailing list](https://lore.kernel.org/all/20240402-linked-list-v1-0-b1c59ba7ae3b@google.com/), [GitHub](https://github.com/Darksonn/linux/commits/b4/linked-list/).
* [Discussion on the memory model issue with t-opsem](https://rust-lang.zulipchat.com/#narrow/stream/136281-t-opsem/topic/.E2.9C.94.20Rust.20and.20the.20Linux.20Kernel.20Memory.20Model/near/422047516)

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The derive macro `SmartPointer` allows you to use custom smart pointers with
trait objects. This means that you will be able to coerce from
`SmartPointer<MyStruct>` to `SmartPointer<dyn MyTrait>` when `MyStruct`
implements `MyTrait`. Additionally, the derive macro allows you to use `self:
SmartPointer<Self>` in traits without making them non-object-safe.

It is not possible to use this feature without the derive macro, as we are not
stabilizing its expansion.

## Coercions to trait objects

By using the macro, the following example will compile:
```rust
#[derive(SmartPointer)]
#[repr(transparent)]
struct MySmartPointer<T: ?Sized>(Box<T>);

impl<T: ?Sized> Deref for MySmartPointer<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}

trait MyTrait {}

impl MyTrait for i32 {}

fn main() {
    let ptr: MySmartPointer<i32> = MySmartPointer(Box::new(4));

    // This coercion would be an error without the derive.
    let ptr: MySmartPointer<dyn MyTrait> = ptr;
}
```
Without the `#[derive(SmartPointer)]` macro, this example would fail with the
following error:
```
error[E0308]: mismatched types
  --> src/main.rs:11:44
   |
11 |     let ptr: MySmartPointer<dyn MyTrait> = ptr;
   |              ---------------------------   ^^^ expected `MySmartPointer<dyn MyTrait>`, found `MySmartPointer<i32>`
   |              |
   |              expected due to this
   |
   = note: expected struct `MySmartPointer<dyn MyTrait>`
              found struct `MySmartPointer<i32>`
   = help: `i32` implements `MyTrait` so you could box the found value and coerce it to the trait object `Box<dyn MyTrait>`, you will have to change the expected type as well
```

## Object safety

Consider the following trait:
```rust
trait MyTrait {
    // Arbitrary self types is enough for this.
    fn func(self: MySmartPointer<Self>);
}

// But this requires #[derive(SmartPointer)].
fn call_func(value: MySmartPointer<dyn MyTrait>) {
    value.func();
}
```
You do not need `#[derive(SmartPointer)]` to declare this trait ([arbitrary
self types][ast] is enough), but the trait will not be object safe unless you
annotate `MySmartPointer` with `#[derive(SmartPointer)]`. If you don't, then
the use of `dyn MyTrait` triggers the following error:
```
error[E0038]: the trait `MyTrait` cannot be made into an object
  --> src/lib.rs:11:36
   |
8  |     fn func(self: MySmartPointer<Self>);
   |                   -------------------- help: consider changing method `func`'s `self` parameter to be `&self`: `&Self`
...
11 | fn call_func(value: MySmartPointer<dyn MyTrait>) {
   |                                    ^^^^^^^^^^^ `MyTrait` cannot be made into an object
   |
note: for a trait to be "object safe" it needs to allow building a vtable to allow the call to be resolvable dynamically; for more information visit <https://doc.rust-lang.org/reference/items/traits.html#object-safety>
  --> src/lib.rs:8:19
   |
7  | trait MyTrait {
   |       ------- this trait cannot be made into an object...
8  |     fn func(self: MySmartPointer<Self>);
   |                   ^^^^^^^^^^^^^^^^^^^^ ...because method `func`'s `self` parameter cannot be dispatched on
```
Note that using the `self: MySmartPointer<Self>` syntax requires that you
implement `Receiver` (or `Deref`), as the derive macro does not emit an
implementation of `Receiver`.

## Requirements for using the macro

Whenever a `self: MySmartPointer<Self>` method is called on a trait object, the
compiler will convert from `MySmartPointer<dyn MyTrait>` to
`MySmartPointer<MyStruct>` using something similar to a transmute. Because of
this, there are strict requirements on the layout of `MySmartPointer`. It is
required that `MySmartPointer` is a `#[repr(transparent)]` struct, and the type
of its non-zero-sized field must either be a standard library pointer type
(reference, raw pointer, NonNull, Box, Arc, etc.) or another user-defined type
also using this derive macro.
```rust
#[derive(SmartPointer)]
#[repr(transparent)]
struct MySmartPointer<T: ?Sized> {
    ptr: Box<T>,
    _phantom: PhantomData<T>,
}
```

### Multiple type parameters

If the type has multiple type parameters, then you must explicitly specify
which one should be used for dynamic dispatch. For example:
```rust
#[derive(SmartPointer)]
#[repr(transparent)]
struct MySmartPointer<#[pointee] T: ?Sized, U> {
    ptr: Box<T>,
    _phantom: PhantomData<U>,
}
```
Specifying `#[pointee]` when the struct has only one type parameter is allowed,
but not required.

## Pinned pointers

The `#[derive(SmartPointer)]` macro is not sufficient to coerce the smart
pointer when it is wrapped in `Pin`. That is, even if `MySmartPointer<MyStruct>`
coerces to `MySmartPointer<dyn MyTrait>`, you will not be able to coerce
`Pin<MySmartPointer<MyStruct>>` to `Pin<MySmartPointer<dyn MyTrait>>`.
Similarly, traits with self types of `Pin<MySmartPointer<Self>>` are not object
safe.

If you implement the unstable unsafe trait called `PinCoerceUnsized` for
`MySmartPointer`, then the smart pointer will gain the ability to be coerced
when wrapped in `Pin`. The trait is not being stabilized by this RFC.

## Example of a custom Rc
[custom-rc]: #example-of-a-custom-rc

The macro makes it possible to implement custom smart pointers. For example,
you could implement your own `Rc` type like this:

```rust
#[derive(SmartPointer)]
#[repr(transparent)]
pub struct Rc<T: ?Sized> {
    inner: NonNull<RcInner<T>>,
}

struct RcInner<T: ?Sized> {
    refcount: usize,
    value: T,
}

impl<T: ?Sized> Deref for Rc<T> {
    type Target = T;
    fn deref(&self) -> &T {
        let ptr = self.inner.as_ptr();
        unsafe { &*ptr.value }
    }
}

impl<T> Rc<T> {
    pub fn new(value: T) -> Self {
        let inner = Box::new(RcInner {
            refcount: 1,
            value,
        });
        Self {
            inner: NonNull::from(Box::leak(inner)),
        }
    }
}

impl<T: ?Sized> Clone for Rc<T> {
    fn clone(&self) -> Self {
        unsafe { (*self.inner.as_ptr()).refcount += 1 };
        Self { inner: self.inner }
    }
}

impl<T: ?Sized> Drop for Rc<T> {
    fn drop(&mut self) {
        let ptr = self.inner.as_ptr();
        unsafe { (*ptr).refcount -= 1 };
        if unsafe { (*ptr).refcount } == 0 {
            drop(unsafe { Box::from_raw(ptr) });
        }
    }
}
```
In this example, `#[derive(SmartPointer)]` makes it possible to use `Rc<dyn
MyTrait>`.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The derive macro will expand into two trait implementations,
[`core::ops::CoerceUnsized`] to enable unsizing coercions and
[`core::ops::DispatchFromDyn`] for dynamic dispatch. This expansion will be
adapted in the future if the underlying mechanisms for unsizing coercions and
dynamically dispatched receivers changes.

As mentioned in the [rationale][why-only-macro] section, this RFC only proposes
to stabilize the derive macro. The underlying traits used by its expansion will
remain unstable for now.

## Input Requirements
[input-requirements]: #input-requirements

The macro sets the following requirements on its input:

1. The definition must be a struct.
2. The struct must have at least one type parameter. If multiple type
   parameters are present, exactly one of them has to be annotated with the
   `#[pointee]` derive helper attribute.
3. The struct must be `#[repr(transparent)]`.
4. The struct must have at least one field.
5. Assume that `T` is a type that can be unsized to `U`, and let `FT` and `FU`
   be the type of the struct's field when the pointee is equal to `T` and `U`
   respectively. If the struct's trait bounds are satisfied for both `T` and
   `U`, then it must be possible to convert `FT` to `FU` using an unsizing
   coercion.

(Adapted from the docs for [`DispatchFromDyn`].)

Points 1, 2 and 3 are verified syntactically by the derive macro. Points 4 and 5
are verified semantically by the compiler when checking the generated
[`DispatchFromDyn`] implementation as it does today.

The `#[pointee]` attribute may also be written as `#[smart_pointer::pointee]`.

## Expansion

The macro will expand to two implementations, one for
[`core::ops::CoerceUnsized`] and one for [`core::ops::DispatchFromDyn`]. This
is enough for a type to participate in unsizing coercions and dynamic dispatch.

The derive macro will implement both traits for the type according to the
following procedure:

- Copy all generic parameters from the struct definition into the impl.
- Add an additional type parameter `U`.
- For every trait bound declared on the trait, add it twice to the trait
  implementation. Once exactly as written, and once with every instance of the
  `#[pointee]` parameter replaced with `U`.
- Add an additional `Unsize<U>` bound to the `#[pointee]` type parameter.
- The generic parameter of the trait being implemented will be `Self`, except
  that the `#[pointee]` type parameter is replaced with `U`.

Given the following example code:
```rust
#[derive(SmartPointer)]
#[repr(transparent)]
struct MySmartPointer<'a, #[pointee] T, A>
where
    T: ?Sized + SomeTrait<T>,
{
    ptr: &'a T,
    phantom: PhantomData<A>,
}
```

we'll get the following expansion:

```rust
#[automatically_derived]
impl<'a, T, A, U> ::core::ops::CoerceUnsized<MySmartPointer<'a, U, A>> for MySmartPointer<'a, T, A>
where
    T: ?Sized + SomeTrait<T>,
    U: ?Sized + SomeTrait<U>,
    T: ::core::marker::Unsize<U>,
{}

#[automatically_derived]
impl<'a, T, A, U> ::core::ops::DispatchFromDyn<MySmartPointer<'a, U, A>> for MySmartPointer<'a, T, A>
where
    T: ?Sized + SomeTrait<T>,
    U: ?Sized + SomeTrait<U>,
    T: ::core::marker::Unsize<U>,
{}
```

## `Receiver` and `Deref` implementations

The macro does not emit a [`Receiver`][ast] implementation. Types that do not
implement `Receiver` can still use `#[derive(SmartPointer)]`, but they can't be
used with dynamic dispatch directly.

The raw pointer type would be an example of a type that (behaves like it) is
annotated with `#[derive(SmartPointer)]` without an implementation of
`Receiver`. In the case of raw pointers, you can coerce from `*const MyStruct`
to `*const dyn MyTrait`, but you must first convert them to a reference before
you can use them for dynamic dispatch.

## Vtable requirements

As seen in the `Rc` example, the macro needs to be usable even if the pointer
is `NonNull<RcInner<T>>` (as opposed to `NonNull<T>`).

## `PinCoerceUnsized`

The standard library defines the following unstable trait:
```rust
/// Trait that indicates that this is a pointer or a wrapper for one, where
/// unsizing can be performed on the pointee when it is pinned.
///
/// # Safety
///
/// If this type implements `Deref`, then the concrete type returned by `deref`
/// and `deref_mut` must not change without a modification. The following
/// operations are not considered modifications:
///
/// * Moving the pointer.
/// * Performing unsizing coercions on the pointer.
/// * Performing dynamic dispatch with the pointer.
/// * Calling `deref` or `deref_mut` on the pointer.
///
/// The concrete type of a trait object is the type that the vtable corresponds
/// to. The concrete type of a slice is an array of the same element type and
/// the length specified in the metadata. The concrete type of a sized type
/// is the type itself.
pub unsafe trait PinCoerceUnsized<U>: CoerceUnsized<U> {}

impl<T, U> CoerceUnsized<Pin<U>> for Pin<T>
where
    T: PinCoerceUnsized<U>,
{}

impl<T, U> DispatchFromDyn<Pin<U>> for Pin<T>
where
    T: PinCoerceUnsized<U> + DispatchFromDyn<U>,
{}
```
The trait is implemented for all standard library types that implement
`CoerceUnsized`.

Although this RFC proposes to add the `PinCoerceUnsized` trait to ensure that
unsizing coercions of pinned pointers cannot be used to cause unsoundness, the
RFC does not propose to stabilize the trait.

# Drawbacks
[drawbacks]: #drawbacks

- Stabilizing this macro limits how the underlying traits can be changed in the
  future, since we cannot change them in ways that make it impossible to
  implement the macro as-is.

- Stabilizing this macro reduces the incentive to stabilize the underlying
  traits, meaning that it may take significantly longer before we do so. This
  RFC does not include support for coercing transparent containers like
  [`Cell`], so hopefully that will be enough incentive to continue work on the
  underlying traits.

- This would be the first example in the standard library of a derive macro that
  does not implement a trait of the same name as the macro. (However, there are
  examples of macros that implement multiple traits: `#[derive(PartialEq)]`
  also implements `StructuralPartialEq`.)

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Why only stabilize a macro?
[why-only-macro]: #why-only-stabilize-a-macro

This RFC proposes to stabilize the `#[derive(SmartPointer)]` macro without
stabilizing what it expands to. This effectively means that the macro is the
only way to use these features for custom types. The rationale for this is that
we currently don't know how to stabilize the traits, and that this is a serious
blocker for making progress on this issue. Stabilizing the macro will unblock
projects that wish to define custom smart pointers, and does not prevent
evolution of the underlying traits.

See also [the section on prior art][prior-art], which discusses a previous
attempt to stabilize the underlying traits.

## Receiver and Deref traits

The vast majority of custom smart pointers will implement `Receiver` (often via
`Deref`, which results in a `Receiver` impl due to the blanket impl). So why
not also emit a `Receiver`/`Deref` impl in the output of the macro. One
advantage of doing so is that this may sufficiently limit the macro so that we
do not need to solve the pin soundness issue discussed in [the unresolved
questions section][unresolved-questions].

However, it turns out that there are quite a few different ways we might
implement `Deref`. For example, consider [the custom `Rc` example][custom-rc]:
```rust
#[derive(SmartPointer)]
#[repr(transparent)]
pub struct Rc<T: ?Sized> {
    inner: NonNull<RcInner<T>>,
}

struct RcInner<T: ?Sized> {
    refcount: usize,
    value: T,
}

impl<T: ?Sized> Deref for Rc<T> {
    type Target = T;
    fn deref(&self) -> &T {
        let ptr = self.inner.as_ptr();
        unsafe { &*ptr.value }
    }
}
```
Making the macro general enough to generate `Deref` impls that are _that_
complex would not be feasible. And it doesn't make sense to stabilize the macro
without support for the custom `Rc` case, as implementing a custom `Arc` in the
Linux Kernel is the primary motivation for this RFC.

Note that having the macro generate a `Receiver` impl instead doesn't work
either, because that prevents the user from implementing `Deref` at all. (There
is a blanket impl of `Receiver` for all `Deref` types.)

## Transparent containers

Smart pointers are not the only use case for implementing the [`CoerceUnsized`]
and [`DispatchFromDyn`] traits. They are also used for "transparent containers"
such as [`Cell`]. That use-case allows coercions such as `Cell<Box<MyStruct>>`
to `Cell<Box<dyn MyTrait>>`. (Coercions where the `Cell` is inside the `Box` are
already supported on stable Rust.)

It is not possible to use the derive macro proposed by this RFC for transparent
containers because they require a different set of where bounds when
implementing the traits. To compare:
```rust
// smart pointer example
impl<T, U> DispatchFromDyn<Box<U>> for Box<T>
where
    T: Unsize<U> + ?Sized,
    U: ?Sized,
{}

// transparent container example
impl<T, U> DispatchFromDyn<Cell<U>> for Cell<T>
where
    T: DispatchFromDyn<U>,
{}
```
Attempting to annotate `#[derive(SmartPointer)]` onto a transparent container
will fail to compile because [it violates the rules for implementing
`DispatchFromDyn`][tc-pg]. Supporting custom transparent containers is out of
scope for this RFC.

[tc-pg]: https://play.rust-lang.org/?version=nightly&mode=debug&edition=2021&gist=c3fe2a11822e4c5e2dae5bfec9d77b9e

## Why not two derive macros?

The derive macro generates two different trait implementations:

- [`CoerceUnsized`] that allows conversions from `SmartPtr<MyStruct>` to
  `SmartPtr<dyn MyTrait>`.
- [`DispatchFromDyn`] that allows conversions from `SmartPtr<dyn MyTrait>` to
  `SmartPtr<MyStruct>`.

It could be argued that these should be split into two separate derive macros.
We are not proposing this for a few reasons:

- If there are two derive macros, then we have to support the case where you
  only use one of them. There isn't much reason to do that, and the authors are
  not aware of any examples where you would prefer to implement one of the
  traits without implementing both.

- Having two different macros means that we lock ourselves into solutions that
  involve two traits that split the feature in the way that we split it today.
  However, it is easy to imagine situations where we would want to split the
  traits in a different way. For example, we might instead want one trait for
  smart pointers, and another trait for transparent containers. Or maybe we just
  want one trait that does both things.

- The authors believe that a convenience `#[derive(SmartPointer)]` macro will
  continue to make sense, even once the underlying traits are stabilized. It is
  significantly easier to use than the expansion.

- If we want the macro to correspond one-to-one to the underlying traits, then
  we would want to use the same names as the underlying traits. However, we
  don't know what the traits will be called when we finally figure out how to
  stabilize them. (One of the traits have already been renamed once!)

Even raw-pointer-like types that do not implement `Receiver` still want to
implement `DispatchFromDyn`, since this allows you to use them as the field
type in other structs that use `#[derive(SmartPointer)]`. For example, the
custom `Rc` has a field of type `NonNull`, and this works since `NonNull` is
`DispatchFromDyn`.

[`Cell`]: https://doc.rust-lang.org/stable/core/cell/struct.Cell.html

## What about `#[pointee]`?

This RFC currently proposes to mark the generic parameter used for dynamic
dispatch with `#[pointee]`. For convenience, the RFC proposes that this is only
needed when there are multiple generic parameters.

There are potential use-cases for smart pointers with additional generic
parameters. Specifically, the `ListArc` type used by the linked lists currently
has an additional const generic parameter to allow you to use the same
refcounted value with multiple lists. People have argued that it would be
better to change this to a generic type instead of a const generic, so it would
be useful to keep the option of having multiple generic types on the struct.

### Conflicts with third-party derive macros

The `#[pointee]` attribute could in principle conflict with other derive macros
that also wish to annotate one of the parameters with an attribute called
`#[pointee]`. To disambiguate such cases, we also allow the attribute to be
spelled `#[smart_pointer::pointee]`.

It is an error to specify both `#[pointee]` and `#[smart_pointer::pointee]`, so
both macros must support this kind of disambiguation.

Another way to avoid conflicts between `#[derive(SmartPointer)]` and third-party
macros is to always assume that the first generic parameter is the pointee.
This RFC does not propose that solution because:

* It prevents the pointee from having a default unless it is the only parameter,
  because parameters with a default must come last.
* If logic such as "the first parameter" becomes commonplace in macro design,
  then it does not really solve the issue with conflicts: you could have two
  macros that both assume that the first parameter is special. And this kind of
  conflict will be more common than attribute conflicts, because the attribute
  will only conflict if both macros use an attribute of the same name.

The authors are not aware of any macros using a `#[pointee]` attribute today.

## Derive macro or not?

Stabilizing this as a derive macro more or less locks us in with the decision
that the compiler will use traits to specify which types are compatible with
trait objects. However, one could imagine other mechanisms. For example, stable
Rust currently has logic saying that any struct where the last field is `?Sized`
will work with unsizing operations. (E.g., if `Wrapper` is such a struct, then
you can convert from `Box<Wrapper<[u8; 10]>>` to `Box<Wrapper<[u8]>>`.) That
mechanism is not specified using a trait.

However, using traits for this functionality seems to be the most flexible. To
solve the unresolved questions, we most likely need to constrain the
implementations of these traits for `Pin` with stricter trait bounds than what
is specified on the struct. That will get much more complicated if we use a
mechanism other than traits to specify this logic.

## `PinCoerceUnsized`

Beyond the addition of the `#[derive(SmartPointer)]` macro, this RFC also
proposes to add a new unstable trait called `PinCoerceUnsized`. This trait is
necessary because the API proposed by this RFC would otherwise by unsound:

> You could use `Pin::new` to create a `Pin<SmartPtr<MyUnpinFuture>>` and coerce
> that to `Pin<SmartPtr<dyn Future>>`. Then, if `SmartPtr` has a malicious
> implementation of the `Deref` trait, then `deref` could return a `&mut dyn
> Future` whose concrete type is not `MyUnpinFuture`, but instead some other
> future type that *does* need to be pinned. Since no unsafe code is involved in
> any of these steps, this means that we are able to safely create a pinned
> pointer to a value that has not been pinned.

Adding the unsafe `PinCoerceUnsized` trait ensures that the user cannot coerce
`Pin<SmartPtr<MyUnpinFuture>>` to `Pin<SmartPtr<dyn Future>>` without using
unsafe to promise that the concrete type returned when calling `deref` on the
resulting `Pin<SmartPtr<dyn Future>>` is `MyUnpinFuture`.

This RFC does not propose to stabilize `PinCoerceUnsized` because of naming
issues. If we do not know whether `CoerceUnsized` will still use that name when
we stabilize it, then we can't stabilize a trait called `PinCoerceUnsized`.
Furthermore, the Linux kernel (which forms the motivation for this RFC) does not
currently need it to be stabilized.

There are some alternatives to `PinCoerceUnsized`. The primary contender for an
alternative solution is `DerefPure`. However, that solution involves a minor
breaking change, and we can always decide to switch to `DerefPure` later even if
we adopt `PinCoerceUnsized` now.

### `StableDeref`

A previous version of this RFC proposed to instead add a trait called
`StableDeref` that pretty much had the same requirements as `PinCoerceUnsized`,
except that it also required the address returned by `deref` to be stable.

The motivation behind adding a `StableDeref` trait instead of `PinCoerceUnsized`
is that `StableDeref` would also be useful for other things, and that both
traits essentially just say that the `Deref` implementation doesn't do anything
unreasonable. The requirement that the address is stable is not strictly
required to keep the API sound, but semantically it is incoherent to have a
pinned pointer whose address can change, so it is not overly burdensome to
require it.

However, this suggestion was abandoned due to an inconsistency with the
`StableDeref` trait defined by the ecosystem. That trait requires that raw
pointers to the contents of the pointer stay valid even if the smart pointer is
moved, but this is not satisfied by `Box` or `&mut T` because moving these
pointers asserts that they are unique. This is a problem because whichever trait
we use for pinned unsizing coercions, it *must* be implemented by `Box` and
`&mut T`.

### `DerefPure`

In a similar manner to the `StableDeref` option, we can use the existing
`DerefPure` trait. This option is a reasonable way forward, but this RFC does
not propose it because it would be a breaking change. (Note that `StableDeref`
is also a breaking change for the same reason.)

Basically, the problem is that `Deref` is a supertrait of `DerefPure`, but there
are a few types that can be coerced when pinned that do not implement `Deref`.
For example, this code compiles today:
```rust
trait MyTrait {}
impl MyTrait for String {}

fn pin_cell_map(p: Pin<Cell<Box<String>>>) -> Pin<Cell<Box<dyn MyTrait>>> {
    p
}
```
The `Cell` type does not implement `Deref`, but the above code still compiles.
Note that since all methods on `Pin` _do_ require `Deref`, such pinned pointers
are useless and impossible to construct. But it is a breaking change
nonetheless.

If this breakage is considered acceptable, then using `DerefPure` instead of a
new `PinCoerceUnsized` would be a reasonable way forward.

### Make the derive macro unsafe

We could just make the macro unsafe in a similar vein to [the unsafe attributes
RFC][unsafe-attribute].
```rust
// SAFETY: The Deref impl is not malicious.
#[unsafe(derive(SmartPointer))]
#[repr(transparent)]
pub struct Rc<T: ?Sized> {
    inner: NonNull<RcInner<T>>,
}
```
This would solve the unsoundness, but this RFC does not propose it because it
raises forwards compatibility hazards. We might start out with an unsafe derive
macro, and then in the future we might decide to instead use the
`PinCoerceUnsized` solution. Then, `#[unsafe(derive(SmartPointer))]` would have
to generate an implementation of `PinCoerceUnsized` trait too, because otherwise
`#[unsafe(derive(SmartPointer))] Pin<Rc<MyStruct>>` would lose the ability to be
unsize coerced, which would be a breaking change. This means that
`#[unsafe(derive(SmartPointer))]` and `#[derive(SmartPointer)]` could end up
expanding to _different_ things.

### Negative trait bounds

There are also various solutions that involve negative trait bounds. For
example, you might instead modify `CoerceUnsized` like this:
```rust
// Permit going from `Pin<impl Unpin>` to` Pin<impl Unpin>`
impl<P, U> CoerceUnsized<Pin<U>> for Pin<P>
where
    P: CoerceUnsized<U>,
    P: Deref<Target: Unpin>,
    U: Deref<Target: Unpin>,
{ }

// Permit going from `Pin<impl !Unpin>` to `Pin<impl !Unpin>`
impl<P, U> CoerceUnsized<Pin<U>> for Pin<P>
where
    P: CoerceUnsized<U>,
    P: core::ops::Deref<Target: !Unpin>,
    U: core::ops::Deref<Target: !Unpin>,
{ }
```
This RFC does not propose it because it is a breaking change and the
`PinCoerceUnsized` or `DerefPure` solutions are simpler. This solution is
discussed in more details in [the pre-RFC for stabilizing the underlying
traits][pre-rfc].

# Prior art
[prior-art]: #prior-art

## Stabilizing subsets of features

There are several prior examples of unstable features that have been blocked
from stabilization for various reasons, where we have been able to make
progress by reducing the scope and stabilizing a subset.

- The most recent example of this is [the arbitrary self types RFC][ast], where
  [it was proposed to reduce the scope][ast-scope] so that we do not block
  progress on the feature.
- Another example of this is [the async fn in traits feature][rpit]. This was
  stabilized even though it is not yet advisable to use it for traits in the
  public API of crates, due to missing parts of the feature.

There have already been [previous attempts to stabilize the underlying
traits][pre-rfc], and they did not make much progress. Therefore, this RFC
proposes to reduce the scope and instead stabilize a derive macro.

[ast-scope]: https://github.com/rust-lang/rfcs/pull/3519#discussion_r1492385549
[rpit]: https://blog.rust-lang.org/2023/12/21/async-fn-rpit-in-traits.html

## Macros whose output is unstable

The Rust testing framework is considered unstable, and the only stable way to
interact with it is via the `#[test]` attribute macro. The macro's output uses
the unstable internals of the testing framework. This allows the testing
framework to be changed in the future.

Note also that the `pin!` macro expands to something that uses an unstable
feature, though it does so for a different reason than
`#[derive(SmartPointer)]` and `#[test]`.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

Bikeshedding over the name remains.

The name `#[derive(SmartPointer)]` leaves some things to be desired, as smart
pointers would generally want to implement some traits that this macro does
*not* expand to. Most prominently, any smart pointer should implement `Deref` or
`Receiver`. Really, the macro just says that this pointer works with unsizing
and dynamic dispatch.

We will settle on the final name prior to stabilization.

# Future possibilities
[future-possibilities]: #future-possibilities

One of the design goals of this RFC is that it should make this feature
available to crates without significantly limiting how the underlying traits
can evolve. The authors hope that we will find a way to stabilize the
underlying traits in the future.

One of the things that is left out of scope of this RFC is coercions involving
custom transparent containers similar to [`Cell`]. They require you to implement
the traits with different where bounds. Adding support for custom transparent
containers makes sense as a future expansion of the feature.

There is a reasonable change that we may be able to lift some of [the
restrictions][input-requirements] on the shape of the struct as well. The
current restrictions are just whatever [`DispatchFromDyn`] requires today, and
proposals for relaxing them have been seen before (e.g., in the
[pre-RFC][pre-rfc].)

One example of a restriction that we could lift is the restriction that there is
only one non-zero-sized field (i.e., that it must be `#[repr(transparent)]`).
This would allow smart pointers to use custom allocators. (Today, types like
`Box` and `Rc` only work with trait objects when using the default zero-sized
allocator.)

This could also allow implementations of `Rc` and `Arc` that store the value and
refcount in two different allocations, like how the C++ `shared_ptr` works.
```rust
#[derive(SmartPointer)]
pub struct Rc<T: ?Sized> {
    refcount: NonNull<Refcount>,
    value: NonNull<T>,
}
```
Implementing this probably requires the `#[derive(SmartPointer)]` macro to know
syntactically which field holds the vtable. One simple way to do that could be
to say that it must be the last field, analogous to the unsized field in structs
that must also be the last field. Another option is to add another attribute
like `#[pointee]` that must be annotated on the field in question.

[ast]: https://github.com/rust-lang/rfcs/pull/3519
[pre-rfc]: https://internals.rust-lang.org/t/pre-rfc-flexible-unsize-and-coerceunsize-traits/18789
[`CoerceUnsized`]: https://doc.rust-lang.org/stable/core/ops/trait.CoerceUnsized.html
[`core::ops::CoerceUnsized`]: https://doc.rust-lang.org/stable/core/ops/trait.CoerceUnsized.html
[`DispatchFromDyn`]: https://doc.rust-lang.org/stable/core/ops/trait.DispatchFromDyn.html
[`core::ops::DispatchFromDyn`]: https://doc.rust-lang.org/stable/core/ops/trait.DispatchFromDyn.html
[unsafe-attribute]: https://github.com/rust-lang/rfcs/pull/3325
