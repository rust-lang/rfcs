# Macro matcher fragment specifiers edition policy

- Start Date: 2023-11-15
- RFC PR: [rust-lang/rfcs#3531](https://github.com/rust-lang/rfcs/pull/3531)

# Summary

This RFC sets out the policy for how the behavior of macro matcher fragment specifiers is updated over an edition when those specifiers fall out of sync with the underlying grammar of Rust.

# Background and motivation

Rust has a syntactic abstraction feature called ["macros by example"][] or `macro_rules`.  This feature allows for writing *macros* that transform source code in a principled way.

Each macro is composed of one or more *rules*.  Each of these rules has a *matcher*.  The matcher defines what pattern of Rust syntax will be matched by the rule.

Within a matcher, different parts of the input Rust syntax can be bound to metavariables using *[fragment specifiers][]*.  These fragment specifiers define what Rust syntax will be matched and bound to each metavariable.  For example, the `item` fragment specifier matches an [item][], `block` matches a [block expression][], `expr` matches an [expression][], and so on.

As we add new features to Rust, sometimes we change its syntax.  This means that, even within an edition, the definition of what exactly constitutes an [expression][], e.g., can change.  However, to avoid breaking macros in existing code covered by our stability guarantee, we often do not update within an edition what code is matched by the relevant fragment specifier (e.g., `expr`).[^no-update-exception]  This *skew* or divergence between the language and the fragment specifiers creates problems over time, including that macros become unable to match newer Rust syntax without dropping down to lower-level specifiers such as `tt`.

Periodically, we need a way to bring the language and the fragment specifiers back into sync.  This RFC defines a policy for how we do that.

[^no-update-exception]: In certain cases we may be able to update the fragment specifier simultaneously with adding new syntax as described in the [policy][].

["macros by example"]: https://doc.rust-lang.org/reference/macros-by-example.html
[block expression]: https://doc.rust-lang.org/reference/expressions/block-expr.html
[expression]: https://doc.rust-lang.org/reference/expressions.html
[fragment specifiers]: https://doc.rust-lang.org/reference/macros-by-example.html#metavariables
[item]: https://doc.rust-lang.org/reference/items.html

# Policy

[policy]: #policy

This section is normative.

When we change the syntax of Rust such that the syntax matched by a fragment specifier no longer exactly aligns with the actual syntax for that production in the Rust grammar, we will:

- In the current edition, the next edition, and as many other editions as practical, add a new fragment specifier that preserves the behavior of the existing fragment specifier.  If there is some semantically meaningful name that makes sense to use for this new fragment specifier, we'll use that.  Otherwise, we'll use the existing name with the identifier of the current stable edition added as a suffix after an underscore.
- In the next edition, change the behavior of the original fragment specifier to match the underlying grammar as of the release of Rust corresponding to first release of that edition.
- When migrating existing code to the new edition, have `cargo fix` replace all instances of the original fragment specifier in macro matchers with the new one that preserves the old behavior.

For example, suppose that the current stable edition is Rust 2021, the behavior of the `expr` fragment specifier has fallen out of sync with the grammar for a Rust [expression][], and that Rust 2024 is the next edition.  Then in Rust 2021, Rust 2024, and as many other editions of Rust as practical, we would add a new fragment specifier named `expr_2021` (assuming no better semantically meaningful name could be found) that would preserve the behavior `expr` had in Rust 2021, we would in Rust 2024 change the behavior of `expr` to match the underlying grammar, and when migrating code to Rust 2024, we would have `cargo fix` replace all instances of `expr` fragment specifiers with `expr_2021`.

A new fragment specifier that preserves the old behavior *must* be made available no later than the first release of Rust for the new edition, but it *should* be made available as soon as the original fragment specifier first diverges from the underlying grammar.

As specified above, we will add the new fragment specifier that preserves the old behavior to the current edition, the next edition, and to as many other editions as practical.  Adding the new specifier to the current and next edition will be done to facilitate migration.  Adding it to as many other editions as practical will be done in keeping with our policy of preferring [uniform behavior across editions][].  Sometimes, however, it may not be practical to add the specifier to some other edition.  E.g., the behavior being preserved may include handling a token that is a keyword in only some editions.  In these cases, we'll add the new fragment specifier only to those editions where it makes sense.

In cases where we're adding new syntax and updating the grammar to include that new syntax, if we can update the corresponding fragment specifier simultaneously to match the new grammar in such a way that we do not risk changing the behavior of existing macros (i.e., because the new syntax previously would have failed parsing), then we will do that so as to prevent or minimize divergence between the fragment specifier and the new grammar.  If that entirely prevents divergence, then no further action will be needed.  Otherwise, the policy defined in this RFC will be used to correct any remaining divergence in the next edition.

[uniform behavior across editions]: https://github.com/rust-lang/rfcs/blob/master/text/3085-edition-2021.md#uniform-behavior-across-editions

# Alternatives

## Keep the old, add specifiers for the new

Changing the behavior of existing fragment specifiers, even over an edition, has an obvious cost: we may change the meaning of existing macros and consequently change the code that they generate.

Having `cargo fix` replace all instances of a changed fragment specifier with the new fragment specifier added for backward compatibility does mitigate this.  But that has some cost in terms of code churn.

Another alternative would be to *never* change the meaning of existing fragment specifiers.  Instead, when changing the grammar of Rust, we would add *new* fragment specifiers that would correspond with this new grammar.  We would not have to wait for new editions to add these.  We could add, e.g., `expr_2023_11`, `expr_2023_12`, etc. each time that we change the grammar.

This would be burdensome in other ways, so we've decided not to do this.

## Add specifier for new edition behavior in all editions

In addition to doing what is specified in this RFC, when releasing a new edition we could also add a new fragment specifier to all editions whose behavior would match that of the original fragment specifier in the new edition.  E.g., when releasing Rust 2024, we would add an `expr_2024` fragment specifier to all editions that would match the behavior of `expr` in Rust 2024.

The upside of doing this would be that people could take advantage of the new behavior without migrating their crates to the new edition.  Conceivably, this could help to allow some crates to make incremental transitions.

However, if later, during the life of the Rust 2024 edition, we were to change the grammar of expressions again and come up with a semantically meaningful name for the fragment specifier that would preserve the Rust 2024 behavior, then we would end up with two identical fragment specifiers for this, `expr_2024` and `expr_some_better_name`.

More importantly, making changed new edition behavior optionally available in older editions is not what we generally do.  As [RFC 3085][] said, [editions are meant to be adopted][].  The way for a crate to opt in to the behavior of the new edition is to upgrade to that edition.

Further, there could be cases where the changed behavior of the fragment specifier does not make sense in older editions, similar to what is discussed in the [policy][] for when it may not be practical to add the new fragment specifier to all other editions.

Consequently, for these reasons, we've decided not to do this.

[RFC 3085]: https://github.com/rust-lang/rfcs/blob/master/text/3085-edition-2021.md
[editions are meant to be adopted]: https://github.com/rust-lang/rfcs/blob/master/text/3085-edition-2021.md#editions-are-meant-to-be-adopted

## Use suffix without underscore

This RFC specifies that, when adding a new fragment specifier that preserves the old behavior, if a better semantically meaningful name cannot be found, we will use the existing name suffixed with the identifier of the current stable edition separated by an underscore.  E.g., we might add `expr_2021`.

However, with the exception of `pat_param`, none of the current fragment specifiers include an underscore.  It's conceivable that we might want to not separate the edition identifier with an underscore (e.g. `expr2021`) or that we might only want to separate the identifier when the fragment specifier already includes an underscore (i.e., we would say `expr2021` but `pat_param_2021`).

According to [RFC 430][], both `expr_2021` and `pat_param_2021` would be the most correct (however, note that the RFC did not specifically consider fragment specifiers).  As the RFC says:

> In `snake_case` or `SCREAMING_SNAKE_CASE`... we have... `PI_2` rather than `PI2`.

Similarly, we named the 2021 version of the Rust prelude `rust_2021` rather than `rust2021`.

In this RFC, we've specified that the underscore separator will always be used.

[RFC 430]: https://github.com/rust-lang/rfcs/blob/master/text/0430-finalizing-naming-conventions.md
