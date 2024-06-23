- Feature Name: `macros-named-capture-groups`
- Start Date: 2024-05-28
- RFC PR: [rust-lang/rfcs#3649](https://github.com/rust-lang/rfcs/pull/3649)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

It will now be possible to give names to capture (repetition) groups in macro
patterns, which can then be referred to directly in the macro body and macro
metavariable expressions.

# Motivation
[motivation]: #motivation

Rust has no way to refer to capture groups directly, so it uses the variables
they capture to refer to them indirectly. This leads to confusing or limited
behavior in a few places:

- Expansion with multiple capture groups is extremely limited. In many cases,
  the ordering and nesting of different groups is restricted based on what can
  be inferred by the contained variables, since the groups themselves are
  ambiguous.
- Metavariable expressions as they currently exist use an unintuitive format:
  syntax like `${count($var, n)}` is used to refer to the `n`th parent group of
  the smallest group that captures `$var`. Referring to groups directly would be
  more straightforward than using a proxy.
- Repetition-related diagnostics are suboptimal because the compiler has limited
  ability to guess what a capture group _should_ refer to when a captured
  groups and variables do not align correctly.
- Repetition mismatch diagnostics can only be emitted after the macro is
  instantiated, rather than when the macro is written. (E.g. "meta-variable
  `foo` repeats 2 times, but `bar` repeats 1 time")
- As a result of the above, using repetition is somewhat fragile; small
  adjustments can break working patterns with little indication of what exactly
  is wrong. Reading code with multiple capture groups can also be confusing.

It is expected that named capture groups will provide a way to remove ambiguity
in expansion and metavariable expressions, as well as unblock diagnostics that
do a better job of guiding the macro mental model.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Capture groups can now take a name by providing an identifier between the `$`
and opening `(`. This group can then be referred to by name in the expansion:

```rust
macro_rules! foo {
    ( $group1( $a:ident ),+ ) => {
        $group1( println!("{}", $a); )+
    }
}
```

This would be approximately equal to the following procedural code:

```rust
let mut ret = TokenStream::new();

// Append an expansion for each time group1 is matched
for Group1Captures { a } in group1 {
    ret += quote!{ println!("{}", #a); };
}
```

Named groups can be used to create code that depends on nested repetitions:

```rust
macro_rules! make_functions {
    ( 
        // Create a function for each name
        names: [ $names($name:ident),+ ],
        // Optionally specify a greeting
        $greetings( greeting: $greeting:literal, )?
    ) => {
        $names(
            // Create a function with the specified name
            fn $name() {
                println!("function {} called", stringify!($name));
                // If a greeting is provided, print it in every function
                $greetings( println!("{}", $greeting) )?
            }
        )+
    }
}

fn main() {
    make_functions! {
        names: [foo, bar],
        greeting: "hello!",
    }

    foo();
    bar();

    // output:
    // function foo called
    // hello!
    // function bar called
    // hello!
}

```

This expansion is not easily possible without named capture groups because
of ambiguity regarding which groups are referred to.

Expansion of the above will approximately follow this procedural model:

```rust
let mut ret = TokenStream::new();

// Append an expansion for each time group1 is matched
for NamesCaptures { name } in greetings {
    let mut fn_body = quote! { println!("function {} called", stringify!($name)); };

    // Append the greeting for each 
    for GreetingCaptures { greeting } in greetings {
        fn_body += quote! { println!("{}", #greeting) };
    }

    // Construct the function item and append to returned tokens
    ret += quote! { fn #name() { #fn_body  } };
}
```


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Macro captures currently include the following grammar node:

> `$` ( _MacroMatch<sup>+</sup>_ ) _MacroRepSep_<sup>?</sup> _MacroRepOp_

This will be expanded to the following:

> `$` ( IDENTIFIER_OR_KEYWORD except crate | RAW_IDENTIFIER | _ )<sup>?</sup> ( _MacroMatch<sup>+</sup>_ ) _MacroRepSep_<sup>?</sup> _MacroRepOp_

As a result, `$identifier( /* ... */ ) /* sep and kleene */` will allow naming
a capture group. It can then be used in expansion:

```rust
$identifier(
  /* expansion within group */
) /* sep and kleene */
```

Names will remain optional; however, if a capture group is given a name, it
_must_ also be referred to by name during expansion. That is, an unnamed
group in expansion will never be matched to a named group in the pattern.

This addition will have implications in a few places:

## Nesting repetition expansions

Nesting or intermixing repetition groups is currently not possible, mostly due
to ambiguity of capture group expansions. Using an example from above:

```rust
macro_rules! make_functions {
    (
    //           ↓ group 1
        names: [ $($name:ident),+ ],
    //  ↓ group 2
        $( greeting: $greeting:literal, )?
    ) => {
        $(  // <- this expansion contains both `$name` and `$greeting`. So is this
            //    an expansion of capture group 1 or 2?
            fn $name() {
                println!("function {} called", stringify!($name));
                $( println!("{}", $greeting) )?
            }
        )+
    }
}
```

Adding named capture groups makes this work, since ambiguity is removed.

It is likely possible to adjust the rules for expansion such that the above
would work with no additional syntax. However, this RFC posits that referring
to groups by name provides an overall better user experience than changing
the rules (more clear code, better diagnostics, and an easier model to follow).

## Zero-length capture groups

As a side effect of more precise repetition, groups in expansion that do not
contain any metavariables will become more straightforward. For example, this
simple counter is not possible as written:

```rust
macro_rules! count {
    ( $( $i:ident ),* ) => {{
        // error: attempted to repeat an expression containing no syntax variables
        //↓  the compiler does not know which group this refers to (here there
        //   is only one choice, but that is not always the case).
        0 $( + 1 )* 
    }};
}
```

Using named groups removes ambiguity so should work:

```rust
// Note: this is just a simple example. Metavariable expressions will provide a
// better way to get the same result with `${count(...)}`.
macro_rules! count {
    ( $idents( $i:ident ),* ) => {{
        0 $idents( + 1 )*
    }};
}
```

Metavariable expressions provide an `${ignore($var)}` operation that enables
the same behavior; `ignore(...)` will simply not be needed with named groups.

There is also no way to act on capture groups that bind only exact tokens but
no variables. An example is extracting the `mut` from a function or binding
signature:


```rust
/// Sample macro that captures exact syntax and tweaks it
macro_rules! match_fn {
    //               ↓ We need to be aware of mutability
    (fn $name:ident ($(mut)? $a:ident: u32)) => {
        //       ↓ we want to reproduce the `mut` here
        fn $name($(mut)? $a: u32) {
        //       ^^^^^^^
        // error: attempted to repeat an expression containing no syntax variables
            println!("hello {}!", $a);
        }
    }
}
fn main() {
    match_fn!(fn foo(a: u32));
    foo(10);
}
```

Adding named capture groups to the above would allow it to work.
`${ignore(...)}` does not directly help here.

## Metavariable expressions

Metavariable expressions currently use a combination of location within the
expansion (i.e. which capture groups contain it), variables captured, and
an index to change the indicated group. For example, `index()` returns the
number of the current expansion.

```rust
macro_rules! innermost1 {
    ( $( $a:ident: $( $b:literal ),* );+ ) => {
        [$( $( ${ignore($b)} ${index(1)}, )* )+]
    };
}
```

In order to understand what `index(1)` is referring to here, one must do the
following:

- Note how many repetition groups exist in the match expression (2).
- Count how many repetition groups the `index(1)`` call is nested in (2).
- Backtrack by one to figure out what exactly is getting indexed (1).

After doing the above, it can be noted that `${index(1)}` in this position
will indicate the current expansion of the outer cature group (the group
containing only `$a`).

Rewritten to use named groups instead:

```rust
macro_rules! innermost1 {
    ( $outer_rep( $a:ident: $inner_rep( $b:literal ),* );+ ) => {
        [$outer_rep( $inner_rep( ${index($outer_rep)}, )* )+]
    };
}
```

It is significantly easier to see what the call to `index` is referring to. As
an added benefit, its meaning will not change if its position is moved in the
code (e.g. moving to be within `$outer_rep`, but not `$inner_rep`).

This RFC proposes that `count`, `index`, and `len` will accept group names
in place of a variable and an index, since these three expressions relate more
to how entire _groups_ are expanded than the variables they take as arguments.

Further reading:

- [`macro_metavar_expr` RFC][`macro_metavar_expr`] and
  [tracking issue](https://github.com/rust-lang/rust/issues/83527)
- [Proposal for possible specific behavior](https://github.com/rust-lang/rust/pull/122808#issuecomment-2124471027)

# Drawbacks
[drawbacks]: #drawbacks

Why should we *not* do this?

- If [`macro_metavar_expr`] stabilizes before this merges, this will add a
  duplicate way of using those expressions. If this RFC is accepted,
  stabilizing only a subset of metavariable expressions that does not conflict
  should be considered.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- Variable definition syntax could be `$ident(/* ... */)` rather than
  `$ident(/* ... */)`. Including the `:` is proposed to be more consistent
  with existing fragment specifiers.
- There is room for macros to become smarter in their expansions without adding
  named capture groups. As mentioned elsewhere in this RFC, it seems like
  adding named groups is a cleaner solution with less cognitive overhead.

# Prior art
[prior-art]: #prior-art

- Regex allows the naming of reepeating capture groups for expansion, including
  those that do not capture anything else.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Syntax: the original proposal was to include a colon, e.g.
  `$group1:(/* ... */)`. A label-like syntax of `$'group1 $(/* ... */)` was
  also proposed.

**Possibly edition-sensitive** the proposed syntaxes are currently rejected
under the `missing_fragment_specifier` lint. That means that
`#![allow(missing_fragment_specifier)]` makes rustc accept the proposed syntax
as valid, which could conflict with this proposal.

# Future possibilities
[future-possibilities]: #future-possibilities

- Exact interaction with metavariable expressions is out of this RFC's scope. 
  There is a proposal around
  <https://github.com/rust-lang/rust/pull/122808#issuecomment-2124471027>.

[`macro_metavar_expr`]: https://rust-lang.github.io/rfcs/3086-macro-metavar-expr.html
