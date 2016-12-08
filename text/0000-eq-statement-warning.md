- Feature Name: eq_statement_warning
- Start Date: 2016-12-07
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Warn by default when encountering a statement which only consists of an equality comparison.

# Motivation
[motivation]: #motivation

It is easy to accidentally write `==` when one actually meant `=`.

Consider the following code:

```rust
fn main() {
    let mut a = 10;
    println!("A is {}", a);
    if 1 == 2 {
        a = 20;
    } else {
        a == 30; // Oops, meant assignment
    }
    println!("A is now {}", a); // Still 10
}
```

At the time of this RFC, no warning is produced for this code,
making it easy not to notice this mistake.
This can result in wasted time trying to debug a simple mistake.

We can rectify this by giving a warning to the user.

I'd like to quote @mbrubeck to provide additional motivation
for why this is a good candidate for an on-by-default builtin lint:

- It catches a typo that is easy to make and difficult to spot,
  and that won't be caught by type checking.

- It can be very narrowly targeted, only matching statements of the form `EXPR == EXPR;`.

- False positives are unlikely, because `==` should rarely if ever have side effects,
  so it almost never makes sense to discard its result.

# Detailed design
[design]: #detailed-design

Add a new lint called `eq_statement`, which checks for statements that are
of the form `lhs == rhs;`. This lint should warn by default.

The message should tell the user that the result of the equality comparison is not used.
It should also hint that the user probably intended `lhs = rhs`.
Optionally, it can also tell the user that they if they only want the side effects, they
can explicitly express that with `let _ = lhs == rhs;`.

# Drawbacks
[drawbacks]: #drawbacks

This adds an additional lint to maintain.

False positives shouldn't be an issue, as `let _ = rhs == rhs;` expresses the same thing
more explicitly.

# Alternatives
[alternatives]: #alternatives

## [RFC 886](https://github.com/rust-lang/rfcs/pull/886)

[RFC 886](https://github.com/rust-lang/rfcs/pull/886) proposes allowing the `must_use` attribute
on functions. With this, `PartialEq::eq` could be marked `must_use`.

This RFC was closed, but it should be reconsidered with this use case serving as
additional motivation.

## Clippy

Clippy already has a lint that warns about this, called `no_effect`.

It looks like this:
```
warning: statement with no effect, #[warn(no_effect)] on by default
 --> src/main.rs:7:9
  |
7 |         a == 30; // Oops, meant assignment
  |         ^^^^^^^^
```

However, not everyone uses clippy, and I believe this is a common enough mistake
to justify including a lint for it in rustc itself.

The `no_effect` lint itself could also be considered for inclusion into rustc
and enabled by default. Note however that `no_effect` is larger in scope than
`eq_statement`, and thus more prone to yield false positives.

# Unresolved questions
[unresolved]: #unresolved-questions

None.
