- Feature Name: `get_expr_type`
- Start Date: 2019-05-30
- RFC PR: [rust-lang/rfcs#2706](https://github.com/rust-lang/rfcs/pull/2706)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Add the ability to retrieve the concrete type of an arbitrary expression so that
it may be reused by other code that relies on the expression.

# Motivation
[motivation]: #motivation

> "Macros will become more powerful than you can possibly imagine."
>
> – Obi-Wan Kenobi (_supposedly_)

Within the context of a macro, this feature would be very useful.

It would allow for:

- Defining a function that takes the concrete type of a given expression.
- Calling upon type-level functions based upon a given expression.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Given some value `x`, we can retrieve its type with just `x.type`. This type can
then be used within various contexts:

```rust
let x = "hellooo";
let y: x.type = x; // C equivalent: typeof(x) y = x;

type X = x.type;

assert_eq_type!(X, &str); // taken from `static_assertions`

assert!(<x.type>::default().is_empty());
```

Note that if the expression resolves to a long operation, the operation will
_not_ be evaluated.

```rust
type T = do_stuff().do_more().type;
```

This feature is especially useful within macros when wanting to generate
functions or types based off of the given expression.

```rust
let x: Value = /* ... */;

macro_rules! do_stuff {
    ($x:expr) => {
        let y = <$x.type>::new(/* ... */);
        $x.do_stuff_with(y);
    }
}

do_stuff!(x);
```

When we get `x.type` here, it is no different than just substituting it directly
with `Value`. This allows for accessing all type-specific functionality.

This isn't possible with the current mechanism for getting the type of an
expression:

```rust
macro_rules! do_stuff {
    ($x:expr) => {
        fn with_type_of<T>(x: T) {
            let y = T::new(/* ... */);
            x.do_stuff_with(y);
        }
        with_type_of($x);
    }
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

I am uncertain of how this would be implemented within the compiler but I
imagine that it would leverage the work that's already been done with `const`
generics.

Some cases to be aware of are non-simple expressions, such as `1 + 1`. It is
understood that this has a type of `i32` by default, but such an expression may
be difficult to parse in a generic context.

The following may only be possible in this form:

```rust
do_stuff::<{1 + 1}.type>();
```

See [unresolved questions](#unresolved-questions) for more regarding the above
example.

# Drawbacks
[drawbacks]: #drawbacks

By using a postfix syntax, one may write a long expression only to realize that
nothing before the `.type` part will actually not be evaluated. However, such
code would be in a context where it's obvious that the expression isn't
important.

```rust
do_thing::<send_request().await?.body().await?.contents.type>();
```

To be fair, this isn't the [weirdest thing in Rust](https://github.com/rust-lang/rust/blob/master/src/test/run-pass/weird-exprs.rs).

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Retrieving the type via the overloaded `.` operator feels like a natural
extension of the language.

As of this writing, the lang team [has come to a final
decision](https://boats.gitlab.io/blog/post/await-decision-ii/) regarding the
syntax for `await`, and that is to make it a postfix operation: `expr.await`.
This sets a precedent of allowing reserved keywords in what would otherwise be
the position for a property access.

## Alternatives

- A `type_of!` macro:

  ```rust
  type T = type_of!(expr);
  ```

  However, this would be more awkward when used in a generic context:

  ```rust
  type V = Vec<type_of!(elem)>;
  ```

  Depending on current parser limitations, it may need to be surrounded by
  braces when in a generic context.

- A freestanding magical `typeof()` "function" in the same style as in C:

  ```rust
  type T = typeof(expr);
  ```

  The `typeof` identifier is already reserved as a keyword and so placing it into
  the `std`/`core` prelude would not be a breaking change.

# Prior art
[prior-art]: #prior-art

## C

As a compiler extension, GCC allows for getting the type of an expression via
[`typeof()`](https://gcc.gnu.org/onlinedocs/gcc/Typeof.html). This is often used
within macros to create intermediate bindings for inputs so as to not evaluate
inputs more than once.

For example, a safe `max` macro that evaluates its arguments exactly once can be
defined as:

```c
#define max(a, b) \
    ({  typeof(a) _a = (a); \
        typeof(b) _b = (b); \
        _a > _b ? _a : _b; })
```

Of course, with Rust's type inference, this same workaround isn't necessary.

## Swift

Swift's `type(of:)` produces runtime metatype value, upon which type-level
functions can be called.

```swift
let value = "Hola"              // String
let valueType = type(of: value) // String.Type

let other = valueType.init(["H", "o", "l", "a"]) // String

assert(value == other)
```

## Other Languages

Many other languages include a `typeof` feature, as can be seen in
[this Wikipedia article](https://en.wikipedia.org/wiki/Typeof).

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- An issue that `const` generics deals with is needing to wrap the expression in
  braces (e.g. `{expr}`).

  What situations would require the expression before `.type` to be wrapped in
  braces as well?

  Would this always be necessary in the context of expressions more complicated
  than providing one without spaces? (e.g. `f::<{a + b}.type>()`)

- Would an expression provided within a macro as a `:expr` be exempt from the
  above requirement?

  ```rust
  macro_rules! call_f {
      ($x:expr) => { f::<$x.type>() }
  }

  call_f!(2 + 2);
  ```

  This works within the context of `const` generics and so one can imagine that
  it would also work here.

- Would the following work?

  ```rust
  type T = (2 + 2).type;
  ```

  Or would it be restricted to:

  ```rust
  type T = {2 + 2}.type;
  ```

# Future possibilities
[future-possibilities]: #future-possibilities

This can push forward the capabilities of macros greatly and make them on-par
with C++ templates.
