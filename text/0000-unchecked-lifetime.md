- Feature Name: (fill me in with a unique ident, `my_awesome_feature`)
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Introduce a new special lifetime `'?` representing an "unchecked" lifetime. calling a method whose signature is generic over any "unchecked" lifetime would require an unsafe operation.

# Motivation
[motivation]: #motivation

When creating self referential structs it is often preferred to use pointers over references because the conditions under which the pointer/reference is valid are not evaluated by the borrow checker. The problem with this general approach is that it does not scale well to more complex types. If we have a the following:
```rust
struct A<T> {
    item: T
    borrower: B<'?> // we want the ref inside this to refer to item
}

struct B<'a, T> {
    actual_ref: &'a T
}
```
there is no choice for a lifetime to replace `'?` with because `'static` may outlive `T` if it contains lifetimes, and we may not want to replace the ref inside `B` with a pointer, because `B` may have value apart from being stored in a self reference.

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

Note that, like `'static`, `'?` is allowed to appear in struct definitions without being declared. This is because the unchecked lifetime instructs the borrow checker to treat any references with this lifetime like raw pointers. This is very unsafe of course, so as a tradeoff, calling any function whose signature contains `'?` (for example the `next` method of `Iter`) requires an unsafe block.

In general using replacing a real lifetime with `'?` should be thought of as a similar transformation to replacing a reference with a pointer. If you are doing it, you are doing it because safe rust does not allow for the type of code you are trying to write, and you're trying to encapsulate the unsafe into a compact part of your code.

If you try to call a method whose arguments or return value include `'?`, that call will need to be wrapped in unsafe, because you are asserting that you know those references are valid despite the borrow checker not knowing.

The addition of the `'?` lifetime also means the addition of two new reference types, `&'? T` and `&'? mut T`. These are in a sense halfway in between references and pointers. Static references can be coerced into mortal references, which can be coerced into unchecked-lifetime references, which can be coerced into raw pointers. The crucial difference between `&'? T` and `*const T` is that it is considered unsound for `&'? T` to be unaligned at any time, instead of only at the time of dereference for raw pointers.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This is the technical portion of the RFC. Explain the design in sufficient detail that:

- Its interaction with other features is clear.
- It is reasonably clear how the feature would be implemented.
- Corner cases are dissected by example.

The section should return to the examples given in the previous section, and explain more fully how the detailed proposal makes those examples work.

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

Discuss prior art, both the good and the bad, in relation to this proposal.
A few examples of what this can include are:

- [Oroboros](https://docs.rs/ouroboros/0.13.0/ouroboros/attr.self_referencing.html) is a crate designed to facilitate self referential struct construction via macros. It is limited to references, and attempts to avoid unsafe.
- [rental](https://docs.rs/rental/0.5.6/rental/), another macro based approach, is limited in a few ways and is somewhat clunky to use.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- What parts of the design do you expect to resolve through the RFC process before this gets merged?
- What parts of the design do you expect to resolve through the implementation of this feature before stabilization?
- What related issues do you consider out of scope for this RFC that could be addressed in the future independently of the solution that comes out of this RFC?

# Future possibilities
[future-possibilities]: #future-possibilities

Think about what the natural extension and evolution of your proposal would
be and how it would affect the language and project as a whole in a holistic
way. Try to use this section as a tool to more fully consider all possible
interactions with the project and language in your proposal.
Also consider how this all fits into the roadmap for the project
and of the relevant sub-team.

This is also a good place to "dump ideas", if they are out of scope for the
RFC you are writing but otherwise related.

If you have tried and cannot think of any future possibilities,
you may simply state that you cannot think of anything.

Note that having something written down in the future-possibilities section
is not a reason to accept the current or a future RFC; such notes should be
in the section on motivation or rationale in this or subsequent RFCs.
The section merely provides additional information.
