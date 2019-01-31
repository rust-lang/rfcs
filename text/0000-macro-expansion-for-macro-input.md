- Feature Name: Macro expansion for macro input
- Start Date: 2018-01-26
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

This is an **experimental RFC** for adding a new feature to the language,
opt-in eager macro expansion. This will allow procedural and declarative macros
to handle unexpanded macro calls that are passed as inputs, as well as allow
macros to access the results of macro calls that they construct themselves.

Reiterating the original description of [what an eRFC
is](https://github.com/rust-lang/rfcs/pull/2033#issuecomment-309057591), this
eRFC intends to be a lightweight, bikeshed-free outline of what a strategy for
eager expansion might look like, as well as to affirm that this is a feature we
want to pursue in the language.

# Motivation

There are a few places where proc macros may encounter unexpanded macros in
their input:

* In attribute and procedural macros:

    ```rust
    #[my_attr_macro(x = a_macro_call!(...))]
    //                  ^^^^^^^^^^^^^^^^^^
    // This call isn't expanded before being passed to `my_attr_macro`, and
    // can't be since attr macros are passed raw token streams by design.
    struct X {...}
    ```

    ```rust
    my_proc_macro!(concat!("hello", "world"));
    //             ^^^^^^^^^^^^^^^^^^^^^^^^^
    // This call isn't expanded before being passed to `my_proc_macro`, and
    // can't be since proc macros are passed raw token streams by design.
    ```

* In proc macros called with metavariables or token streams:

    ```rust
    macro_rules! m {
        ($($x:tt)*) => {
            my_proc_macro!($($x)*);
        },
    }

    m!(concat!("a", "b", "c"));
    // ^^^^^^^^^^^^^^^^^^^^^^
    // This call isn't expanded before being passed to `my_proc_macro`, and
    // can't be because `m!` is declared to take a token tree, not a parsed
    // expression that we know how to expand.
    ```

In these situations, proc macros need to either re-call the input macro call as
part of their token output, or simply reject the input. If the proc macro needs
to inspect the result of the macro call (for instance, to check or edit it, or
to re-export a hygienic symbol defined in it), the author is currently unable
to do so.

Giving proc macro authors the ability to handle these situations will allow
proc macros to 'just work' in more contexts, and without surprising users who
expect macro calls to interact well with more parts of the language.
Additionally, supporting the 'proc macro definition' use case above allows proc
macro authors to use macros from other crates _as macros_, rather than as proc
macro definition functions.

As a side note, allowing macro calls in built-in attributes would solve a few
outstanding issues (see
[rust-lang/rust#18849](https://github.com/rust-lang/rust/issues/18849) for an
example). 

An older motivation to allow macro calls in attributes was to get
`#[doc(include_str!("path/to/doc.txt"))]` working, in order to provide an
ergonomic way to keep documentation outside of Rust source files. This was
eventually emulated by the accepted [RFC
1990](https://github.com/rust-lang/rfcs/pull/1990), indicating that macros in
attributes could be used to solve problems at least important enough to go
through the RFC process.

# Detailed design

As an eRFC, this section doesn't focus on the details of the _implementation_
of eager expansion. Instead, it outlines the required and desirable outcomes of
any eventual solution. Additionally, we recount the rough design of possible
APIs that have already come up in discussion around this topic.

The rough plan is to implement minimally-featured prototype versions of each
API in order to get feedback on their relative strengths and weaknesses,
before focusing on polishing the best candidate for eventual stabilisation.

In the following examples, assume `expands_input!` is a procedural macro that
needs its input to be fully expanded.

## Proc macro library

Procedural macros are exposed as Rust functions of type `fn(TokenStream) ->
TokenStream`. The most natural way for a proc macro author to expand a macro
encountered in the input `TokenStream` would be to have access to a similar
function `please_expand(input: TokenStream) -> Result<TokenStream, SomeError>`,
which used the global compiler context to resolve and expand any macros in
`input`. 

As an example, we could implement `expands_input!` like this:

```rust
#[proc_macro]
fn expands_input(input: TokenStream) -> TokenStream {
    let tokens = match please_expand(input) {
        Ok(tokens) => tokens,
        Err(e) => {
            // Handle the error. E.g. if there was an unresolved macro,
            // signal to the compiler that the current expansion should be
            // aborted and tried again later.
        }
    },
    ...
}
```

## Tagged tokens

Similarly to how we store hygiene and span information on tokens themselves, we
could store eager-expansion information as well. A macro would 'mark' some of
the tokens it produces as eagerly expanded.

As an example, this invocation:
```rust
expands_input! {
    concat!("a", "b")
}
```
Would expand into this:
```rust
expands_input! {
//  These tokens are marked as "eager": they will get expanded before any
//  surrounding macro invocations.
//  vvvvvvvvvvvvvvvvv
    concat!("a", "b")
}
```
To be clear, this means the implementation of `expands_input!` produces tokens
which are _also_ an invocation of `expands_input!`, but in this case some of
the produced tokens have been modified by being marked for eager expansion.

Then, as the comment suggests, after the next round of expansions we would
have this:
```rust
expands_input! {
    "ab"
}
```

## Macro callbacks

The compiler already does some limited eager expansion (e.g. in `env!`). We can
expose that functionality as a special declarative macro. Proc macros could use
it to perform a process similar to the recursive expansion as described in the
section on [tagged tokens](#tagged-tokens).  Additionally, it provides a
straightforward API for decl macros to do the same thing.

Some toy syntax:

```rust
expand! {
    let item_tokens: item = { mod foo { m!{} } };
    let expr_tokens: expr = { concat!("a", "b") };
    my_proc_macro!(
        some args;
        #item_tokens;
        some more args;
        #expr_tokens
    );
}
```

The intent here is that `expand!` accepts one or more declarations of the form
`let $name: $expansion_type = { $($tokens_to_expand)* };`, followed by a 'target'
token tree where the expansion results should be interpolated. It then expands
each declaration and interpolates the resulting tokens into the target.
For this example we're using the interpolation syntax from the [`quote`
crate](https://docs.rs/quote/0.6.11/quote/macro.quote.html).

More explicitly, this invocation:
```rust
expands_input! {
    concat!("a", "b")
}
```
Should expand into this:
```rust
expand! {
    let e: expr = { concat!("a", "b") };
    expands_input! {
        #e
    }
}
```
Which in turn should expand into this:
```rust
expands_input! {
    "ab"
}
```

## Desirable behaviour
All of the above designs should solve simple examples of the motivating problem.
For instance, they all _should_ enable `#[doc(include_str!("path/to/doc.txt"))]`
to work. However, there are a multitude of possible complications that a more
polished implementation would handle.

To be clear: these aren't blocking requirements for an early experimental
prototype implementation. They aren't even hard requirements for the final,
stabilised feature! However, they are examples where an implementation might
behave unexpectedly for a user if they aren't handled, or are handled poorly.
See the [appendix](#appendix-a-corner-cases) for a collection of 'unit tests'
that exercise these ideas.

### Expansion order
Depending on the order that macros get expanded, a definition might not be in
scope yet. An advanced implementation would delay expansion of an eager macro
until all its macro dependencies are available. See the appendix on [delayed
definitions](#delayed-definitions) and [paths within nested
macros](#paths-within-nested-macros).

### Path resolution
In Rust 2018, macros can be invoked by a path expression. These paths can be
complicated, involving `super` and `self`. An advanced implementation would
have an effective policy for how to resolve such paths. See the appendix on
[paths within a macro](#paths-within-a-macro), [paths from inside a macro to
outside](#paths-from-inside-a-macro-to-outside), and [paths within nested
macros](#paths-within-nested-macros).

### Changing definitions
Since a macro usually changes its contents, any macros defined within its
arguments isn't safe to use as a macro definition. A correct implementation
would be careful to ensure that only 'stable' definitions are resolved and
expanded, where 'stable' means the definition won't change at any point where
an invocation might be expanded. See the appendix on [mutually-dependent
expansions](#mutually-dependent-expansions), and [paths that disappear during
expansion](#paths-that-disappear-during-expansion). 

# Rationale and alternatives

The primary rationale is to make procedural and attribute macros work more
smoothly with other features of Rust - mainly other macros.

Recalling the examples listed in the [motivation](#motivation) above, a few but
not all situations of proc macros receiving unexpanded macro calls could be
avoided by changing the general 'hands off' attitude towards proc macros and
attribute macros, and more aggressively parse and expand their inputs. This
effectively bans macro calls as part of the input grammar, which seems drastic,
and wouldn't handle cases of indirection via token tree (`$x:tt`) parameters.

We could encourage the creation of a 'macros for macro authors' crate with
implementations of common macros - for instance, those in the standard library
- and make it clear that macro support isn't guaranteed for arbitrary macro
calls passed in to proc macros. This feels unsatisfying, since it fractures the
macro ecosystem and leads to very indirect unexpected behaviour (for instance,
one proc macro may use a different macro expansion library than another, and
they might return different results). This also doesn't help address macro
calls in built-in attributes.

# Unresolved questions

* How do these proposals interact with hygiene?
* How do the [proc macro library](#proc-macro-library) and [tagged
  token](#tagged-tokens) proposals get used by declarative macros?

# Appendix A: Corner cases

Some examples, plus how this proposal would handle them assuming full
implementation of all [desirable behaviour](#desirable-behaviour).

### Paths from inside a macro to outside

Should compile: the definition of `m!` is stable (that is, it won't be changed
by further expansions), so the invocation of `m!` is safe to expand. 
```rust
macro m() {}

expands_input! {
    mod a {
        super::m!();
    }
}
```

### Paths within a macro

Should compile: the definitions of `ma!` and `mb!` are stable (that is, they
won't be changed by further expansions), so the invocations are safe to expand. 
```rust
expands_input! {
    mod a {
        pub macro ma() {}
        super::b::mb!();
    };

    mod b {
        pub macro mb() {}
        super::a::ma!();
    };
}
```

### Paths within nested macros

Should compile.
```rust
expands_input! {
    expands_input! {
        mod b {
            // This invocation...
            super::a::x!();
        }
    }

    mod a {
        // Should resolve to this definition.
        pub macro x() {}
    }
}
```
```rust
#[expands_body]
mod a {
    #[expands_body]
    mod b {
        // This invocation...
        super::x!();
    }

    // Should resolve to this definition...
    macro x() {}
}

// And not this one!
macro x{}
```

### Paths that disappear during expansion

Should not compile: assuming `deletes_everything` always expands into an empty
token stream, the invocation of `m!` relies on a definition that won't be
stable after further expansion.
```rust
#[deletes_everything]
macro m() {}

m!();
```

### Mutually-dependent expansions

Should not compile: each expansion would depend on a definition that might not
be stable after further expansion, so the mutually-dependent definitions
shouldn't resolve.
```rust
#[expands_body]
mod a {
    pub macro ma() {}
    super::b::mb!();
}

#[expands_body]
mod b {
    pub macro mb() {}
    super::a::ma!();
}
```

Should not compile: the definition of `m!` isn't stable with respect to the
invocation of `m!`, since `expands_args` might change the definition.
```rust
#[expands_args(m!())]
macro m() {}
```

Should not compile: the definition of `m!` isn't stable with respect to the
invocation of `m!`, since `expands_args_and_body` might change the definition.
TODO: is this the expected behaviour?
```rust
#[expands_args_and_body(m!())]
macro m() {}
```

### Delayed definitions

Should compile:
    * If the first invocation of `expands_input!` is expanded first, it should
      notice that it can't resolve `x!` and have its expansion delayed.
    * When the second invocatoin of `expands_input!` is expanded, it provides a
      stable definition of `x!`. This should allow the first invocation to be
      're-expanded'.
```rust
macro make($name:ident) {
    macro $name() {}
}

expands_input! {
    x!();
}

expands_input! {
    make!(x);
}
```

### Non-contiguous expansion tokens

Should compile: assuming `expands_untagged_input` removes the relevant
semicolon-delineated token streams before trying to expand its input, the
resulting tokens are valid items. TODO: should 'interpolating' the unexpanded
tokens be the responsibility of the proc macro?
```rust
expands_untagged_input! {
    mod a {
        super::b::m!();
    }
    dont_expand: foo bar;
    mod b {
        pub macro m() {};
    }
}
```
```rust
expands_untagged_input! {
    mod a {
        dont_expand: m1!();
        m2!();
    }
}
```
