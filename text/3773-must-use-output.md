- Feature Name: `must-use-output`
- Start Date: 2025-02-16
- RFC PR: [rust-lang/rfcs#3773](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Allow adding a `#[must_use_output]` attribute on function parameters that triggers a warning if the function was called with a reference to a value that is later not used (only dropped). Also add the attribute to relevant library functions such as `Vec::push`.

# Motivation
[motivation]: #motivation

Today, if you write this code in Rust:

```rust
fn main() {
    let mut x = 0;
    x += 42;
}
```

you'll get a warning about unused write. It's likely that either you forgot to use the value of `x` or you wrote a needless operation that has no meaningful effect on the program.

However this code:

```rust
fn main() {
    let mut vec = Vec::new();
    vec.push(42);
}
```

has a similar problem and yet it doesn't trigger a warning. This can be even be dangerous in some situations; for instance, if the `Vec` is collecting multiple errors (to report messages similar to `rustc`) and the code is supposed to check if the `Vec` is empty before proceeding.

In some cases there are even multiple arguments that should be accessed after a call to a function. For instance `core::mem::swap` is almost always useless if both arguments are not accessed - if none of them are it only changes the drop order which doesn't matter in the vast majority of the code, and if only one of them is it's better to use a simple assignment rather than swap.

Note also that some functions may be useless to call even when they take in truly immutable (`Freeze`) references. For instance `clone` is wasteful if the original is not accessed.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

You may use the `#[must_use_output]` attribute in front of any function argument with a reference type (`&T` or `&mut T`) to indicate to the caller that calling the function counts as writing to the referenced value for the purposes of checking for useless writes, as well as a `#[must_use]` warning if the referenced value is not later accessed. This can also be an indicator of other bugs.

As an example, this code:

```rust
fn main() {
    let mut vec = Vec::new();
    vec.push(42);
}
```

will emit a warning saying "`vec` is not used after the call to `push`" because `Vec::push` has `self` marked with this attribute:

```rust
impl<T> Vec<T> {
    pub fn push(#[must_use_output] &mut self, item: T) { /* ... */ }
}
```

This is conceptually similar to `#[must_use]` on the function, but for an output parameter rather than the return value. Like `#[must_use]`, it helps you (or downstream consumers) to catch likely mistakes.

A common case when this is useful is when the function has no side effects other than mutation through the passed reference and allocation. A typical example is methods modifying collections, builders or similar objects.

Note that this is currently subject to some limitations: if the reference was passed in as an argument, returned from another function or obtained from a pointer the warning will not trigger. This may change in the future and it may start producing more warnings in some cases.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Whenever a function parameter is marked with `#[must_use_output]`, and the caller does not later access the value, a warning is emitted similar to using the `+=` operator on integers and later not using the value.

It is an error to put this attribute on parameters with types other than reference types. Raw pointers, smart pointers, and generics that hide the reference, are forbidden, mainly for simplicity of initial implementation. They can be implemented and allowed later, if it's even possible at all.

The compiler makes no distinction whether the reference is unique or shared because writes can happen through both; there doesn't seem to be a reason to forbid `Freeze` references and there is at least one case when even a `Freeze` reference is useful.

The standard library functions get annotated with this attribute as well, including but not limited to:

- Modifications of collections that don't return a value
- `core::mem::swap`
- `Clone::clone`
- `*_assign` methods in `core::ops` traits
- `OpenOptions` and similar builders

# Drawbacks
[drawbacks]: #drawbacks

- Adds another attribute to the language, complicating it
- In some cases analysis will fail and perhaps people will have false sene of security
- People could misuse it to attempt to enforce soundness of `unsafe` code (which it cannot do)

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- We could also simply not do this but the potential mistakes it catches are real.
- The name could be something different and preferably shorter. The name used here was suggested by Josh Triplett and is pretty readable and it can be used before stabilization in the absence of better one. A pretty short and clear `#[must_read]` was also suggested by Kevin Reid.
- We could write `#[must_use]` on a reference parameter instead. The downside would be that this could be mistaken for saying that the *callee* must use the parameter, rather than the *caller*.
- Make it a `clippy` lint instead. However not everyone uses `clippy` and the need to communicate which arguments are considered important would diminish its effect. `#[must_use]` is a native rustc lint, and this should be as well, using the same machinery.
- Try to somehow analyze the code and not require the attribute. This seems hard and it could lead into libraries accidentally triggering warnings in users code if they change the body of the function.
- Implement a more full feature — e.g. by doing some things in "Future possibilities" section. However, this feature should be useful even without them.
- Have the attribute on the function instead listing the names of paramters. This could make it nicer to extend to support the "or" relationship described in "Future possibilities".

# Prior art
[prior-art]: #prior-art

I'm not aware of anything other than the unused write warning and `#[must_use]` attribute which are somewhat related.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Attribute name.
- Perhaps some limitations are not as difficult as I imagine and they could be lifted right away.
- Should we emit a warning if a function parameter does not have that attribute but is passed to a function with the attribute to encourage propagation?

# Future possibilities
[future-possibilities]: #future-possibilities

A custom message could be added just like `#[must_use = "..."]` is already available.

The limitations above might be lifted somehow. For instance I think that it'd be useful to also emit a warning if the reference was obtained from a function parameter that is itself *not* marked `#[must_use_output]`.

Have a way to explain to the compiler how smart pointers work so that this can be used on `Pin` as well.

Have an attribute that can express "or" relationship between parameters and return types. For instance, `Vec::pop` is sometimes used to remove the last element and then pass the vec to something else (e.g. trim `\n` at the end) and sometimes it's used to get the last value and the vec is no longer relevant. Someting like `#[must_use_at_least_one(self, return)]` on function could handle this case.
