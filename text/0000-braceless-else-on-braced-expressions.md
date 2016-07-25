- Feature Name: Braceless Else on Braced Expressions
- Start Date: 2016-07-25
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

Make braces around an `else` body optional on single expressions that require braces to increase
brevity.

# Motivation
[motivation]: #motivation

Rust doesn't require parenthesis around the condition of an `if`, but does require braces around
the bodies of `if` and `else`.

This is good because it avoids the ambiguity and the potential for mistakes around nested `if`s and
`else`s that exists in other C-family languages.

But this can get sverbose when writing single nested expressions after `else` clauses.

```rust
if foo() {
    do_this()
} else if bar() {
    do_this_other_thing()
} else {
    match baz() {
        A => do_that(),
        B => do_the_other(),
        _ => do_something_else()
    }
}
```

A better solution would be to *omit* the braces around the body of an `else`.

```
if foo() {
    do_this()
} else if bar() {
    do_this_other_thing()
} else match baz() {
    A => do_that(),
    B => do_the_other(),
    _ => do_something_else()
}
```

This type of syntax should only be allowed on *single expressions that require braced blocks*. This
is to prevent `goto fail` errors when writing code.

```
if foo() {
    do_this()
} else
    do_that();
    do_the_other(); // oops, this line always runs
```

An interesting observation is that this type of syntax already exists in the form of an `else if`
clause. An `else if` clause can be expanded into an equivalent nested `else ... { if ... }`. This
proposal can be seen as the natural progression from the previous syntax to the other expressions.

# Detailed design
[design]: #detailed-design

Braces can only be omitted around the body of an `else` if the body is a single expression which
itself requires braces.

This applies to the following expressions:

- `for`
- `loop`
- `while`
- `match`

The following code:

```
if foo() {
    do_this()
} else <expr> {
    do_that()
}
```

can be expanded to:

```
if foo() {
    do_this()
} else {
    <expr> {
        do_that();
    }
}
```

where `<expr>` is a valid expression from the previously-stated list.

This rule applies to *all* `else` clause for any expression. So, if in the future we have something
similar to:

```
loop {
    // ...
} else {
    match bar() {
        A() => abc(),
        B() => def(),
        _   => ghi()
    }
}
```

it can be rewritten into the following code.

```
loop {
    // ...
} else match bar() {
    A() => abc(),
    B() => def(),
    _   => ghi()
}
```

# Drawbacks
[drawbacks]: #drawbacks

- This syntax may look foreign to newcomers and may lead to mistaken assumptions such as:

  ```
  if foo() {
      do_this()
  } else match bar() {
      A => do_that(),
      B => do_the_other(),
      _ => do_something_else()
  } else {
      // syntax error
  }
  ```

  There's no solution for this, other than allowing `else` clauses on these expressions. For the
  previous example, `match` *does* have a sort of `else` clause in the form of an `_ => ...` match.
  So something like:

  ```
  if foo() {
      do_this()
  } else match bar() {
      A => do_that(),
      B => do_the_other()
  } else { // hypothetical syntax
      do_something_else()
  }
  ```

  should theoretically work. However this does not exist yet in current Rust and is outside the
  scope of this proposal.

# Alternatives
[alternatives]: #alternatives

Don't do this.

# Unresolved questions
[unresolved]: #unresolved-questions

None.
