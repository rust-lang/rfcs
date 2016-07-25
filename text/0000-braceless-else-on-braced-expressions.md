- Feature Name: Braceless Else on Braced Expressions
- Start Date: 2016-07-25
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

Make braces around `else` body optional on single expressions that require braces to increase
brevity.

# Motivation
[motivation]: #motivation

Rust doesn't require parenthesis around the condition of an `if`, but does require braces around
the bodies of `if` and `else`.

This is good because it avoids the ambiguity and the potential for mistakes around nested `if`s and
`else`s that exists in other C-family languages.

But this can be too verbose when writing stuff like `match` statements inside `else` clauses.

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

However, there's another problem. If we do this for *any* time of expression or statement, a
potential bug may occur.

```
if foo() {
    do_this()
} else
    do_that();
    do_the_other(); // oops, this line always runs
```

A solution is to limit this behavior to only *single expressions which itself require braces*. This
includes `match`, `for`, `loop`, and `while`.

An interesting observation is that this type of syntax already exists in the form of an `else if`
clause. Writing the following code:

```rust
if foo() {
    do_this()
} else if bar() {
    do_that()
} else if baz() {
    do_this_other_thing();
}
```

is the same as writing:

```rust
if foo() {
    do_this()
} else {
    if bar() {
        do_that()
    } else {
        if baz() {
            do_this_other_thing();
        }
    }
}
```

This proposal can be seen as the natural progression from the previous syntax to the other
expressions.

# Detailed design
[design]: #detailed-design

Braces can only be omitted around the body of an `else` if the body is a single expression which
itself requires braces.

So these examples are valid:

```
if foo() {
    do_this()
} else match bar() {
    A => do_that(),
    B => do_the_other()
}

if foo() {
    do_this()
} else loop {
    do_this_forever();
}

if foo() {
    do_this()
} else for x in 0..10 {
    do_this_ten_times();
}
```

This example however is not:

```
if foo() {
    do_this()
} else do_that();
```

An extra `else` clause should be declared as a syntax error if the previous `else <expr>` has no
`else` clause.

```
if foo() {
    do_this()
} else match bar() {
    A => do_that(),
    B => do_the_other()
} else {
    // syntax error
}
```

As this is equivalent to:

```
if foo() {
    do_this()
} else {
    match bar() {
        A => do_that(),
        B => do_the_other()
    } else {
        // syntax error
    }
}
```

As of writing this includes all expressions except `if`.

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
  } else { // hypothetical
      do_something_else()
  }
  ```

  should theoretically work. However this does not exist yet in current Rust and is outside the
  scope of this proposal.

# Alternatives
[alternatives]: #alternatives

None.

# Unresolved questions
[unresolved]: #unresolved-questions

None.
