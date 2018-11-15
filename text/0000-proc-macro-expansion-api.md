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
    fn my_proc_macro(tokens: TokenStream) -> TokenStream {
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
fn string_length(tokens: TokenStream) -> TokenStream {
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
    pub fn generation(&self) -> Option<usize>;
    pub fn set_generation(self, generation: usize) -> Self;
    pub fn increment_generation(self, count: usize) -> Self;
    pub fn into_tokens(self) -> TokenStream;
}
```

The constructor `from_tokens` takes in either a bang macro or attribute macro with arguments (`my_proc_macro!(some args)` or `#[my_attr_macro(some other args)]`).

The method `generation` lets you inspect the existing generation number (if any) of the input. This might be useful to figure out when a macro you've encountered in your tokens will be expanded, in order to ensure that some other macro expands before or after it.

The builder methods `set_generation` and `increment_generation` annotate the tokens passed in to tell the compiler to expand them at the appropriate generation (if the macro doesn't have a generation, `increment_generation` sets it to 1).

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
fn string_length(tokens: TokenStream) -> TokenStream {
    // Handle being given a macro...
    if let Ok(_: syn::Macro) = syn::parse(tokens) {
        // First, mark the macro tokens so that the compiler
        // will expand the macro at some point.
        let input_tokens =
                ExpansionBuilder::from_tokens(tokens)
                    .unwrap()
                    .increment_generation(0)
                    .into_tokens();

        // Here's the trick - in our expansion we _include ourselves_,
        // but delay our expansion until after the inner macro is expanded!
        let new_tokens = quote! {
            string_length!(#tokens)
        };
        return ExpansionBuilder::from_tokens(TokenStream::from(new_tokens))
                .unwrap()
                .increment_generation(1)
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
// New generation 2 macro tokens.
// vvvvvvvvvvvvvvv----------------------------v
   string_length!(concat!("hello, ", "world!"));
//                ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
// New generation 1 macro tokens. 
```

Now, in the next macro expansion loop, the compiler will find those generation-1 macro tokens and expand them. After that, the tokens look like this:

```rust
// Still generation 2 macro tokens.
// vvvvvvvvvvvvvvv---------------v
   string_length!("hello, world!");
```

And now `string_length!` expands happily!

Obviously the above code is fairly verbose, but thankfully there are some utility functions. TODO: what do we want to ensure is available as part of a library?

# Reference-level explanation

The proposed additions to the proc macro API in `proc_macro` are outlined above in the [API overview](#macro-generation-api). Here we focus on technical challenges.

Currently, the compiler does actually perform something similar to the loop described in th section on [expansion order](#macro-generations-and-expansion-order). We could 'just' augment the step that identifies potential macro calls to also inspect the otherwise unstructured token trees within macro arguments.

This proposal requires that some tokens contain extra semantic information similar to the existing `Span` API. Since that API (and its existence) is in a state of flux, details on what this 'I am a macro call that you need to expand!' idea may need to wait until those have settled.

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

* How does this proposal affect expansion within the _body_ of an attribute macro call? Currently builtin macros like `#[cfg]` are special-cased to expand before things like `#[derive]`; can we unify this behaviour under the new system?
