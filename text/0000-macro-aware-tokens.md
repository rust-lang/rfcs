- Feature Name: Macro Generations and Expansion Order
- Start Date: 2018-01-26
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Add an API for procedural macros to expand macro calls in token streams. This will allow proc macros to handle unexpanded macro calls that are passed as inputs, as well as allow proc macros to access the results of macro calls that they construct themselves.

# Motivation

There are a few places where proc macros may encounter unexpanded macros in their input:

* In attribute and procedural macros:

    ```rust
    #[my_attr_macro(x = a_macro_call!(...))]
    //                  ^^^^^^^^^^^^^^^^^^
    // This call isn't expanded before being passed to `my_attr_macro`, and can't be
    // since attr macros are passed raw token streams by design.
    struct X {...}
    ```

    ```rust
    my_proc_macro!(concat!("hello", "world"));
    //             ^^^^^^^^^^^^^^^^^^^^^^^^^
    // This call isn't expanded before being passed to `my_proc_macro`, and can't be
    // since proc macros are passed raw token streams by design.
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
    // This call isn't expanded before being passed to `my_proc_macro`, and can't be
    // because `m!` is declared to take a token tree, not a parsed expression that we know
    // how to expand.
    ```

In these situations, proc macros need to either re-call the input macro call as part of their token output, or simply reject the input. If the proc macro needs to inspect the result of the macro call (for instance, to check or edit it, or to re-export a hygienic symbol defined in it), the author is currently unable to do so. This implies an additional place where a proc macro might encounter an unexpanded macro call, by _constructing_ it:

* In a proc macro definition:

    ```rust
    #[proc_macro]
    pub fn my_proc_macro(tokens: TokenStream) -> TokenStream {
        let token_args = extract_from(tokens);
    
        // These arguments are a token stream, but they will be passed to `another_macro!`
        // after being parsed as whatever `another_macro!` expects.
        //                                                  vvvvvvvvvv
        let other_tokens = some_other_crate::another_macro!(token_args);
        //                 ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
        // This call gets expanded into whatever `another_macro` expects to be expanded
        // as. There is currently no way to get the resulting tokens without requiring the
        // macro result to compile in the same crate as `my_proc_macro`.
        ...
    }
    ```

Giving proc macro authors the ability to handle these situations will allow proc macros to 'just work' in more contexts, and without surprising users who expect macro calls to interact well with more parts of the language. Additionally, supporting the 'proc macro definition' use case above allows proc macro authors to use macros from other crates _as macros_, rather than as proc macro definition functions.

As a side note, allowing macro calls in built-in attributes would solve a few outstanding issues (see [rust-lang/rust#18849](https://github.com/rust-lang/rust/issues/18849) for an example). 

An older motivation to allow macro calls in attributes was to get `#[doc(include_str!("path/to/doc.txt"))]` working, in order to provide an ergonomic way to keep documentation outside of Rust source files. This was eventually emulated by the accepted [RFC 1990](https://github.com/rust-lang/rfcs/pull/1990), indicating that macros in attributes could be used to solve problems at least important enough to go through the RFC process.

# Guide-level explanation

## Macro Calls in Macro Input

When implementing a procedural or attribute macro you should account for the possibility that a user might provide a macro call in their input. As an example of where this might trip you up when writing a procedural macro, here's a silly one that evaluates to the length of the string literal passed in:

```rust
extern crate syn;
#[macro_use]
extern crate quote;

#[proc_macro]
pub fn string_length(tokens: TokenStream) -> TokenStream {
    let lit: syn::LitStr = syn::parse(tokens).unwrap();
    let len = str_lit.value().len();
    
    quote!(#len)
}
```

If you call `string_length!` with something obviously wrong, like `string_length!(struct X)`, you'll get a parser error when `unwrap` gets called, which is expected. But what do you think happens if you call `string_length!(stringify!(struct X))`?

It's reasonable to expect that `stringify!(struct X)` gets expanded and turned into a string literal `"struct X"`, before being passed to `string_length`. However, in order to give the most control to proc macro authors, Rust usually doesn't touch any of the ingoing tokens passed to a procedural macro.

A similar issue happens with attribute macros, but in this case there are two places you have to watch out: the attribute arguments, as well as the body. Consider this:


```rust
#[my_attr_macro(value = concat!("Hello, ", "world!"))]
mod whatever {
    procedural_macro_that_defines_a_struct! {
        ...
    }
}
```

If `#[my_attr_macro]` is expecting to see a struct inside of `mod whatever`, it's going to run into trouble when it sees that macro instead. The same happens with `concat!` in the attribute arguments: Rust doesn't look at the input tokens, so it doesn't even know there's a macro to expand!

Thankfully, there's a way to _tell_ Rust to treat some tokens as macros, and to expand them before trying to expand _your_ macro.

## Macro Generations and Expansion Order

Rust uses an iterative process to expand macros, as well as to control the relative timing of macro expansion. The idea is that we expand any macros we can see (the current 'generation' of macros), and then expand any macros that _those_ macros had in their output (the _next_ 'generation'). In more detail, the processing loop that Rust performs is roughly as follows:

1. Set the current macro generation number to 1.
2. Parse _everything_. This lets us get the `mod` structure of the crate so that we can resolve paths (and macro names!).
3. Collect all the macro invocations we can see.
    * This includes any macros that we parsed, as well as any macros that have been explicitly marked inside any bare token streams (that is, within `bang_macro!` and `#[attribute_macro]` arguments).
    * If the macro doesn't have a generation number, assign it to the current generation.
4. Identify which macros to expand, and expand them. A macro might indicate that it should be run _later_ by having a higher generation number than the current generation; we skip those until the generation number is high enough, and expand the rest.
6. Increment the current generation number, then go back to step 2.

By carefully controlling the order in which macros get expanded, we can work with this process to handle the issues we identified earlier.

## Macro Generation API

The `proc_macro` crate provides an API for annotating some tokens with metadata that tells the compiler if and when to expand them like a normal macro invocation. The API revolves around an `ExpansionBuilder`, a builder-pattern struct that lets you adjust the relevant token information:

```rust
struct ExpansionBuilder {...};

impl ExpansionBuilder {
    pub fn from_tokens(tokens: TokenStream) -> Result<Self, ParseError>;
    pub fn generation(&self) -> Option<isize>;
    pub fn set_generation(self, generation: isize) -> Self;
    pub fn adjust_generation(self, count: isize) -> Self;
    pub fn into_tokens(self) -> TokenStream;
}
```

The constructor `from_tokens` takes in either a bang macro or attribute macro with arguments (`my_proc_macro!(some args)` or `#[my_attr_macro(some other args)]`).

The method `generation` lets you inspect the existing generation number (if any) of the input. This might be useful to figure out when a macro you've encountered in your tokens will be expanded, in order to ensure that some other macro expands before or after it.

The builder methods `set_generation` and `adjust_generation` annotate the tokens passed in to tell the compiler to expand them at the appropriate generation (if the macro doesn't have a generation, `adjust_generation(count)` sets it to `count`).

Finally, the method `into_tokens` consumes the `ExpansionBuilder` and provides the annotated tokens.

## Using Generations to Handle Macro Calls

Let's use our `string_length!` procedural macro to demonstrate how to use `ExpansionBuilder` to handle macros in our input. Say we get called like this:

```rust
// Generation 0 macro tokens.
// vvvvvvvvvvvvvvv----------------------------v
   string_length!(concat!("hello, ", "world!"));
```

The bits marked with `v` are tokens that the compiler will find, and decide are a generation 0 macro. Notice that this doesn't include the arguments! So, in the implementation of `string_length!`:

```rust
#[proc_macro]
pub fn string_length(tokens: TokenStream) -> TokenStream {
    // Handle being given a macro...
    if let Ok(_: syn::Macro) = syn::parse(tokens) {
        // First, mark the macro tokens so that the compiler
        // will expand the macro at some point.
        let input_tokens =
                ExpansionBuilder::from_tokens(tokens)
                    .unwrap()
                    .adjust_generation(0)
                    .into_tokens();

        // Here's the trick - in our expansion we _include ourselves_,
        // but delay our expansion until after the inner macro is expanded!
        let new_tokens = quote! {
            string_length!(#tokens)
        };
        return ExpansionBuilder::from_tokens(TokenStream::from(new_tokens))
                .unwrap()
                .adjust_generation(1)
                .into_tokens();
    }

    // Otherwise, carry on!
    let lit: syn::LitStr = syn::parse(tokens).unwrap();
    let len = str_lit.value().len();
    
    quote!(#len)
}
```

The resulting tokens look like this: 

```rust
// New generation 1 macro tokens.
// vvvvvvvvvvvvvvv----------------------------v
   string_length!(concat!("hello, ", "world!"));
//                ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
// New generation 0 macro tokens.
```

Now, in the next macro expansion loop, the compiler will find those generation-0 macro tokens and expand them. After that, the tokens look like this:

```rust
// Still generation 1 macro tokens.
// vvvvvvvvvvvvvvv---------------v
   string_length!("hello, world!");
```

And now `string_length!` expands happily!

### Macro Generation Utilities

Unforunately, the above is fairly verbose. Fortunately, `syn` provides a utility function `mark_macros` for finding and marking macros within the tokens of a well-formed item or expression:

```rust
#[proc_macro]
pub fn string_length(tokens: TokenStream) -> TokenStream {
    if let Ok((generation, tokens)) = syn::mark_macros(&tokens, 0) {
        let tokens = quote! {
           string_length!(#tokens)
        }.into();

        let (_, tokens) = syn::mark_macros(&tokens, generation + 1).unwrap();
        return tokens.into();
    }

    // The rest remains the same.
    ...
}

```

In more detail, `mark_macros(tokens, gen)` will look for any unmarked eligible macro tokens in `tokens` and mark them to be expanded in generation `gen`. If any macro tokens were encountered (including existing ones!), `mark_macros` returns the highest generation encountered as well as the tokens. This lets you use `mark_macros` as a catch-all test for any unexpanded macros in `tokens`.

### An Example: Attribute Macros

Let's look at another example: handling macros in attribute macros. Consider this:

```rust
#[my_attr_macro(concat!("hello, ", "world!"))]
mod foo {
    #[another_attr_macro(include_str!("some/path"))]
    a_proc_macro! {
        ...
    }
}
```

If `#[my_attr_macro]` doesn't want to deal with _any_ macros in its input, it can handle this quite easily:

```rust
#[proc_macro_attribute]
pub fn my_attr_macro(args: TokenStream, body: TokenStream) -> TokenStream {
    if let Ok((args_gen, args)) = syn::mark_macros(&args, 0) {
        let tokens = quote! {
            #[my_attr_macro(#args)]
            #body
        }.into();

        let (_, tokens) = syn::mark_macros(&tokens, args_gen + 1).unwrap();
        return tokens.into();
    }

    if let Ok((body_gen, body)) = syn::mark_macros(&body, 0) {
        let tokens = quote! {
            #[my_attr_macro(#args)]
            #body
        }.into();

        let (_, tokens) = syn::mark_macros(&tokens, body_gen + 1).unwrap();
        return tokens.into();
    }

    // Otherwise, carry on.
    ...
}

```

This definition of `my_attr_macro` will recursively call itself after marking any macros in its argument tokens to be expanded. Once those are all done, it repeats the process with the tokens in its body.

Looking at the example call to `#[my_attr_macro]` above, this is the order in which the macros will get marked and expanded:

TODO: This example is verbose, but is also a _very_ clear demonstration of how the above solution somves the problem of complicated inner expansions. Is there a more concise example? Is there a better way to present it?

* First, the compiler sees `#[my_proc_macro(...)]` and marks it as generation 0.
* Then, the compiler expands the generation 0 `#[my_proc_macro(...)]`:
    * Since there are macros in the arguments, it expands into a generation 1 call to itself wrapping a newly-marked generation 0 `concat!(...)`.
* The compiler sees the generation 0 `concat!(...)` and expands it.
* The compiler sees the generation 1 `#[my_proc_macro(...)]` and expands it:
    * Since there are no macros in the arguments, it marks the call to `#[another_attr_macro(...)]` as generation 0, and expands into a generation 1 call to itself wrapping the new macro-marked body.
* The compiler sees the generation 0 `#[another_attr_macro(...)]` and expands it:
    * If `another_attr_macro` is implemented similarly to `my_attr_macro`, it'll mark `include_str!(...)` as generation 0 and expand into a call to itself marked as generation 1.
* The compiler sees the generation 0 `include_str!(...)` and expands it.
* The compiler sees the generation 1 `#[my_attr_macro(...)]` and expands it:
    * `my_attr_macro` sees that the body has a macro marked generation 1, so it expands into itself (again), but this time marked generation 2.
* The compiler sees the generation 1 `#[another_attr_macro(...)]` and expands it:
    * Since there are no macros in the arguments to `another_attr_macro`, it checks the body for macros. It marks the call to `a_proc_macro!` as generation 0 and expands into itself marked as generation 1.
* The compiler sees the generation 0 `a_proc_macro!(...)` call and expands it.
* The compiler sees the generation 1 `#[another_attr_macro(...)]` and expands it.
* The compiler sees the generation 2 `#[my_attr_macro(...)]` and expands it.

Since `mark_macros` is so flexible, it can be used to implement a variety of expansion policies. For instance, `my_attr_macro` could decide to mark the macros in its arguments and body at the same time, rather than handling one then the other.

# Reference-level explanation

The proposed additions to the proc macro API in `proc_macro` are outlined above in the [API overview](#macro-generation-api). Here we focus on technical challenges.

Currently, the compiler does actually perform something similar to the loop described in th section on [expansion order](#macro-generations-and-expansion-order). We could 'just' augment the step that identifies potential macro calls to also inspect the otherwise unstructured token trees within macro arguments.

This proposal requires that some tokens contain extra semantic information similar to the existing `Span` API. Since that API (and its existence) is in a state of flux, details on what this 'I am a macro call that you need to expand!' idea may need to wait until those have settled.

The token structure that `ExpansionBuilder` should expect is any structure that parses into a complete procedural macro call or into a complete attribute macro call (TODO: should this include the outer `#[...]`? Should this include the body?). This provides the path used to resolve the macro, as well as the delimited argument token trees.

The token structure that `ExpansionBuilder` produces should have the exact same structure as the input (a path plus a delimited argument token tree, as well as any other sigils). The _path_ and the _delimiter_ node of the arguments should be marked, but the _content nodes_ of the arguments should be unchanged.

# Drawbacks

This proposal:

* Relies on proc macro authors doing macro expansion. This might partition the macro ecosystem into expansion-ignoring (where input macro calls are essentially forbidden for any part of the input that needs to be inspected) and expansion-handling (where they work fine _as long as_ the proc macro author has used the expansion API correctly).

* Leads to frustrating corner-cases involving macro paths. For instance, consider the following:

    ```rust
    macro baz!(...);
    foo! {
        mod b {
            super::baz!();
        }
    }
    ```

    The caller of `foo!` probably imagines that `baz!` will be expanded within `b`, and so prepends the call with `super`. However, if `foo!` naively lifts the call to `super::baz!`, then the path will fail to resolve because macro paths are resolved relative to the location of the call. Handling this would require the macro implementer to track the path offset of its expansion, which is doable but adds complexity.

* Commits the compiler to a particular macro expansion order, as well as a way for users to position themselves within that order. What future plans does this interfere with?

# Rationale and alternatives

The primary rationale is to make procedural and attribute macros work more smoothly with other features of Rust - mainly other macros.

Recalling the examples listed in the [motivation](#motivation) above, a few but not all situations of proc macros receiving unexpanded macro calls could be avoided by changing the general 'hands off' attitude towards proc macros and attribute macros, and more aggressively parse and expand their inputs. This effectively bans macro calls as part of the input grammar, which seems drastic, and wouldn't handle cases of indirection via token tree (`$x:tt`) parameters.

We could encourage the creation of a 'macros for macro authors' crate with implementations of common macros - for instance, those in the standard library - and make it clear that macro support isn't guaranteed for arbitrary macro calls passed in to proc macros. This feels unsatisfying, since it fractures the macro ecosystem and leads to very indirect unexpected behaviour (for instance, one proc macro may use a different macro expansion library than another, and they might return different results). This also doesn't help address macro calls in built-in attributes.

# Unresolved questions

* This API allows for a first-pass solution to the problems listed in the [motivation](#motivation). Does it interfere with any known uses of proc macros? Does it prevent any existing techniques from working or cut off potential future ones?

* What sort of API do we need to be _possible_ (even as a third party library) for this idea to be ergonomic for macro authors?

* The attribute macro example above demonstrates that a macro can mark emitted tokens with previous or current macro generations. What should the 'tiebreaker' be? Some simple choices:
    * The order that macros are encountered by the compiler (presumably top-down within files, unclear across files).
    * The order that macros are marked (when a macro expands into some tokes marked with generation `N`, they get put in a queue after all the existing generation `N` macros).

* How does this proposal affect expansion within the _body_ of an attribute macro call? Currently builtin macros like `#[cfg]` are special-cased to expand before things like `#[derive]`; can we unify this behaviour under the new system?

* How does this handle inner attributes?

* How does this handle the explicit token arguments that are passed to declarative macros?
