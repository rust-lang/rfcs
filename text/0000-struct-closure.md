- Feature Name: `struct_closure`
- Start Date: 2022-08-05
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Concrete Closure Types

# Motivation
[motivation]: #motivation

This proposal exists entirely out of spite for the `char::is_ascii*` family of
functions.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Closures in Rust are represented by the struct `core::ops::Closure`, while
callables are represented by the traits `Fn`, `FnMut` and `FnOnce`. This
enables closures to participate specially in trait resolution, and be
distinguished from other types of callables (particularly user-defined ones).

In general, when writing generic code, it is preferred to use the `Fn*` traits.
However, in some edge-cases it can be useful to use `Closure` or even both.

## Examples

The main user of the `Closure` struct is `Pattern`:

```rust
fn takes_char(x: char) -> bool {
    true
}

fn takes_char_ref(x: &char) -> bool {
    true
}

// these both work:
"hello".trim_start_matches(takes_char);
"hello".trim_start_matches(takes_char_ref);
```

This wouldn't be possible if `Pattern` were instead relying on the traits,
as it's entirely possible for a type to implement multiple `Fn` traits, but a
closure will never do so.

Due to historical reasons, `char` provides some functions that take `&self` and
some which take `self`:

```rust
char::is_ascii(&'a');
char::is_numeric('9');
```

and they both work as `Pattern`, thanks to `Closure`:

```rust
"hello".trim_start_matches(char::is_ascii);
"hello".trim_start_matches(char::is_numeric);
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Currently the `Fn*` traits are pretty unstable, including their internal
representation. This proposal simply introduces a new, special lang item:

```rust
#[rustc_paren_sugar]
#[lang="closure"]
struct Closure<F: FnOnce<Args>, Args> {
    // compiler special...
}

impl<F, Args> Fn<Args> for Closure<F, Args> where F: Fn<Args> {
    // details omitted...
}
impl<F, Args> FnMut<Args> for Closure<F, Args> where F: FnMut<Args> {
    // details omitted...
}
impl<F, Args> FnOnce<Args> for Closure<F, Args> where F: FnOnce<Args> {
    // details omitted...
}
```

Some things to note: The details of `Args` are subject to change. This proposal
relies on that, because otherwise this cannot have the correct `for<'a>`
bounds. This proposal does not define the details of `Args`, but suggests they
might be of `fn` type. (Indeed, it would be easiest to implement this by
defining `Args` to be an `fn` type. Tho do note the `Args` in `Closure` does
not need to be the same `Args` in `Fn*`, as long as the compiler knows how to
map between them.)

In general, it'd be awkward to use this in a function. Consider:

```rust
fn foo<F: Fn...>(closure: Closure<F, ...>)
```

It's much simpler to just use F directly. But in a trait, things are different,
because you can't have multiple generic impls like that, so it actually becomes
useful:

```rust
// this DOES NOT WORK
impl<F: FnMut(char) -> bool> Pattern for F { ... }
impl<F: FnMut(&char) -> bool> Pattern for F { ... }

// but this would, because these types don't overlap
impl<F: FnMut(char) -> bool> Pattern for Closure<F, fn(char) -> bool> { ... }
impl<F: FnMut(&char) -> bool> Pattern for Closure<F, fn(&char) -> bool> { ... }
```

This is, for all intents and purposes, a "zero-cost" wrapper over
closures (except for one layer of type system nesting); and unlike `fn` types,
it doesn't incur the additional pointer overhead, being the same size as the
underlying closure.

Additionally, as this is the main goal of the RFC, we add the following impl to
`Pattern`:

```rust
impl<'a, F> Pattern<'a> for Closure<F, fn(&char) -> bool>
where
    F: FnMut(&char) -> bool,
{
    pattern_methods!(CharRefPredicateSearcher<'a, F>, MultiCharEqPattern, CharRefPredicateSearcher);    
}
```

Which, for all intents and purposes, is the same as the existing generic impl,
except with `&char` instead of `char`.

# Drawbacks
[drawbacks]: #drawbacks

- Small extra cost due to additional wrapper type. This could affect existing
    users of closures with longer compilation times, but it shouldn't affect
    runtime performance.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This proposal intentionally distinguishes closures from other kinds of
callables, solely to fix the issue with our beloved `char::is_ascii*` family of
functions. Given that the sole reason for this proposal is to fix that issue,
there's actually a huge range of possible alternatives. These are a few
examples:

- We might be able to `impl Pattern for fn(&char) -> bool`. This does raise the
    question of whether we *want* non-closure types which happen to be callable
    with a bare `char` and return a `bool` to also be usable as a `Pattern`, or
    only "true" closures. It also raises the question of whether we want to
    accept closures which take `&char`, or only functions. This has not been
    discussed previously. See unresolved questions.

    This proposal rejects this alternative by principle: If `Pattern` works on
    `(char) -> bool` closures (*including* `(char) -> bool` functions), and
    `(&char) -> bool` functions, it creates an inconsistency where
    `(&char) -> bool` closures don't work for seemingly no good reason.
- There have been past discussions about potential "overlapping impls" as a way
    of solving this, but they never got very far and were quite underspecified.
    This proposal is much narrower in scope, so it shouldn't have those issues.
- Adding `std::char::is_ascii*` which take `char` is fragile at best.

# Prior art
[prior-art]: #prior-art

Tentatively, the way function pointers work today is the prior art.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

1. This proposal raises the question of whether we want non-closure callables
    to be `Pattern`. Note that, today, it is not possible to specialize
    `Pattern` for such callables.

    This can be resolved before stabilization: this proposal allows either
    option to be chosen, e.g.:

    ```rust
    // any (char) -> bool callable
    impl<F: FnMut(char) -> bool> Pattern for F { ... }
    // only (&char) -> bool closures
    impl<F: FnMut(&char) -> bool> Pattern for Closure<F, fn(&char) -> bool> { ... }
    ```

    or even with specialization:

    ```rust
    // any (char) -> bool callable (specializable)
    impl<F: FnMut(char) -> bool> Pattern for F { ... }
    // only (char) -> bool closures
    impl<F: FnMut(char) -> bool> Pattern for Closure<F, fn(char) -> bool> { ... }
    // only (&char) -> bool closures
    impl<F: FnMut(&char) -> bool> Pattern for Closure<F, fn(&char) -> bool> { ... }
    ```
2. This proposal changes type inference for closures. Specifically, adding
    `impl<F: Fn(&char) -> bool> Pattern for Closure<F, fn(&char)>` would break
    inference for existing users of `str.matches(|x| x.whatever)`, tho this is
    not that big of a deal, as the edition machinery can handle this today, in
    a similar way to how it can handle `IntoIterator` for arrays. But the
    unresolved question is whether we want to define new stability rules for
    trait impls on closures.

    Today, Rust mostly considers new trait impls to be non-breaking, but with
    workarounds for where such impls cause major ecosystem breakage (arrays).
    Should new impls for Closure where there was once only one be considered a
    breaking, must-never-do change, such as in the case of `Pattern`, or should
    the user be responsible for annotating the closure? This should be resolved
    before merging this RFC. (Or, indeed, this RFC should be rejected if any of
    this is unacceptable - after all, the proposal sets out to solve `Pattern`
    for the `char::is_ascii*` family of functions, and this would prevent
    that.)
3. The `Args`. Just like for `Fn*`, this must be resolved before stabilization.

# Future possibilities
[future-possibilities]: #future-possibilities

N/A.
