- Feature Name: opt-in macro expansion API
- Start Date: 2018-01-26
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

This is an RFC for adding a new feature to the language, opt-in eager macro
expansion. This will:
* Allow procedural and declarative macros to handle unexpanded macro calls that
  are passed as inputs,
* Allow macros to access the results of macro calls that they construct
  themselves,
* Enable macros to be used where the grammar currently forbids it.

# Motivation

## Expanding macros in input 

There are a few places where proc macros may encounter unexpanded macros in
their input:

* In declarative macros:
    ```rust
    env!(concat!("PA", "TH"));
    //   ^^^^^^^^^^^^^^^^^^^
    // Currently, `std::env!` is a compiler-builtin macro because it often
    // needs to expand input like this, and 'normal' macros aren't able
    // to do so.
    ```

* In procedural macros:
    ```rust
    my_proc_macro!(concat!("hello", "world"));
    //             ^^^^^^^^^^^^^^^^^^^^^^^^^
    // This call isn't expanded before being passed to `my_proc_macro`, and
    // can't be since proc macros are passed opaque token streams by design.
    ```

* In attribute macros:
    ```rust
    #[my_attr_macro(x = a_macro_call!(...))]
    //                  ^^^^^^^^^^^^^^^^^^
    // This call isn't expanded before being passed to `my_attr_macro`, and
    // can't be since attr macros are passed opaque token streams by design.
    struct X {...}
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

## Mutually-recursive macros

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
eagerly expand its input, then this invocation:
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
We might be tempted to 'trim down' our `expand!` macro to just expanding its
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
unresolvable *now*? How does a proc macro tell the compiler to 'delay' its
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
See [appendix A](#appendix-a) for a collection of 'unit tests' that exercise
these ideas.

### Interoperability
A good implementation will behave 'as expected' when asked to eagerly expand
*any* macro, whether it's a `macro_rules!` decl macro, or a 'macros 2.0' `macro
foo!()` decl macro, or a compiler-builtin macro. Similarly, a good
implementation will allow any kind of macro to perform such eager expansion.

### Path resolution
In Rust 2018, macros can be invoked by a path expression. These paths can be
complicated, involving `super` and `self`. An advanced implementation would
have an effective policy for how to resolve such paths. See appendix A on
[paths within a macro](#paths-within-a-macro), [paths from inside a macro to
outside](#paths-from-inside-a-macro-to-outside), and [paths within nested
macros](#paths-within-nested-macros).

### Expansion order
Depending on the order that macros get expanded, a definition might not be in
scope yet. An advanced implementation would delay expansion of an eager macro
until all its macro dependencies are available. See appendix A on [delayed
definitions](#delayed-definitions) and [paths within nested
macros](#paths-within-nested-macros).

This is more subtle than it might appear at first glance. An advanced
implementation needs to account for the fact that macro definitions ca vary
during expansion (see [appendix B](#appendix-b)). In fact, expansions
can be mutually-dependent *between* nested eager macros (see [appendix
C](#appendix-c)).

A guiding principle here is that, as much as possible, the result of eager
expansion shouldn't depend on the *order* that macros are expanded. This makes
expansion resilient to changes in the compiler's expansion process, and avoids
unexpected and desirable behaviour like being source-order dependent.
Additionally, the existing macro expansion process *mostly* has this property
and we should aim to maintain it.

A correct but simple implementation should be forwards-compatible with the
behaviour described in the appendices (perhaps by producing an error whenever
such a situation is detected).

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
invocation grammar: a macro might need to somehow 'escape' `$!` in its output
to prevent the compiler from trying to treat the surrounding tokens as an
invocation. This adds an unexpected and unnecessary burden on macro authors.

# Unresolved questions

* How do these proposals interact with hygiene?
* Are there any corner-cases concerning attribute macros that aren't covered by
  treating them as two-argument proc-macros?

<a id="appendix-a"></a>
# Appendix A: Corner cases

Some examples, plus how this proposal would handle them assuming full
implementation of all [desirable behaviour](#desirable-behaviour). Assume in
these examples that hygiene has been 'taken care of', in the sense that two
instances of the identifier `foo` are in the same hygiene scope.

### Paths from inside a macro to outside

#### Should compile:
The definition of `m!` isn't going to vary through any further expansions, so
the invocation of `m!` is safe to expand.
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
The definitions of `ma!` and `mb!` aren't within a macro, so the definitions
won't vary through any further expansions, so it's safe to expand the
invocations.
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
This demonstrates that we shouldn't expand an invocation if the corresponding
definition is 'in' an attribute macro. In this case, `#[deletes_everything]`
expands into an empty token stream.
```rust
#[deletes_everything]
macro m() {}

m!();
```

### Mutually-dependent expansions

#### Should not compile:
Each expansion would depend on a definition that might vary in further
expansions, so the mutually-dependent definitions shouldn't resolve.
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
The definition of `m!` isn't available if only expanding the arguments
in `#[expands_args]`.
```rust
#[expands_args(m!())]
macro m() {}
```

#### Not sure if this should compile:
The definition of `m!` is available, but it also might be different after
`#[expands_args_and_body]` expands.
```rust
#[expands_args_and_body(m!())]
macro m() {}
```

### Delayed definitions

#### Should compile:
* If the first invocation of `my_eager_macro!` is expanded first, it should
  notice that it can't resolve `x!` and have its expansion delayed.
* When the second invocation of `my_eager_macro!` is expanded, it provides a
  definition of `x!` that won't vary after further expansion. This should
  allow the first invocation to be 're-expanded'.
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

<a id="appendix-b"></a>
# Appendix B: varying definitions during expansion
Here we discuss an important corner case involving the precise meaning of
"resolving a macro invocation to a macro definition". We're going to explore
the situation where an eager macro 'changes' the definition of a macro (by
adjusting and emitting an input definition), even while there are invocations
of that macro which are apparently eligible for expansion. The takeaway is that
eager expansion is sensitive to expansion order *outside of* eager macros
themselves.

Warning: this section will contain long samples of intermediate macro expansion!

In these examples, assume that hygiene has been 'taken care of', in the sense
that two instances of the identifier `foo` are in the same hygiene scope (for
instance, through careful manipulation in a proc macro, or by being a shared
`$name:ident` fragment in a decl macro).

## The current case
<a id="normal-append-definition"></a>
Say we have two macros, `append_hello!` and `append_world!`, which are normal
declarative macros that add `println!("hello");` and  `println!("world");`,
respectively, to the end of any declarative macros that they parse in their
input; they leave the rest of their input unchanged. For example, this:

```rust
append_hello! {
    struct X();

    macro foo() {
        <whatever>
    }
}
```
Should expand into this:
```rust
struct X();

macro foo() {
    <whatever>
    println!("hello");
}
```

<a id="current-append-example"></a>
Now, what do we expect the following to print?
```rust
foo!();
append_world! {
    foo!();
    append_hello! {
        foo!();
        macro foo() {};
    }
}
```

The expansion order is this:
* `append_world!` expands, because the outermost invocations of `foo!` can't
  be resolved. The result is:
    ```rust
    foo!();
    foo!();
    append_hello! {
        foo!();
        macro foo() {};
    }
    ```
* `append_hello!` expands, because the two outermost invocations of `foo!`
  still can't be resolved. The result is:
    ```rust
    foo!();
    foo!();
    foo!();
    macro foo() {
        println!("hello");
    }
    ```
And now it should be clear that we expect the output:
```
hello
hello
hello
```

Notice that because there can only be one definition of `foo!`, that definition
is either inside the arguments of another macro (like `append_hello!`) and
can't be resolved, or it's at the top level.

In a literal sense, the definition of `foo!` *doesn't exist* until it's at the
top level; before that point it's just some tokens in another macro that
*happen to parse* as a definition.

In a metaphorical sense, the 'intermediate definitions' of `foo!` don't exist
because we *can't see their expansions*: they are 'unobservable' by any
invocations of `foo!`. This isn't true in the eager case!

## The eager case
<a id="eager-append-definition"></a>
Now, consider eager variants of `append_hello!` and `append_world!` (call
them `eager_append_hello!` and `eager_append_world!`) which eagerly expand
their input using `expand!`, *then* append the `println!`s to any macro
definitions they find using their [non-eager](#normal-append-definition)
counterpart, so that this:
```rust
eager_append_hello! {
    macro foo() {}
    foo!();
    concat!("a", "b");
}
```
Expands into:
```rust
expand! {
    #tokens = {
        macro foo() {};
        foo!(); // This will expand to an empty token stream.
        concat!("a", "b");
    };
    append_hello!{ #tokens }
}
```
Which expands into:
```rust
append_hello! {
    macro foo() {};
    "ab";
}
```
Which finally expands into:
```rust
macro foo() {
    println!("hello");
};
"ab";
```

Let's take our [previous example](#current-append-example) and replace the
`append` macros with their eager variants. What do we expect the following to
print?
```rust
foo!();         // foo-outer
eager_append_world! {
    foo!();     // foo-middle
    eager_append_hello! {
        foo!(); // foo-inner
        macro foo() {};
    }
}
```

The expansion order is this:
* The compiler expands `eager_append_world!`, since `foo!` can't be resolved.
  The result is:
    ```rust
    foo!();             // foo-outer
    expand! {           // expand-outer
        #tokens = {
            foo!();     // foo-middle
            eager_append_hello! {
                foo!(); // foo-inner
                macro foo() {};
            }
        };
        append_world! {
            #tokens
        }
    }
    ```
* The compiler tries to expand the right-hand-side of the `#tokens = { ... }` line
  within `expand!`. The `foo!` invocations still can't be resolved, so the compiler
  expands `eager_append_world!`. The result is:
    <a id="ambiguous-expansion-choices"></a>
    ```rust
    foo!();                 // foo-outer
    expand! {               // expand-outer
        #tokens = {
            foo!();         // foo-middle
            expand! {       // expand-inner
                #tokens = {
                    foo!(); // foo-inner
                    macro foo() {};
                };
                append_hello! {
                    #tokens
                }
            }
        };
        append_world! {
            #tokens
        }
    }
    ```

At this point, we have several choices. We hand-waved
[earlier](#mutually-recursive-macros) that the tokens within `expand!` should
be expanded "exactly as though the compiler were parsing and expanding these
tokens directly". Well, as far as the compiler can tell, there are three
invocations of `foo!` (the ones labelled `foo-outer`, `foo-middle`, and
`foo-inner`), and there's a perfectly good definition `macro foo()` for us to
use.

### Outside-in
* Say we expand the invocations in this order: `foo-outer`, `foo-middle`,
  `foo-inner`.  Using the 'current' definition of `foo!`, these all become
  empty token streams and the result is:
    ```rust
    expand! {           // expand-outer
        #tokens = {
            expand! {   // expand-inner
                #tokens = {
                    macro foo() {};
                };
                append_hello! {
                    #tokens
                }
            }
        };
        append_world! {
            #tokens
        }
    }
    ```
* The only eligible macro to expand is `expand-inner`, which is ready to
  interpolate `#tokens` (which contains no macro calls) into `append_hello!`.
  The result is:
    ```rust
    expand! {           // expand-outer
        #tokens = {
            append_hello! {
                macro foo() {};
            }
        };
        append_world! {
            #tokens
        }
    }
    ```
* The next expansions are `append_hello!` within `expand-outer`, then
  `expand-outer`, then `append_world!`, and the result is:
    ```rust
    macro foo() {
        println!("hello");
        println!("world");
    }
    ```
And nothing gets printed because all the invocations of `foo!` disappeared earlier.

### Inside-out
* Starting from where we made our [expansion
  choice](#ambiguous-expansion-choices), say we expand `foo-inner`. At this
  point, `expand-inner` is now eligible to finish expansion and interpolate
  `#tokens` into `append_hello!`. If it does so, the result is:
    ```rust
    foo!();                 // foo-outer
    expand! {               // expand-outer
        #tokens = {
            foo!();         // foo-middle
            append_hello! {
                macro foo() {};
            }
        };
        append_world! {
            #tokens
        }
    }
    ```
* At this point, the definition of `foo!` is 'hidden' by `append_hello!`, so neither
  `foo-outer` nor `foo-middle` can be resolved. The next expansion is `append_hello!`,
  and the result is:
    ```rust
    foo!();                 // foo-outer
    expand! {               // expand-outer
        #tokens = {
            foo!();         // foo-middle
            macro foo() {
                println!("hello");
            };
        };
        append_world! {
            #tokens
        }
    }
    ```
* Here, we have a similar choice to make between expanding `foo-outer` and
  `foo-middle`.  If we expand `foo-outer` with the 'current' definition of
  `foo!`, it becomes `println!("hello");`. Instead, we'll continue 'inside-out'
  and fully expand `foo-middle` next.  For simplicity, we'll write the result
  of expanding `println!("hello");` as `<println!("hello");>`. The result is:
    ```rust
    foo!();                 // foo-outer
    expand! {               // expand-outer
        #tokens = {
            <println!("hello")>;
            macro foo() {
                println!("hello");
            };
        };
        append_world! {
            #tokens
        }
    }
    ```
* `expand-outer` is ready to complete, so we do that:
    ```rust
    foo!();                 // foo-outer
    append_world! {
        <println!("hello")>;
        macro foo() {
            println!("hello");
        };
    }
    ```
* Then we expand `append_world!`:
    ```rust
    foo!();                 // foo-outer
    <println!("hello")>;
    macro foo() {
        println!("hello");
        println!("world");
    };
    ```
And we expect the output:
```
hello
world
hello
```

## Choosing expansion order 
It's apparent that eager expansion means we have more decisions to make with
respect to expansion order, and that these decisions *matter*. The fact that
eager expansion is recursive, and involves expanding the 'leaves' before
backtracking, hints that we should favour the 'inside-out' expansion order.

In this example, we feel that this order matches each invocation with the
'correct' definition: an expansion of `foo!` outside of `eager_append_hello!`
acts as though `eager_append_hello!` expanded 'first', which is what it should
mean to expand eagerly!

[Appendix C](#appendix-c) explores an example that goes through this behaviour
in more detail, and points to a more general framework for thinking about eager
expansion.

<a id="appendix-c"></a>
# Appendix C: mutually-dependent eager expansions
Here we discuss an important corner case involving nested eager macros which
depend on definitions contained in each other. By the end, we will have
motivation for a specific and understandable model for how we 'should' think
about eager expansion.

Warning: this section will contain long samples of intermediate macro expansion!
We'll elide over some of the 'straightforward' expansion steps. If you want to
get a feel for what these steps involve, [appendix B](#appendix-b) goes through
them in more detail.

For these examples we're going to re-use the definitions of [`append_hello!`,
`append_world!`](#normal-append-definition), [`eager_append_hello!`, and
`eager_append_world!`](#eager-append-definition) from appendix B.

In these examples, assume that hygiene has been 'taken care of', in the sense
that two instances of the identifier `foo` are in the same hygiene scope (for
instance, through careful manipulation in a proc macro, or by being a shared
`$name:ident` fragment in a decl macro).

## A problem
Assume `id!` is the identity macro (it just re-emits whatever its inputs are).
What do we expect this to print?
```rust
eager_append_world! {
    eager_append_hello! {
        id!(macro foo() {}); // id-inner
        bar!();              // bar-inner
    };
    id!(macro bar() {});     // id-outer
    foo!();                  // foo-inner
};
foo!();                      // foo-outer
bar!();                      // bar-outer
```

<a id="appendix-c-after-eager-expansion"></a>
We can skip ahead to the case where both of the eager macros have expanded into
`expand!`:
```rust
expand! {                            // expand-outer
    #tokens = {
        expand! {                    // expand-inner
            #tokens = {
                id!(macro foo() {}); // id-inner
                bar!();              // bar-inner
            };
            append_hello! { #tokens };
        };
        id!(macro bar() {});         // id-outer
        foo!();                      // foo-inner
    };
    append_world! { #tokens };
};
foo!();                              // foo-outer
bar!();                              // bar-outer
```

Hopefully you can convince yourself that there's no way for `expand-inner` to
finish expansion without expanding `id-outer` within `expand-outer`, and
there's no way for `expand-outer` to finish expansion without expanding
`id-inner` within `expand-inner`; this means we can't *just* use the
'inside-out' expansion order that we looked at in [appendix B](#appendix-b).

## A solution
A few simple rules let us make progress in this example while recovering the
desired 'inside-out' behaviour discussed [earlier](#inside-out).

Assume that the compiler associates each `expand!` macro with an *expansion
context* which tracks macro invocations and definitions that appear within the
expanding tokens. Additionally, assume that these form a tree: if an eager
macro expands another eager macro, as above, the 'inner' definition scope is a
child of the outer definition scope (which is a child of some global 'root'
scope).

With these concepts in mind, at [this point](#appendix-c-after-eager-expansion)
our contexts look like this:
```toml
ROOT = {
    Definitions = [
        "id", "append_hello", "append_world",
        "eager_append_hello", "eager_append_world",
    ],
    Invocations = [
        "foo-outer",
        "bar-outer",
    ],
    Child-Contexts = {
        expand-outer = {
            Definitions = [],
            Invocations = [
                "id-outer",
                "foo-inner",
            ],
            Child-Contexts = {
                expand-inner = {
                    Definitions = [],
                    Invocations = [
                        "id-inner",
                        "bar-inner",
                    ],
                    Child-Contexts = {}
                }
            }
        }
    }
}
```

Now we use these rules to direct our expansions:
* An `expand!` invocation can only use a definition that appears in its own
  context, or its parent context (or grandparent, etc).
* An `expand!` invocation is 'complete' once its context has no invocations
  left. At that point the resulting tokens are interpolated and the context is
  destroyed.

Notice that, under this rule, both `id-outer` and `id-inner` are eligible for
expansion. After we expand them, our tokens will look like this:
```rust
expand! {               // expand-outer
    #tokens = {
        expand! {       // expand-inner
            #tokens = {
                macro foo() {};
                bar!(); // bar-inner
            };
            append_hello! { #tokens };
        };
        macro bar() {};
        foo!();         // foo-inner
    };
    append_world! { #tokens };
};
foo!();                 // foo-outer
bar!();                 // bar-outer
```
And our contexts will look like this:
```toml
ROOT = {
    Definitions = [
        "id", "append_hello", "append_world",
        "eager_append_hello", "eager_append_world",
    ],
    Invocations = [
        "foo-outer",
        "bar-outer",
    ],
    Child-Contexts = {
        expand-outer = {
            Definitions = [
#               A new definition!
#               vvvvvvvvvvv
                "macro bar",
            ],
            Invocations = [
                "foo-inner",
            ],
            Child-Contexts = {
                expand-inner = {
                    Definitions = [
#                       A new definition!
#                       vvvvvvvvvvv
                        "macro foo", 
                    ],
                    Invocations = [
                        "bar-inner",
                    ],
                    Child-Contexts = {}
                }
            }
        }
    }
}
```

At this point, `foo-inner` *isn't* eligible for expansion because the
definition of `macro foo` is in a child context of the invocation context. This
is how we prevent `foo-inner` from being expanded 'early' (that is, before the
definition of `macro foo` gets modified by `append_hello!`).

However, `bar-inner` *is* eligible for expansion. The definition of `macro bar`
can only be modified once `expand-outer` finishes expanding, but `expand-outer`
can't continue expanding until `expand-inner` finishes expanding. Since the
definition can't vary for as long as `bar-inner` is around, it's 'safe' to
expand `bar-inner` whenever we want.  Once we do so, the tokens look like this:
```rust
expand! {               // expand-outer
    #tokens = {
        expand! {       // expand-inner
            #tokens = {
                macro foo() {};
            };
            append_hello! { #tokens };
        };
        macro bar() {};
        foo!();         // foo-inner
    };
    append_world! { #tokens };
};
foo!();                 // foo-outer
bar!();                 // bar-outer
```
And the context is unsurprising: 
```toml
ROOT = {
    Definitions = [
        "id", "append_hello", "append_world",
        "eager_append_hello", "eager_append_world",
    ],
    Invocations = [
        "foo-outer",
        "bar-outer",
    ],
    Child-Contexts = {
        expand-outer = {
            Definitions = [
                "macro bar",
            ],
            Invocations = [
                "foo-inner",
            ],
            Child-Contexts = {
                expand-inner = {
                    Definitions = [
                        "macro foo", 
                    ],
                    Invocations = [],
                    Child-Contexts = {}
                }
            }
        }
    }
}
```

Our second rule kicks in now that `expand-inner` has no invocations. We
'complete' `expand-inner` by performing the relevant interpolation, resulting
in these tokens:
```rust
expand! {               // expand-outer
    #tokens = {
        append_hello! { 
            macro foo() {};
        };
        macro bar() {};
        foo!();         // foo-inner
    };
    append_world! { #tokens };
};
foo!();                 // foo-outer
bar!();                 // bar-outer
```
And these contexts:
```toml
ROOT = {
    Definitions = [
        "id", "append_hello", "append_world",
        "eager_append_hello", "eager_append_world",
    ],
    Invocations = [
        "foo-outer",
        "bar-outer",
    ],
    Child-Contexts = {
        expand-outer = {
            Definitions = [
                "macro bar",
            ],
            Invocations = [
                "foo-inner",
                "append_hello!",
            ],
            Child-Contexts = {}
        }
    }
}
```
And from here the expansions are unsurprising.

## Macro race conditions
It can be instructive to see what kind of behaviour these rules *don't* allow.
This example is derived from a similar example in [appendix
A](#mutually-dependent-expansions):
```rust
eager_append_hello! {
    macro foo() {};
    bar!();
}

eager_append_world! {
    macro bar() {};
    foo!();
}
```
You should be able to convince yourself that the rules above will 'deadlock':
neither of the eager macros will be able to expand to completion, and that
the compiler should error with something along the lines of:
```
Error: can't resolve invocation to `bar!` because the definition
       is in an unexpandable macro
|   eager_append_hello! {
|       macro foo() {};
|       bar!();
|       ------ invocation of `bar!` occurs here.
|   }
|
|   eager_append_world! {
|   ^^^^^^^^^^^^^^^^^^^ this macro can't be expanded
|                       because it needs to eagerly expand
|                       `foo!`, which is defined in an
|                       unexpandable macro.
|       macro bar() {};
|       -------------- definition of `bar` occurs here.
|       foo!();
|   }
```
And a similar error for `foo!`.

This is a good outcome! The alternative would be to expand `foo!()` even though
the definition of `macro foo` will be different after further expansion, or
likewise for `bar!()`; the end result would depend on which eager macro
expanded first!

## Eager expansion as dependency tree
The 'deadlock' example highlights another way of viewing this 'context tree'
model of eager expansion. Normal macro expansion has one kind of dependency
that constrains expansion order: an invocation depends on its definition. Eager
expansion adds another kind of dependency: the result of one eager macro can
depend on the result of another eager macro.

Our rules are (we think) the weakest rules that force the compiler to resolve
these dependencies in the 'right' order, while leaving the compiler with the
most flexibility otherwise (for instance in the [previous
example](#appendix-c-after-eager-expansion), it *shouldn't matter* whether the
compiler expands `id-inner` or `id-outer` first. It should even be able to
expand them concurrently!).

## Expansion context details 
In the above examples, we associated an expansion context with each invocation
to `expand!`.  An alternative is to associate a context with *each* expansion
binding *within* an invocation to expand, so that this invocation:
```rust
expand! {
    #tokens_1 = {
        foo!();
    };
    #tokens_2 = {
        macro foo() {};
    };
    bar! { #tokens_1 };
}
```
Has this context tree:
```toml
ROOT = {
    Definitions = [],
    Invocations = [],
    Child-Contexts = {
        expand = {
            "#tokens_1" = {
                Definitions = [],
                Invocations = [
                    "foo!()",
                ],
            },
            "#tokens_2" = {
                Definitions = [
                    "macro foo()",
                ],
                Invocations = [],
            },
        }
    }
}
```

In this case, having the contexts be separate should lead to a similar deadlock
as [above](#macro-race-conditions): The context for `#tokens_1` can't see the
definition in `#context_2`, but `expand!` can't continue without expanding the
invocation of `foo!`.

Is this the expected behaviour? What use-cases does it prevent? What use-cases
does it allow?
