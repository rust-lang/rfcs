- Feature Name: `mut_pattern_shorthand`
- Start Date: 2018-04-13
- RFC PR: 
- Rust Issue: 

# Summary
[summary]: #summary

Simply put, this RFC allows you to write `let mut (x, y) = (1, 2);` instead of
`let (mut x, mut y) = (1, 2);`.
This also applies to `if let`, `while let`, `for`, `match` and `fn` arguments.
In other words, `mut PAT` becomes a pattern.

# Motivation
[motivation]: #motivation

## Reduced repetition

Simply put, this RFC improves the writing ergonomics for users by reducing the
number of places where `mut` has to be added in pattern matching.
Instead, it is sufficient to put `mut` in a single place to make all bindings
introduced by destructuring mutable.

## Improved readability through reduced noise

As a result of reduced repetition, the noise of `mut` is also lessened.
Therefore, readers can focus on the structure of the thing being destructured
and pattern matched on instead of `mut` being prefixed on a lot or all bindings.
The single `mut` introduced as in `let mut (x, (y, z)) = ..` should still be
sufficiently close in visual scope to not require holding more things in short
term memory.

## Macro

Another effect of reduced repetition is that it now becomes easy to create
a macro that introduces mutable bindings everywhere by simply adding a single
`mut`. This way, you can take a `pat` fragment and prefix it with `mut` to make
the entire pattern introduce mutable bindings.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This RFC is all about syntactic sugar and letting you write things you already
could in more ergonomic ways. Let's go through a few before-and-after examples
of what this RFC means and changes from a user's perspective.

Note that many of these examples are artificial for the purposes of simplicity.
Not all of these example uses is the recommended style; in some scenarios,
the current way of writing things is and will still be idiomatic Rust.

## `let` bindings

Before:

```rust
let (mut x, mut y) = (1, 2);
let ((mut x, mut y), z) = ((1, 2), 3); // <- NOTE: z is immutable!
let ((mut x, mut y), mut z) = ((1, 2), 3); // <- NOTE: everything is mutable.
let Ok(mut x) | Err(mut x) = Ok(1); // <- NOTE: > 1 variant (but irrefutable).

struct Person<'a> { age: usize, hobby: &'a str, pocket: [&'a str; 3] }

let Person { age, mut hobby, pocket: [mut x, mut y, mut z] } // Some mutability.
  = Person { age: 1, hobby: "designing Rust"
           , pocket: ["key", "phone", "headphones"] };
```

After:

```rust
let mut (x, y) = (1, 2);
let (mut (x, y), z) = ((1, 2), 3);
let mut ((x, y), z) = ((1, 2), 3);
let mut Ok(x) | Err(x) = Ok(1); // <- NOTE: or-patterns bind more tightly!

struct Person<'a> { age: usize, hobby: &'a str, pocket: [&'a str; 3] }

let Person { age, mut hobby, pocket: mut [x, y, z] }
  = Person { age: 1, hobby: "designing Rust"
           , pocket: ["key", "phone", "headphones"] };
```

## `if let`

Before:

```rust
if let Some(mut x) = None { ... }

if let Ok((mut x, mut y)) = Ok((1, 2)) { ... }

enum E<T> { A(T), B(T), C(T) }

if let E::A(mut x) | E::B(mut x) = E::A(1) { ... }
```

After:

```rust
if let mut Some(x) = None { ... }

if let Ok(mut (x, y)) = Ok((1, 2)) { ... }
// Or equivalently:
if let mut Ok((x, y)) = Ok((1, 2)) { ... }

enum E<T> { A(T), B(T), C(T) }

if let mut E::A(x) | E::B(x) = E::A(1) { ... }
```

## `while let`

Before:

```rust
let mut arr = vec![(1, 2), (2, 3), (3, 4)];
let mut iter = arr.drain(..);

while let Some((mut x, mut y)) = iter.next() { ... }
```

After:

```rust
let mut arr = vec![(1, 2), (2, 3), (3, 4)];
let mut iter = arr.drain(..);

while let mut Some((x, y)) = iter.next() { ... }
// Or equivalently:
while let Some(mut (x, y)) = iter.next() { ... }
```

## `for` loops

Before:

```rust
let mut arr = vec![(1, 2), (2, 3), (3, 4)];
for (mut x, mut y) in arr.drain(..) { ... }
```

After:

```rust
let mut arr = vec![(1, 2), (2, 3), (3, 4)];
for mut (x, y) in arr.drain(..) { ... }
```

## `match` expressions

Before:

```rust
match (1, 2) {
    (mut x, mut y) => {}
}

enum E<T> { A(T), B(T), C(T) }

match E::A(1) {
    E::A(mut x) | E::B(mut x) => { x = 3; },
    E::C(mut x) => { x = 2; }
}
```

After:

```rust
match (1, 2) {
    mut (x, y) => {}
}

enum E<T> { A(T), B(T), C(T) }

match E::A(1) {
    mut E::A(x) | E::B(x) => { x = 3; },
    mut E::C(x) => { x = 2; }
}
```

## `fn` arguments

Before:

```rust
fn do_stuff((mut x, mut y): (usize, usize)) { ... }
```

After:

```rust
fn do_stuff(mut (x, y): (usize, usize)) { ... }
```

## Lambdas

Before:

```rust
let lam = |(mut x, mut y): (u8, u8)| { x = x + 1; (x, y) };
```

After:

```rust
let lam = |mut (x, y): (u8, u8)| { x = x + 1; (x, y) };
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Grammar

Grammatically, given that `PAT` is a valid pattern, then `mut PAT` is a valid
pattern as well.
In other words, the following production is added to the grammar:

```
MutPat. PAT ::= "mut" PAT ;
```

Note that `mut PAT | .. | PAT` is interpreted as `mut (PAT | .. | PAT)` and
**not** `mut PAT | (.. | PAT)`.

## Semantics

The semantics of a `mut PAT` pattern is defined as the semantics of the
desugared form where any binding introduced is `mut`.
Consult the [guide level explanation][guide-level-explanation] for some examples
of desugarings.

## Warnings

The following snippet `let mut (mut x, mut y) = ...;`, while grammatically valid,
also has redundant `mut`s either outside the tuple or inside the tuple.
When such a redundancy happens, the compiler should unconditionally warn the
user. Furthermore, when `let mut (x, y)` results in partial unused `mut`ability
then the compiler should also warn.

# Drawbacks
[drawbacks]: #drawbacks

## Risk of over-`mut`ability

There's a risk that users will get used to the ergonomics of `mut (x, y)`
patterns and write that instead of writing `(mut x, y)` when `y` does not need
to be mutable. We can mitigate this with the usual mutability lints done by the
compiler when it notices unneccessary mutability like so:

```
warning: variable does not need to be mutable
 --> src/main.rs:4:17
  |
4 |     let mut (x, y) = (1, 2);
  |         --^     ^
  |           |     |
  |           ------- help: rewrite this as `(mut x, y)`
  |
  = note: #[warn(unused_mut)] on by default
```

## Longer edit distances and `mut`-locality

With this RFC, mutability of bindings is no longer simply controlled on the
particular binding itself directly. Instead, the mutability of bindings can
be "far away" if you consider an example such as `let mut (a, (b, (c, d))) = ..`.
For some, this will decrease readability, and the speed of editing existing code.
For others, readability will be increased thanks to reduced noise.
The ability to edit can also be increased when you only need to add `mut` after
`let` to introduce mutable bindings. A mitigating factor to reduced readability
for some is the fact that using this new syntax is entirely optional.

## Complicating the grammar

As always, this RFC proposes changes which makes Rust's grammar more complex.
To this, new lints that point out unnecessary mutability will also need to be
added.

# Rationale and alternatives
[alternatives]: #alternatives

## Do nothing

Of course, we could opt not to do this, in which case the ergonomic improvements
would not be gained.

## A more conservative approach

We could also take a more conservative approach,
only permitting `let mut (x, (y, z)) = ..` and not `let (x, mut (y, z)) = ..`.

We could also only allow this for `let` bindings and not `if let`, `while let`,
`for`, `match` and `fn` arguments.

## Rationale for precedence in `mut PAT | .. | PAT`

The RFC as currently specified interprets `mut PAT | .. | PAT` as
`mut (PAT | .. | PAT)` instead of `mut PAT | (.. | PAT)`.

We argue that this is correct because if it would be interpreted in the second
way, then you'd get the following:

```rust
enum E<T> { A(T), B(T), C(T) }

match E::A(1) {
    E::A(mut x) | E::B(x) => { x = 3; },
    E::C(mut x) => { x = 2; }
}
```

which results in an error:

```rust
error[E0409]: variable `x` is bound in inconsistent ways within the same match arm
 --> src/main.rs:6:24
  |
6 |     E::A(mut x) | E::B(x) => { x = 3; },
  |              -         ^ bound in different ways
  |              |
  |              first binding
```

# Prior art
[prior-art]: #prior-art

The combination of pattern matching and mutability is not particularly common.
As such, the RFC's author is not aware of any prior art.

# Unresolved questions
[unresolved]: #unresolved-questions

Except for the following unresolved questions, there are no other.

1. Should the proposal take a more conservative route?
2. Is the precedence of `mut` with or-patterns (`PAT | PAT`) the right design?