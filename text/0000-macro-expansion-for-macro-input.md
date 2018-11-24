- Feature Name: Macro expansion for macro input
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

## Recursion in procedural macros

We're going to discuss a technique that doesn't get mentioned a lot when discussing procedural and attribute macros, which is _recursively calling macros_. If you've ever had a look at a fairly complicated declarative macro (otherwise known as "macros by example" that are defined with the `macro` keyword or `macro_rules!` special syntax), or had to implement one yourself, then you've probably encountered something like the recursion in [lazy-static](https://github.com/rust-lang-nursery/lazy-static.rs/blob/master/src/lib.rs). If you look at the implementation of the `lazy_static!` macro, you can see that it calls `__lazy_static_internal!`, which sometimes calls itself _and_ `lazy_static!`.

But recursion isn't just for declarative macros! Rust's macros are designed to be as flexible as possible for macro authors, which means the macro API is always pretty abstract: you take some tokens in, you put some tokens out. Sometimes, the easiest implementation of a procedural macro isn't to do all the work at once, but to do some of it now and the rest in another call to the same macro, after letting the compiler look at your intermediate tokens.

As an example, we're going to look at using recursive expansion to solve an issue you might encounter when you're writing a procedural macro: expanding macro calls in your input.

## Macro calls in macro input

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

Thankfully, there's a way to _tell_ Rust to treat some tokens as macros, and to expand them before trying to expand _your_ macro. But first, we need to understand how Rust finds and expands macros.

## Macro expansion and marking

Rust uses an iterative process to expand macros. The loop that Rust performs is roughly as follows:

1. Look for and expand any macros we can parse in expression, item, or attribute position.
    - Skip any macros that have explicitly marked macros in their arguments.
    - Are there any new macros we can parse and expand? Go back to step 1.
2. Look for and expand any explicitly marked macros where there are raw tokens (like inside the arguments to a proc macro or attribute macro).
    - Are there any new explicitly marked macros we can expand? Go back to step 2.
    - Otherwise, go back to step 1.

Other than some details about handing macros we can't resolve yet (maybe because they're defined by another macro expansion), that's it!

In order to explicitly mark a macro for the compiler to expand, we actually just mark the `!` or `#` token on the macro call. The compiler looks around the token for the other bits it needs.

In most cases, when you're writing a proc or attribute macro you don't really need that level of precision when marking macros. Instead, you just want to expand every macro in your input before continuing.

The `syn` crate has a utility attribute `#[syn::expand_input]` which converts a normal proc or attribute macro into one that does that expansion. For example, if we add `#[syn::expand_input]` to our `string_length` proc macro above, we get something like:


```rust
#[proc_macro]
pub fn string_length(tokens: TokenStream) -> TokenStream {
    if let Some(marked_tokens) = syn::find_and_mark_all_macros(&tokens) {
        return quote!(string_length!(#marked_tokens));
    }
    // Otherwise, continue as before.
    ...
}

```

Notice that in the `quote!` output, the _argument_ to the new call to `string_length!` is marked by `syn::find_and_mark_all_macros`, but the _new call itself_ is unmarked. Recalling the macro expansion process we outlined earlier, that means the arguments will all get expanded before `string_length!` gets expanded again.

# Reference-level explanation

Currently, the compiler does actually perform something similar to the loop described in the section on [expansion order](#macro-expansion-and-marking). We could augment the step that identifies potential macro calls to also inspect the otherwise unstructured token trees within macro arguments.

This proposal requires that some tokens contain extra semantic information similar to the existing `Span` API. Since that API (and its existence) is in a state of flux, details on what this 'I am a macro call that you need to expand!' idea may need to wait until those have settled.

## Identifying and parsing marked tokens

The parser may encounter a token stream when parsing a bang (proc or decl) macro, or in the arguments to an attribute macro, or in the body of an attribute macro.

When the parser encounters a marked `#` token, it's part of an attribute `#[...]` and so the parser can forward-parse all of the macro path, token arguments, and body, and add the call to the current expansion queue.

- In a minimal implementation we want to keep expansion result interpolation as simple as possible - this means avoiding enqueuing an expansion that expands _inside of_ another enqueued expansion.
- One solution is to recursively parse the input of marked macros until we find a marked macro with none in its input, adding only this innermost call to the expansion queue.

    This increases the amount of re-parsing (since after every expansion we're repeatedly parsing macro bodies looking for innermost calls) at the cost of fewer re-expansions (since each marked macro will only ever see its input after all marked macros have been expanded).

If a marked `#` token is part of an inner-attribute `#![...]` then the situation is similar: the parser can forward-parse the macro path and token arguments, and with a little work can forward-parse the body.

When the parser encounters a marked `!` token, it needs to forward-parse the token arguments, but also needs to _backtrack_ to parse the macro path. In a structured area of the grammar (such as in an attribute macro body or a structured decl macro) this would be fine, since we would already be parsing an expression or item and hence have the path ready. In an _unstructured_ area we would actually have to backtrack within the token stream and 'reverse parse' a path: is this an issue?

## Delayed resolution of unresolved macros

The existing expansion loop adds any currently unresolved macros to a _resolution_ queue. When re-parsing macro output, if any newly defined macros would allow those unresolved macros to be resolved, they get added to the current expansion queue. If there are unresolved macros but no macros to expand, the compiler reports the unresolvable definition.

The new expansion order described [above](#macro-expansion-and-marking) is designed to expand all marked macros as much as possible before trying to expand unmarked ones. We know that marked macros are always in token position, so expansion-eligible unmarked macros are the only way to introduce new macro definitions.

In the new order, we still accumulate unresolved macros (marked and unmarked), and we still remove them from the resolution queue to the relevant expansion queue whenever they get defined. The only difference is an extra error case, where a resolved unmarked macro has an unresolved marked macro in its input, and there are no unmarked macros to expand. In this case, the resolution queue still contains the unresolved marked macro, and so the compiler again reports the unresolvable definition.

## Handling non-macro attributes

There are plenty of attributes that are informative, rather than transformative (for instance, `#[repr(C)]` has no visible effect on the annotated struct, and never gets 'expanded away'). We don't want to force users of the macro-marking process to need a complete list of non-expanding or built-in attributes, so we ignore marked built-in attributes during expansion.

Using the 'expand innermost marks first' process described [earlier](#identifying-and-parsing-marked-tokens), we can guarantee that when a macro is expanded, every marked macro in its input has already been fully expanded. Hence, if a macro encounters marked attributes, it can infer that the attributes don't expand and should be preserved.

# Drawbacks

This proposal:

* Leads to frustrating corner-cases involving macro paths. For instance, consider the following:

    ```rust
    macro baz!(...);
    foo! {
        mod b {
            super::baz!();
        }
    }
    ```

    The caller of `foo!` probably imagines that `baz!` will be expanded within `mod b`, and so prepends the call with `super`. However, if `foo!` naively marks the call to `super::baz!`, then the path will fail to resolve because macro paths are resolved relative to the location of the call. Handling this would require the macro implementer to track the path offset of its expansion, which is doable but adds complexity.
    * For nested attribute macros, this shouldn't be an issue: the compiler parses a full expression or item and hence has all the path information it needs for resolution.

* Commits the compiler to a particular (but loose) macro expansion order, as well as a (limited) way for users to position themselves within that order. What future plans does this interfere with? What potentially unintuitive expansion-order effects might this expose?
    * Parallel expansion has been brought up as a future improvement. The above specified expansion order blocks macro expansion on the expansion of any 'inner' marked macros, but doesn't specify any other orderings. Is this flexible enough?
    * There are some benefits to committing specifically to the 'expand innermost marks first' process described [earlier](#identifying-and-parsing-marked-tokens). Is this too strong a commitment?

# Rationale and alternatives

The primary rationale is to make procedural and attribute macros work more smoothly with other features of Rust - mainly other macros.

Recalling the examples listed in the [motivation](#motivation) above, a few but not all situations of proc macros receiving unexpanded macro calls could be avoided by changing the general 'hands off' attitude towards proc macros and attribute macros, and more aggressively parse and expand their inputs. This effectively bans macro calls as part of the input grammar, which seems drastic, and wouldn't handle cases of indirection via token tree (`$x:tt`) parameters.

We could encourage the creation of a 'macros for macro authors' crate with implementations of common macros - for instance, those in the standard library - and make it clear that macro support isn't guaranteed for arbitrary macro calls passed in to proc macros. This feels unsatisfying, since it fractures the macro ecosystem and leads to very indirect unexpected behaviour (for instance, one proc macro may use a different macro expansion library than another, and they might return different results). This also doesn't help address macro calls in built-in attributes.

# Unresolved questions

* How does this proposal affect expansion within the _body_ of an attribute macro call? Currently builtin macros like `#[cfg]` are special-cased to expand before things like `#[derive]`; can we unify this behaviour under the new system?

* How to handle proc macro path parsing for marked `!` tokens.

* How to maintain forwards-compatibility with more semantic-aware tokens. For instance, in the future we might mark modules so that the compiler can do the path offset tracking discussed in the [drawbacks](#drawbacks).

* Is there a better way to inform users about non-expanding attributes than the implicit guarantee described [above](#handling-non-macro-attributes)? In particular, this requires us to commit to the 'innermost mark first' expansion order.
    * Should it be an _error_ for a macro to see an expandable marked macro in its input?
    * What are the ways for a user to provide a non-expanding attribute (like `proc_macro_derive`)? Does this guarantee work with those?
