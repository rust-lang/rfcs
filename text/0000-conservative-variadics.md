- Feature Name: `conservative_variadic_functions`
- Start Date: 2017-02-22
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Add a conservative form of variadic length functions (also known as varargs).
The feature focuses on the most pressing use case, a variable number of
arguments which differ in type, as this tends to require dynamic dispatch or
boxing in the language today. To remain compatible with as many future
enhancements as possible, and to avoid maintenance burden, we avoid adding any
new syntax or functionality beyond the *bare minimum* required for the feature.

# Motivation
[motivation]: #motivation

Variadic functions (and variadic generics, which are related but out of scope
for this RFC) have long been a requested feature of Rust. (See [#376] and
[#960]). Previous proposals have been rejected or not gotten off the ground
either because they were too broad, or had performance penalties that seemed out
of place for a systems language.

[#376]: https://github.com/rust-lang/rfcs/issues/376
[#960]: https://github.com/rust-lang/rfcs/issues/960

Without language level support for variadic functions, the closest alternative
is to pass a slice. There are two major drawbacks to this approach:

- If you want to take ownership of the values passed, you will have to move it
  to the heap, or require that the type be `Copy`
- The values must be of the same type. If you wish to take values which conform
  to a trait, you must pass a trait object which at minimum means dynamic
  dispatch, and could potentially mean boxing. Object safety also tends to be
  pervasive throughout a library.

To provide as conservative a feature as possible, this RFC will focus purely on
the case where the types differ. Taking ownership of a variable number of values
of the same type where that type is not `Copy` is a much smaller use case, and
one that will likely be resolved by [RFC #1909], [RFC #1915], or some other RFC
which allows `[T]` to be passed directly.

[RFC #1909]: https://github.com/rust-lang/rfcs/pull/1909
[RFC #1915]: https://github.com/rust-lang/rfcs/pull/1915

We will look at two concrete library cases which would be reflected by this RFC.

The first is the signature of [`Statement#execute` from the rust-postgres
crate](https://docs.rs/postgres/0.13.4/postgres/stmt/struct.Statement.html#method.execute).
This function takes `&[&ToSql]` as its final parameter, as an attempt to emulate
variadic functions but avoid allocation. This leads to an API which is often
difficult to use, and forces dynamic dispatch.

The second case we will look at is a [proposal by the `diesel`
crate](https://github.com/diesel-rs/diesel/pull/747) which is in a direction
similar to this RFC. Their API focuses on using tuples, which they found painful
for a variety of reasons.

Ultimately both of these crates are trying to emulate a variadic function, and
are creating APIs which are more painful to use as a result.

# Detailed design
[design]: #detailed-design

We need to answer two questions to provide variadic functions. What does
declaring a function as variadic look like, and how do we represent it?

Variadic arguments would be represented as a heterogeneous list, often known as
an hlist, which we will call a "variadic list" for this feature. The concrete
types would live in `core::ops::variadic_list`, and would be defined as:

```rust
pub struct Cons<T, U>(pub T, pub U);
pub struct Nil;
```

This RFC has chosen to introduce a new type for this, rather than attempting to
use tuples as has been proposed in the past (notably by [RFC #1582]). Ultimately
any usage of tuples for this would require treating them as an hlist to be at
all ergonomic. Rather than introducing additional magic around tuples, it makes
more sense to provide an explicit type which is fit for the task.

[RFC #1582]: https://github.com/rust-lang/rfcs/pull/1582

We would derive as many traits as possible from the standard library for these
types, but no additional functionality would be provided for them initially. All
behavior needed can be fully written with pattern matching. It is expected that
third party crates would appear to make common patterns easier, which may
eventually be adopted in the standard library.

A function which wishes to be variadic would be annotated with `#[variadic]`.
A function with this annotation must have it's last argument be generic, with
its type being the final type parameter of the function. That parameter must not
be used elsewhere in the signature. Violating these rules results in a compiler
error:

```rust
//valid
#[variadic]
fn foo<T>(args: T) {
}

// valid
#[variadic]
fn foo<T, Args>(arg1: T, rest: Args) {
}

// error, last argument must be generic
#[variadic]
fn foo(args: Vec<i32>) {
}

// error, variadic parameter must be the final parameter
#[variadic]
fn foo<Args, T>(arg1: T, rest: Args) {
}

// error, variadic parameter must only be used as the final argument
#[variadic]
fn foo<Args>(arg1: Args, rest: Args) {
}
```

This RFC purposely avoids introducing new syntax here. If usage becomes common
enough to warrant it, new syntax can be added in the future which desugars to
this.

When a function is marked as variadic, all arguments after the last non-variadic
argument are wrapped in a variadic list. For example, given the declaration: `fn
foo<Args>(arg1: &str, rest: Args);`, the following desugaring would occur:

```rust
foo("hello"); // becomes foo("hello", Nil);
foo("hello", 1); // becomes foo("hello", Cons(1, Nil));
foo("hello", 1, Bar); // becomes foo("hello", Cons(1, Cons(Bar, Nil)));
```

The last type paremeter of a variadic function may be omitted by the caller. For
example, `fn foo<T, Args>(arg1: T, rest: Args);` may be invoked as
`foo::<i32>(1, "rest", "of", "arguments");` However, the last argument may be
provided if the caller wishes to do so. Allowing this is important because
*variadic functions do not differ at the type level*. For purposes of
implementing the `Fn` family of traits, a function with `#[variadic]` is treated
the same as a function without it. The last parameter is generic, and it may
need to be provided if type inference were to fail. In the case where arguments
are to be passed, the trait implemented by the function is `Fn(Cons<T, Cons<U,
Nil>>)`, not `Fn(T, U)`. In the future variadic functions may be changed to also
implement the `Fn` traits with unrolled parameters, or a `FnVariadic` trait may
be introduced. However, this RFC purposely omits them.

The declarer of a variadic function cannot control the concrete types which are
passed to it. However, it is allowed to place any constraints on the type
parameter, and is encouraged to do so. Looking at the concrete example of the
`rust-postgres` crate, the signature of `&[&ToSql]` could be replaced with an
hlist given the following impls:

```rust
use std::ops::variadic_list::{Cons, Nil};

impl<Head, Tail> ToSql for Cons<Head, Tail> where
    Head: ToSql,
    Tail: ToSql,
{
    fn to_sql(&self, types: &mut [Type], out: &mut Vec<u8>) -> Result<(), Error> {
        self.0.to_sql(types, out)?;
        self.1.to_sql(types, out)?;
        Ok(())
    }
}

impl ToSql for Nil {
    fn to_sql(&self, _: &mut [Type], _: &mut Vec<u8>) -> Result<(), Error> {
        Ok(())
    }
}
```

Since this trait would no longer need to be object safe, it would be able to
make additional changes like allowing any type which is `Write`. It is expected
that most usage of this feature will have a single trait bound on the argument,
and will provide an impl for both `Cons` and `Nil`. Rust's ability to determine
what impl is missing based on existing blanket impls should provide sufficient
error messages in the case where an argument doesn't satisfy the requirements,
but additional work may need to be done to ensure that the span points to the
correct argument. If sufficient impls are not provided, the compiler is allowed
to provide an error which makes it clear that the arguments are ultimately
wrapped in this type.

# How We Teach This
[how-we-teach-this]: #how-we-teach-this

"Variadic functions" are a well established name for this feature, and should
continue to be used. This RFC opts for the name "variadic list" over
"heterogeneous list" or "hlist", as that structure is less common outside of
functional languages, and to better associate them with this feature rather than
a more general structure.

This idea is fairly distinct from existing Rust patterns, and is best presented
separately. This feature will primarily affect library authors, and does not
need to be presented to brand new users.

That said, it will require additions to the Rust Reference for documentation
purposes, and warrants a chapter in _The Rust Programming Language_ and _Rust by
Example_

# Drawbacks
[drawbacks]: #drawbacks

This presents an entirely new concept to the language (granted, in a limited
fashion) which increases the overall complexity. In particular, the "phantom
type parameter" which may be omitted is potentially confusing (arguably more
confusing than always requiring `_` if we are providing type parameters). We
also lock down a concrete representation of variadic functions. While we can
provide plenty of syntax which desugars to this form, it will be difficult to
change outright.

# Alternatives
[alternatives]: #alternatives

The only real alternative is a more involved form of variadics

# Unresolved questions
[unresolved]: #unresolved-questions

Are there better names for `Cons` and `Nil`? While they are fitting given the
data structure, they are not usually associated with variadic functions, and may
be alien to users who are not familiar with singly linked lists.
