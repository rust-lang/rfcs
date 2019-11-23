- Feature Name: `eager_macro_expansion`
- Start Date: 2018-01-26
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Expose an API for procedural macros to opt in to eager expansion. This will:
* Allow procedural and declarative macros to handle unexpanded macro calls that
  are passed as inputs,
* Allow macros to access the results of macro calls that they construct
  themselves,
* Enable macros to be used where the grammar currently forbids it.

# Motivation
[motivation]: #motivation

## Expanding macros in input 
[expanding-macros-in-input]: #expanding-macros-in-input

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
[interpolating-macros-in-output]: #interpolating-macros-in-output

Macros are currently not allowed in certain syntactic positions. Famously, they
aren't allowed in identifier position, which makes `concat_idents!` [almost
useless](https://github.com/rust-lang/rust/issues/29599). If macro authors have
access to eager expansion, they could eagerly expand `concat_idents!` and
interpolate the resulting token into their output.

# Guide-level explanationnn
[guide-level-explanation]: #guide-level-explanation

We expose the following API in the `proc_macro` crate, to allow proc macro
authors to iteratively expand all macros within a token stream:

```rust
pub struct ExpansionBuilder { /* No public fields. */ }
pub enum ExpansionError {
    LexError(LexError),
    ParseError(Diagnostic),
    /* What other errors do we want to report?
       What failures can the user reasonably handle? */
}

impl ExpansionBuilder {
    /// Creates a new macro expansion request to iteratively expand all macro
    /// invocations that occur in `tokens`.
    ///
    /// Expansion results will be interpolated within the input stream before
    /// being returned.
    pub fn from_tokens(tokens: TokenStream) -> Self;

    /// Sends the token stream to the compiler, then awaits the results of 
    /// expansion.
    ///
    /// The main causes for an expansion not completing right away are:
    /// - Procedural macros performing IO or complex analysis.
    /// - The input token stream referring to a macro that hasn't been defined
    ///   yet.
    pub async fn expand(self) -> Result<TokenStream, ExpansionError>;
}
```

## Simple examples

Here is an example showing how a proc macro can find out what the result of
`concat!("hello ", "world!")` is. We assume we have access to a function
`await_future<T>(impl Future<T>) -> T` which polls a future to completion and
returns the result.

```rust
let tokens = quote!{
    concat!("hello ", "world!")
};

let expansion = ExpansionBuilder::from_tokens(tokens);
let result = await_future(expansion.expand()).unwrap();

let expected_result = quote!("hello world!");
assert_eq!(result.into_string(), expected_result.into_string());
```

Here is an example showing what we mean by "interpolating" expansion results
into the input tokens.

```rust
let tokens = quote!{
    let x = concat!("hello ", "world!");
};

let expansion = ExpansionBuilder::from_tokens(tokens);
let result = await_future(expansion.expand()).unwrap();

// As we saw above, the invocation `concat!(...)` expands into the literal
// "hello world!". This literal gets interpolated into `tokens` at the same
// location as the expanded invocation.
let expected_result = quote!{
    let x = "hello world!";
};

assert_eq!(result.into_string(), expected_result.into_string());
```

Here is an example showing what we mean by "iteratively expanding" macros: if a
macro expands into a token stream which in turn contains macro invocations,
those invocations will also be expanded, and so on.

```rust
let tokens = quote!{
    let x = vec![concat!("hello, ", "world!"); 1]
};

let expansion = ExpansionBuilder::from_tokens(tokens);
let result = await_future(expansion.expand()).unwrap();

// `vec![concat!(...); 1]` expands into `std::vec::from_elem(concat!(...), n)`.
// Instead of returning this result, the compiler continues expanding the input.
//
// As before, the results of these expansions are interpolated into the same
// location as their invocations.
let expected_result = quote!{
    let x = std::vec::from_elem("hello world!", 1)
};

assert_eq!(result.into_string(), expected_result.into_string());
```

## A more complex example

We're going to show how we could write a procedural macro that could be used by
declarative macros for eager expansion.

As an example, say we want to create `eager_stringify!`, an eager version of
`stringify!`. If we write `stringify!(let x = concat!("hello ", "world!"))`, the
result is the string `let x = concat ! ("hello ", "world!")`, whereas we want
`eager_stringify!(let x = concat!("hello ", "world!"))` to become the string
`let x = "hello world!"`.

We could write `eager_stringify!` as a fairly straighforward proc macro using
`ExpansionBuilder`. However, since decl macros are much quicker and easier to
write and use, it would be nice to have a reusable "utility" which we could use
to define `eager_stringify!`.

Let's call our utility macro `expand!`. The idea is that users of `expand!` will
invoke it with:
- The tokens they want to expand.
- An identifier, to refer to the expansion result.
- The token stream they want to insert the result into, which can use the
  identifier to determine where to insert the result.

For example, this invocation of `expand!` should reproduce the intended behaviour of our
earlier `eager_stringify!(concat!(...))` example:

```rust
expand! {
    input = { let x = concat!("hello ", "world!"); },
    name = foo,
    output = { stringify!(#foo) }
};
```

Let's assume we already have the following:
- A function `parse_input(TokenStream) -> (TokenStream, Ident, TokenStream)`
  which parses the input to `expand!` and extracts the right-hand sides of
  `input`, `name`, and `output`.
- A function `interpolate(tokens: TokenStream, name: Ident, output:
  TokenStream) -> TokenStream` which looks for instances of the token sequence
  `#$name` inside `output` and replaces them with `tokens`, returning the
  result. For example, `interpolate(quote!(a + b), foo, quote!([#foo, #bar]))`
  should return `quote!([a + b, #bar])`.

Then we can implement `expand!` as a proc macro:

```rust
#[proc_macro]
pub fn expand(input: TokenStream) -> TokenStream {
    let (input, name, output) = parse_input(input);

    let expansion = ExpansionBuilder::from_tokens(input);
    let result = await_future(expansion.expand()).unwrap();

    return interpolate(result, name, output);
}
```

Finally, we can implement `eager_stringify!` as a decl macro:

```rust
pub macro eager_stringify($($inputs:tt)*) {
    expand! {
        input = { $($inputs)* },
        name = foo,
        output = { stringify!(#foo) }
    }
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Why use a builder pattern?

## Why is `expand` aysnchronous?

## Why take in a token stream?

## Corner cases

### Name resolution

### Expansion context

### Expansion order
