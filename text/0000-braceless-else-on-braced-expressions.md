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
        B => do_the_other()
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
    B => do_the_other()
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
clause. An `else if` clause can be expanded into an equivalent nested `else ... { if ... }`.

# Detailed design
[design]: #detailed-design

This proposal can be seen as the natural progression from an `else if` clause to the other
expressions.

Braces can only be omitted around the body of an `else` if the body is a single expression which
itself requires braces.

This applies to the following expressions:

- `if`
- `if let`
- `for`
- `loop`
- `while`
- `while let`
- `match`

The following code:

```
if foo() {
    do_this()
} else {
    <expr> {
        do_that();
    }
}
```

can be flattened to:

```
if foo() {
    do_this()
} else <expr> {
    do_that()
}
```

where `<expr>` is a valid expression from the previously-stated list.

Additional `else` clauses after the `else <expr>` clause can only be valid as long as the
previous expression has a valid `else` clause.

```
// the following code is valid

if foo() {
    do_this()
} else <expr> {
    do_that()
} else {
    do_something()
}

// as long as

<expr> {
    // ...
} else {
    // ...
}

// is valid
```

Currently the only expression that satisfies this rule are the `if` and `if let` expressions.

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

  This does *not* work, because `match` doesn't have have an `else` clause. The previous code
  actually gets expanded to:

  ```
  if foo() {
      do_this()
  } else {
      match bar() {
          A => do_that(),
          B => do_the_other(),
          _ => do_something_else()
      } else {
          // syntax error
      }
  }
  ```

  Ideally most of the supported expressions should have an `else` clause. For example, the `match`
  expression can have an equivalent `else` for the `_ => ...` match. This is outside the scope
  of this RFC and should be addressed in a separate proposal.

# Alternatives
[alternatives]: #alternatives

Don't do this.

# Unresolved questions
[unresolved]: #unresolved-questions

None.
