- Feature Name: unsized-expr
- Start Date: Sat Mar 14 19:56:22 CET 2015
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Add the concept of unsized return values.

# Motivation

In Rust, functions should almost never return heap-allocated structures. This is
an anti-pattern:

```rust
fn f() -> Box<X> { /* ... */ }
```

Instead, they should return the structure by value which allows the caller to
decide where to store it:

```rust
fn f() -> X { /* ... */ }

// On the stack:
let x: X = f();

// On the heap:
let x: Box<X> = box f();
```

Note that the `box f()` syntax should be just as fast as the function that
returns a `Box<X>` because the caller will pass a hidden pointer to the
pre-allocated heap-memory to `f`.

[boxrfc]: https://github.com/rust-lang/rfcs/blob/master/text/0809-box-and-in-for-stdlib.md

[RFC #809][boxrfc] greatly extended the power of `box` by removing the special
case for `Box` and turning it into a more general protocol. It also added the
`in` keyword that allows the user to specify precisely where the memory should
be allocated. In the future, all of the following should be possible:

```rust
let x: Box<X> = box f();
let mut y: Vec<X> = box f();

// Appends the result of `f()` to `y`
in y.append() { f() }
```

Note that the last line will be faster than `y.push(f())` because it passes a
pre-allocated slot in `y` to `f` so that we avoid a copy.

With this protocol in place, we can extend the general idea of "never box return
values" to unsized types. This RFC introduces the following syntax:

```rust
fn f() -> [u8] { /* ... */ }
```

Which can be used like this:

```rust
let x: Box<[u8]> = box f();
let mut y: Vec<u8> = box f();

// Appends the result of `f()` to `y`
in y.append() { f() }
```

Similarly for traits:

```rust
fn f() -> fmt::Debug { /* ... */ }

let x: Box<fmt::Debug> = box f();
```

Note that the return value of `f()` is not restricted to a single implementation
of `fmt::Debug`, i.e., the following code is valid:

```rust
fn f(flag: bool) -> fmt::Debug {
    if flag {
        *"hello"
    } else {
        1234
    }
}
```

Here we also used the implicit conversions `str -> fmt::Debug` and
`i32 -> fmt::Debug`.

(Note that this concept is orthogonal to the proposed `f() -> impl Trait`
syntax. Functions declared with an `impl Trait` return type will return one
concrete implementation of the trait and the return value can be stored on the
stack.)

This gives the user much greater control over allocations. The user can decide
freely if they want a `Trait` to be returned in a `Rc<Trait>`, `Arc<Trait>`, or
`Box<Trait>`. They can also decide if they want a slice to be appended to a
vector or if they want a completely new vector to be allocated.

## New concepts

It's also interesting to see that this syntax allows us to express new concepts
in safe Rust that could previously only be expressed with unsafe code.

Consider the following concept:

** The class of types that can be turned into a slice **

With the proposed syntax, this can be expressed as follows:

```rust
trait IntoSlice<T> {
    fn into_slice(self) -> [T];
}

impl<'a, T: Copy> IntoSlice<T> for &'a [T] {
    fn into_slice(self) -> [T] { self[..] }
}

impl<T> IntoSlice<T> for Box<[T]> {
    fn into_slice(self) -> [T] { *self }
}
```

It is not possible to have one trait that encapsulates both of these
implementations safely without the `[T]` syntax. Let's see why:

First of all, since we want the trait to be implemented for all `Box<[T]>`, not
just those `T` that implement `Copy`, the trait has to take `self` by value.
Since the return value has to work for `&'a [T]`, the return value has to be
`&'a [T]`. Therefore the trait has to look like this:

```rust
trait IntoSlice<'a, T> {
    fn into_slice(self) -> &'a [T];
}
```

But this cannot be implemented for `Box<[T]>` because we either leak the heap
allocation or return a reference to an already deallocated slice.

(For similar reasons it's not possible to implement the trait on
`&mut Box<[T]>`)

# Detailed design

Allow the return types of functions to be unsized.

Allow the type of an expression to be unsized if it is *founded in* a `box` or
`in` expression and the allocator accepts the unsized type.

Not all allocators accept all unsized types, e.g., `Box` can hold both slices
and traits but `Vec` can only hold slices.

An expression is *founded in* a `box` or `in` expression if it is either used
directly in a `box` or `in` expression, or if it is used as the value of an
expression which is founded in a `box` or `in` expression, or if it is used as
the return value of a function.

Some examples of valid expressions and functions:

```rust
box *"";
box { *"" };
box { { *"" } };

fn f(n: usize) -> str {
    if n == 0 {
        *""
    } else {
        f()
    }
}
```

The following implementation is not necessarily good but is documented here to
show the feasibility of the concept:

## Implementation

If a function returns an unsized type, it is passed a hidden pointer to the
allocator which is used to store the return value. Once the return value has
been determined, the function asks the allocator to allocate an appropriate
amount of memory and stores the return value in there. The allocator also
receives the necessary metadata: The length of the slice for slices and the
vtable pointer for traits.

# Drawbacks

None right now.

# Alternatives

None right now.

## What other designs have been considered?

None.

## What is the impact of not doing this?

Many unnecessary allocations and deallocations and a decreased expressiveness.

## What is the impact of not doing this **right now**?

This potentially affects the interface of all functions that currently return a
vector.

# Unresolved questions

None right now.
