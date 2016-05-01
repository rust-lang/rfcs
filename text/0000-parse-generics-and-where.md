- Feature Name: `parse_generics_and_where`
- Start Date: 2016-02-10
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Introduce two macros to the standard library: `parse_generics!` and `parse_where!`.  These are the "missing pieces" which will allow stable macros to completely parse and transform arbitrary Rust items.

# Motivation
[motivation]: #motivation

Currently, there is a significant hole in `macro_rules!` and its ability to parse Rust constructs: generics.  Specifically, this is the product of `macro_rules!`'s inability to "back out" of a grammar production.  It is possible to parse a generic parameter list *without* lifetimes, but not *with* lifetimes. \[1]

This means that, *in general*, `macro_rules!` is simply not capable of parsing any Rust construct that might contain a generic parameter list as part of its declaration.

Obviously, such items *are* supported by the `item` matcher.  However, because `macro_rules!` provides no way to *deconstruct* parsed AST nodes, this is of little practical use in any situation where a macro's expansion depends on information contained *within* the item's definition.  In particular, macros would be of considerable use for wrapping functions or types, or automatically deriving trait implementations were it not for this limitation.

Secondly, parsing generic parameter lists is quite difficult and inefficient due to the use of `<...>` in the syntax.  Because angle brackets are not considered as a single token tree, they can *only* be parsed with rather laborious recursive rules.

Note that `where` clauses are *also* impossible to parse in general for the same reason.

Third, even if the ability to deconstruct or extract information from AST nodes was added, this would not help anyone wishing to use generics as part of otherwise non-standard macro syntax.

This has led to the rather frustrating situation of the recommended serialisation crate, `serde`, being so difficult to use that people still recommend the otherwise inferior `rustc_serialize`.  `serde` would be easy to recommend if it weren't for the requirement of a nightly compiler, the *extraordinarily* slow to compile `syntex`, or having to hand-roll serialisation code (using a largely undocumented and somewhat obtuse interface).

There are other problems.  This author has written a number of boilerplate-reducing crates (such as `enum_derive` and `newtype_derive`) which *fundamentally* cannot support generic types.  A feature which was literally *the very first request* received.

The proverbial light at the end of the tunnel is held to be the stabilisation of procedural macros; however, this author has two problems with this:

1. There is no timetable *whatsoever* for when this might happen.  In the interim, the above continues to be stubbornly impossible.

2. Even once stabilised, it seems ridiculous that parsing a perfectly ordinary language construct requires abandoning the "built-in" macro syntax in favour of compiler plugins.

Thus, this author believes that parsing generics *should* be possible in Rust *without* the need to resort to procedural macros.  However, making this possible "the right way" would require significant alterations to `macro_rules!`.  Given the formative plans for completely rebooting the macro system, however, this seems unlikely to take place.  What is more, such plans have a similar issue to stable procedural macros: no known timetable.

As such, this RFC proposes what the author believes to be the absolute minimum necessary to enable `macro_rules!` to parse generic Rust items: two new macros to be added to the standard library.

[A proof of concept implementation](https://github.com/DanielKeep/rust-parse-generics) has been written, along with examples demonstrating that the two macros have the necessary power.

In addition, a stable shim crate has been written which provides a subset of the functionality of these macros.  The shims allow the underlying implementation to the swapped out for the proof of concept plugin, providing a path for both experimentation and migration.  This allows crates in the wider ecosystem to begin adopting them ahead of stabilisation.

Finally, the author plans to follow this RFC up with another which would permit the use of `macro_rules!`-defined macros in `#[derive]` attributes.  Without these two macros, `macro_rules!`-based derivations will be of significantly limited value.

\[1]: This is true even *if* a lifetime token matcher were to be introduced.  `macro_rules!` would *still* have to be able to distinguish between `$t:ltime` and `$t:ident` in the same position, which it cannot currently do for *any* matchers.

# Detailed design
[design]: #detailed-design

It should be noted that the exact invocation and expansion syntax is open for discussion.

## `parse_generics!`

This macro will parse a generic parameter list from a sequence of token trees.  It has the following invocation syntax:

```rust
parse_generics! {
    { constr, params, ltimes, tnames },
    then callback! { callback arguments ... },
    < 'a, 'b: 'a, T, U: 'a + Clone, ... > tail ...
}
```

*Note*: the callback arguments may be contained in *any* kind of `tt` group: `(...)`, `[...]`, or `{...}`.

It expands to the following:

```rust
callback! {
    callback arguments ...
    {
        constr: [ 'a, 'b: 'a, T, U: 'a + Clone, ..., ],
        params: [ 'a, 'b, T, U, ..., ],
        ltimes: [ 'a, 'b, ],
        tnames: [ T, U, ..., ],
    },
    tail ...
}
```

The first part consists of a list of fields to include in the expansion.  Each field can be invoked once, and will appear in the same order in the output.  This allows users to specify what information they need.  As another example:

```rust
parse_generics! {
    { tnames, ltimes },
    then callback! { callback arguments ... },
    <'a, T, U, V> tail ...
}
```

Expands to:

```rust
callback! {
    callback arguments ...
    {
        tnames: [ T, U, V, ],
        ltimes: [ 'a, ],
    },
    tail ...
}
```

As a convenience, the list of fields may be written `{..}` (like a "catch-all" struct pattern):

```rust
parse_generics! {
    { .. }, then callback! { callback arguments ... },
    < 'a, 'b: 'a, T, U: 'a + Clone, ... > tail ...
}
```

Expands to:

```rust
callback! {
    callback arguments ...
    {
        constr: [ 'a, 'b: 'a, T, U: 'a + Clone, ..., ],
        params: [ 'a, 'b, T, U, ..., ],
        ltimes: [ 'a, 'b, ],
        tnames: [ T, U, ..., ],
        ..
    },
    tail ...
}
```

The trailing `..` in the expansion is to encourage users to match everything after the input they're interested in using `$($other:tt)*`, as the set of fields may expand in the future.

Finally, fields may be suffixed with an `?` to indicate that the field might not exist.  Such fields are included if they are recognised, and ignored otherwise.  This exists as a forward-compatibility defence; it allows users to change behaviour based on the existence or non-existence of features, without needing to resort to version matching and conditional compilation.

For example:

```rust
parse_generics! {
    { tnames, cnames? }, then callback! { callback arguments ... },
    <A, B> tail ...
}
```

Would expand under the current implementation as:

```rust
callback! {
    callback arguments ...
    {
        tnames: [ A, B, ],
    },
    tail ...
}
```

But might expand to the following under a future version (such as one where `const` generic parameters exist):

```rust
callback! {
    callback arguments ...
    {
        tnames: [ A, B, ],
        cnames: [],
    },
    tail ...
}
```

It is also valid for the invocation to have *no* generic parameter list whatsoever:

```rust
parse_generics! {
    { constr, params, ltimes, tnames },
    then callback! { callback arguments ... },
    tail ...
}
```

Expands to:

```rust
callback! {
    callback arguments ...
    {
        constr: [],
        params: [],
        ltimes: [],
        tnames: [],
    },
    tail ...
}
```

The "output" (which is, in fact, the *input* to the callback macro) consists of:

- `callback arguments ...` - an arbitrary sequence of tokens passed to the callback, to allow information to be passed through the `parse_generics!` invocation.

- `constr` - a list of generic parameters *with* their inline constraints, as they were originally defined.  Aside from the presence of a terminating comma, this should be *effectively* the same as the sequence of tokens inside the `<...>` of the parsed parameter list.

- `params` - a list of generic parameters *without* their constraints, suitable for passing to a generic instantiation.

- `ltimes` - a list of lifetime parameters.

- `tnames` - a list of generic type parameters.

- `tail ...` - all tokens *after* the generic parameter list.  If there was no generic parameter list at the start of the provided input, it should be the *entirety* of the input.

### Explanations

* *Why allow the generic parameter list to be omitted?* - So that this macro can be used without requiring multiple rules to test for the existence/non-existence of the parameter list.

* *Why a callback?* - Firstly, there is nothing a macro can expand to that could represent the information of interest.  Secondly, even if there *was*, macros cannot parse the output of another macro, which would defeat the purpose of defining this macro in the first place.  Callbacks are the only solution.

* *Why callback, then tokens?* - Personal preference; tails feel more natural as the last thing passed to a macro.

  The order *could* be reversed by changing the invocation syntax to something similar to:

  ```rust
  parse_generics! {
    (< 'a, ... > tail ...),
    then callback! { callback arguments ... }
  }
  ```

* *Why include the `then` keyword?* - This helps distinguish the callback from a regular invocation, and also leaves the door open for extending the invocation syntax in future.  For example, if Rust gains generic value parameters in the future, the invocation syntax will need to be expanded to let users *request* these be included in the output.

  In addition, if some future macro system supports expanding to arbitrary token trees, the macro could be modified to allow the callback to be omitted.  Having a "keyword" in place reduces risks of ambiguity.

* *Why this expansion syntax?* - Because it looks vaguely like a `struct` literal (sans type name).

  The square brackets invoke a sequence of some kind.  They also allow for the contents of these "fields" to be matched blindly with `$($something:tt)*` and then substituted again.  This is *especially* important for lifetimes, which cannot be otherwise captured.

  Having the "leftover" tokens in the tail position feels natural.

* *Why use names?* - Because macros are hard to read at the *best* of times.  The names serve to help visually break up the "meta token soup" that complex macros can start to become.  They also act as safety tokens; they make it less likely that users will accidentally get the order of the fields wrong.

* *Why terse names?* - Macros often require many, repetitive rules.  Long names are a chore to type and worsen the signal-to-noise ratio.  The names do not need to be long to serve their purpose; they merely need be long *enough* and reasonably memorable.

  I am not particularly attached to these *specific* names, but I *do* like that they line up so nicely.

* *Why these particular outputs?* - Each serves a specific purpose:

  - `constr`: can be substituted directly as `impl<$($constr)*>` when wrapping a type, or `fn $name<$($constr)*>` when wrapping a function.

  - `params`: can be substituted directly when *instantiating* a generic item.

  - `ltimes`: necessary where you wish to constrain lifetimes by some newly introduced lifetime.

  - `tnames`: necessary where you wish to constrain type parameters, such as for mechanically derived trait implementations.

* *Why specify fields?* - This has a few benefits:

  1. It means users cannot be confused about the order of the outputs: they are in whatever order the user specified.

  2. If they forget and misspell the name of a field, the macro can emit an actual error, as opposed to a `macro_rules!` matching failure (which can often refer to an unrelated token).

  3. It allows fields not important to the task at hand to be omitted, saving matching effort.

  4. It allows new fields to be introduced in a backward-compatible manner.

  5. Allowing non-existent fields to be denoted with `?` affords forward compatibility.

  6. The `{ .. }` shorthand provides a compromise between robustness against future changes and convenience.

* *Why comma terminators, rather than comma separators?* - These are easier for `macro_rules!` to parse, particularly for recursive rules.

## `parse_where!`

This macro will parse a `where` clause from a sequence of token trees.  It has the following invocation syntax:

```rust
parse_where! {
    { clause, preds }, then callback! { callback arguments ... },
    where 'a: 'b, A: 'a + B, ... tail ...
}
```

*Note*: the callback arguments may be contained in *any* kind of `tt` group: `(...)`, `[...]`, or `{...}`.

It expands to the following:

```rust
callback! {
    callback arguments ...
    {
        clause: [ where 'a: 'b, A: 'a + B, ..., ],
        preds: [ 'a: 'b, A: 'a + B, ..., ],
    },
    tail ...
}
```

It is also valid for the invocation to have *no* `where` clause:

```rust
parse_where! {
    { clause, preds }, then callback! { callback arguments ... },
    tail ...
}
```

Expands to:

```rust
callback! {
    callback arguments ...
    {
        clause: [],
        preds: [],
    },
    tail ...
}
```

The field list behaves the same way as the `parse_generics!` field list.

Aside from similar components in the output of `parse_generics!`, `parse_where!`'s output consists of:

- `clause` - contains *either* a complete `where` clause, *or* nothing at all.  This is to allow the clause to be passed through unmodified.

- `preds` - a list of `where` predicates, with a terminating comma, provided the list is non-empty.  This is to optimise for adding additional predicates where there may or may not be existing ones.

### Explanations

In addition to the relevant questions for `parse_generics!`...

* *Why use record syntax when there are only two fields?* - A desire for uniformity.  It also means that additional fields can be added in the future with less hassle on the part of users.

  These could be equality constraints, or perhaps value constraints.  It might be worthwhile to extract these into independent sections.  Or it might not.

  In an earlier revision of this RFC, there was *only* the `preds` field.  `clause` was introduced for practical considerations.  It is not unthinkable for this to happen again in the future.

* *Why both `clause` and `preds`?* - Six of one, a half dozen of the other.  Having the `where` keyword included in `clause` simplifies passing predicates through unmodified, but makes appending new predicates more difficult.

  I propose that the *optimal* solution is to push to have `where` clauses accept sequences of zero predicates, which simplifies the problem dramatically.

# Drawbacks
[drawbacks]: #drawbacks

* *Additional maintenance burden.* - The compiler appears to lack a general facility for converting parsed AST nodes back to tokens, meaning these macros have a somewhat higher-than-obvious maintenance burden.

* *It uses two names in the global macro namespace.* - Nevertheless, they should not interfere with existing downstream code.

* *They are complex and very public.* - Macros cannot be hidden away in the cellar, down a broken flight of stairs, in a disused lavatory, in a locked filing cabinet with a sign saying "beware of the leopard" like other items.  They will be publicly visible at the top-level documentation for the standard library.

  What is more, the invocation syntax and usage are very unusual (even to experienced Rust developers).  Anyone coming across these casually (which is fairly likely) will probably be *very* confused.

  Really, this is more an issue with `rustdoc` than anything else, but it is still a concern.

* *They will promote more, and more complex, usage of `macro_rules!`* - Which is rather *the point*, really.

# Alternatives
[alternatives]: #alternatives

* *Wait for stable procedural macros.* - These limitations are frustrating *right now*.  I also do not believe the language should try to push people away from using a stable feature like `macro_rules!`; problems and holes in it should be solved where practical, leaving procedural macros as the sledgehammer for when all else fails.

* *Go down to the pub and drink until this all blows over.* - I don't drink, the doors are locked, and I've hidden the keys.  No booze until you sort this mess out!

* *Less explicit invocation syntax.* - Invocation could be changed to the following syntax, which is easier to explain and implement:

  ```rust
  parse_generics! {
      // Note: no field list:
      then callback! { callback arguments ... },
      tail ...
  }
  ```

  The output would always contain all fields, in a hard-coded order.  Whilst simpler, this will introduce both forward- and backward-compatibility problems if the output ever needs to expand (which is not unlikely).

* *More explicit invocation syntax.* - Invocation could be changed to the following syntax, which is more robust against potential future changes:

  ```rust
  parse_generics! {
      { .. }, then callback! { callback arguments ... },
      // Note: input in a delimited tt:
      { < 'a, 'b: 'a, T, U: 'a + Clone, ... > tail ... }
  }
  ```

  Placing the tail in an explicit delimited `tt` ensures the callback parameter can be dropped without *any* possibility for ambiguity (imagine a user-defined syntax in which an optional generic parameter list is followed by a literal `then` token).

* *Dropping field names.* - Expansion could drop the use of "field names" in favour of relying on position.  The expansion from the first example would become:

  ```rust
  callback! {
      callback arguments ...
      {
          [ 'a, 'b: 'a, T, U: 'a + Clone, ..., ]
          [ 'a, 'b, T, U, ..., ]
          [ 'a, 'b, ]
          [ T, U, ..., ]
      },
      tail ...
  }
  ```

  This also has similar issues with simply dropping the field list from the input.

* *Stick to the stable shim implemenations.* - The majority of the functionality of these macros can be handled in stable `macro_rules!`.  It would be simpler to just use those instead.

  However, this runs into several serious problems: the stable macros require a *significant* amount of recursion (proportional to the number of tokens), and can only support a limited, fixed set of lifetime names.  To put it nicely, the shims only *mostly* work, *some* of the time.

# Unresolved questions
[unresolved]: #unresolved-questions

* During implementation of the shim library, a third kind of complex production was found: generic parameter constraints.  Although *typically* subsumed by `parse_generics!` and `parse_where!`, it is not unthinkable that being able to parse them independently might be of use.

  Although unlikely to be added to this proposal, it might be worth considering support for them, as the current abilities of `macro_rules!` are *even more* woefully inadequate for parsing them.

* The exact invocation and expansion syntaxes.
