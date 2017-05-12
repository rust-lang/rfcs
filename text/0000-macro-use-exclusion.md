- Feature Name: macro_use_exclusion
- Start Date: 2016-02-27
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Allow importing all _but_ some selected macros from an external crate through a new `#[macro_use(not(...))]` syntax.

# Motivation
[motivation]: #motivation

When using macros from several different crates, naming conflicts can sometimes occur.
Consider the following declarations:

```rust
#[macro_use] extern crate log;
#[macro_use] extern crate nom;
```

As of this writing, both `log` and `nom` crates export an `error!` macro. Due to the order of declarations, however,
`error!` from `log` will be overwritten by the definition from `nom`.

Proposed syntax would prevent that from happening:

```rust
#[macro_use] extern crate log;
#[macro_use(not(error))] extern crate nom;
```

The `error!` macro from `nom` is now explicitly excluded, and thus no longer imported.
All _other_ `nom` macros, however, are still being imported, for they were not excluded by this `#[macro_use]` attribute.

(Every macro from `log` is of course also imported, prior to the ones from `nom` -- no change whatsoever here).


# Detailed design
[design]: #detailed-design

## Syntax

A new syntactic variant for `#[macro_use]` attribute is added, applicable only to `extern crate` declarations:

```rust
#[macro_use(not(excluded_macro_1, excluded_macro_2, ...))]
extern crate some_crate;
```

Largely inspired by analogous variant of `#[cfg]`, the `not(...)` meta-element accepts a comma-separated, non-empty list
of identifiers. Those are the names of macros exported by the external crate that should *not* be imported.

All other macros exported by the external crate are imported as usual.

Every identifier on the `not(...)` list should correspond to a macro that's actually exported by the external crate.
Mentioning a non-existent macro name should be signaled by at least a compiler warning.

## Other instances of macro_use

`#[macro_use(not(...))]` would be a distinct syntactical variant of the general `#[macro_use]` attribute.
It is specifically invalid to mix it with the other flavor that lists all imported macros explicitly:

```rust
// ERROR
#[macro_use(foo, not(bar))]
extern crate some_crate;
```

This is because of the obvious contradiction introduces with such a construct.

Additionally, the new syntax is applicable to `extern crate` declarations only.
Just like with the `#[macro_use(...)]` variant, it is an error to place it before module declarations:

```rust
// ERROR
#[macro_use(not(foo))]
mod macros;
```

## No-op usages

Preventing unintended name shadowing is the primary motivation for this feature, but it is not a _requirement_.
Identifiers listed in `#[macro_use(not(...))]` construct may or may not be defined in the current parser context;
in either case, the macros whose names are mentioned should _always_ be excluded from importing.

In other words, if we assume no other code in the crate besides the following (and that `some_crate` exports `foo!`):

```rust
#[macro_use(not(foo))]
extern crate some_crate;

foo!();
```

then the compiler should signal an error upon encountering `foo!()` (unrecognized macro name) but no sooner.

Similarly, it is not an error to reaffirm the shadowing of macros that would already occur under current rules:

```rust
#[macro_use(not(foo))] extern crate first_crate;
#[macro_use] extern crate second_crate;
```

Even if both `first_crate` and `second_crate` export `foo!`, the above code should still be valid.

# Drawbacks
[drawbacks]: #drawbacks

This proposal may be superseded by a more thorough overhaul of the `#[macro_use]` attribute, including features such
as name aliasing or qualification/namespacing.
In such case, additional care will have to be taken to preserve the desired level of compatibility.

# Alternatives
[alternatives]: #alternatives

## Reverse the order of crate declarations

An obvious workaround that can prevent undesired shadowing of macro names is to change the order of `extern crate`
declarations. The initial example could be rewritten as:

```rust
#[macro_use] extern crate nom;
#[macro_use] extern crate log;
```

causing the `error!` macro from `log` to shadow the one from `nom`, as intended. Such a reversal, however, disrupts
the natural ordering of `extern crate` declarations and would thus warrant at least a comment, and possibly require other
steps to appease lint / style checkers.

More importantly, not every conflict can be resolved this way. Consider the following situation:

```rust
#[macro_use] extern crate crate_one;   // exports: foo!, bar!
#[macro_use] extern crate crate_two;   // exports: foo!, bar!
```

If the desired outcome is to use `foo!` from `crate_one` and `bar!` from `crate_two`, then no valid ordering exists.
(This is an equivalent of trying to apply topological sort to a cyclic graph).

## List imported macros explicitly

The other option is to utilize the existing `#[macro_use(...)]` syntax and list all the used macros explicitly.
This introduces additional maintenance burden, which for crates exporting dozens of macros (like `nom`) would be
very significant.

# Unresolved questions
[unresolved]: #unresolved-questions

* Is it a compiler warning or error to try excluding a non-existent macro from importing?
* Which, if any, of the no-op usages should constitute a compiler warning.
