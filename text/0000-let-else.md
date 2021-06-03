- Feature Name: `let-else`
- Start Date: 2021-05-31
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Introduce a new `let PATTERN = EXPRESSION_WITHOUT_BLOCK else DIVERGING_BLOCK;` construct (informally called a
**let-else statement**), the counterpart of if-let expressions.

If the pattern match from the assigned expression succeeds, its bindings are introduced *into the
surrounding scope*. If it does not succeed, it must diverge (return `!`, e.g. return or break).
Technically speaking, let-else statements are refutable `let` statements.

This RFC is a modernization of a [2015 RFC (pull request 1303)][old-rfc] for an almost identical feature.

# Motivation
[motivation]: #motivation

`let else` simplifies some very common error-handling patterns.
It is the natural counterpart to `if let`, just as `else` is to regular `if`.

[if-let expressions][if-let] offer a succinct syntax for pattern matching single patterns.
This is particularly useful for unwrapping types like `Option`, particularly those with a clear "success" variant
for the given context but no specific "failure" variant.
However, an if-let expression can only create bindings within its body, which can force 
rightward drift, introduce excessive nesting, and separate conditionals from error paths.

let-else statements move the "failure" case into the body block, while allowing
the "success" case to continue in the surrounding context without additional nesting.

let-else statements are also more succinct and natural than emulating the equivalent pattern with `match` or if-let,
which require intermediary bindings (usually of the same name).

## Examples

The following two code examples are possible options with current Rust code.

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

```rust
let a = match x {
    Some(a) => a,
    _ => return Err("bad x"),
}
let b = match y {
    Some(b) => b,
    _ => return Err("bad y"),
}
let c = match z {
    Some(c) => c,
    _ => return Err("bad z"),
}
// ...
do_something_with(a, b, c);
// ...
```

Both of the above examples would be able to be written as:

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

which succinctly avoids bindings of the same name, rightward shift, etc.

let-else is even more useful when dealing with enums which are not `Option`/`Result`, consider how the
following code would look without let-else (transposed from a real-world project written in part by the author):

```rust
impl ActionView {
    pub(crate) fn new(history: &History<Action>) -> Result<Self, eyre::Report> {
        let mut iter = history.iter();
        let event = iter
            .next()
            .ok_or_else(|| eyre::eyre!("Entity has no history"))?;

        let Action::Register {
            actor: String,
            x: Vec<String>
            y: u32,
            z: String,
        } = event.action().clone() else {
            // RFC Author's note:
            //   Without if-else this was separated from the conditional 
            //   by a substantial block of code which now follows below.
            return Err(eyre::eyre!("must begin with a Register action"));
        };

        let created = *event.created();
        let mut view = ActionView {
            registered_by: (actor, created),
            a: (actor.clone(), x, created),
            b: (actor.clone(), y, created),
            c: (z, created),
            d: Vec::new(),

            e: None,
            f: None,
            g: None,
        };
        for event in iter {
            view.update(&event)?;
        }
        Ok(view)
    }
}
```

## A practical refactor with `match`

It is possible to use `match` expressions to emulate this today, but at a
significant cost in length and readability.

A refactor on an http server codebase in part written by the author to move some if-let conditionals to early-return `match` expressions
yielded 4 changes of large if-let blocks over `Option`s to use `ok_or_else` + `?`, and 5 changed to an early-return `match`.
The commit of the refactor was +531 −529 lines of code over a codebase of 4111 lines of rust code.
The largest block was 90 lines of code which was able to be shifted to the left, and have its error case moved up to the conditional,
showing the value of early-returns for this kind of program.

While that refactor was positive, it should be noted that such alternatives were unclear the authors when they were less experienced rust programmers,
and also that the resulting `match` code includes syntax boilerplate (e.g. the block) that could theoretically be reduced today but also interferes with rustfmt's rules:

```rust
let features = match geojson {
    GeoJson::FeatureCollection(features) => features,
    _ => {
        return Err(format_err_status!(
            422,
            "GeoJSON was not a Feature Collection",
        ));
    }
};
```

However, with if-let this could be more succinct & clear:

```rust
let GeoJson::FeatureCollection(features) = geojson else {
    return Err(format_err_status!(
        422,
        "GeoJSON was not a Feature Collection",
    ));
};
```

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

A common pattern in non-trivial code where static guarantees can not be fully met (e.g. I/O, network or otherwise) is to check error cases when possible before proceeding,
and "return early", by constructing an error `Result` or an empty `Option`, and returning it before the "happy path" code.

This pattern serves no practical purpose to a computer, but it is helpful for humans interacting with the code.
Returning early helps improve code clarity in two ways:
- Ensuring the returned result in near the conditional, visually, as the following logic may be lengthy.
- Reduces rightward shift, as the error return is now in the block, rather than the following logic.

This RFC proposes _(Rust provides)_ an extension to `let` assignment statements to help with this pattern, an `else { }` which can follow a pattern match
as a `let` assigning statement:

```rust
let Some(a) = an_option else {
    // Called if `an_option` is not `Option::Some(T)`.
    // This block must diverge (stop executing the existing context to the parent block or function).
    return;
};

// `a` is now in scope and is the type which the `Option` contained.
```

This is a counterpart to `if let` expressions, and the pattern matching works identically, except that the value from the pattern match
is assigned to the surrounding scope rather than the block's scope.

# Reference-level explanations
[reference-level-explanation]: #reference-level-explanation

let-else is syntactical sugar for `match` where the non-matched case diverges.
```rust
let pattern = expr else {
    /* diverging expr */
};
```
desugars to
```rust
let (each, binding) = match expr { 
    pattern => (each, binding),
    _ => { 
        /* diverging expr */
    }
};
```

Any expression may be put into the expression position except an `if {} else {}` as explain below in [drawbacks][].
While `if {} else {}` is technically feasible this RFC proposes it be disallowed for programmer clarity to avoid an `... else {} else {}` situation.
Rust already provides us with such a restriction, [`ExpressionWithoutBlock`][expressions].

Any pattern that could be put into if-let's pattern position can be put into let-else's pattern position.

The `else` block must diverge. This could be a keyword which diverges (returns `!`), or a panic.
This likely necessitates a new subtype of `BlockExpression`, something like `BlockExpressionDiverging`.
Allowed keywords:
- `return`
- `break`
- `continue`

If the pattern does not match, the expression is not consumed, and so any existing variables from the surrounding scope are
accessible as they would normally be.

# Drawbacks
[drawbacks]: #drawbacks

## The diverging block

"Must diverge" is an unusual requirement, which doesn't exist elsewhere in the language as of the time of writing, 
and might be difficult to explain or lead to confusing errors for programmers new to this feature.

This also necessitates a new block expression subtype, something like `BlockExpressionDiverging`.

## `let PATTERN = if {} else {} else {};`

One unfortunate combination of this feature with regular if-else expressions is the possibility of `let PATTERN = if { a } else { b } else { c };`.
This is likely to be unclear if anyone writes it, but does not pose a syntactical issue, as `let PATTERN = if y { a } else { b };` should always be
interpreted as `let Enum(x) = (if y { a } else { b });` (still a compile error as there no diverging block: `error[E0005]: refutable pattern in local binding: ...`)
because the compiler won't interpret it as `let PATTERN = (if y { a }) else { b };` since `()` is not an enum.

This can be overcome by making a raw if-else in the expression position a compile error and instead requiring that parentheses are inserted to disambiguate:
`let PATTERN = (if { a } else { b }) else { c };`.

Rust already provides us with such a restriction, and so the expression can be restricted to be a [`ExpressionWithoutBlock`][expressions].

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

let-else attempts to be as consistent as possible to similar existing syntax.

Fundamentally it is treated as a `let` statement, necessitating an assignment and the trailing semicolon.

Pattern matching works identically to if-let, no new "negation" pattern matching rules are introduced.

Operator precedence with `&&` in made to be like if-let, requiring that a case which is an error prior to this RFC be changed to be a slightly different error.
This is for a possible extension for let-else similar to the (yet unimplemented) if-else-chains feature, as mentioned in [future-possibilities][] with more detail.
Specifically, while the following example is an error today, by the default `&&` operator rules it would cause problems with if-let-chains like `&&` chaining:

```rust
let a = false;
let b = false;

// The RFC proposes boolean matches like this be either:
// - Made into a compile error, or
// - Made to be parsed internally like if-let-chains: `(let true = a) && b else { ... };`
let true = a && b else {
    return;
};
```

The expression can be any [`ExpressionWithoutBlock`][expressions], in order to prevent `else {} else {}` confusion, as noted in [drawbacks][#drawbacks].

The `else` must be followed by a block, as in `if {} else {}`. This else block must be diverging as the outer
context cannot be guaranteed to continue soundly without assignment, and no alternate assignment syntax is provided.
## Alternatives

While this feature can effectively be covered by functions such `or_or`/`ok_or_else` on the `Option` and `Result` types combined with the Try operator (`?`),
such functions do not exist automatically on custom enum types and require non-obvious and non-trivial implementation, and may not be map-able
to `Option`/`Result`-style functions at all (especially for enums where the "success" variant is contextual and there are many variants).

### `unless let ... {}` / `try let ... {}`

An often proposed alternative is to add an extra keyword to the beginning of the let-else statement, to denote that it is different than a regular `let` statement.

One possible benefit of adding a keyword is that it could make a possible future extension for similarity to the (yet unimplemented) [if-let-chains][] feature more straightforward.
However, as mentioned in the [future-possibilities][] section, this is likely not necessary.

This syntax has prior art in the Swift programming language, which includes a [guard-let-else][swift] statement
which is roughly equivalent to this proposal except for the choice of keywords.

### `let PATTERN = EXPR else return EXPR;`

A potential alternative to requiring parentheses in `let PATTERN = (if { a } else { b }) else { c };` is to change the syntax of the `else` to no longer be a block
but instead an expression which starts with a diverging keyword, such as `return` or `break`.

Example:
```
let Some(foo) = some_option else return None;
```

This RFC avoids this because it is overall less consistent with `else` from if-else, which require blocks.

This was originally suggested in the old RFC, comment at https://github.com/rust-lang/rfcs/pull/1303#issuecomment-188526691

### `else`-block fall-back assignment

A fall-back assignment alternate to the diverging block has been proposed multiple times in relation to this feature in the [original rfc][] and also in out-of-RFC discussions.

This RFC avoids this proposal, because there is no clear syntax to use for it which would be consistent with other existing features.
Also use-cases for having a single fall-back are much more rare and unusual, where as use cases for the diverging block are very common.
This RFC proposes that most fallback cases are sufficiently or better covered by using `match`.

An example, using a proposal to have the binding be visible and assignable from the `else`-block.
Note that this is incompatible with this RFC and could probably not be added as an extension from this RFC.

```rust
enum AnEnum {
    Variant1(u32),
    Variant2(String),
}

let AnEnum::Variant1(a) = x else {
    a = 42;
};
```

Another potential alternative for fall-back which could be added with an additional keyword as a future extension:

```rust
enum AnEnum {
    Variant1(u32),
    Variant2(String),
}

let AnEnum::Variant1(a) = x else assign a {
    a = 42;
};
```

### `if !let PAT = EXPR { BODY }`

The [old RFC][old-rfc] originally proposed this general feature via some kind of pattern negation as `if !let PAT = EXPR { BODY }`.

This RFC avoids adding any kind of new or special pattern matching rules. The pattern matching works as it does for if-let.
The general consensus in the old RFC was also that the negation syntax is much less clear than `if PATTERN = EXPR_WITHOUT_BLOCK else { /* diverge */ };`,
and partway through that RFC's lifecycle it was updated to be similar to this RFC's proposed let-else syntax.

### Complete Alternative

Don't make any changes; use existing syntax like `match` (or `if let`) as shown in the motivating example, or write macros to simplify the code.

# Prior art
[prior-art]: #prior-art

This RFC is a modernization of a [2015 RFC (pull request 1303)][old-rfc].

A lot of this RFC's proposals come from that RFC and its ensuing discussions.

The Swift programming language, which inspired Rust's if-let expression, also
includes a [guard-let-else][swift] statement which is roughly equivalent to this
proposal except for the choice of keywords.

The `match` alternative in particular is fairly prevalent in rust code on projects which have many possible error conditions.

The Try operator allows for an `ok_or_else` alternative to be used where the types are only `Option` and `Result`,
which is considered to be idiomatic rust.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None known at time of writing due to extensive pre-discussion in Zulip:
https://rust-lang.zulipchat.com/#narrow/stream/213817-t-lang/topic/.60let.20pattern.20.3D.20expr.20else.20.7B.20.2E.2E.2E.20.7D.60.20statements

# Future possibilities
[future-possibilities]: #future-possibilities

## if-let-chains

An RFC exists for a (unimplemented at time of writing) feature called [if-let-chains][]:

```rust
if let Some(foo) = expr() && foo.is_baz() && let Ok(yay) = qux(foo) { ... }
```

While this RFC does not introduce or propose the same thing for let-else it attempts to allow it to be a future possibility for
potential future consistency with if-let-chains.

The primary obstacle is existing operator order precedence.
Given the above example, it would likely be parsed as follows with ordinary operator precedence rules for `&&`:
```rust
let Some(foo) = (expr() && foo.is_baz() && let Ok(yay) = qux(foo) else { ... })
```

However, given that all existing occurrences of this behavior before this RFC are type errors anyways,
a specific boolean-only case can be avoided and thus parsing can be changed to lave the door open to this possible extension.
This boolean case is always equivalent to a less flexible `if` statement and as such is not useful.

```rust
let maybe = Some(2);
let has_thing = true;

// Always an error regardless, because && only operates on booleans.
let Some(x) = maybe && has_thing else {
    return;
};
```

```rust
let a = false;
let b = false;

// The RFC proposes boolean matches like this be either:
// - Made into a compile error, or
// - Made to be parsed internally like if-let-chains: `(let true = a) && b else { ... };`
let true = a && b else {
    return;
};
```

Note also that this does not work today either, because booleans are refutable patterns:
```
error[E0005]: refutable pattern in local binding: `false` not covered
 --> src/main.rs:5:9
  |
5 |     let true = a && b;
  |         ^^^^ pattern `false` not covered
  |
  = note: `let` bindings require an "irrefutable pattern", like a `struct` or an `enum` with only one variant
```

## Fall-back assignment

This RFC does not suggest that we do any of these, but notes that they would be future possibilities.

If fall-back assignment as discussed above in [rationale-and-alternatives][] is desirable, it could be added with an additional keyword as a future extension:

```rust
enum AnEnum {
    Variant1(u32),
    Variant2(String),
}

let AnEnum::Variant1(a) = x else assign a {
    a = 42;
};
```

Another potential form of the fall-back extension:

```rust
let Ok(a) = x else match {
    Err(e) => return Err(e.into()),
}
```

[expressions]: https://doc.rust-lang.org/reference/expressions.html#expressions
[old-rfc]: https://github.com/rust-lang/rfcs/pull/1303
[if-let]: https://rust-lang.github.io/rfcs/0160-if-let.html
[if-let-chains]: https://rust-lang.github.io/rfcs/2497-if-let-chains.html
[swift]: https://developer.apple.com/library/prerelease/ios/documentation/Swift/Conceptual/Swift_Programming_Language/ControlFlow.html#//apple_ref/doc/uid/TP40014097-CH9-ID525
