- Feature Name: `unsafe_lifetime`
- Start Date: 2021-11-24
- RFC PR: [rust-lang/rfcs#3199](https://github.com/rust-lang/rfcs/pull/3199)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Introduce a new special lifetime `'unsafe` which is outlived by all other lifetimes. Using a type through a 'unsafe reference, or which is instantiated with an 'unsafe lifetime parameter is rarely possible without unsafe.

# Motivation
[motivation]: #motivation

When creating self-referential structs it is often preferred to use pointers over references because the conditions under which the pointer/reference is valid are not evaluated by the borrow checker. The problem with this general approach is that it does not scale well to more complex types. If we have a the following:
```rust
struct A<T> {
    item: T
    borrower: B<'?, T> // we want the ref inside this to refer to item
}

struct B<'a, T> {
    actual_ref: &'a T
}
```
there is no choice for a lifetime to replace `'?` with because `'static` may outlive `T` if it contains lifetimes, and we may not want to replace the ref inside `B` with a pointer, because `B` may have value apart from being stored in a self-reference.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

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
    buffer: [T; 32]
    iter: std::slice::Iter<'a, T>
}
```
while this can work to set your iter up and potentially implement methods on ArrayIter, 'a has no meaning to someone consuming this struct. what do they instantiate this lifetime as? there is not a scope to which this lifetime has any meaningful connection, so it really pollutes your type.

Replacing all self-references with pointers works, but not when you are not the implementor of the type which uses the lifetime.
```rust
struct ArrayIter<T> {
    buffer: [T; 32]
    iter: MyPointerBasedReimplementationOfIter
}
```
This approach is unreasonable for all but the simplest borrowing types, as it requires you to fully re-implement anything intended for use with references to work in terms of pointers.

Using the `'static` lifetime almost works, but has one important failing:
```rust
struct ArrayIter<T> {
    buffer: [T; 32]
    iter: std::slice::Iter<'static, T>
}
```
What if T is not `'static`? using the static lifetime here restricts our generic parameter T to being 'static, which is a concession we may not be ok with making.

So how do we get all of the above? We use the unsafe lifetime `'unsafe`
```rust
struct ArrayIter<T> {
    buffer: [T; 32]
    iter: std::slice::Iter<'unsafe, T>
}
```

Note that, like `'static`, `'unsafe` is allowed to appear in struct definitions without being declared. This is because the unsafe lifetime, like`'static`, is a specific lifetime and not a generic parameter.`'unsafe` can be thought of as "a lifetime which is outlived by all possible lifetimes. Dereferencing a reference with the unsafe lifetime is unsafe. Additionally, `'unsafe` is not acceptable when `'a` is expected because for any lifetime`'a`, `'unsafe` does not live long enough to satisfy its requirements.

In general replacing a real lifetime with `'unsafe` should be thought of as a similar transformation to replacing a reference with a pointer. If you are doing it, you are doing it because safe rust does not allow for the type of code you are trying to write, and you're trying to encapsulate the unsafe into a compact part of your code.

If you try to call a function which has a lifetime specifier (whether or not it has been elided in the signature) using a `'unsafe` lifetime you will get a compilation error, because any lifetime that the borrow checker could possibly choose for this call to your function would outlive a `'unsafe` reference.

The addition of the `'unsafe` lifetime also means the addition of two new reference types, `&'unsafe T` and `&'unsafe mut T`. These are in a sense halfway in between references and pointers. Dereferencing them is unsafe. `'static` references can be coerced into `'a` references, which can be coerced into `'unsafe` references, which can be coerced into raw pointers.

Now we also need to be able to actually produce a value to assign to our `std::slice::Iter<'unsafe, T>` field, which can be done by assigning to a value of the same type, except with a normal lifetime in place of unsafe, though there's an important catch. Any assignment to a "place" (whether to a variable, through a mutable reference, or setting a field of a struct) is unsafe if the type of the "place" uses 'unsafe instead of a generic lifetime specifier. The user must guarantee that the lifetime replaced by 'unsafe is valid when the value contained in place is dropped.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The unsafe lifetime is unique in a few ways:
1. It is outlived by all other lifetimes (in the same way that `'static` outlives all other lifetimes).
2. The check that a reference does not outlive borrowed content is skipped when the borrowed content has `'unsafe` lifetime.
3. Assigning to or reading from `&'unsafe T` and `&'unsafe mut T` is unsafe.
    - The programmer must ensure the references must be valid, or this is UB. 
4. Assigning to a variable whose type is instantiated with `'unsafe` in place of at least one of its lifetimes is unsafe.
    - The programmer must ensure the `'unsafe` is valid when the variable is dropped (because step 5).
5. Dropping a value whose type is instantiated with `'unsafe` first transmutes the value to the same type with `'unsafe` replaced with `'_`.

One important consequence of rule 1, which we will call rule 1.5: `'unsafe` references cannot be used where normal references are expected, because the borrow checker must negotiate a lifetime between the two, but any lifetime that can be assigned to `'a` necessarily outlives `'unsafe`.

Let's examine some of the implications of these features.

It is safe in general to store into a value expecting `'unsafe`, just like with raw pointers

```rust
struct A {
    val: &'unsafe usize
}

impl A {
    fn store(&mut self, some_ref: &usize) {
        // assigning to val, not through val.
        // unsafe due to rule 4. If we were assigning through it would be rule 3.
        unsafe { self.val = some_ref; }
    }
}
```
An important but subtle point here is that it is only legal to have `&mut self` because of rule 2. Without it all references to A would outlive their borrowed content.

The only way to use a type that has been stored with `&'unsafe` is to use unsafe, or transmute it to a normal lifetime, which of course requires unsafe.
```rust
impl A {
    // illegal
    fn read_illegal(&self) -> usize {
        *self.val // throws error due to rule 4
    }

    // legal, requiring unsafe.
    fn read_deref(&self) -> usize {
        unsafe { *self.val }
    }

    // legal, requiring unsafe.
    fn read_transmute(&self) -> usize {
        *(unsafe { core::mem::transmute::<&'unsafe usize, &usize>(self.val) })
    }
}
```
Additionally, an important result of rule 1.5 is that methods defined on a reference to a type cannot be called on an unsafe reference unless they were specifically written to accept an unsafe reference, as unsafe references do not live long enough to be used where a normal reference would, like the method signature.
```rust
impl A {
    // errors
    fn get(&self) -> usize {
        self.val.clone() // clone expects &self, so this errors as val does not live long enough.
    }
}
```

# Drawbacks
[drawbacks]: #drawbacks

- One more pointer type is potentially confusing.
- Potentially a trap for newer rust developers to just declare structs with unsafe lifetimes without realizing this is just as dangerous as throwing raw pointers around.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- There isn't any analogue in the higher type system to the lifetime erasure that occurs when coalescing from reference to pointer.
- An alternative could be a macro-based library. This approach seems unlikely to check all the boxes though

# Prior art
[prior-art]: #prior-art

previous rfc:
[unsafe lifetime by jpernst](https://github.com/rust-lang/rfcs/pull/1918/)
Many things are similar between these two RFCs, but there are a couple important differences that solve the problems with the first unsafe lifetime attempt:
- Instead of `'unsafe` satisfying *all* constraints, it satisfies *no* constraints. This makes it so existing functions cannot be called with unsafe lifetimes.
- operations which create values whose types contain `'unsafe` are generally all safe. These values are mostly unusable unless they are transmuted back to a useful lifetime, which is unsafe.

Some crates for dealing with self reference:
- [owning_ref](https://crates.io/crates/owning_ref) is an early attempt to make self references ergonomic but is slightly clunky and not generic enough for some use cases.
- [Oroboros](https://docs.rs/ouroboros/0.13.0/ouroboros/attr.self_referencing.html) is a crate designed to facilitate self-referential struct construction via macros. It is limited to references, and attempts to avoid unsafe.
- [rental](https://docs.rs/rental/0.5.6/rental/), another macro based approach, is limited in a few ways and is somewhat clunky to use.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- While this RFC does aim to make self-referential structs more possible, it does not aim to make them common or even easy. Pinning is not addressed at all, and must be well understood for any self-referential struct, and this RFC aims to be compatable with and separate from the pinning API.

# Future possibilities
[future-possibilities]: #future-possibilities

This may be valuable to ffi if you know that a pointer is aligned, as then using `&'unsafe` may be more appropriate in this scenario
