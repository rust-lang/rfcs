Summary
=======

Extend `for`, `loop`, and `while` loops to allow them to return values other
than `()`:

- add an optional `else` clause that is evaluated if the loop ended without
  using `break`;
- add an optional expression parameter to `break` expressions to break out of
  a loop with a value.

Motivation
==========

Quite often a variable is used with loops to keep track of something. For
example, the following code could be used to find a value in a list:

```rust
fn find(list: Vec<int>, val: int) -> Option<uint> {
    let mut index = None;
    for (i, v) in list.iter().enumerate() {
        if *v == val {
            index = Some(i);
            break
        }
    }
    index
}
```

However, this code relies on mutable state when it really shouldn’t be
necessary—what this code is actually doing is simply setting a variable to a
value *one time*, with a default value if the assignment statement was never
reached.

Loops also don’t fit in with Rust’s idea of ‘everything is an expression’: while
loops *are* expressions, their value is completely useless. Making loops return
meaningful values would fit in better with Rust’s idea of being able to use
everything as an expression. Iterator adaptors can often be used to similar
effect, but they aren’t always as flexible as `for`, `loop`, and `while` loops:
they cannot `return` from the enclosing function and do not provide any
guarantees to the compiler about how they run the given closure, preventing the
compiler from knowing that a variable will only be initialised once, for
example.

Detailed design
===============

Extend `for`, `while`, and `while let` (but not `loop`) to have an optional
extra `else` clause. This clause is evaluated if and only if the loop finished
iterating without reaching a `break` statement. This `else` clause, when
reached, makes the loop expression evaluate to the value within. When omitted,
it is equivalent to an empty `else` clause returning `()`. The syntax for this
additional clause comes from Python, where it is used for the same thing.

Add an optional expression parameter to `break` statements following the label
(if any). A `break` expression with a value breaks out of the loop and makes the
loop evaluate to the given expression. `break` expressions have the same
precedence as `return` expressions. The type of the `break` statement’s
expression must be the same as that of the `else` clause and that of any other
`break` expression. Because `loop` loops have no `else` clause, their `break`s
only need to match types with each other.

An advantage of having this new kind of construct is that the compiler can know
that either the main loop body or the `else` clause will *always* be run at
least once. This means that the following code would be valid:

```rust
let haystack = vec![1i, 2, 3, 4];
let needle = 2;

let x;
frobnicate(for (i, v) in haystack.iter().enumerate() {
    if v >= needle {
        x = i;
        break v
    }
} else {
    x = -1;
    0
});
```

because the compiler knows that `x` will be assigned to exactly once.

Examples
--------

The following statement:

```rust
let x = while w {
    code;
    if cond { break brk }
} else {
    els
};
```

would iterate like a normal `while` loop does today. However, if `cond`
evaluates to `true`, then the entire loop would evaluate to `brk`, setting `x`
to `brk`. If `cond` never evaluated to `true` in its entire cycle (i.e., the
`break` statement was never reached), then the loop would evaluate to `els`,
thus setting `x` to `els`.

In other words, it would be roughly equivalent to something like this:

```rust
let x = {
    let _res;
    loop {
        if w {
            code;
            if cond { _res = brk; break }
        } else {
            _res = els;
            break
        }
    }
    _res
};
```

This ‘translation’ also helps explain the use of the `else` keyword here: the
`else` clause is run if the condition (here `w`) failed, much like how the
`else` clause is run in an `if` expression if the condition failed.

### Valid samples

- ```rust
  let x: int = while cond {
      break 1
  };
  ```

  Here the `else` clause is allowed to be omitted (inferred to be an empty block
  of type `()`) because the type of the body block is `!`, which unifies with
  `()`.

- ```rust
  let x: int = while cond {
      foo();
      if let Some(foo) = bar() { break foo }
  } else {
      0
  };
  ```

  The types of the `else` and `break` clauses are the same, and they also match
  the type of the variable the loop is assigned to, so this typechecks.

- ```rust
  let z: int;
  let x: int = 'a: while cond {
      let y: f64 = while foo() {
          if bar() { z = 1; break 'a 1 }
          if baz() { break 1.618 }
      } else {
          6.283
      };
      if y > 5 { z = 2; break y as int }
  } else {
      z = 3;
      0
  };
  ```

  This example demonstrates labelled `break`s/`continue`s: the type of the
  expression passed to the `break` has to be the same as the type of the loop
  with the corresponding label. Additionally, `z` is always going to be assigned
  to exactly once: every assignment inside the outer `while` loop’s main body is
  followed by a `break` for the outer loop, and it is assigned to exactly once
  in the `else` clause.

### Invalid samples

- ```rust
  let x = while cond {
      if foo() { break 1i }
  };
  ```

  This example would not typecheck, because the type of the `break`’s expression
  (`int`) does not match the type of the (omitted) `else` block (`()`).

- ```rust
  let x: int;
  while cond {
      if foo() { x = 1 }
  } else {
      x = 2
  }
  ```

  In this example, `x` could be assigned to more than once, so this would be
  invalid.

Drawbacks
=========

* Complexity. This adds some complexity which perhaps could be considered
  unnecessary. However, this does have precedent in languages like Python, and
  so presumably does have some demand.
* The syntax is not very obvious: `else` perhaps suggests what would run if the
  loop didn’t iterate over anything.

Alternatives
============

* Do nothing. Instead, rely on using mutable variables to keep track of
  something within a loop, or use the methods provided by the `Iterator` trait.
  However, the same argument could be used to propose that `for` loops should be
  removed altogether in favour of `map`, which presumably would not be a popular
  change, if only because `break` and returning from the outer function would be
  disallowed.
* Use `nobreak` or something instead of `else`. This makes things a lot clearer,
  but has the downside of introducing a new keyword, making this a
  backward-incompatible change. Alternatively, `!break` could be used, avoiding
  the introduction of a new keyword. Unfortunately, this looks quite cryptic,
  and could be tricky to parse (although it is not ambiguous), especially given
  that `!break` is currently a valid expression.
* Make iterators yield `Result<T, E>` instead of `Option<T>` when calling
  `next`, and adjust `for` loops to evaluate to the expression parameter of any
  `break` encountered or the `Err` part of `next`’s return value. This could be
  done in addition to this proposal.

Unresolved questions
====================

None.
