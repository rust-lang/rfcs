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
* Are there any corner-cases concerning attribute macros that aren't covered by
  treating them as two-argument proc-macros?
* What are the new expansion order rules? (See [Appendix
  B](#appendix-b-macro-expansion-order-example) for an exploration of one
  possible issue.)

# Appendix A: Corner cases

Some examples, plus how this proposal would handle them assuming full
implementation of all [desirable behaviour](#desirable-behaviour). Assume in
these examples that hygiene has been 'taken care of', in the sense that two
instances of the identifier `foo` are in the same hygiene scope.

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

# Appendix B: Macro expansion order example
Here we discuss an important corner case involving the precise meaning of
"resolving a macro invocation to a macro definition". We're going to explore
the situation where an eager macro changes the definition of a macro, even
while there are invocations of that macro which are apparently eligible for
expansion.

Warning: this section will contain long samples of intermediate macro expansion!

In these examples, assume that hygiene has been 'taken care of', in the sense
that two instances of the identifier `foo` are in the same hygiene scope (for
instance, through careful manipulation in a proc macro, or by being a shared
`$name:ident` fragment in a decl macro).

### The current case
Say we have two macros, `appends_hello!` and `appends_world!`, which are normal
declarative macros that add `println!("hello");` and  `println!("world");`,
respectively, to the end of any declarative macros that they parse in their
input; they leave the rest of their input unchanged.  For example, this:

```rust
appends_hello! {
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

Now, what do we expect the following to print?
```rust
foo!();
appends_world! {
    foo!();
    appends_hello! {
        foo!();
        macro foo() {};
    }
}
```

The expansion order is this:
* `appends_hello!` expands, because the outermost invocations of `foo!` can't
  be resolved. The result is:
    ```rust
    foo!();
    foo!();
    appends_hello! {
        foo!();
        macro foo() {};
    }
    ```
* `appends_world!` expands, because the two outermost invocations of `foo!`
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

### The eager case
Now, consider eager variants of `appends_hello!` and `appends_world!` (call
them `eager_appends_hello!` and `eager_appends_world!`) which eagerly expand
their input using `expand!`, *then* append the `println!`s to any macro
definitions they find, so that this:
```rust
eager_appends_hello! {
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
        concat!("a", b");
    };
    appends_hello!{ #tokens }
}
```
Which expands into:
```rust
appends_hello! {
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

Now, what do we expect the following to print?
```rust
foo!();         // foo-outer
eager_appends_world! {
    foo!();     // foo-middle
    eager_appends_hello! {
        foo!(); // foo-inner
        macro foo() {};
    }
}
```

The expansion order is this:
* The compiler expands `eager_appends_world!`, since `foo!` can't be resolved.
  The result is:
    ```rust
    foo!();             // foo-outer
    expand! {           // expand-outer
        #tokens = {
            foo!();     // foo-middle
            eager_appends_hello! {
                foo!(); // foo-inner
                macro foo() {};
            }
        };
        appends_world! {
            #tokens
        }
    }
    ```
* The compiler tries to expand the right-hand-side of the `#tokens = { ... }` line
  within `expand!`. The `foo!` invocations still can't be resolved, so the compiler
  expands `eager_appends_world!`. The result is:
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
                appends_hello! {
                    #tokens
                }
            }
        };
        appends_world! {
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
                appends_hello! {
                    #tokens
                }
            }
        };
        appends_world! {
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
            appends_hello! {
                macro foo() {};
            }
        };
        appends_world! {
            #tokens
        }
    }
    ```
* The next expansions are `appends_hello!` within `expand-outer`, then
  `expand-outer`, then `appends_world!`, and the result is:
    ```rust
    macro foo() {
        println!("hello");
        println!("world");
    }
    ```
And nothing gets printed because all the invocations of `foo!` disappeared earlier.

### Inside-out
* Say we expand `foo-inner`. At this point, `expand-inner` is now eligible to
  finish expansion and interpolate `#tokens` into `appends_hello!`. If it does
  so, the result is
    ```rust
    foo!();                 // foo-outer
    expand! {               // expand-outer
        #tokens = {
            foo!();         // foo-middle
            appends_hello! {
                macro foo() {};
            }
        };
        appends_world! {
            #tokens
        }
    }
    ```
* At this point, the definition of `foo!` is 'hidden' by `appends_hello!`, so neither
  `foo-outer` nor `foo-middle` can be resolved. The next expansion is `appends_hello!`,
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
        appends_world! {
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
        appends_world! {
            #tokens
        }
    }
    ```
* `expand-outer` is ready to complete, so we do that:
    ```rust
    foo!();                 // foo-outer
    appends_word! {
        <println!("hello")>;
        macro foo() {
            println!("hello");
        };
    }
    ```
* Then we expand `appends_word!`:
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

### The problem
It's apparent that eager expansion means we have more decisions to make with
respect to expansion order. Which of the above expansions seems reasonable?
Which ones are surprising? Is there a simple principle that suggests one of
these over the others?

