- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Modify the `||` expression sugar so that it can expand to either `F`,
`&F`, or `&mut F`, where `F` is a fresh struct type implementing one
of the `Fn`/`FnMut`/`FnOnce` traits.

# Motivation

There are numerous reasons that one might prefer to write a function
that takes a closure object (e.g., `&mut FnMut()`) rather than having
that function be generic over the closure type (e.g., `F` where
`F:FnMut()`). For example, closure objects reduce code bloat, they
work better with object safety restrictions, and they avoid infinite
monomorphic expansion for recursion functions.

Unfortunatelly, if one writes such a function in the natural way
today, it introduces an ergonomic speed bump for callers:

```rust
fn do_something(closure: &mut FnMut(i32) -> i32) {
    ...
}
```

Anyone who wishes to call `do_something()` will have to echo the `&mut` on the caller side:

```rust
do_something(&mut |x| x*2)
```

As you can see, for simple closures, the `&mut` can easily outweigh
the closure body itself.

The problem arises because today the `||` expression always expands to
an instance of some fresh struct type `F` representing the
environment. This RFC proposes to allow `||` to expand to either `&F`,
`&mut F`, or `F`, depending on the *expected type*. This would mean
that the call to `do_something` could be written as `do_something(|x| x*2)`.

Informally, the *expected type* is basically the surrounding
context. We already use the expected type to infer the argument types
(we also use it, currently, to decide whether which `Fn` trait the
struct type `F` will implement, though that will hopefully be
improved). In practice the expected type information is usually
derived from the argument types declared on a function (when the
closure literal is passed as an argument to the function call).

# Detailed design

When type-checking a `||` expression, examine the expected type. If it
is a reference (either shared or mutable), introduce an auto-ref on
the result of the closure. This is fairly straightforward and builds
on the existing compiler infrastructure for doing this sort of thing.

# Drawbacks

The primary drawback is that the expansion of `||` becomes more
complicated.  Using the expected type means that some seemingly
innocous program transforms may yield unexpected errors. For example,
imagine a call to the `do_something()` function we saw before:

```rust
do_something(|x| x*2)
```

If we pull that closure out into a variable, the context changes, and there is no
expected type to use for inference. Hence, we must insert the `&mut` ourselves:

```rust
let closure = |x| x*2; // this won't actually compile, read on
do_something(&mut closure)
```

This downside is greatly mitigated, however, by the fact that we
already lean on the expected type when inferring argument types for
closures. In fact, the program above will not compile as written,
because there is no basis for inferring the parameter argument types,
which means they would require explicit annotations:

```rust
let closure = |x: i32| x*2;
do_something(&mut closure)
```

(In fact, the program would likely require an explicit `&mut:`
annotation as well so as to specify that the closure is a `FnMut`
closure, but we hope to remove the need for that annotation shortly.)

# Alternatives

The original plan was to permit DST values to be passed "by value". This would
mean that the `do_something` function could be written:

```rust
fn do_something(closure: FnMut(i32) -> i32) {
    ...
}
```

This would also avoid the need for callers to write `&mut |x|
x*2`. However, while this feature is still planned, it is not expected
to be proposed or implemented before the beta. Introducing auto-ref is
a simple measure that removes most of the ergonomic pain. It is also
forwards compatible with the ability to pass trait objects by value.

# Unresolved questions

None.
