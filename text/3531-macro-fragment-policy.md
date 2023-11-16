# Macro matcher fragment specifiers edition policy

- Start Date: 2023-11-15
- RFC PR: [rust-lang/rfcs#3531](https://github.com/rust-lang/rfcs/pull/3531)

# Summary

This RFC sets out the policy for how the behavior of macro matcher fragment specifiers is updated over an edition when those specifiers fall out of sync with the underlying grammar of Rust.

# Background and motivation

Rust has a syntactic abstraction feature called ["macros by example"][] or `macro_rules`.  This feature allows for writing *macros* that transform source code in a principled way.

Each macro is composed of one or more *rules*.  Each of these rules has a *matcher*.  The matcher defines what pattern of Rust syntax will be matched by the rule.

Within a matcher, different parts of the input Rust syntax can be bound to metavariables using *[fragment specifiers][]*.  These fragment specifiers define what Rust syntax will be matched and bound to each metavariable.  For example, the `item` fragment specifier matches an [item][], `block` matches a [block expression][], `expr` matches an [expression][], and so on.

As we add new features to Rust, sometimes we change its syntax.  This means that, even within an edition, the definition of what exactly constitutes an [expression][], e.g., can change.  However, to avoid breaking macros in existing code covered by our stability guarantee, we do not update within an edition what code is matched by the relevant fragment specifier (e.g., `expr`).  This *skew* or divergence between the language and the fragment specifiers creates problems over time, including that macros become unable to match newer Rust syntax without dropping down to lower-level specifiers such as `tt`.

Periodically, we need a way to bring the language and the fragment specifiers back into sync.  This RFC defines a policy for how we do that.

["macros by example"]: https://doc.rust-lang.org/reference/macros-by-example.html
[block expression]: https://doc.rust-lang.org/reference/expressions/block-expr.html
[expression]: https://doc.rust-lang.org/reference/expressions.html
[fragment specifiers]: https://doc.rust-lang.org/reference/macros-by-example.html#metavariables
[item]: https://doc.rust-lang.org/reference/items.html

# Policy

This section is normative.

When we have changed the syntax of Rust such that the syntax matched by a fragment specifier no longer exactly aligns with the actual syntax for that production in the Rust grammar, then for the next edition of Rust, we will:

- Add a new fragment specifier that preserves the behavior of the fragment specifier in the last edition.  If there is some semantically meaningful name that makes sense to use for this new fragment specifier, we'll use that.  Otherwise, we'll use the existing name with the identifier of the last edition added as a suffix.
- Change the behavior of the fragment specifier to match the underlying grammar as of the release of Rust corresponding to first release of the new edition.
- Have `cargo fix` replace all instances of the original fragment specifier in macro matchers with the new one that preserves the old behavior.

For example, suppose that the behavior of the `expr` fragment specifier fell out of sync with the grammar for a Rust [expression][] during Rust 2021 and that Rust 2024 is the next edition.  Then in Rust 2024, we would add a new fragment specifier named `expr2021` (assuming no better semantically meaningful name could be found) that would preserve the behavior `expr` had in Rust 2021, we would change the behavior of `expr` to match the underlying grammar, and we would have `cargo fix` replace all instances of `expr` fragment specifiers with `expr2021`.

# Alternatives

## Keep the old, add specifiers for the new

Changing the behavior of existing fragment specifiers, even over an edition, has an obvious cost: we may change the meaning of existing macros and consequently change the code that they generate.

Having `cargo fix` replace all instances of a changed fragment specifier with the new fragment specifier added for backward compatibility does mitigate this.  But that has some cost in terms of code churn.

Another alternative would be to *never* change the meaning of existing fragment specifiers.  Instead, when changing the grammar of Rust, we would add *new* fragment specifiers that would correspond with this new grammar.  We would not have to wait for new editions to add these.  We could add, e.g., `expr2023_11`, `expr2023_12`, etc. each time that we change the grammar.

This would be burdensome in other ways, so we've decided not to do this.
