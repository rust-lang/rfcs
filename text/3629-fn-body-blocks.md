- Feature Name: `fn_body_blocks`
- Start Date: 2024-05-06
- RFC PR: [rust-lang/rfcs#3629](https://github.com/rust-lang/rfcs/pull/3629)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Allow the definition of functions with a single-block construct in their body,
without requiring top-level braces.

Examples:

```rust
unsafe fn read_bool(x: *const bool) -> bool
unsafe {
    *x
}
```

```rust
fn countup(limit: usize) -> impl Iterator<Item = usize>
gen {
    for i in 0..limit {
        yield i;
    }
}
```

```rust
fn is_some<T>(x: Option<T>) -> bool
match x {
    Some(_) => true,
    None => false,
}
```

# Motivation
[motivation]: #motivation

This provides a concise shorthand for functions which consist of a single block
construct. These are relatively common and can otherwise lead to rightward
drift.

Another use case comes once `unsafe fn` no longer implies an `unsafe { }` body
([RFC 2585](https://github.com/rust-lang/rfcs/pull/2585)). For cases where
users want the old behavior, being able to define a function with a top-level
`unsafe { ... }` block avoids requiring an additional nested block.

This function body block syntax also gives a single, consistent, unified syntax
to support functions that pair with new expression blocks that are under
development, such as `gen { }`, `try { }` and `async gen { }`. While we might
be tempted to add syntax such as

```rust
gen fn foo() -> i32 {
    yield 1;
    yield 2;
    yield 3;
}
```

for each new kind of block, this becomes less necessary if we can instead
write:

```rust
fn foo() -> impl Iterator<Item = i32>
gen {
    yield 1;
    yield 2;
    yield 3;
}
```

(This would be further improved by syntax to simplify the
`impl Iterator<Item = i32>` type.)

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Functions whose entire body consists of a construct like `match` that takes a
single braced block can omit the outer braces:

```rust
fn function(x: usize)
match x {
    0 => println!("Zero"),
    _ => println!("Nonzero"),
}

fn countdown(mut count: usize)
while count > 0 {
    println!("{count}");
    count -= 1;
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The full list of block constructs permitted at the top level of a function:

- `unsafe`
- `loop`
- `while` and `while let`
- `for`
- `async`
- `match`
- `if` and `if let`, if and only if there's no `else`.
- `try` (once it exists)
- `gen` (once it exists)
- `async gen` (once it exists)

# Drawbacks
[drawbacks]: #drawbacks

## Choice Paralysis

By having two ways to declare a function, it is not always clear which version
is preferred. This adds additional cognitive load when defining a function.

One of the authors of this RFC has experienced this while programming with
Julia, which has two ways to declare functions.

We can mitigate this for Rust with clear guidelines, formatting rules, and
Clippy lints, to steer users towards the canonical version in contexts where
that is possible.

## Human Visual Parsing

If formatted poorly, the block construct could "disappear" into the function
signature. We recommend that the default Rust style use a newline to separate
the type from the block, making this visually straightforward to parse.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Rationale: Generalizes to many function types

The key strength of this proposal is that is a single concept that can subsume
many additional features under consideration.

For example, we are considering adding `gen { }` blocks that evaluate to
iterators. This means code like the following will be common:

```rust
fn countup(limit: usize) -> impl Iterator<Item = usize> {
    gen {
        for i in 0..limit {
            yield i;
        }
    }
}
```

This immediately suggests adding a new `gen fn` form, which would let us write
the same function as:

```rust
gen fn countup(limit: usize) -> usize {
    for i in 0..limit {
        yield i;
    }
}
```

With this proposal, we could forego the new `gen fn` form and instead write:

```rust
fn countup(limit: usize) -> impl Iterator<Item = usize>
gen {
    for i in 0..limit {
        yield i;
    }
}
```

The story is similar for other new block types such as `try { }` and
`async gen { }`. For instance:

```rust
fn might_fail() -> Result<(), E>
try {
    func()?;
    another_func()?;
}
```

## Rationale: Allows concisely named return types for `async fn`

Another benefit is that it gives us a concise way to write what are effectively
async functions where the returned future is named.

While `async fn foo() -> i32` as a function that returns
`impl Future<Output = i32>` is concise and convenient in many cases, there are
times when it is helpful to be able to name the type of the returned future.

This proposal would allow:

```rust
fn foo() -> NamedFutureType
async {
    ...
}
```

## Alternative: Syntax Options

Another option would be to require a `=` before the body. This could permit a
wider variety of expressions, such as `fn inc(x: u32) = x + 1;`.

However, this would introduce more parsing complexity, for both the compiler
and the user. In particular, this would likely require a `;` for constructs
that don't end with a `}`.

## Alternative: Allow `if`/`if let` with `else`

We could allow functions to have a top-level `if` or `if let` with an `else`.
The compiler would not have problems parsing this. However, this would result
in having multiple braced blocks associated with a single function, which seems
more error-prone both for humans and for extremely simplistic code parsers
(e.g. those used within some code editors).

For simplicity, this proposal does not permit `if` or `if let` blocks that have
an `else`. The compiler can recognize attempts to do this and offer a rustfix
suggestion to wrap the function body in braces.

# Prior art
[prior-art]: #prior-art

- In [Julia](https://docs.julialang.org/en/v1/manual/functions/), functions can
  be spelled
  ```julia
  function f(x, y)
      x + y
  end
  ```
  or `f(x, y) = x + y`.
- C# and JavaScript have a `=>` form for defining functions.

# Future possibilities
[future-possibilities]: #future-possibilities

We could introduce shorthand syntax for types that are produced by expression
blocks, such as `async T` for `impl Future<Output = T>` and `gen T` for
`impl Iterator<Item = T>`.

In addition to being useful in their own right, these types would work very
well with this proposal, since the shorthand applies in the cases where we
expect the function body block syntax to be used most often.

For example:

```rust
fn countup(limit: usize) -> gen usize
gen {
    for x in 0..limit {
        yield i;
    }
}

fn do_something_asynchronously() -> async ()
async {
    do_something().await;
}
```

Note that while this is an improvement, this results in the keyword (`gen` or
`async`) appearing twice. We may want to seek an alternative that allows
writing the keyword just once. However, we also want to preserve orthogonality
(type syntax having the same meaning everywhere, block constructs having the
same meaning everywhere), and avoid special cases.
