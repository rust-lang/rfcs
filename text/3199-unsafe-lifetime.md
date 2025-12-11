- Feature Name: `unsafe_lifetime`
- Start Date: 2021-11-24
- RFC PR: [rust-lang/rfcs#3199](https://github.com/rust-lang/rfcs/pull/3199)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Introduce a new special lifetime `'erased` which represents an erased lifetime parameter. Dereferencing a `&'erased T` or `&'erased mut T` is unsafe, and all generic parameters (whether lifetime or type) have the implicit bound `'a: '!erased` or `T: '!erased` unless they explicitly opt out with `'a: '?erased`. Additionally (and perhaps optionally) introduce a restricted version of transmute called `unerase_lifetimes` which takes `T<'a, 'erased, 'erased>` and transmutes to `T<'a, 'b, 'c>`.

# Motivation
[motivation]: #motivation

There are two particularly useful applications of a `'erased` lifetime:
- Enabling self referential structs, which are only really possible with 
- Resolving the dropcheck eyepatch by eliminating the requirement for the `#[may_dangle]` attribute.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## Self Reference

The borrow checker does not understand self referential types. If we want to write the following:
```rust
struct A<T> {
    item: T
    borrower: B<'?, T> // we want the ref inside this to refer to item
}

impl<T> A {
    pub fn a_func(&self) -> &'a T {
        let t: &T = self.borrower.b_func()
        todo!()
    }
}

struct B<'a, T> {
    actual_ref: &'a T
}

impl<'a, T> B {
    pub fn b_func(&self) -> &'a T { todo!() }
}
```
What should the lifetime parameter `'?` be? There are traditionally three choices:
- introduce a new lifetime parameter
- replace all self-references with pointers
- use `'static` and transmute our lifetime into it

Right now, there is only one way to really achieve this, which is to replace the refs with pointers:
```rust
struct A<T> {
    item: T
    borrower: B<T> // we want the ref inside this to refer to item
}

impl<T> A {
    pub fn a_func(&self) -> &'a T {
        let t_1: &T = unsafe { self.borrower.b_func_1().as_ref().unwrap() }
        let t_2: &T = self.borrower.b_func_2()
        todo!()
    }
}

struct B<T> {
    actual_ref: *const T
}

impl<T> B {
    // which of these is best?
    pub fn b_func_1<'a>(&self) -> &'a T { todo!() }
    pub fn b_func_2(&self) -> *const T { todo!() }
}
```
This works, but the we have shifted the burden of unsafe away from where the requirements are actually being upheld, which is in struct `A`. The original lifetime based implementation of `B` doesn't care that the lifetime is self referential; only that it is valid whenever `B` is used, and therefore can be verified by the borrow checker. It makes more sense for `A`, the struct creating and storing self references, to bear the unsafe burden of dereferencing than for `B` to.

How do we use the `'erased` lifetime to resolve this?
```rust
struct A<T> {
    item: T
    borrower: B<'erased, T> // we want the ref inside this to refer to item
}

impl<T> A {
    pub fn a_func(&self) -> &'a T {
        let t: &T = unsafe { self.borrower.unerase_lifetimes() }.b_func()
        todo!()
    }
}

struct B<'a: '?erased, T> {
    actual_ref: &'a T
}

impl<'a, T> B {
    pub fn b_func(&self) -> &'a T { todo!() }
}
```

Note that the use of unsafe is confined to `A` and it's `impl`s, which is where the unsafe action of creating self references actually occurs. The only modification made to `B` is marking `'a` as `'?erased`. The same modification is not made to the `impl` for `B`, which means that a value of type `B<'erased, T>` cannot be used as the self parameter of `b_func` without first being unerased.

## Dropcheck Eyepatch

Right now the implementation of box looks something like this:
```rust
pub struct Box<T: ?Sized> {
    pointer: *mut T,
};

unsafe impl<#[may_dangle] T: ?Sized> Drop for Box<T> {
    fn drop(&mut self) {
        // FIXME: Do nothing, drop is currently performed by compiler.
    }
}
```
but the same effect could be achieved by this:
```rust
pub struct Box<T: ?Sized + '?erased> {
    pointer: *mut T,
};

impl<T: ?Sized + '?erased> Drop for Box<T> {
    fn drop(&mut self) {
        // FIXME: Do nothing, drop is currently performed by compiler.
    }
}
```

There are situations where, when writing unsafe code, you may need to store a type without encoding its lifetime in the type system. The existence of raw pointers in the language is an acknowledgement of this need, but it is not always perfectly ergonomic to use pointers in this scenario. Consider the following self-referential struct:
```rust
struct ArrayIter<T> {
    buffer: [T; 32]
    iter: std::slice::Iter<'?, T>
}
```
which we create so that `iter` is constructed from a slice of buffer. What should the lifetime parameter `'?` be? There are traditionally three choices:
- introduce a new lifetime parameter
- replace all self-references with pointers
- use `'static` and transmute our lifetime into it

First, let's explain why none of these really work, then show the fourth option, proposed in this RFC.

Introducing a new lifetime parameter has some problems:
```rust
struct ArrayIter<'a, T> {
    iter: std::slice::Iter<'a, T>
    buffer: [T; 32]
}
```
while this can work to set your iter up and potentially implement methods on ArrayIter, 'a has no meaning to someone consuming this struct. what do they instantiate this lifetime as? there is not a scope to which this lifetime has any meaningful connection, so it really pollutes your type.

Replacing all self-references with pointers works, but not when you are not the implementor of the type which uses the lifetime.
```rust
struct ArrayIter<T> {
    iter: MyPointerBasedReimplementationOfIter
    buffer: [T; 32]
}
```
This approach is unreasonable for all but the simplest borrowing types, as it requires you to fully re-implement anything intended for use with references to work in terms of pointers.

Using the `'static` lifetime almost works, but has one important failing:
```rust
struct ArrayIter<T> {
    iter: std::slice::Iter<'static, T>
    buffer: [T; 32]
}
```
What if T is not `'static`? using the static lifetime here restricts our generic parameter T to being 'static, which is a concession we may not be ok with making. It turns out this is extremely pervasive. Even if we went with the nuclear approach:
```rust
#![feature(generic_const_exprs)]

struct ArrayIter<T> {
    iter: [u8; std::mem::size_of::<std::slice::Iter<'static, T>>()]
    buffer: [T; 32]
}
```
and just transmuted whenever needed, this still requires T to be `'static.`

So how do we get all of the above? We use the unsafe lifetime `'erased`
```rust
struct ArrayIter<T> {
    iter: ManuallyDrop<std::slice::Iter<'erased, T>>
    buffer: [T; 32]
}

impl Drop for ArrayIter<T> {
    fn drop(&mut self) {
        let iter: &mut ManuallyDrop<Y<'_>> = unsafe { core::mem::transmute(&mut self.iter) };
        iter.drop();
    }
}
```

Note that, like `'static`, `'erased` is allowed to appear in struct definitions without being declared. This is because the unsafe lifetime, like`'static`, is a specific lifetime and not a generic parameter.`'erased` can be thought of as "a lifetime which is outlived by all possible lifetimes. Dereferencing a reference with the unsafe lifetime is unsafe. Additionally, `'erased` is not acceptable when `'a` is expected because it never lives long enough.

In general replacing a real lifetime with `'erased` should be thought of as a similar transformation to replacing a reference with a pointer. If you are doing it, you are doing it because safe rust does not allow for the type of code you are trying to write, and you're trying to encapsulate the unsafe into a compact part of your code.

If you try to call a function which has a lifetime specifier (whether or not it has been elided in the signature) using a `'erased` lifetime you will get a compilation error, because any lifetime that the borrow checker could possibly choose for this call to your function would outlive a `'erased` reference.

The addition of the `'erased` lifetime also means the addition of two new reference types, `&'erased T` and `&'erased mut T`. These are in a sense halfway in between references and pointers. Dereferencing them is unsafe. `'static` references can be coerced into `'a` references, which can be coerced into `'erased` references, which can be coerced into raw pointers.

Now we also need to be able to actually produce a value to assign to our `std::slice::Iter<'erased, T>` field, which can be done by assigning to a value of the same type, except with a normal lifetime in place of unsafe.

Wrapping in `ManuallyDrop` is required when using a type with `'erased` substituted for a lifetime parameter, and so to drop the iter we need a drop impl

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The unsafe lifetime is unique in a few ways, which we will refer to as the "rules":
1. It is outlived by all other lifetimes (in the same way that `'static` outlives all other lifetimes).
    - `'a: 'erased` for *all* `'a`
    - `'erased: 'a` for *no* `'a`
2. The check that a reference does not outlive borrowed content is skipped when the borrowed content has `'erased` lifetime.
3. Assigning to or reading from `&'erased T` and `&'erased mut T` is unsafe.
    - The programmer must ensure the references must be valid, or this is UB. 
4. Similar to union fields, the types of owned values which have a generic lifetime parameter instantiated to `'erased` must either:
    - be of the shape `ManuallyDrop<_>`
    - have a Drop impl specifically compatible with `'erased` for that lifetime parameter.

Let's examine some of the implications of these features.

## Rule 1
### Coercion
If we want to get a type `T<'erased, 'b>` amd we have a type `T<'a, 'b>`, we can simply coerce `T<'a, 'b>` into `T<'erased, 'b>`

in this example our coercion target is simply a `&'erased usize`
```rust
struct A {
    // references are COPY, so rule 4 is satisfied
    val: &'erased usize
}

impl A {
    fn store(&mut self, some_ref: &usize) {
        // assigning to val, not through val.
        // note that '_: 'erased by rule 1a, so this is allowed.
        self.val = some_ref;
    }
}
```

### Unusability
`'erased` references cannot be used where normal references are expected, because the borrow checker must negotiate a lifetime between the two, but any lifetime that can be assigned to `'a` necessarily outlives `'erased`. This is very important for preventing `'erased` from making its way into old functions without clearly using unsafe code for that purpose.

```rust
fn my_existing_function(&usize) { ... }

fn main() {
    let a = 17;
    let b: &usize = &a;
    let c: &'erased usize = &a;

    // this is fine of course
    my_existing_function(b)
    // this fails because the lifetime of c ('erased) does not satisfy '_ by rule 1b
    my_existing_function(c)
}
```

## Rule 2

lets examine the first example from rule 1 again:
```rust
struct A {
    val: &'erased usize
}

impl A {
    fn store(&mut self, some_ref: &usize) {
        self.val = some_ref;
    }
}
```
An important but subtle point here is that it is only legal to have `&mut self` at all because of rule 2. Without it all references to A would outlive their borrowed content.

## Rule 3
This is clearly required as we've thrown away the lifetime information, so we need to assure the compiler that the type is still valid. This is the same idea as casting a ref to a pointer, then needing to use unsafe to dereference.

```rust
impl A {
    // illegal
    fn read_illegal(&self) -> usize {
        *self.val // throws error due to rule 4
    }

    // legal, requiring unsafe.
    fn read_legal(&self) -> usize {
        unsafe { *self.val }
    }
}
```

## Rule 4
Drop poses a problem for types which have had lifetimes replaced with `'erased`. Consider the following:

```rust
struct X {
    y: Y<'erased>
}

struct Y<'a> {
    some_val: &'a usize
}

impl<'a> Drop for Y<'a> {
    fn drop(&mut self) { ... }
}
```

What should happen when an `X` is dropped? we need to drop `y`, but we have a problem because in order to drop `y`, we need to call `drop`, but drop is implemented for `Y<'a>`, which means `Y<'erased>` doesn't satisfy the impl block, even though it may still need to run it. There are two approaches we can use:

### ManuallyDrop
if we want all the special logic to be contained in X, which is often the case when writing self referential code, we use the `ManuallyDrop` approach:
```rust
struct X {
    y: ManuallyDrop<Y<'erased>>
}

impl Drop for X {
    fn drop(&mut self) { 
        // we need to be sure that our lifetime is valid here.
        let y: &mut ManuallyDrop<Y<'_>> = unsafe { core::mem::transmute(&mut self.y) };
        y.drop();
    }
}

struct Y<'a> {
    some_val: &'a usize
}

impl<'a> Drop for Y<'a> {
    fn drop(&mut self) { ... }
}
```
note that y is now of the shape `ManuallyDrop<_>`, and that we had to use unsafe to properly call the drop, because we needed a `ManuallyDrop<Y<'_>>`, not a `ManuallyDrop<Y<'erased>>`

### Impl Drop
If we want the type `Y` to be especially convenient for use with the unsafe lifetime, we can use the second approach:
```rust
struct X {
    y: Y<'erased>
}

struct Y<'a> {
    some_val: &'a usize
}

impl Drop for Y<'erased> {
    fn drop(&mut self) { ... }
}
```
If Y doesn't need to dereference some_val in its drop implementation then `'erased` is all it needs, and the default drop behavior on `X` is fine.

# Drawbacks
[drawbacks]: #drawbacks

- One more pointer type is potentially confusing.
- Potentially a trap for newer rust developers to just declare structs with unsafe lifetimes without realizing this is just as dangerous as throwing raw pointers around.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- There isn't any analogue in the higher type system to the lifetime erasure that occurs when coalescing from reference to pointer.

## Alternatives
### Do nothing
It is not technically possible to have self reference to non `'static` types unless you reimplement with pointers, but this is also a niche thing to want to do. adding language complexity may not be worth it for this niche.

### Add special size_of operator syntax
Add a way to perform `std::mem::size_of::<SomeType<'?, T>>()` as a const fn. If there is a way to get the size of a type regardless of its lifetime specifiers in a const context, then everything here is possible (with use of the `generic_const_exprs` feature). The following would work:
```rust
#![feature(generic_const_exprs)]

use core::pin::Pin;
use core::slice::Iter;

// the only reason we need this 'static bound is because there is no way to ask rust
// what the size of Iter<'anything, T> is, which doesn't change depending on the lifetime.
// if we could do core::mem::size_of::<Iter<'whatever, T>>(), we would not need the bound on T.
struct Test<T>
where [u8; core::mem::size_of::<Iter<'?, T>>()]: Sized
{
    // this is an Iter<'self, T> with the type fully erased.
    // this is absurdly unsafe.
    my_iter: [u8; core::mem::size_of::<Iter<'?, T>>()],

    // this must be treated as immutably borrowed.
    my_vec: Vec<T>,
}

impl<T> Test<T>
where [u8; core::mem::size_of::<Iter<'?, T>>()]: Sized
{
    // using this function is UB if init is not called before any other methods, or before dropping.
    pub unsafe fn new(my_vec: Vec<T>) -> Self {
        Self {
            my_iter: [0; core::mem::size_of::<Iter<'?, T>>()],
            my_vec,
        }
    }
    
    pub fn init(self: Pin<&mut Self>) {
        let this = unsafe { self.get_unchecked_mut() };
        let iter = this.my_vec.iter();
        this.my_iter = unsafe {core::mem::transmute(iter) };;
    }
    
    fn get_my_iter(&mut self) -> &mut Iter<'_, T> {
        unsafe { core::mem::transmute(&mut self.my_iter) }
    }
    
    pub fn next(self: Pin<&mut Self>) -> Option<&T> {
        let this = unsafe { self.get_unchecked_mut() };
        this.get_my_iter().next()
    }
}

impl<T> Drop for Test<T>

where [u8; core::mem::size_of::<Iter<'?, T>>()]: Sized{
    fn drop(&mut self) {
        unsafe {
            let iter: &mut core::mem::ManuallyDrop<Iter<'_, T>> = core::mem::transmute(&mut self.my_iter);
            core::mem::ManuallyDrop::drop(iter);
        }
    }
}
```

### Add some sort of `'self` lifetime

As the primary use case this is trying to support is self reference, maybe it should just be made explicit. I suspect this would work similarly to `'erased` as it has been layed out here, just with a more restricted domain.

# Prior art
[prior-art]: #prior-art

previous rfc:
[unsafe lifetime by jpernst](https://github.com/rust-lang/rfcs/pull/1918/)
Many things are similar between these two RFCs, but there are a couple important differences that solve the problems with the first unsafe lifetime attempt:
- Instead of `'erased` satisfying *all* constraints, it satisfies *no* constraints. This makes it so existing functions cannot be called with unsafe lifetimes.
- operations which create values whose types contain `'erased` are generally all safe. These values are mostly unusable unless they are transmuted back to a useful lifetime, which is unsafe.

Some crates for dealing with self reference:
- [owning_ref](https://crates.io/crates/owning_ref) is an early attempt to make self references ergonomic but is slightly clunky and not generic enough for some use cases.
- [Oroboros](https://docs.rs/ouroboros/0.13.0/ouroboros/attr.self_referencing.html) is a crate designed to facilitate self-referential struct construction via macros. It is limited to references, and attempts to avoid unsafe.
- [rental](https://docs.rs/rental/0.5.6/rental/), another macro based approach, is limited in a few ways and is somewhat clunky to use.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- While this RFC does aim to make self-referential structs more possible, it does not aim to make them common or even easy. Pinning is not addressed at all, and must be well understood for any self-referential struct, and this RFC aims to be compatable with and separate from the pinning API.

# Future possibilities
[future-possibilities]: #future-possibilities

This example may solve some of the problems that are currently solved by `#[may_dangle]`:
```rust
struct Y<'a> {
    some_val: &'a usize
}

impl Drop for Y<'erased> {
    fn drop(&mut self) { ... }
}
```

Adding more permissive `Drop` impls throughout the standard library may make these types more ergonomic with no effect to code which does not use the `'erased` lifetime.

This may be valuable to ffi if you know that a pointer is aligned, as using `&'erased` may be more appropriate in this scenario
