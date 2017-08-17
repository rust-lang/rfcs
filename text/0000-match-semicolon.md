- Feature Name: match_semicolons
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Allow semicolons at the end of branches in a `match` expression, throwing away the return value of
the branch.

# Motivation
[motivation]: #motivation

A relatively common pattern when designing Rust function APIs is to have a return value which
doesn't necessarily have to be used by the caller. A good example of this is the `Vec::pop()`
function, which returns an `Option<T>` that may be ignored by the calling function. Ignoring this in
a function body is handled with semicolons, throwing away the return value.

However, that return value cannot be easily thrown away inside of a match expression. Doing so
requires turning the function call into a block, like so:

```rust
let x = 2;
let mut vec = vec![0, 1, 2, 3];

match x {
    0 |
    1 => println!("doing nothing!"),
    2 => {vec.pop();}
    _ => println!("some other stuff")
}
```

Doing so adds unnecessary visual clutter to the `match` expression, making it harder to read and
understand at a glance, as well as making it more difficult to write.

# Detailed design
[design]: #detailed-design

Allow `match` branches to end in a semicolon in addition to a comma, with the semicolon throwing
away the return value of the expression in the `match` branch. This would allow the above `match` to
be rewritten like this:

```rust
match x {
    0 |
    1 => println!("doing nothing!"),
    2 => vec.pop();
    _ => println!("some other stuff")
}
```

With this, the `$pat => $expr;` syntax would be de-sugared into `$pat => {$expr;}`. Lints that would
be triggered on `{$expr;}` would also be triggered on `$expr` - for example,
`vec.binary_search(&6);` would trigger a `must_use` lint on the `Result` return type.

# How We Teach This
[how-we-teach-this]: #how-we-teach-this

If added, this pattern should mentioned in both the [`match` expressions section](https://doc.rust-lang.org/reference/expressions.html#match-expressions)
of the Rust reference, and the [`match` chapter of _The Rust Programming Language_](https://doc.rust-lang.org/book/match.html).

# Drawbacks
[drawbacks]: #drawbacks

* This change makes throwing away the return value far less visible in a match expression, making it
  quite a bit easier for a reader of the code to miss. However, that behavior usually isn't
  necessary to understanding the semantics of the `match` expression so shouldn't impair
  understanding of the code.
* This change also has the risk of splitting code formatting opinions for `match` expressions,
  with some rustaceans potentially using semicolons after *every* branch in a `match` expression
  that naturally returns `()` instead of just being used to throw away unused return values, like
  so:

  ```rust
  let x = 5;

  match x {
      1 => println!("one");
      2 => println!("two");
      3 => println!("three");
      4 => println!("four");
      5 => println!("five");
      _ => println!("something else");
  }
  ```

# Alternatives
[alternatives]: #alternatives

* Just keep the current system in place
* Introduce some other syntax for throwing away `match` branch return values, such as `;,`
* Have match branches automatically throw away return values when other branches return `()`

# Unresolved questions
[unresolved]: #unresolved-questions
