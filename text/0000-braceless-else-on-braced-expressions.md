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

But this can get needlessly verbose when writing single nested expressions after `else` clauses.

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

A better solution would be to *omit* the braces around the body of an `else`. This leads to a
more flattened and easily navigable hierarchy.

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

Additionally, this type of syntax should only be allowed on *single expressions that require braced
blocks*. This is to prevent `goto fail`-type errors when writing code.

```
if foo() {
    do_this()
} else
    do_that();
    do_the_other(); // oops, this line always runs
```

# Detailed design
[design]: #detailed-design

## Examples

```
if foo() {
    do_this()
} else match baz() {
    A => do_that(),
    B => do_the_other()
}
```

```
if foo() {
    do_this()
} else for i in 0..10 {
    do_something_ten_times(i);
}
```

```
if foo() {
    do_this()
} else loop {
    do_something_forever();
}
```

```
if foo() {
    do_this()
} else while let ("Bacon", b) = get_dish() {
    println!("Bacon is served with {}", b);
}
```

## Description

This proposal can be seen as the generalized version of `else if` that extends to include other
expressions.

Braces can be omitted around the body of an `else` if the body is a single expression which itself
requires braces. This includes the following expressions:

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
} else <expr> {
    do_that()
}
```

is expanded to:

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

Additional `else` clauses after the `else <expr>` clause is only be valid as long as the previous
expression has a valid `else` clause.

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

Currently the only expressions that satisfies this rule are the `if` and `if let` expressions.

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
