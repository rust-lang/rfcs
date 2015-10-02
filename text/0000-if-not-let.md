- Feature Name: let-else statement
- Start Date: 2015-09-30
- RFC PR:
- Rust Issue:

# Summary

Introduce a new `let PAT = EXPR else { BODY }` construct (informally called an
**let-else statement**).

If the pattern match succeeds, its bindings are introduced *into the
surrounding scope*.  If it does not succeed, it must diverge (e.g., return or
break).  You can think of let-else as a “refutable `let` statement.”

This simplifies some common error-handling patterns, and reduces the need for
special-purpose control flow macros.

# Motivation

[if-let expressions][if-let] offer a succinct syntax for pattern matching
with only one “success” path. This is particularly useful for unwrapping
types like `Option`. However, an if-let expression can only create bindings
within its body, which can force rightward drift and excessive nesting.

## Example

For example, this code written with current Rust syntax:

```rust
if let Some(a) = x {
    if let Some(b) = y {
        if let Some(c) = z {
            // ...
            do_something_with(a, b, c);
            // ...
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
let Some(a) = x else {
    return Err("bad x");
}
let Some(b) = y else {
    return Err("bad y");
}
let Some(c) = z else {
    return Err("bad z");
}
// ...
do_something_with(a, b, c);
// ...
```

## Versus `match`

It's possible to use `match` statements to emulate this today, but at a
significant cost in length and readability.  For example, this real-world code
from Servo:

```rust
let subpage_layer_info = match layer_properties.subpage_layer_info {
    Some(ref subpage_layer_info) => *subpage_layer_info,
    None => return,
};
```

is equivalent to this much simpler let-else statement:

```rust
let Some(ref subpage_layer_info) = layer_properties.subpage_layer_info else {
    return
}
```

The Swift programming language, which inspired Rust's if-let expression, also
includes a [guard-let-else][swift] statement which is equivalent to this
proposal except for the choice of keywords.

# Detailed design

Extend the Rust statement grammar to include the following production:

```
stmt_let_else = 'let' pat '=' expr 'else' block
```

The pattern must be refutable.  The body of the let-else statement (the
`block`) is evaluated only if the pattern match fails.  Any bindings created
by the pattern match will be in scope after the let-else statement (but not
within its body).

The body must diverge (i.e., it must panic, loop infinitely, call a diverging
function, or transfer control out of the enclosing block with a statement such
as `return`, `break`, or `continue`).  Therefore, code immediately following
the let-else statement is evaluated only if the pattern match succeeds.

The following code:

```rust
let pattern = expression else {
    body
}
```

is equivalent to this code in current Rust:

```rust
// `(a, b, c, ...)` is the list of all bindings in `pattern`.
let (a, b, c, ...) = match expression {
    pattern => (a, b, c, ...),
    _ => { body }
};
```

# Drawbacks

* “Must diverge” is an unusual requirement, which might be difficult to
  explain or lead to confusing errors for programmers new to this feature.

* To a human scanning the code, it's not obvious when looking at the start of
  a statement whether it is a `let ... else` or a regular `let` statement.

# Alternatives

* Don't make any changes; use existing syntax like `if let` and `match` as
  shown above, or write macros to simplify the code.

* Use the same semantics with different syntax. For example, the original
  version of this RFC used `if !let PAT = EXPR { BODY }`.

# Unresolved questions

* How much implementation complexity does this add?

[if-let]: https://github.com/rust-lang/rfcs/blob/master/text/0160-if-let.md
[swift]: https://developer.apple.com/library/prerelease/ios/documentation/Swift/Conceptual/Swift_Programming_Language/ControlFlow.html#//apple_ref/doc/uid/TP40014097-CH9-ID525
