- Feature Name: Macro expansion for macro input
- Start Date: 2018-01-26
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

This is an **experimental RFC** for adding a new feature to the language,
opt-in eager macro expansion. This will:
* Allow procedural and declarative macros to handle unexpanded macro calls that are passed as inputs,
* Allow macros to access the results of macro calls that they construct themselves,
* Enable macros to be used where the grammar currently forbids it.

Reiterating the original description of [what an eRFC
is](https://github.com/rust-lang/rfcs/pull/2033#issuecomment-309057591), this
eRFC intends to be a lightweight, bikeshed-free outline of what a strategy for
eager expansion might look like, as well as to affirm that this is a feature we
want to pursue in the language.

# Motivation

## Expanding macros in input 

There are a few places where proc macros may encounter unexpanded macros in
their input:

* In attribute macros:

    ```rust
    #[my_attr_macro(x = a_macro_call!(...))]
    //                  ^^^^^^^^^^^^^^^^^^
    // This call isn't expanded before being passed to `my_attr_macro`, and
    // can't be since attr macros are passed opaque token streams by design.
    struct X {...}
    ```

* In procedural macros:
    ```rust
    my_proc_macro!(concat!("hello", "world"));
    //             ^^^^^^^^^^^^^^^^^^^^^^^^^
    // This call isn't expanded before being passed to `my_proc_macro`, and
    // can't be since proc macros are passed opaque token streams by design.
    ```

* In declarative macros:
    ```rust
    env!(concat!("PA", "TH"));
    //   ^^^^^^^^^^^^^^^^^^^
    // Currently, `std::env!` is a compiler-builtin macro because it often
    // needs to expand input like this, and 'normal' macros aren't able
    // to do so.
    ```

In these situations, macros need to either re-emit the input macro invocation
as part of their token output, or simply reject the input. If the proc macro
needs to inspect the result of the macro call (for instance, to check or edit
it, or to re-export a hygienic symbol defined in it), the author is currently
unable to do so.

Giving proc macro authors the ability to handle these situations will allow
proc macros to 'just work' in more contexts, and without surprising users who
expect macro calls to interact well with more parts of the language.

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

## Interpolating macros in output

Macros are currently not allowed in certain syntactic positions. Famously, they
aren't allowed in identifier position, which makes `concat_idents!` [almost
useless](https://github.com/rust-lang/rust/issues/29599). If macro authors have
access to eager expansion, they could eagerly expand `concat_idents!` and
interpolate the resulting token into their output.

## Expanding third-party macros

Currently, if a proc macro author defines a useful macro `useful!`, and another
proc macro author wants to use `useful!` within their own proc macro
`my_proc_macro!`, they can't: they can *emit an invocation* of `useful!`, but
they can't *inspect the result* of that invocation. Eager expansion would
allow this kind of macro-level code sharing.

# Detailed design

As an eRFC, this section doesn't focus on the details of the _implementation_
of eager expansion. Instead, it outlines the required and desirable outcomes of
any eventual solution. Additionally, we recount the rough design of possible
APIs that have already come up in discussion around this topic.

The rough plan is to implement minimally-featured prototype versions of each
API in order to get feedback on their relative strengths and weaknesses,
before focusing on polishing the best candidate for eventual stabilisation.

## Mutually recursive macros

One way to frame the issue is that there is no guaranteed way for one macro
invocation `foo!` to run itself *after* another invocation `bar!`.  You could
attempt to solve this by designing `bar!` to expand `foo!` (notice that you'd
need to control the definitions of both macros!).

The goal is that this invocation:
```rust
foo!(bar!())
```
Expands into something like:
```rust
bar!(<some args for bar>; foo!())
```
And now `foo!` *expects* `bar!` to expand into something like:
```rust
foo!(<result of expanding bar>)
```

This is the idea behind the third-party [`eager!`
macro](https://docs.rs/eager/0.1.0/eager/macro.eager.html). Unfortunately this
requires a lot of coordination between `foo!` and `bar!`, which isn't possible
if `bar!` were already defined in another library.

We can directly provide this missing ability through a special compiler-builtin
macro, `expand!`, which expands some arguments before interpolating the results
into another argument. Some toy syntax:

```rust
expand! {
    #item_tokens = { mod foo { m!{} } };
    #expr_tokens = { concat!("a", "b") };
    my_proc_macro!(
        some args;
        #item_tokens;
        some more args;
        #expr_tokens
    );
}
```

The intent here is that `expand!` accepts one or more declarations of the form
`#<name> = { <tokens to expand> };`, followed by a 'target' token tree where
the expansion results should be interpolated.

The contents of the right-hand sides of the bindings (in this case `mod
foo { m!{} }}` and `concat!("a", "b")`) should be parsed and expanded exactly
as though the compiler were parsing and expanding those tokens directly.

Once the right-hand-sides of the bindings have been expanded, the results are
interpolated into the final argument. For this toy syntax we're using the
interpolation syntax from the [`quote`
crate](https://docs.rs/quote/0.6.11/quote/macro.quote.html), but there are
alternatives (such as the unstable `quote!` macro in the [`proc_macro`
crate](https://doc.rust-lang.org/proc_macro/macro.quote.html)).

Let's step through an example. If `my_eager_macro!` wants to use `expand!` to
eagerly expand it's input, then this invocation:
```rust
my_eager_macro! {
    concat!("a", "b")
}
```
Should expand into this:
```rust
expand! {
    #new_input = { concat!("a", "b") };
    my_eager_macro! {
        #new_input
    }
}
```
Which in turn should expand into this:
```rust
my_eager_macro! {
    "ab"
}
```

### Recursion is necessary
We might be tempted to 'trim down' our `expand!` macro to just expanding it's
input, and not bothering with the recursive expansion:

```rust
macro trimmed_expand( <tokens> ) {
    expand! {
        #expanded_tokens = { <tokens> };
        #expanded_tokens
    }
}
```

However, this encounters the same problem that we were trying to solve in the
first place: how does `my_eager_macro!` use the *result* of `trimmed_expand!`?

Recursive expansion is seemingly necessary for any solution that doesn't
inspect macro inputs. For proposals that include inspecting macro inputs, see
the section on [alternatives](#rationale-and-alternatives).

### Use by procedural macros
The previous example indicates how a declarative macro might use `expand!` to
'eagerly' expand its inputs before itself. Conveniently, it turns out that the
changes required to get a procedural macro to use `expand!` are quite small.
For example, if we have an implementation `fn my_eager_macro_impl(TokenStream)
-> TokenStream`, then we can define an eager proc macro like so:

```rust
#[proc_macro]
fn my_eager_macro(input: TokenStream) -> TokenStream {
    quote!(
        expand! {
            ##expanded_input = {#input};
            my_eager_macro_impl!(##expanded_input)
        }
    )
}

#[proc_macro]
fn my_eager_macro_impl(TokenStream) -> TokenStream { ... }
```

Where the double-pound `##` tokens are to escape the interpolation symbol `#`
within `quote!`.

This transformation is simple enough that it could be implemented as an
`#[eager]` attribute macro.

### Identifier macros
At first glance, `expand!` directly solves the motivating case for
`concat_idents!` discussed [above](#interpolating-macros-in-output):

```rust
expand! {
    #name = concat_idents!(foo, _, bar);
    fn #name() {}
}

foo_bar();
```

This touches on possible issues concerning identifier hygiene. Note that the
semantics behind the interpolation of `#name` in the above example are quite
simple and literal ("take the tokens that get produced by `concat_idents!`, and
insert the tokens into the token tree `fn () {}`"); this means `expand!` should
be future-compatible with a hypothetical set of hygiene-manipulating utility
macros.

## Proc macro library

Procedural macros are exposed as Rust functions of type `fn(TokenStream) ->
TokenStream`. The most natural way for a proc macro author to expand a macro
encountered in the input `TokenStream` would be to have access to a similar
function `please_expand(input: TokenStream) -> Result<TokenStream, SomeError>`,
which used the global compiler context to iteratively resolve and completely
expand all macros in `input`.

As an example, we could implement `my_eager_macro!` like this:

```rust
#[proc_macro]
fn my_eager_macro(input: TokenStream) -> TokenStream {
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

### Name resolution and expansion order
Currently, the macro expansion process allows macros to define other macros,
and these macro-defined macros can be referred to *before they're defined*.
For example ([playground](https://play.rust-lang.org/?version=nightly&mode=debug&edition=2018&gist=1ac93c0b84452b351a10a619f38c6ba6)):
```rust
macro make($name:ident) {
    macro $name() {}
}

foo!();
make!(foo);
```

How this currently works internally is that the compiler repeatedly collects
definitions (`macro whatever`) and invocations `whatever!(...)`. When the
compiler encounters an invocation that doesn't have an associated definition,
it 'skips' expanding that invocation in the hope that another expansion will
provide the definition.

This poses an issue for a candidate proc macro `please_expand` API: if we can't
expand a macro, how do we know if the macro is *unresolvable* or just
unresolvable *now*? How does a proc macro tell the compiler to 'delay' it's
expansion?

## Desirable behaviour
The above designs should solve simple examples of the motivating problem.  For
instance, they all _should_ provide enough functionality for a new,
hypothetical implementation of `#[doc]` to allow
`#[doc(include_str!("path/to/doc.txt"))]` to work. However, there are a
multitude of possible complications that a more polished implementation would
handle.

To be clear: these aren't blocking requirements for an early experimental
prototype implementation. They aren't even hard requirements for the final,
stabilised feature! However, they are examples where an implementation might
behave unexpectedly for a user if they aren't handled, or are handled poorly.
See the [appendix](#appendix-a-corner-cases) for a collection of 'unit tests'
that exercise these ideas.

### Interoperability
A good implementation will behave 'as expected' when asked to eagerly expand
*any* macro, whether it's a `macro_rules!` decl macro, or a 'macros 2.0' `macro
foo!()` decl macro, or a compiler-builtin macro. Similarly, a good
implementation will allow any kind of macro to perform such eager expansion.

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
Because macros can define other macros, there can be references *outside* a
macro invocation to a macro defined in that invocation, as well as *inside*.
For example:

```rust
foo!();     // foo-outer
my_eager_macro! {
    macro foo() { ... };
    foo!(); // foo-inner
}
```

A naive implementation of eager expansion might 'pretend' that the source file
literally looked like:

```rust
foo!();
macro foo() { ... };
foo!();
```

However, there's no guarantee that the tokens finally emitted by
`my_eager_macro!` will contain the same definition of `foo!`, or even that it
contains such a definition at all!

This means a correct implementation of eager expansion has to be careful about
which macros it 'speculatively expands'. It's fine to expand `foo-inner`  while
eagerly expanding `my_eager_macro!`, but it's *not* fine to expand `foo-outer`
until `my_eager_macro!` is fully expanded.

We can label this concept 'stability':
- From the point of view of the outer invocation of `foo!`, the definition of
  `foo!` is *unstable*: `my_eager_macro!` might change or remove the
  definition.
- From the point of view of the inner invocation of `foo!` the definition *is*
  stable: nothing is going to change the definition before the invocation is
  expanded.

The concept of a definition being 'stable' *relative to* an invocation is more
useful when this situation is nested:

```rust
foo!();               // foo-outer
my_eager_macro! {     // my_eager_macro-outer
    foo!();           // foo-middle
    my_eager_macro! { // my_eager_macro-inner
        foo!();       // foo-inner
        macro foo() { ... };
    }
}
    
```

A correct implementation will ensure each call to `foo!` is expanded only once
the corresponding definition is 'stable'. In detail:
- `foo-inner` is always fine to expand: the definition of `foo!` can't be
  changed before `foo!` might be expanded.
- `foo-middle` can only be expanded once `my_eager_macro-inner` is fully
  expanded.
- `foo-outer` can only be expanded once `my_eager_macro-outer` is fully
  expanded.

See the appendix on [mutually-dependent
expansions](#mutually-dependent-expansions), and [paths that disappear during
expansion](#paths-that-disappear-during-expansion).

# Rationale and alternatives

The primary rationale is to make procedural and attribute macros work more
smoothly with other features of Rust - mainly other macros.

## Alternative: third-party expansion libraries
We could encourage the creation of a 'macros for macro authors' crate with
implementations of common macros (for instance, those in the standard library)
and make it clear that macro support isn't guaranteed for arbitrary macro calls
passed in to proc macros. This feels unsatisfying, since it fractures the macro
ecosystem and leads to very indirect unexpected behaviour (for instance, one
proc macro may use a different macro expansion library than another, and they
might return different results). This also doesn't help address macro calls in
built-in attributes.

## Alternative: global eager expansion
Opt-out eager expansion is backwards-incompatible with current macro behaviour:
* Consider `stringify!(concat!("a", "b"))`. If expanded eagerly, the result is
  `"ab"`. If expanded normally, the result is `concat ! ( "a" , "b" )`.
* Consider `quote!(expects_a_struct!(struct #X))`. If we eagerly expand
  `expects_a_struct!` this will probably fail: `expects_a_struct!` expects a
  normal complete struct declaration, not a `quote!` interpolation marker
  (`#X`).

Detecting these macro calls would require the compiler to parse arbitrary token
trees within macro arguments, looking for a `$path ! ( $($tt)*)` pattern, and
then treating that pattern as a macro call. Doing this everywhere essentially
bans that pattern from being used in custom macro syntax, which seems
excessive.

## Alternative: eager expansion invocation syntax
[RFC 1628](https://github.com/rust-lang/rfcs/pull/1628) proposes adding an
alternative invocation syntax to explicitly make the invocation eager (the
proposal text suggests `foo$!(...)`). The lang team couldn't reach
[consensus](https://github.com/rust-lang/rfcs/pull/1628#issuecomment-415617835)
on the design.

In addition to the issues discussed in RFC 1628, any proposal which marks
macros as eager 'in-line' with the invocation runs into a simiar issue to the
[global eager expansion](#alternative-global-eager-expansion) suggestion, which
is that it bans certain token patterns from macro inputs.

Additionally, special invocation syntax makes macro *output* sensitive to the
invocation grammar: a macro might need to somehow 'escape' `$!` in it's output
to prevent the compiler from trying to treat the surrounding tokens as an
invocation. This adds an unexpected and unnecessary burden on macro authors.

# Unresolved questions

* How do these proposals interact with hygiene?
* How should eager attribute expansion work?

# Appendix A: Corner cases

Some examples, plus how this proposal would handle them assuming full
implementation of all [desirable behaviour](#desirable-behaviour).

### Paths from inside a macro to outside

#### Should compile:
The definition of `m!` is stable (that is, it won't be changed
by further expansions), so the invocation of `m!` is safe to expand.
```rust
macro m() {}

my_eager_macro! {
    mod a {
        super::m!();
    }
}
```

### Paths within a macro

#### Should compile:
The definitions of `ma!` and `mb!` are stable (that is, they won't be changed
by further expansions), so the invocations are safe to expand.
```rust
my_eager_macro! {
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

#### Should compile:
```rust
my_eager_macro! {
    my_eager_macro! {
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

#### Should compile:
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

#### Should not compile:
Assuming `deletes_everything` always expands into an empty token stream, the
invocation of `m!` relies on a definition that won't be stable after further
expansion.
```rust
#[deletes_everything]
macro m() {}

m!();
```

### Mutually-dependent expansions

#### Should not compile:
Each expansion would depend on a definition that might not be stable after
further expansion, so the mutually-dependent definitions shouldn't resolve.
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

#### Should not compile:
The definition of `m!` isn't stable with respect to the invocation of `m!`,
since `expands_args` might change the definition.
```rust
#[expands_args(m!())]
macro m() {}
```

#### Should not compile:
The definition of `m!` isn't stable with respect to the invocation of `m!`,
since `expands_args_and_body` might change the definition.
```rust
#[expands_args_and_body(m!())]
macro m() {}
```

### Delayed definitions

#### Should compile:
* If the first invocation of `my_eager_macro!` is expanded first, it should
  notice that it can't resolve `x!` and have its expansion delayed.
* When the second invocation of `my_eager_macro!` is expanded, it provides a
  stable definition of `x!`. This should allow the first invocation to be
  're-expanded'.
```rust
macro make($name:ident) {
    macro $name() {}
}

my_eager_macro! {
    x!();
}

my_eager_macro! {
    make!(x);
}
```
