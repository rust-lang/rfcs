- Feature Name: const-static-type-elision
- Start Date: 2017-04-29
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

Allow type annotations to be elided on all const and static items with a unique
type, including in traits/impls.

# Motivation
[motivation]: #motivation

In most cases, the type for constant and static items is obvious, and requiring
a redundant type annotation is a papercut many programmers would want to avoid.
For example, these two declarations would result in compiler errors in the
current version of Rust:

```rust
const GREETING = "Hello, world!"; // unique type: &'static str
static NUM = 42i32; // unique type: i32
```

This is usually no more than a small annoyance, but the risk involved in eliding
the types also seems small.

In the terms of
[the ergonomics initiative blog post](https://blog.rust-lang.org/2017/03/02/lang-ergonomics.html),
this change is broadly applicable, but the power is restrained by the
limitations on type inference described below.

# Detailed design
[design]: #detailed-design

Rust would allow `const` and `static` items to elide type annotations and infer
the type, but only if type inference can infer a unique type for the expression
*before* applying any fallback rules. So if we have the following items:

```rust
struct S {
  a: i32
}

const A: bool = true;
const B: i32 = 42i32;
const C: &str = "hello";
const D: S = S { a: 1 };
const E: [f32; 2] = [1.0f32, 2.5f32];
```

They could be written like this:
```rust
struct S {
  a: i32
}

const A = true;
const B = 42i32;
const C = "hello";
const D = S { a: 1 };
const E = [1.0f32, 2.5f32];
```

To minimize the reasoning footprint, type elision would use only local type
inference, rather than attempting to infer a type based on a later use of the
item as with `let`-bound variables. For example, the following would result in a
type error, because there are multiple possible types for the literal 42
(e.g. `i16`, `i32`, etc.), even though the use in `get_big_number` would require
it to be `i64`.

```rust
const THE_ANSWER = 42; // nothing in RHS indicates this must be i64

fn get_big_number() -> i64 {
    THE_ANSWER
}
```

## Integer/Float Fallback

The fallback rules (specifically, defaulting integer literals to `i32` and float
literals to `f64`) are disallowed in cases where multiple typings are valid to
prevent the type of an exported item from changing only by removing a type
annotation. For example, say some crate exports the following:

```rust
const X: i64 = 5;
```

If the developer later decides to elide the type annotation, then fallback would
infer the type of `X` as `i32` rather than `i64`. If `X` is exported but not
used within the crate, then this change could break downstream code without the
crate developer realizing it. Admittedly, that scenario is unlikely, but
ruling out fallback is the most conservative option and could always be added
back in later.

Fallback is acceptable, however, if the overall type is still unique even
without the fallback rules, as in this example:

```rust
const fn foo<T>(_: T) -> char { 'a' }
const X = foo(22);
```

## Closures

This design would allow closures (rather than just references to closures) to be
used as `const`/`static` items, because the programmer no longer has to write
down an inexpressible type. This shouldn't pose any particular difficulties from
an implementation perspective, but it's worth being aware of.

Documentation projects such as rustdoc may need to deal with this as a special
case. @withoutboats
[suggests](https://internals.rust-lang.org/t/pre-rfc-elide-type-annotations-from-const-and-static-items/5175/2?u=jschuster)
coercing closures to fn types as one possible solution.

# How We Teach This
[how-we-teach-this]: #how-we-teach-this

_The Rust Reference_ should record the rules for when the annotation is
optional. _The Rust Programming Language_ and _Rust by Example_ should remove
the sections that say annotations are required, and they may want to consider
removing annotations from their examples of `const` and `static` items (see
"Unresolved questions" below).

# Drawbacks
[drawbacks]: #drawbacks

* Some users may find it more difficult to understand large constant expressions
  without a type annotation. Better IDE support for inferred types would help
  mitigate this issue.
* Const functions may make it more difficult for a programmer to infer the type
  of a const/static item just be reading it. Most likely, though, most uses of
  const functions in this context will be things like `AtomicUsize::new(0)`
  where the type is obvious.

# Alternatives
[alternatives]: #alternatives

* Allow numeric literals in const/static items to fall back to `i32` or `f64` if
  they are unconstrained after type inference for the whole expression, as is
  done with normal `let` assignments. If the constant is visible outside its
  crate but not used within the crate, this could change the constant's type
  without any warning from the compiler. That case is likely rare, though, and
  experienced Rust programmers would likely expect this kind of fallback,
  especially for simple cases like `const A = 42;`.

# Unresolved questions
[unresolved]: #unresolved-questions

* Should _The Rust Programming Language_ remove the annotations used when
  introducing constants?
