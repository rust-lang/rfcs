- Feature Name: `let-else`
- Start Date: 2021-05-31
- RFC PR: [rust-lang/rfcs#3137](https://github.com/rust-lang/rfcs/pull/3137)
- Rust Issue: [rust-lang/rust#87335](https://github.com/rust-lang/rust/issues/87335)

# Summary
[summary]: #summary

Introduce a new `let PATTERN: TYPE = EXPRESSION else DIVERGING_BLOCK;` construct (informally called a
**let-else statement**), the counterpart of if-let expressions.

If the pattern match from the assigned expression succeeds, its bindings are introduced *into the
surrounding scope*. If it does not succeed, it must diverge (return `!`, e.g. return or break).
Technically speaking, let-else statements are refutable `let` statements.
The expression has some restrictions, notably it may not end with an `}` or be just a `LazyBooleanExpression`.

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

let-else is particularly useful when dealing with enums which are not `Option`/`Result`, and as such do not have access to e.g. `ok_or()`.
Consider the following example transposed from a real-world project written in part by the author:

Without let-else, as this code was originally written:
```rust
impl ActionView {
    pub(crate) fn new(history: &History<Action>) -> Result<Self, eyre::Report> {
        let mut iter = history.iter();
        let event = iter
            .next()
            // RFC comment: ok_or_else works fine to early return when working with `Option`.
            .ok_or_else(|| eyre::eyre!("Entity has no history"))?;

        if let Action::Register {
            actor: String,
            x: Vec<String>
            y: u32,
            z: String,
        } = event.action().clone() {
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

            // more lines omitted

            Ok(view)
        } else {
            // RFC comment: Far away from the associated conditional.
            Err(eyre::eyre!("must begin with a Register action"));
        }
    }
}
```

With let-else:
```rust
impl ActionView {
    pub(crate) fn new(history: &History<Action>) -> Result<Self, eyre::Report> {
        let mut iter = history.iter();
        let event = iter
            .next()
            // RFC comment: ok_or_else works fine to early return when working with `Option`.
            .ok_or_else(|| eyre::eyre!("Entity has no history"))?;

        let Action::Register {
            actor: String,
            x: Vec<String>
            y: u32,
            z: String,
        } = event.action().clone() else {
            // RFC comment: Directly located next to the associated conditional.
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

        // more lines omitted

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

However, with let-else this could be more succinct & clear:

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

Most expressions may be put into the expression position with two restrictions:
1. May not end with a `}` (before macro expansion). (Such things must be put in parentheses.)
2. May not be just a lazy boolean expression (`&&` or `||`). (Must not be a [`LazyBooleanExpression`][lazy-boolean-operators].)

While allowing e.g. `if {} else {}` directly in the expression position is technically feasible this RFC proposes it be
disallowed for programmer clarity so as to avoid `... else {} else {}` situations as discussed in the [drawbacks][] section.
Boolean matches are not useful with let-else and so lazy boolean expressions are disallowed for reasons noted in [future-possibilities][].
These types of expressions can still be used when combined in a less ambiguous manner with parentheses,
which is allowed under the two expression restrictions.
Invisible groupings from macros expansions are also allowed, however macro expansion representations to humans should include parentheses
around the expression output in this position if it ends in a `}` where possible (or otherwise show the invisible grouping).

Any refutable pattern that could be put into if-let's pattern position can be put into let-else's pattern position.

If the pattern is irrefutable, rustc will emit the `irrefutable_let_patterns` warning lint, as it does with an irrefutable pattern in an `if let`.

The `else` block must _diverge_, meaning the `else` block must return the [never type (`!`)][never-type]).
This could be a keyword which diverges (returns `!`), such as `return`, `break`, `continue` or `loop { ... }`, a diverging function like `std::process::abort` or `std::process::exit`, or a panic.

If the pattern does not match, the expression is not consumed, and so any existing variables from the surrounding scope are
accessible as they would normally be.

For patterns which match multiple variants, such as through the `|` (or) syntax, all variants must produce the same bindings (ignoring additional bindings in uneven patterns),
and those bindings must all be names the same. Valid example:
```rust
let MyEnum::VariantA(_, _, x) | MyEnum::VariantB { x, .. } = a else { return; };
```

let-else does not combine with the `let` from if-let, as if-let is not actually a _let statement_.
If you ever try to write something like `if let p = e else { } { }`, instead use a regular if-else by writing `if let p = e { } else { }`.

## Desugaring example

```rust
let Some(x) = y else { return; };
```

Desugars to

```rust
let x = match y {
    Some(x) => x,
    _ => {
        let nope: ! = { return; };
        match nope {}
    }
}
```

# Drawbacks
[drawbacks]: #drawbacks

## The diverging block

"Must diverge" is an unusual requirement, which doesn't exist elsewhere in the language as of the time of writing,
and might be difficult to explain or lead to confusing errors for programmers new to this feature.

However, rustc does have support for representing the divergence through the type-checker via `!` or any other uninhabited type,
so the implementation is not a problem.

## `let PATTERN = if {} else {} else {};`

One unfortunate combination of this feature with regular if-else expressions is the possibility of `let PATTERN = if { a } else { b } else { c };`.
This is likely to be unclear if anyone writes it, but does not pose a syntactical issue, as `let PATTERN = if y { a } else { b };` should always be
interpreted as `let Enum(x) = (if y { a } else { b });` (still a compile error as there no diverging block: `error[E0005]: refutable pattern in local binding: ...`)
because the compiler won't interpret it as `let PATTERN = (if y { a }) else { b };` since `()` is not an enum.

This can be overcome by making a raw if-else in the expression position a compile error and instead requiring that parentheses are inserted to disambiguate:
`let PATTERN = (if { a } else { b }) else { c };`.

This restriction can be made by checking if the expression ends in `}` after parsing but _before_ macro expansion.

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

// The RFC proposes boolean patterns with a lazy boolean operator (&& or ||)
//  be made into a compile error, for potential future compatibility with if-let-chains.
let true = a && b else {
    return;
};
```

The expression must not end with a `}`, in order to prevent `else {} else {}` (and similar) confusion, as noted in [drawbacks][#drawbacks].

The `else` must be followed by a block, as in `if {} else {}`. This else block must be diverging as the outer
context cannot be guaranteed to continue soundly without assignment, and no alternate assignment syntax is provided.

## Alternatives

While this feature can partly be covered by functions such `ok_or`/`ok_or_else` on the `Option` and `Result` types combined with the Try operator (`?`),
such functions do not exist automatically on custom enum types and require non-obvious and non-trivial implementation, and may not be map-able
to `Option`/`Result`-style functions at all (especially for enums where the "success" variant is contextual and there are many variants).
These functions will also not work for code which wishes to return something other than `Option` or `Result`.
Moreover, this does not cover diverging blocks that do something other than return with an error or target an enclosing `try` block,
for example if the diverging expression is `continue e` or `break 'outer_loop e`.

### Naming of `else` (`let ... otherwise { ... }`)

One often proposed alternative is to use a different keyword than `else`, such as `otherwise`.
This is supposed to help disambiguate let-else statements from other code with blocks and `else`.

This RFC avoids this as it would mean losing symmetry with if-else and if-let-else, and would require adding a new keyword.
Adding a new keyword could mean more to teach and could promote even more special casing around let-else's semantics.

### Comma-before-else (`, else { ... }`)

Another proposal very similar to renaming `else` it to have it be proceeded by some character such as a comma.

It is possible that adding such additional separating syntax would make combinations with expressions which have blocks
easier to read and less ambiguous, but is also generally inconsistent with the rest of the rust language at time of writing.

### Introducer syntax (`guard let ... {}`)

Another often proposed alternative is to add some introducer syntax (usually an extra keyword) to the beginning of the let-else statement,
to denote that it is different than a regular `let` statement.

One possible benefit of adding a keyword is that it could make a possible future extension for similarity to the (yet unimplemented) [if-let-chains][] feature more straightforward.
However, as mentioned in the [future-possibilities][] section, this is likely not necessary.

One drawback of this alternative syntax: it would introduce a binding without either starting a new block containing that binding or starting with a `let`.
Currently, in Rust, only a `let` statement can introduce a binding *in the current block* without starting a new block.
(Note that [`static`][] and [`const`][] are _items_, which can be forward-referenced.)
This alternative syntax would potentially make it more difficult for Rust developers to scan their code for bindings, as they would need to look for both `let` and `unless let`.
By contrast, a let-else statement begins with `let` and the start of a let-else statement looks exactly like a normal let binding.

This syntax has prior art in the Swift programming language, which includes a [guard-let-else][swift] statement
which is roughly equivalent to this proposal except for the choice of keywords.

### `if !let PAT = EXPR { BODY }`

The [old RFC][old-rfc] originally proposed this general feature via some kind of pattern negation as `if !let PAT = EXPR { BODY }`.

This RFC avoids adding any kind of new or special pattern matching rules. The pattern matching works as it does for if-let.
The general consensus in the old RFC was also that the negation syntax is much less clear than `if PATTERN = EXPR_WITHOUT_BLOCK else { /* diverge */ };`,
and partway through that RFC's lifecycle it was updated to be similar to this RFC's proposed let-else syntax.

The `if !let` alternative syntax would also share the binding drawback of the `unless let` alternative syntax.

### `let PATTERN = EXPR else DIVERGING_EXPR;`

A potential alternative to requiring parentheses in `let PATTERN = (if { a } else { b }) else { c };`
is to change the syntax of the `else` to no longer be a block but instead _any_ expression which diverges,
such as a `return`, `break`, or any block which diverges.

Example:
```rust
let Some(foo) = some_option else return None;
```

This RFC avoids this because it is overall less consistent with `else` from if-else, which requires block expressions.

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

Another potential alternative for fall-back:

```rust
enum AnEnum {
    Variant1(u32),
    Variant2(String),
}

let Ok(a) = x else match {
    Err(e) => return Err(e.into()),
}
```

### Assign to outer scope from `match`

Another alternative is to allow assigning to the outer scope from within a `match`.

```rust
match thing {
  Happy(x) => let x, // Assigns x to outer scope.
  Sad(y) => return Err(format!("We were sad because of {}", y)),
  Tragic(z) => return Err(format!("We cried hard because of {}", z)),
}
```

However this is not an obvious opposite to if-let, and would introduce an entirely new positional meaning of `let`.

### `||` in pattern-matching

A more complex, more flexible, but less obvious alternative is to allow `||` in any pattern matches as a fall-through match case fallback.
Such a feature would likely interact more directly with [if-let-chains][], but could also be use to allow refutable patterns in let statements
by covering every possible variant of an enum (possibly by use of a diverging fallback block similar to `_` in `match`).

For example, covering the use-case of let-else:
```rust
let Some(x) = a || { return; };
```

With a fallback:
```rust
let Some(x) = a || b || { return; };
```

Combined with `&&` as proposed in if-let-chains, constructs such as the following are conceivable:

```rust
let Enum::Var1(x) = a || b || { return anyhow!("Bad x"); } && let Some(z) = x || y || { break; };
// Complex. Both x and z are now in scope.
```

This is not a simple construct, and could be quite confusing to newcomers.

That said, such a thing is not perfectly obvious to write today, and might be just as confusing to read:
```rust
let x = if let Enum::Var1(v) = a {
    v
} else if let Enum::Var1(v) = b {
    v
} else {
    anyhow!("Bad x")
};
let z = if let Some(v) = x {
    v
} else if let Some(v) = y {
    v
} else {
    break;
};
// Complex. Both x and z are now in scope.
```

This is, as stated, a much more complex alternative interacting with much more of the language, and is also not an obvious opposite of if-let expressions.

### Macro

Another suggested solution is to create a macro which handles this.
A crate containing such a macro is mentioned in the [Prior art](#prior-art) section of this RFC.

This crate has not been widely used in the rust crate ecosystem with only 47k downloads over the ~6 years it has existed at the time of writing.

### Null Alternative

Don't make any changes; use existing syntax like `match` (or `if let`) as shown in the motivating example, or write macros to simplify the code.

# Prior art
[prior-art]: #prior-art

This RFC is a modernization of a [2015 RFC (pull request 1303)][old-rfc].

A lot of this RFC's proposals come from that RFC and its ensuing discussions.

The Swift programming language, which inspired Rust's if-let expression, also
includes a [guard-let-else][swift] statement which is roughly equivalent to this
proposal except for the choice of keywords.

A `guard!` macro implementing something very similar to this RFC has been available on crates.io since 2015 (the time of the old RFC).
- [Crate for `guard!`][guard-crate]
- [GitHub repo for `guard!`][guard-repo]

The `match` alternative in particular is fairly prevalent in rust code on projects which have many possible error conditions.

The Try operator allows for an `ok_or_else` alternative to be used where the types are only `Option` and `Result`,
which is considered to be idiomatic rust.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

## Readability in practice

Will `let ... else { ... };` be clear enough to humans in practical code, or will some introducer syntax be desirable?

## Conflicts with if-let-chains

Does this conflict too much with the if-let-chains RFC or vice-versa?

Neither this feature nor that feature should be stabilized without considering the other.

## Amount of special cases

Are there too many special-case interactions with other features?

## Grammar clarity

Does the grammar need to be clarified?

This RFC has some slightly unusual grammar requirements.

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
a specific boolean-only case can be avoided and thus parsing can be changed to leave the door open to this possible extension.
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

// The RFC proposes boolean patterns with a lazy boolean operator (&& or ||)
//  be made into a compile error, for potential future compatibility with if-let-chains.
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

If fall-back assignment as discussed above in [rationale-and-alternatives][] is desirable, it could be added a few different ways,
not all potential ways are covered here, but the ones which seem most popular at time of writing are:

### let-else-else-chains

Where the pattern is sequentially matched against each expression following an else, up until a required diverging block if the pattern did not match on any value.
Similar to the above-mentioned alternative of `||` in pattern-matching, but restricted to only be used with let-else.

```rust
let Some(x) = a else b else c else { return; };
```

Another way to look at let-else-else-chains: a `match` statement takes one expression and applies multiple patterns to it until one matches,
while let-else-else-chains would take one pattern and apply it to multiple expressions until one matches.

This has a complexity issue with or-patterns, where expressions can _easily_ become exponential.
(This is already possible with or-patterns with guards but this would make it much easier to encounter.)

```rust
let A(x) | B(x) = foo() else bar() else { return; };
```

### let-else-match

Where the `match` must cover all patters which are not the let assignment pattern.

```rust
let Ok(a) = x else match {
    Err(e) => return Err(e.into()),
}
```

## `||` in pattern-matching

A variant of `||` in pattern-matching could still be a non-conflicting addition if it was allowed to be refutable, ending up with constructs similar to the
above mentioned let-else-else-chains. In this way it would add to let-else rather than replace it.

```rust
let Some(x) = a || b else { return; };
```

## let-else within if-let

This RFC naturally brings with it the question of if let-else should be allowable in the `let` position within if-let,
creating a potentially confusing and poorly reading construct:

```rust
if let Some(x) = y else { return; } {
    // I guess this RFC had it coming for it
}
```

However, since the `let` within if-let is part of the if-let expression and is not an actual `let` statement, this would have to be
explicitly allowed. This RFC does not propose we allow this. Rather, rust should avoid ever allowing this,
because it is confusing to read syntactically, and it is functionally similar to `if let p = e { } else { }` but with more drawbacks.

[`const`]: https://doc.rust-lang.org/reference/items/constant-items.html
[`static`]: https://doc.rust-lang.org/reference/items/static-items.html
[expressions]: https://doc.rust-lang.org/reference/expressions.html#expressions
[guard-crate]: https://crates.io/crates/guard
[guard-repo]: https://github.com/durka/guard
[if-let]: https://rust-lang.github.io/rfcs/0160-if-let.html
[if-let-chains]: https://rust-lang.github.io/rfcs/2497-if-let-chains.html
[lazy-boolean-operators]: https://doc.rust-lang.org/reference/expressions/operator-expr.html#lazy-boolean-operators
[never-type]: https://doc.rust-lang.org/std/primitive.never.html
[old-rfc]: https://github.com/rust-lang/rfcs/pull/1303
[swift]: https://developer.apple.com/library/prerelease/ios/documentation/Swift/Conceptual/Swift_Programming_Language/ControlFlow.html#//apple_ref/doc/uid/TP40014097-CH9-ID525
