- Feature Name: Else clauses for `for` and `while` loops
- Start Date: 2021-08-19
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Else clauses for `for` and `while` loops would clarify 'break with value' semantics for these loops, and allow for a more natural way of running code after a loop has completed without breaking.

# Motivation
[motivation]: #motivation

This RFC proposes the introduction of for/else and while/else loops; an extension to loop syntax that allows code to run if the loop is never broken. Using these loops simplifies the previous conversations around `for`/`while` break-with value semantics. It also allows for a more natural way of running code after a loop has completed without breaking.

For example, the following code represents a very common pattern:

```rs
let mut found = false;

for value in data {
    if value == target {
        found = true;
        break;
    }
}

if !found {
    println!("Couldn't find {}", target);
    return;
}
```

This requires an additional variable, increasing visual complexity; and allows a future maintainer to add more code between the initial variable, the loop, and the variable checking code, which could change behaviour (e.g. if the checking code returns on failure, code added prior to the check would run unconditionally).

This proposal introduces the following syntax, with the intent of preventing these issues:

```rs
for value in data {
    if value == target {
        break;
    }
} else {
    println!("Couldn't find {}", target);
    return;
}
```

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When using a `loop` block, it can return a value by `break`ing with that value. For example:

```rs
let x = loop {
    if (...) {
        break 42;
    } else {
        // do work
    }
};
```

This code sets x to 42, after doing some work.

If we instead want to loop over some values, and either return one of these values or a default value, we can use a `for` block:

```rs
let x = for i in 0..10 {
    if (...) {
        break i;
    }
} else {
    11 // default value
};
// x is now a usize; either a value 0-9, or 11 if our condition was never met
```

If we wanted to instead use a while loop, we could use a `while` block instead:

```rs
let x = while (...) {
    // do work
    if (...) {
        break Some(value);
    }
} else {
    Some(42) // default value
};
```

The reason the `else` keyword is reused here is twofold:

1. It is not a new keyword, meaning all code written before the implementation will still work identically (as currently break-with-value semantics are not allowed in for/while loops)
2. It makes sense within the semantics of `for`/`while` desugaring; while loops desugar to code similar to the following:

    ```rs
    loop {
        if (...) {
            ... // while loop code
        } else {
            break {
                ... // else clause code - for while loops without an else clause, this is an empty block
            };
        }
    }
    ```

    For loops, on the other hand, desugar to code similar to the following:

    ```rs
    loop {
        if let Some(value) = iterator.next() {
            ... // for loop code
        } else {
            break {
                ... // else clause code - for for loops without an else clause, this is an empty block
            };
        }
    }
    ```

    As for loops are essentially a special case of while-let loops, it's easy to see how those desugar:

    ```rs
    loop {
        if let (...) {
            ... // while-let loop code
        } else {
            break {
                ... // else clause code - for while-let loops without an else clause, this is an empty block
            };
        }
    }
    ```

    Seeing how the constructs desugar, it is easy to see how the `else` clause can be used to add behaviour when the loop terminates without breaking.

It is allowed to assign a for or while loop without an else clause to a variable, as long as it does not contain a break-with-value. The value of the variable is `()`.

However, it is *not* allowed to assign a for or while loop (containing a break-with-value) without an else clause to a variable, as this would lead to an inconsistent type (as using break-with-value would then need to return `Option<T>`, which is incompatible with `()`).

Sample error:
```rs
fn main() {
    let x = for i in 0..2 { break 2; };
    println!("{}", x);
}
```

Leads to:

```
error[EXXXX]: `break` with value from a `for` loop missing an `else` clause
 --> src/main.rs:2:13
  |
2 |     let x = for i in 0..2 { break 2; };
  |             ^^^^^^^^^^^^^^^^^^^^^^^^^^ for loops which break with a value must have an `else` clause
  |             |
  |             you can't `break` with a value in a `for` loop without an `else` clause
  |
help: add an `else` clause to this `for` loop
  |1
2 |     let x = for i in 0..2 { break 2; } else { 42 };
  |                                        ^^^^^^^^^^^
```

It is also not allowed to create a for/else or while/else with inconsistent types.

Sample error:
```rs
fn main() {
    let x = for i in 0..2 { break 2; } else {"foo"};
    println!("{}", x);
}
```

Leads to:

<!-- I created this mock error before realising that the `loop` with incompatible types case gives a simpler error message: Perhaps this could be used to improve that case?

```
error[E0308]: mismatched types
 - -> src/main.rs:2:13
  |
2 |     let x = for i in 0..2 { break 2; } else {"foo"};
  |         -                         -          ----- expected `usize` because of prior `break` with a value
  |         |                         |
  |         |                         found value type `usize`
  |         cannot determine consistent type; can be either `usize` or `&'static str`
``` -->

```
error[E0308]: mismatched types
 --> src/main.rs:2:46
  |
2 |     let x = for i in 0..2 { break 2; } else {"foo"};
  |                                              ^^^^^ expected integer, found `&str`
```

Similarly, incompatible break-with-values are disallowed, as with `loop` blocks.

Sample error:
```rs
fn main() {
    let x = for i in 0..2 {
        if i == 0 {
            break 2;
        } else {
            break "hello";
        }
    };
    println!("{}", x);
}

```

Leads to:

```
error[E0308]: mismatched types
 --> src/main.rs:6:19
  |
6 |             break "hello";
  |                   ^^^^^^^ expected integer, found `&str`
```

Loops are allowed to use an `else` clause even if they do not contain a break-with-value. This is useful for code that is only executed if the loop terminates normally. Loops which do not contain a break-with-value will not be used as an implicit return value (so they do not need to be followed with a semicolon).

This loop does not contain a break-with-value, and will not be used as an implicit return value:

```rs
for i in 0..2 {
    if i == 3 {
        break;
    }
} else {
    println!("`i` never reached 3.");
}
```

This loop, however, contains a break-with-value, and will be used as an implicit return value, with the type `&str`:

```rs
for i in 0..2 {
    if i == 3 {
        break "found 3";
    }
} else {
    "didn't find 3"
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

For/else and while/else largely do not interact with other features of the language, however:

- For/else and while/else loops are valid expressions, and their type must be consistent.
  - All `break` statements must contain a value of the same type, and the `else` clause must implicitly return the same type.
  - Alternatively, all `break` statements must not contain a value, and the `else` clause must implicitly return `()`.
- For/else and while/else loops which do not contain a break-with-value are not used as implicit return values, but may be assigned to variables (and return the unit type `()`, as for/while loops without else clauses do).
- For and while loops without an else clause are still not allowed to contain a break-with-value.
- Else clauses may not contain breaks (this is an unresolved decision). This is because it could be seen as ambiguous whether the break refers to the `else` clause or an enclosing loop.
- For/else and while/else loops may contain break-with-values and else-values of type `Option<T>` or `Result<T, E>`, and these types would need to be inferred from use.
  - It is *not* valid to use an implicit return of `()` in place of `None` in the else clause, nor is it valid to use a bare `break;` in place of `break None;` within the loop.
- `loop` blocks would likely be allowed to have an `else` clause (but this would trigger an unreachable statement warning).
- For/else and while/else loops with a break-with-value of `()` do not need to explicitly return `()` from the else clause, and are a valid implicit return value:

    ```rs
    for i in 0..2 {
        if i == 3 {
            break ();
        }
    } else {
        println!("`i` never reached 3.");
    }
    ```

    The author, however, asserts that this is bad style and implicit return of `()` should be used instead:

    ```rs
    for i in 0..2 {
        if i == 3 {
            break;
        }
    } else {
        println!("`i` never reached 3.");
    }
    // implicit return of () due to block end instead of the loop
    ```

The desugaring of `for`/`else` and `while`/`else` (including while-let/else) constructs is listed in the previous section, however for clarity, it is also listed here:
```rs
while cond {
    code();
} else {
    other_code();
}
// desugars to
loop {
    if cond {
        code();
    } else {
        break {
            other_code();
        };
    }
}

while let Pattern(value) = real_value {
    code();
} else {
    other_code();
}
// desugars to
loop {
    if let Pattern(value) = real_value {
        code();
    } else {
        break {
            other_code();
        };
    }
}

for variable in iterator {
    code();
} else {
    other_code();
}
// desugars to
loop {
    if let Some(variable) = iterator.next() {
        code();
    } else {
        break {
            other_code();
        };
    }
}
```

# Drawbacks
[drawbacks]: #drawbacks

- Accepting this RFC locks in this behaviour; for/else, while/else, and while-let/else could not be used for a different behaviour.
- Without an understanding of the underlying implementation of loops, the choice of the keyword `else` can seem arbitrary.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- Why is this design the best in the space of possible designs?
  - This design creates a clear and consistent behaviour for break-with-value in for and while loops, and also allows for a very common idiom of searching through data (and acting when no value is found)
- What other designs have been considered and what is the rationale for not choosing them?
  - `else` branches executing when the loop body *never* executes. The reason this option was not chosen is that it would not allow a consistent behaviour for break-with-value, and would not satisfy the most common use of this construct.
- What is the impact of not doing this?
  - If this RFC is not resolved, then for and while loops cannot contain break-with-value, (`loop` blocks are currently the only place a break-with-value can be used), and searching data will require an additional single-use sentry variable, which is not ideal due to the potential for code to be inserted in the middle of the construct by a future maintainer.

# Prior art
[prior-art]: #prior-art

This feature is in both [Python (with identical semantics)](https://docs.python.org/3/tutorial/controlflow.html#break-and-continue-statements-and-else-clauses-on-loops) and Guarded Command Language. Additionally, there has been [a proposal for this feature in Golang (closed by bot)](https://github.com/golang/go/issues/41348) and [in JuliaLang](https://github.com/JuliaLang/julia/issues/1289) (with many people proposing/preferring the Python-influenced semantics used in the JuliaLang thread, in addition to backing up the motivation).

Unfortunately, many Python users are unfamiliar with the syntax; of [an informal survey](https://blog.glyphobet.net/blurb/2187/), only around 25% knew the meaning and 55% gave an incorrect meaning.

Knuth mentioned this style of guard in [this paper](http://citeseerx.ist.psu.edu/viewdoc/download?doi=10.1.1.103.6084&rep=rep1&type=pdf), but the author of this RFC is too tired to find the specific page.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- What parts of the design do you expect to resolve through the RFC process before this gets merged?
  - `break` statements within the `else` clauses instead be allowed as (author's preference) a break from an enclosing loop or as an alternative to implicit scope return.
  - How much of the loop's return can be left implicit (e.g. `None` returns of an `Option`-type loop) and how much must be made explicit.
  - Possible alternatives to `else` as a keyword (e.g. `nobreak`).
- What parts of the design do you expect to resolve through the implementation of this feature before stabilization?
  - Whether or not `loop` blocks can have an `else` clause attached (the author is neutral on the outcome of this decision)

# Future possibilities
[future-possibilities]: #future-possibilities

The author cannot think of any future possibilities outside of the scope of this RFC; it is primarily self-contained. This RFC should not limit other future possibilities, other than those which would reuse the proposed syntax.
