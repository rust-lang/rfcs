- Feature Name: `guard_patterns`
- Start Date: 2024-05-13
- RFC PR: [rust-lang/rfcs#3637](https://github.com/rust-lang/rfcs/pull/3637)
- Tracking Issue: [rust-lang/rust#129967](https://github.com/rust-lang/rust/issues/129967)

# Summary

[summary]: #summary

This RFC proposes to add a new kind of pattern, the **guard pattern.** Like match arm guards, guard patterns restrict another pattern to match only if an expression evaluates to `true`. The syntax for guard patterns, `pat if condition`, is compatible with match arm guard syntax, so existing guards can be superceded by guard patterns without breakage.

# Motivation

[motivation]: #motivation

Guard patterns, unlike match arm guards, can be nested within other patterns. In particular, guard patterns nested within or-patterns can depend on the branch of the or-pattern being matched. This has the potential to simplify certain match expressions, and also enables the use of guards in other places where refutable patterns are acceptable. Furthermore, by moving the guard condition closer to the bindings upon which it depends, pattern behavior can be made more local.

# Guide-level explanation

[guide-level-explanation]: #guide-level-explanation

Guard patterns allow you to write guard expressions to decide whether or not something should match anywhere you can use a pattern, not just at the top level of `match` arms.

For example, imagine that you're writing a function that decides whether a user has enough credit to buy an item. Regular users have to pay 100 credits, but premium subscribers get a 20% discount. You could implement this with a match expression as follows:

```rust
match user.subscription_plan() {
    Plan::Regular if user.credit() >= 100 => {
        // Complete the transaction.
    }
    Plan::Premium if user.credit() >= 80 => {
        // Complete the transaction.
    }
    _ => {
        // The user doesn't have enough credit, return an error message.
    }
}
```

But this isn't great, because two of the match arms have exactly the same body. Instead, we can write

```rust
match user.subscription_plan() {
    (Plan::Regular if user.credit() >= 100) | (Plan::Premium if user.credit() >= 80) => {
        // Complete the transaction.
    }
    _ => {
        // The user doesn't have enough credit, return an error message.
    }
}
```

Now we have just one arm for a successful transaction, with an or-pattern combining the two arms we used to have. The two nested patterns are of the form

```rust
pattern if expr
```

This is a **guard pattern**. It matches a value if `pattern` (the pattern it wraps) matches that value, _and_ `expr` evaluates to `true`. Like in match arm guards, `expr` can use values bound in `pattern`.

## For New Users

For new users, guard patterns are better explained without reference to match arm guards. Instead, they can be explained by similar examples to the ones currently used for match arm guards, followed by an example showing that they can be nested within other patterns and used outside of match arms.

# Reference-level explanation

[reference-level-explanation]: #reference-level-explanation

## Supersession of Match Arm Guards

Rather than being parsed as part of the match expression, guards in match arms will instead be parsed as a guard pattern. For this reason, the `if` pattern operator must have lower precedence than all other pattern operators.

That is,

```rs
// Let <=> denote equivalence of patterns.

x @ A(..) if pred       <=> (x @ A(..)) if pred
&A(..) if pred          <=> (&A(..)) if pred
A(..) | B(..) if pred   <=> (A(..) | B(..)) if pred
```

## Precedence Relative to `|`

Consider the following match expression:

```rust
match foo {
    A | B if c | d => {},
}
```

This match arm is currently parsed as `(A | B) if (c | d)`, with the first `|` being the or-operator on patterns and the second being the bitwise OR operator on expressions. Therefore, to maintain backwards compatability, `if` must have lower precedence than `|` on both sides (or equivalently, for both meanings of `|`). For that reason, guard patterns nested within or-patterns must be explicitly parenthesized:

```rust
// This is not an or-pattern of guards:
    a if b | c if d
<=> (a if (b | c)) if d

// Instead, write
(a if b) | (c if d)
```

## In Assignment-Like Contexts

There's an ambiguity between `=` used as the assignment operator within the guard
and used outside to indicate assignment to the pattern (e.g. in `if let`)
Therefore guard patterns appearing at the top level in those places must also be parenthesized:

```rust
// Not allowed:
let x if guard(x) = foo() {} else { loop {} }
if let x if guard(x) = foo() {}
while let x if guard(x) = foo() {}

// Allowed:
let (x if guard(x)) = foo() {} else { loop {} }
if let (x if guard(x)) = foo() {}
while let (x if guard(x)) = foo() {}
```

Therefore the syntax for patterns becomes

> **<sup>Syntax</sup>**\
> _Pattern_ :\
> &nbsp;&nbsp; &nbsp;&nbsp; _PatternNoTopGuard_\
> &nbsp;&nbsp; | _GuardPattern_
>
> _PatternNoTopGuard_ :\
> &nbsp;&nbsp; &nbsp;&nbsp; `|`<sup>?</sup> _PatternNoTopAlt_ ( `|` _PatternNoTopAlt_ )<sup>\*</sup>

With `if let` and `while let` expressions now using `PatternNoTopGuard`. `let` statements and function parameters can continue to use `PatternNoTopAlt`.

## Bindings Available to Guards

The only bindings available to guard conditions are

- bindings from the scope containing the pattern match, if any; and
- bindings introduced by identifier patterns _within_ the guard pattern.

This disallows, for example, the following uses:

```rust
// ERROR: `x` bound outside the guard pattern
let (x, y if x == y) = (0, 0) else { /* ... */ }
let [x, y if x == y] = [0, 0] else { /* ... */ }
let TupleStruct(x, y if x == y) = TupleStruct(0, 0) else { /* ... */ }
let Struct { x, y: y if x == y } = Struct { x: 0, y: 0 } else { /* ... */ }

// ERROR: `x` cannot be used by other parameters' patterns
fn function(x: usize, ((y if x == y, _) | (_, y)): (usize, usize)) { /* ... */ }
```

Note that in each of these cases besides the function, the condition is still possible by moving the condition outside of the destructuring pattern:

```rust
let ((x, y) if x == y) = (0, 0) else { /* ... */ }
let ([x, y] if x == y) = [0, 0] else { /* ... */ }
let (TupleStruct(x, y) if x == y) = TupleStruct(0, 0) else { /* ... */ }
let (Struct { x, y } if x == y) = Struct { x: 0, y: 0 } else { /* ... */ }
```

In general, guards can, without changing meaning, "move outwards" until they reach an or-pattern where the condition can be different in other branches, and "move inwards" until they reach a level where the identifiers they reference are not bound.

## As Macro Arguments

Currently, `if` is in the follow set of `pat` and `pat_param` fragments, so top-level guards cannot be used as arguments for the current edition. This is identical to the situation with top-level or-patterns as macro arguments, and guard patterns will take the same approach:

1. Update `pat` fragments to accept `PatternNoTopGuard` rather than `Pattern`.
2. Introduce a new fragment specifier, `pat_no_top_guard`, which works in all editions and accepts `PatternNoTopGuard`.
3. In the next edition, update `pat` fragments to accept `Pattern` once again.

# Drawbacks

[drawbacks]: #drawbacks

Rather than matching only by structural properties of ADTs, equality, and ranges of certain primitives, guards give patterns the power to express arbitrary restrictions on types. This necessarily makes patterns more complex both in implementation and in concept.

# Rationale and alternatives

[rationale-and-alternatives]: #rationale-and-alternatives

## "Or-of-guards" Patterns

Earlier it was mentioned that guards can "move outwards" up to an or-pattern without changing meaning:

```rust
    (Ok(Ok(x if x > 0))) | (Err(Err(x if x < 0)))
<=> (Ok(Ok(x) if x > 0)) | (Err(Err(x) if x < 0))
<=> (Ok(Ok(x)) if x > 0) | (Err(Err(x)) if x < 0)
// Cannot move outwards any further, because the conditions are different.
```

In most situations, it is preferable to have the guard as far outwards as possible; that is, at the top-level of the whole pattern or immediately within one alternative of an or-pattern.
Therefore, we could choose to restrict guard patterns so that they appear only in these places.
This RFC refers to this as "or-of-guards" patterns, because it changes or-patterns from or-ing together a list of patterns to or-ing together a list of optionally guarded patterns.

Note that, currently, most patterns are actually parsed as an or-pattern with only one choice.
Therefore, to achieve the effect of forcing patterns as far out as possible guards would only be allowed in or-patterns with more than one choice.

There are, however, a couple reasons where it could be desirable to allow guards further inwards than strictly necessary.

### Localization of Behavior

Sometimes guards are only related to information from a small part of a large structure being matched.

For example, consider a function that iterates over a list of customer orders and performs different actions depending on the customer's subscription plan, the item type, the payment info, and various other factors:

```rust
match order {
    Order {
        // These patterns match based on method calls, necessitating the use of a guard pattern:
        customer: customer if customer.subscription_plan() == Plan::Premium,
        payment: Payment::Cash(amount) if amount.in_usd() > 100,

        item_type: ItemType::A,
        // A bunch of other conditions...
    } => { /* ... */ }
    // Other similar branches...
}
```

Here, the pattern `customer if customer.subscription_plan() == Plan::Premium` has a clear meaning: it matches customers with premium subscriptions. Similarly, `Payment::Cash(amount) if amount.in_usd() > 100` matches cash payments of amounts greater than 100USD. All of the behavior of the pattern pertaining to the customer is in one place, and all behavior pertaining to the payment is in another. However, if we move the guard outwards to wrap the entire order struct, the behavior is spread out and much harder to understand -- particularly if the two conditions are merged into one:

```rust
// The same match statement using or-of-guards.
match order {
    Order {
        customer,
        payment: Payment::Cash(amount),
        item_type: ItemType::A,
        // A bunch of other conditions...
    } if customer.subscription_plan() == Plan::Premium && amount.in_usd() > 100 => { /* ... */ }
    // Other similar branches...
}
```

### Pattern Macros

If guards can only appear immediately within or-patterns, then either

- pattern macros can emit guards at the top-level, in which case they can only be called immediately within or-patterns without risking breakage if the macro definition changes (even to another valid pattern!); or
- pattern macros cannot emit guards at the top-level, forcing macro authors to use terrible workarounds like `(Some(x) if guard(x)) | (Some(x) if false)` if they want to use the feature.

This can also be seen as a special case of the previous argument, as pattern macros fundamentally assume that patterns can be built out of composable, local pieces.

## Deref and Const Patterns Must Be Pure, But Not Guards

It may seem odd that we explicitly require const patterns to use pure `PartialEq` implementations (and the upcoming [proposal](https://hackmd.io/4qDDMcvyQ-GDB089IPcHGg) for deref patterns to use pure `Deref` implementations), but allow arbitrary side effects in guards. The ultimate reason for this is that, unlike const patterns and the proposed deref patterns, guard patterns are always refutable.

Without the requirement of `StructuralPartialEq` we could write a `PartialEq` implementation which always returns `false`, resulting either in UB or a failure to ensure match exhaustiveness:

```rust
const FALSE: EvilBool = EvilBool(false);
const TRUE: EvilBool = EvilBool(true);

match EvilBool(false) {
    FALSE => {},
    TRUE => {},
}
```

And similarly, with an impure version of the proposed deref patterns, we could write a `Deref` impl which alternates between returning `true` or `false` to get UB:

```rust
match EvilBox::new(false) {
    deref!(true) => {} // Here the `EvilBox` dereferences to `false`.
    deref!(false) => {} // And here to `true`.
}
```

However, this is not a problem with guard patterns because they already need an irrefutable alternative anyway.
For example, we could rewrite the const pattern example with guard patterns as follows:

```rust
match EvilBool(false) {
    x if x == FALSE => {},
    x if x == TRUE => {},
}
```

But this will always be a compilation error because the `match` statement is no longer assumed to be exhaustive.

# Prior art

[prior-art]: #prior-art

This feature has been implemented in the [Unison](https://www.unison-lang.org/docs/language-reference/guard-patterns/), [Wolfram](https://reference.wolfram.com/language/ref/Condition.html), and [E ](<https://en.wikipedia.org/wiki/E_(programming_language)>) languages.

Guard patterns are also very similar to Haskell's [view patterns](https://ghc.gitlab.haskell.org/ghc/doc/users_guide/exts/view_patterns.html), which are more powerful and closer to a hypothetical "`if let` pattern" than a guard pattern as this RFC proposes it.

# Unresolved questions

[unresolved-questions]: #unresolved-questions

## Allowing Mismatching Bindings When Possible

Ideally, users would be able to write something to the effect of

```rust
match Some(0) {
    Some(x if x > 0) | None => {},
    _ => {}
}
```

This is also very useful for macros, because it allows

1. pattern macros to use guard patterns freely without introducing new bindings the user has to be aware of in order to use the pattern macro within a disjunction, and
2. macro users to pass guard patterns to macros freely, even if the macro uses the pattern within a disjunction.

As mentioned above, this case is not covered by this RFC, because `x` would need to be bound in both cases of the disjunction.

### Possible Design

[@tmandry proposed](https://github.com/rust-lang/rfcs/pull/3637#issuecomment-2307839511) amending the rules for how names can be bound in patterns to the following:

1. Unchanged: If a name is bound in any part of a pattern, it shadows existing definitions of the name.
2. Unchanged: If a name bound by a pattern is used in the body, it must be defined in every part of a disjunction and be the same type in each.
3. Removed: ~~Bindings introduced in one branch of a disjunction must be introduced in all branches.~~
4. Added: If a name is bound in multiple parts of a disjunction, it must be bound to the same type in every part. (Enforced today by the combination of 2 and 3.)

## How to Refer to Guard Patterns

Some possibilities:

- "Guard pattern" will likely be most intuitive to users already familiar with match arm guards. Most likely, this includes anyone reading this, which is why this RFC uses that term.
- "`if`-pattern" agrees with the naming of or-patterns, and obviously matches the syntax well. This is probably the most intuitive name for new users learning the feature.
- Some other possibilities: "condition/conditioned pattern," "refinement/refined pattern," "restriction/restricted pattern," or "predicate/predicated pattern."

[future-possibilities]: #future-possibilities

# Future Possibilities

## Allowing `if let`

Users expect to be able to write `if let` where they can write `if`. Allowing this in guard patterns would make them significantly more powerful, but also more complex.

One way to think about this is that patterns serve two functions:

1. Refinement: refutable patterns only match some subset of a type's values.
2. Destructuring: patterns use the structure common to values of that subset to extract data.

Guard patterns as described here provide _arbitrary refinement_. That is, guard patterns can match based on whether any arbitrary expression evaluates to true.

Allowing `if let` allows not just arbitrary refinement, but also _arbitrary destructuring_. The value(s) bound by an `if let` pattern can depend on the value of an arbitrary expression.
