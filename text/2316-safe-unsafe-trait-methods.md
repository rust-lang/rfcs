- Feature Name: safe_unsafe_trait_methods
- Start Date: 2018-01-30
- RFC PR: [rust-lang/rfcs#2316](https://github.com/rust-lang/rfcs/pull/2316)
- Rust Issue: [rust-lang/rust#87919](https://github.com/rust-lang/rust/issues/87919)

# Summary
[summary]: #summary

This RFC allows safe implementations of `unsafe` trait methods.
An `impl` may implement a trait with methods marked as `unsafe` without
marking the methods in the `impl` as `unsafe`. This is referred to as
*overconstraining* the method in the `impl`. When the trait's `unsafe`
method is called on a specific type where the method is known to be safe,
that call does not require an `unsafe` block.

# Motivation
[motivation]: #motivation

A trait which includes unsafe methods in its definition permits its impls to
define methods as unsafe. Safe methods may use `unsafe { .. }` blocks inside
them and so both safe and `unsafe` methods may use unsafe code internally.

The key difference between safe and unsafe methods is the same as that
between safe and unsafe functions. Namely, that calling a safe method with
inputs and state produced by other safe methods never leads to memory
unsafety, while calling a method marked as `unsafe` may lead to such unsafety.
As such, it is up to the caller of the `unsafe` method to fulfill a set of
invariants as defined by the trait's documentation (the contract).

The safe parts of Rust constitute a language which is a subset of unsafe Rust.
As such, it is always permissible to use the safe subset within unsafe contexts.
This is currently however not fully recognized by the language as `unsafe` trait
methods must be marked as `unsafe` in `impl`s even if the method bodies in such
an `impl` uses no unsafe code. This is can currently be overcome by defining a
safe free function or inherent method somewhere else and then simply delegate
to that function or method. Such a solution, however, has two problems.

## 1. Needless complexity and poor ergonomics.

When an `unsafe` method doesn't rely on any unsafe invariants, it still
must be marked `unsafe`. Marking methods as `unsafe` increases the amount of
scrutiny necessary during code-review. Extra care must be given to ensure that
uses of the function are correct. Additionally, usage of `unsafe` functions
inside an `unsafe` method does not require an `unsafe` block, so the method
implementation itself requires extra scrutiny.

One way to avoid this is to break out the internals of the method into a
separate safe function. Creating a separate function which is only used
at a single place is cumbersome, and does not encourage the keeping of
`unsafe` to a minimum. The edit distance is also somewhat increased.

## 2. `unsafe` method `impl`s might not require any `unsafe` invariants

The implemented trait method for that specific type, which you know only has
a safe implementation and does not really need `unsafe`, can't be used in a
safe context. This invites the use of an `unsafe { .. }` block in that context,
which is unfortunate since the compiler could know that the method is really
safe for that specific type.

## In summation

The changes proposed in this RFC are intended to increase ergonomics and
encourage keeping `unsafe` to a minimum. By doing so, a small push in favor
of correctness is made.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Concretely, this RFC will permit scenarios like the following:

## *Overconstraining*

First consider a trait with one or more unsafe methods.
For simplicity, we consider the case with one method as in:

```rust
trait Foo {
    unsafe fn foo_computation(&self) -> u8;
}
```

You now define a type:

```rust
struct Bar;
```

and you implement `Foo` for `Bar` like so:

```rust
impl Foo for Bar {
    // unsafe <-- Not necessary anymore.
    fn foo_computation(&self) -> u8 { 0 }
}
```

Before this RFC, you would get the following error message:

```
error[E0053]: method `foo_computation` has an incompatible type for trait
  --> src/main.rs:11:5
   |
4  |     unsafe fn foo_computation(&self) -> u8;
   |     --------------------------------------- type in trait
...
11 |     fn foo_computation(&self) -> u8 { 0 }
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ expected unsafe fn, found normal fn
   |
   = note: expected type `unsafe fn(&Bar) -> u8`
              found type `fn(&Bar) -> u8`
```

But with this RFC implemented, you will no longer get an error in this case.

This general approach of giving up (restricting) capabilities that a trait
provides to you, such as the ability to rely on caller-upheld invariants
for memory safety, is known as *overconstraining*.

## Taking advantage of *overconstraining*

You now want to use `.foo_computation()` for `Bar`, and proceed to do so as in:

```rust
fn main() {
    // unsafe { <-- no unsafe needed!

    let bar = Bar;
    let val = bar.foo_computation();

    // other stuff..

    // }
}
```

This is permitted since although `foo_computation` is an `unsafe` method as
specified by `Foo`, the compiler knows that for the specific concrete type `Bar`,
it is defined as being safe, and may thus be called within a safe context.

## Regarding API stability and breaking changes

Note however, that the ability to call *overconstrained* methods with
the absence of `unsafe` in a safe context means that introducing `unsafe`
later is a breaking change if the type is part of a public API.

## Impls for generic types

Consider the type `Result<T, E>` in the standard library defined as:

```rust
pub enum Result<T, E> {
    Ok(T),
    Err(E),
}
```

Let's now implement `Foo` for `Result<T, E>` without using `unsafe`:

```rust
impl<T, E> Foo for Result<T, E> {
    fn foo_computation(&self) -> u8 {
        // Let's assume the implementation does something interesting..
        match *self {
            Ok(_) => 0,
            Err(_) => 1,
        }
    }
}
```

Since `Result<T, E>` did not use `unsafe` in its implementation of `Foo`, you
can still use `my_result.foo_computation()` in a safe context as shown above.

## Recommendations

If you do not plan on introducing `unsafe` for a trait implementation of
your specific type that is part of a public API, you should avoid marking
the `fn` as `unsafe`. If the type is internal to your crate, you should
henceforth never mark it as `unsafe` unless you need to. If your needs
change later, you can always mark impls for internal types as `unsafe` then.

Tools such as `clippy` should preferably lint for use of `unsafe`,
where it is not needed, to promote the reduction of needless `unsafe`.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Assuming a `trait` which defines some `fn`s marked as `unsafe`, an `impl`
of that trait for a given type may elect to not mark those `fn`s as `unsafe`
in which case the bodies of those `fn`s in that `impl` are type checked as
safe and not as `unsafe`. A Rust compiler will keep track of whether the
methods were implemented as safe or `unsafe`.

When a trait method is called for a type in a safe context, the type checker
will resolve the `impl` for a specific known and concrete type. If the `impl`
that was resolved implemented the called method without an `unsafe` marker,
the compiler will permit the call. Otherwise, the compiler will emit an error
since it can't guarantee that the implementation was marked as safe.

With respect to a trait bound on a type parameter `T: Trait` for a trait with
unsafe methods, calling any method of `Trait` marked as `unsafe` for `T` is
only permitted within an `unsafe` context such as an `unsafe fn` or within an
`unsafe { .. }` block.

# Drawbacks
[drawbacks]: #drawbacks

While this introduces no additional syntax, it makes the rule-set of the
language a bit more complex for both the compiler and the for users of the
language. The largest additional complexity is probably for the compiler
in this case, as additional state needs to be kept to check if the method
was marked as safe or `unsafe` for an `impl`.

# Rationale and alternatives
[alternatives]: #alternatives

[RFC 2237]: https://github.com/rust-lang/rfcs/pull/2237

This RFC was designed with the goal of keeping the language compatible
with potential future effects-polymorphism features. In particular, the
discussion and design of [RFC 2237] was considered. No issues were found
with respect to that RFC.

No other alternatives have been considered. There is always the obvious
alternative of not implementing the changes proposed in any RFC. For this RFC,
the impact of not accepting it would be too keep the problems as explained
in the [motivation] around.

# Unresolved questions
[unresolved]: #unresolved-questions

There are currently no unresolved questions.