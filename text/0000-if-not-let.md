- Feature Name: if-not-let statement
- Start Date: 2015-09-30
- RFC PR:
- Rust Issue:

# Summary

Introduce a new `if !let PAT = EXPR { BODY }` construct (informally called an
**if-not-let statement**).  This works much like an if-let expression, but
executes its body when pattern matching fails.

This narrows the gap between regular `if` syntax and `if let` syntax, while
also simplifying some common error-handling patterns.

# Motivation

[if-let expressions][if-let] offer a succinct syntax for pattern matching
with only one "success" path. This is particularly useful for unwrapping
types like `Option`. However, an if-let expression can only create bindings
within its body, which can force rightward drift and excessive nesting.

`if !let` is a logical extension of `if let` that moves the failure case into
the body, and allows the success case to follow without extra nesting.

For example, this code written with current Rust syntax:

```rust
if let Some(a) = x {
    if let Some(b) = y {
        if let Some(c) = z {
            /*
             * do something with a, b, and c
             */
        } else {
            return Err("bad z");
        }
    } else {
        return Err("bad y");
    }
} else {
    return Err("bad x");
}
```

would become:

```rust
if !let Some(a) = x {
    return Err("bad x");
}
if !let Some(b) = y {
    return Err("bad y");
}
if !let Some(c) = z {
    return Err("bad z");
}
/*
 * do something with a, b, and c
 */
```

It's possible to use `match` statements to emulate this today, but at a
significant cost in length and readability.  For example, this real-world code
from Servo:

```rust
let subpage_layer_info = match layer_properties.subpage_layer_info {
    Some(ref subpage_layer_info) => *subpage_layer_info,
    None => return,
};
```

is equivalent to this much simpler if-not-let statement:

```rust
if !let Some(ref subpage_layer_info) = layer_properties.subpage_layer_info {
    return
}
```

The Swift programming language, which inspired Rust's if-let expression, also
includes a [guard-let-else][swift] statement which are equivalent to this
proposal except for the choice of keywords.

# Detailed design

Extend the Rust statement grammar to include the following production:

```
stmt_if_not_let = 'if' '!' 'let' pat '=' expr block
```

The pattern must be refutable.  The body of the if-not-let statement (the
`block`) is evaluated only if the pattern match fails.  Any bindings created
by the pattern match will be in scope after the if-not-let statement (but not
within its body).

The body must diverge (i.e., it must panic, loop infinitely, call a diverging
function, or transfer control out of the enclosing block with a statement such
as `return`, `break`, or `continue`).  Therefore, code immediately following
the if-not-let statement is evaluated only if the pattern match succeeds.

An if-not-let statement has no `else` clause, because it is not needed.
(Instead of an `else` clause, code can simply be placed after the body.)

The following code:

```rust
{
    if !let pattern = expression {
        /* handle error */
    }
    /* do something with `pattern` here */
}
```

is equivalent to this code in current Rust:

```rust
match expression {
    pattern => {
        /* do something with `pattern` here */
    }
    _ => {
        /* handle error */
    }
}
```


# Drawbacks

* “Must diverge” is an unusual requirement, which might be difficult to
  explain or lead to confusing errors for programmers new to this feature.

* Allowing an `if` statement to create bindings that live outside of its body
  may be surprising.

* `if !let` is not very visually distinct from `if let` due to the
  similarities between the `!` and `l` glyphs.

# Alternatives

* Don't make any changes; use existing syntax like `if let` and `match` as
  shown above, or write macros to simplify the code.

* Consider alternate syntaxes for this feature, perhaps closer to Swift's `guard
  let else`.

# Unresolved questions

* Is it feasible to implement the check that the body diverges?

[if-let]: https://github.com/rust-lang/rfcs/blob/master/text/0160-if-let.md
[swift]: https://developer.apple.com/library/prerelease/ios/documentation/Swift/Conceptual/Swift_Programming_Language/ControlFlow.html#//apple_ref/doc/uid/TP40014097-CH9-ID525
