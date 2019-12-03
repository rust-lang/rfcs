- Feature Name: `eager_macro_expansion`
- Start Date: 2018-01-26
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Expose an API for procedural macros to opt in to eager expansion. This will:
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
    struct X {
        my_field_definition_macro!(...)
    //  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Same with this one.
    }
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

# Guide-level explanation

We expose the following API in the `proc_macro` crate, to allow proc macro
authors to iteratively expand all macros within a token stream:

```rust
use proc_macro::{LexError, Diagnostic, TokenStream};

pub struct ExpansionBuilder { /* No public fields. */ }
pub enum ExpansionError {
    LexError(LexError),
    ParseError(Diagnostic),
    MissingDefinition(Diagnostic),
    /* What other errors do we want to report?
       What failures can the user reasonably handle? */
}

impl ExpansionBuilder {
    /// Creates a new macro expansion request to iteratively expand all macro
    /// invocations that occur in `tokens`.
    ///
    /// Expansion results will be interpolated within the input stream before
    /// being returned.
    ///
    /// `tokens` should parse as valid Rust -- for instance, as an item or
    /// expression.
    pub fn from_tokens(tokens: TokenStream) -> Self;

    /// Sends the expansion requeset to the compiler, then awaits the results of
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
use proc_macro::quote;

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

For example, this invocation of `expand!` should reproduce the intended
behaviour of our earlier `eager_stringify!(let x = ...)` example:

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

The current implementation of procedural macros is as a form of inter-process
communication: the compiler creates a new process that contains the proc macro
logic, then sends a request (a remote procedure call, or RPC) to that process to
return the result of expanding the proc macro with some input token stream.

This interaction works the other way as well: for example, if a proc macro wants
to access span information, it does so by sending a request to the compiler.
This RFC adds the `ExpansionBuilder` API as a way to construct a new kind of
request to the compiler -- a request to expand macro invocations in a
token stream.

## Why is expansion aysnchronous?

Depending on the order in which macros get expanded by the compiler, a proc
macro using the `ExpansionBuilder` API might try to expand a token stream
containing a macro that isn't defined, but _would_ be defined if some other
macro were expanded first. For example:

```rust
macro make_macro($name:ident) {
    macro $name () { "hello!" }
}

make_macro!(foo);

my_eager_macro!{ let x = foo!(); }
```

If `my_eager_macro!` tries to expand `foo!()` _after_ `make_macro!(foo)` is
expanded, all is well: the compiler will see the new definition of `macro foo`,
so when `my_eager_macro!` uses `ExpansionBuilder` to expand `foo!()`, the
compiler knows what to return. However, what should we do if the compiler tries
to expand `my_eager_macro!` _before_ expanding `make_macro!(foo)`? There are
several options:

* A: Only expand macros in a non-blocking order. This is hard, because the
  knowledge that `my_eager_macro!` depends on `foo!` being defined is only
  available once `my_eager_macro!` is executing. Similarly, we only know that
  `make_macro!` defines `foo!` after it has finished expanding.
* B: The compiler could indicate to `my_eager_macro!` that its expansion request
  can't be completed yet, due to a missing definition. This means
  `my_eager_macro!` needs to handle that outcome, preferably by indicating to
  the compiler that the compiler should retry the expansion of `my_eager_macro!`
  once a definition of `foo!` is available.
* C: The compiler could delay returning a complete expansion result until it is
  able to, while allowing `my_eager_macro!` to make as much progress as it can
  without the result.

This RFC goes with option C by making `expand` an `async fn`, since this
provides a clear indication to proc macro authors that they should consider and
handle this scenario. Additionally, this behaviour of `expand` -- delaying the
return of expansion results until all the necessary definitions are available --
is probably the outcome that most authors would opt-in to if given the choice
via option B.

## Why take in a token stream?

We could imagine an alternative `ExpansionBuilder` API which required the user
to construct a _single_ macro invocation at a time, perhaps by exposing
constructors like this:

```rust
use syn::{Macro, Attribute};

impl ExpansionBuilder {
    pub fn bang_macro(macro: Macro) -> Self;
    pub fn attribute_macro(
        macro: Attribute,
        body: TokenStream
    ) -> Self;
}
```

This would force proc macro authors to traverse their inputs, perform the
relevant expansion, and then interpolate the results. Presumably utilities would
show up in crates like `syn` to make this easier. However, this alternative API
_doesn't_ handle cases where the macro invocation uses local definitions or
relative paths. For example. how would a user of `bang_macro` use it to expand
the invocation of `bar!` in the following token stream?

```rust
quote!{
    mod foo {
        pub macro bar () {}
    }

    foo::bar!();
}
```

By contrast, the proposed `from_tokens` interface makes handling these cases the
responsibility of the compiler.

## Why use a builder pattern?

The builder pattern lets us start off with a fairly bare-bones API which then
becomes progressively more sophisticated as we learn what proc macro authors
need from an eager expansion API. For example:
* It isn't obvious how to treat requests to expand expressions from a proc
  macro that has been invoked in item position; we might need to add a new
  constructor `from_expr_tokens`.
* The proposed API only does complete, recursive expansion. Some proc macros
  might need to expand invocations "one level deep" in order to inspect
  intermediate results; the builder pattern lets us add that level of
  fine-grained control.
* The builder pattern also lets us deprecate methods which overreach or
  underperform. If it turns out that the reasons for [accepting a token
  stream](#why-take-in-a-token-stream) are offset by an unexpected increase in
  implementation complexity, we might backpedal and expose a more constrained
  API.

## Corner cases

### Attribute macros

We assume that for nested attribute macros the least surprising behaviour is
for them to be expanded "outside in". For example, if I have two eager
attribute macros `my_eager_foo` and `my_eager_bar`, then in this example
`my_eager_foo` would see the result of expanding `my_eager_bar`:

```rust
#[my_eager_foo]
#[my_eager_bar]
mod whatever { ... }
```

The situation is less clear for outer attributes. Which macro should be
expanded first in this case?

```rust
#[my_eager_foo]
mod whatever {
    #![my_eager_bar]
    ...
}
```

### Helper attributes

'Derive' attribute macros can define additional 'helper' attributes, used by
the invoker to annotate derive macro input. When expanding invocations, the
compiler must be careful not to try and expand these helper attributes as
though they were actual invocations.

Here is an example to justify why this is best dealt with by the compiler. Say
I have an eager derive macro `derive_foo` for the trait `Foo` with the helper
attribute `foo_helper`, and consider this invocation:

```rust
#[derive(Foo)]
struct S {
    #[some_other_eager_attr_macro]
    #[foo_helper]
    field: usize
}
```

When `derive_foo` eagerly expands `#[some_other_eager_attr_macro]`, that macro
in turn will try to expand the token stream `#[foo_helper] field: usize`. Two
things could go wrong here:

* If there is an attribute macro called `#[foo_helper]` in scope, it might get
  expanded. This is probably not the behaviour expected by the invoker of
  `#[derive(Foo)]`, nor of the author of `derive_foo`.
* If there isn't such a macro, the compiler might report a missing definition.

Both of these issues are handled by the compiler keeping track of the fact that
`#[foo_helper]` is being expanded "inside" a macro derive context, and leaving
the helper attribute in-place.

### Deadlock

Two eager macro expansions could depend on each other. For example, assume that
`my_eager_identity!` is a macro which expands its input, then returns the
result unchanged. Here are two invocations of `my_eager_identity!` which can't
proceed until the other finishes expanding:

```rust
my_eager_identity! {
    macro foo() {}
    bar!();
}

my_eager_identity! {
    macro bar() {}
    foo!();
}
```

The compiler can detect this kind of deadlock, and should report it to the
user. The error message should be similar to the standard "cannot find macro"
error message, but with more context about having occurred during an eager
expansion:

```
error: cannot find macro `bar` in this scope during eager expansion
  |
1 | my_eager_identity! {
  | ------------------- when eagerly expanding within this macro
...
3 |     bar!();
  |     ^^^ could not find a definition for this macro

error: cannot find macro `baz` in this scope during eager expansion
  |
6 | my_eager_identity! {
  | ------------------- when eagerly expanding within this macro
...
8 |     baz!();
  |     ^^^ could not find a definition for this macro
```

### Expansion context

### Name resolution

### Hygiene

### Expansion order
