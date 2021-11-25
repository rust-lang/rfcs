- Feature Name: `unchecked_lifetime`
- Start Date: 2021-11-24
- RFC PR: [rust-lang/rfcs#3199](https://github.com/rust-lang/rfcs/pull/3199)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Introduce a new special lifetime `'?` representing an "unchecked" lifetime which is outlived by all other lifetimes. Most uses of types with this lifetime are then unsafe, as detailed in this RFC.

# Motivation
[motivation]: #motivation

When creating self referential structs it is often preferred to use pointers over references because the conditions under which the pointer/reference is valid are not evaluated by the borrow checker. The problem with this general approach is that it does not scale well to more complex types. If we have a the following:
```rust
struct A<T> {
    item: T
    borrower: B<???> // we want the ref inside this to refer to item
}

struct B<'a, T> {
    actual_ref: &'a T
}
```
there is no choice for a lifetime to replace `???` with because `'static` may outlive `T` if it contains lifetimes, and we may not want to replace the ref inside `B` with a pointer, because `B` may have value apart from being stored in a self reference.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

There are situations where, when writing unsafe code, you may need to store a type without encoding its lifetime in the type system. The existence of raw pointers in the language is an acknowledgement of this need, but it is not always perfectly ergonomic to use pointers in this scenario. Consider the following self referential struct:
```rust
struct ArrayIter<T> {
    buffer: [T; 32]
    iter: std::slice::Iter<???, T>
}
```
which we create so that `iter` is constructed from a slice of buffer. What should the lifetime parameter `???` be? There are traditionally three choices:
- introduce a new lifetime parameter
- replace all self references with pointers
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

Replacing all self references with pointers works, but not when you are not the implementor of the type which uses the lifetime.
```rust
struct ArrayIter<T> {
    buffer: [T; 32]
    iter: MyPointerBasedIterType
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

So how do we get all of the above? We use the "unchecked lifetime" `'?`
```rust
struct ArrayIter<T> {
    buffer: [T; 32]
    iter: std::slice::Iter<'?, T>
}
```

Note that, like `'static`, `'?` is allowed to appear in struct definitions without being declared. This is because the unchecked lifetime instructs the borrow checker to treat any references with this lifetime like raw pointers. This is very unsafe of course, so as a tradeoff, dereferencing a reference with the unchecked lifetime is unsafe, and `'?` is not acceptable when `'a` is expected.

In general using replacing a real lifetime with `'?` should be thought of as a similar transformation to replacing a reference with a pointer. If you are doing it, you are doing it because safe rust does not allow for the type of code you are trying to write, and you're trying to encapsulate the unsafe into a compact part of your code.

If you try to call a method whose arguments or return value include `'?`, that call will need to be wrapped in unsafe, because you are asserting that you know those references are valid despite the borrow checker not knowing.

The addition of the `'?` lifetime also means the addition of two new reference types, `&'? T` and `&'? mut T`. These are in a sense halfway in between references and pointers. Dereferencing them is unsafe. Static references can be coerced into normal references, which can be coerced into unchecked-lifetime references, which can be coerced into raw pointers. The crucial difference between `&'? T` and `*const T` is that it is considered unsound for `&'? T` to be unaligned at any time, instead of only at the time of dereference for raw pointers.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The unchecked lifetime is unique in a few ways:
- It is outlived by all other lifetimes (in the same way that `'static` outlives all other lifetimes).
- The check that a reference does not outlive borrowed content is skipped when the borrowed content has `'?` lifetime. (This is the check that goes *unchecked*).
- It cannot be used where a normal reference is expected.
- assigning to or reading from `&'? T` and `&'? mut T` is unsafe

Let's examine some of the implications of these features.

It is safe in general to store into a value expecting `'?`, just like with raw pointers

```rust
struct A {
    val: &'? usize
}

impl A {
    fn store(&self, some_ref: &usize) {
        self.val = some_ref; // assigning to val, not through val.
    }
}
```

The only way to use a type that has been stored with `&'?` is to transmute it to a normal lifetime, which of course requires unsafe.
```rust
impl A {
    // illegal
    fn read(&self) -> usize {
        *self.val // throws error due to rule 4
    }

    // legal, requiring unsafe.
    fn read(&self) -> usize {
        unsafe { *self.val }
    }
}
```

# Drawbacks
[drawbacks]: #drawbacks

- One more pointer type is potentially confusing.
- Potentially a trap for newer rust developers to just declare structs with unchecked lifetimes without realizing this is just as dangerous as throwing raw pointers around.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- There isn't any analogue in the higher type system to the lifetime erasure that occurs when coalescing from reference to pointer.
- An alternative could be a macro-based library. This approach seems unlikely to check all the boxes though

# Prior art
[prior-art]: #prior-art

- [Oroboros](https://docs.rs/ouroboros/0.13.0/ouroboros/attr.self_referencing.html) is a crate designed to facilitate self referential struct construction via macros. It is limited to references, and attempts to avoid unsafe.
- [rental](https://docs.rs/rental/0.5.6/rental/), another macro based approach, is limited in a few ways and is somewhat clunky to use.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- While this RFC does aim to make self referential structs more possible, it does not aim to make them common or even easy. Pinning is not addressed at all, and must be well understood for any self referential struct, and this RFC aims to be compatable with and separate from the pinning API.

# Future possibilities
[future-possibilities]: #future-possibilities

This may be valuable to ffi if you know that a pointer is aligned, as then using `&'?` may be more appropriate in this scenario
