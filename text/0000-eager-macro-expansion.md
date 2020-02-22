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
    /// Creates a new macro expansion request to expand macro invocations that
    /// occur in `tokens`.
    ///
    /// Expansion results will be interpolated within the input stream before
    /// being returned.
    ///
    /// By default, macro invocations in `tokens` will have their expansion
    /// results recursively expanded, until there are no invocations left to
    /// expand. To change this, use the `set_max_depth` method on the returned
    /// instance of `ExpansionBuilder`.
    ///
    /// `tokens` should parse as valid Rust -- for instance, as an item or
    /// expression.
    pub fn from_tokens(tokens: TokenStream) -> Self;

    /// Sends the expansion request to the compiler, then awaits the results of
    /// expansion.
    ///
    /// The returned future will be ready unless expansion requires expanding a
    /// procedural macro or a macro that hasn't been defined yet but might be
    /// after further expansion. In those cases, the returned future will be
    /// woken once all the required expansions have completed.
    pub async fn expand(self) -> Result<TokenStream, ExpansionError>;

    /// Sets the maximum depth of expansion. For example, if the depth is 2,
    /// then the result of expanding the following tokens:
    /// ```rust
    /// {
    ///     vec![vec![vec![17; 1]; 2]; 3];
    ///     concat!("a", "b");
    /// }
    /// ```
    /// Will be the following tokens:
    /// ```rust
    /// {
    ///     std::vec::from_elem(std::vec::from_elem(vec![17; 1]. 2), 3);
    ///     "ab";
    /// }
    /// ```
    ///
    /// Notice that, since the innermost invocation of `vec!` is "inside" two
    /// other invocatoins of `vec!`, it is left unexpanded.
    ///
    /// If `depth` is 0, all macro invocations in the input will be recursively
    /// expanded to completion; this is the default behaviour for an instance
    /// of `ExpansionBuilder` created with `ExpansionBuilder::from_tokens`. 
    pub fn set_max_depth(&mut self, depth: usize) -> &mut self;
}
```

In addition, we allow proc macros to be `async` so that macro authors can more
easily make use of `ExpansionBuilder::expand`.

## Simple examples

Here is an example showing how a proc macro can find out what the result of
`concat!("hello ", "world!")` is.

```rust
use proc_macro::quote;

let tokens = quote!{
    concat!("hello ", "world!")
};

let expansion = ExpansionBuilder::from_tokens(tokens);
let result = expansion.expand().await.unwrap();

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
let result = expansion.expand().await.unwrap();

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
    let x = vec![concat!("hello ", "world!"); 1]
};

let expansion = ExpansionBuilder::from_tokens(tokens);
let result = expansion.expand().await.unwrap();

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
`stringify!`. Remember that `stringify!` turns the tokens in its input into
strings and concatenates them:
```rust
assert_eq!(
    stringify!(let x = concat!("hello ", "world!")),
    r#"let x = concat ! ("hello ", "world!")")
```
We want `eager_stringify!` to behave similarly, but to expand any macros it
sees in its input before concatenating the resulting tokens:
```rust
assert_eq!(
    eager_stringify!(let x = concat!("hello ", "world!")),
    r#"let x = "hello world!""#)
```
As an aside, this means `eager_stringify!` needs to be able to parse its input.

We could write `eager_stringify!` as a fairly straightforward proc macro using
`ExpansionBuilder`. However, since decl macros are much quicker and easier to
write and use, it would be nice to have a reusable "utility" macro which we
could use to define `eager_stringify!`.

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
  result. For example, the token stream returned by:
  ```rust
  interpolate(quote!(a + b), foo, quote!([#foo, #bar]))
  ```
  Should be the same token stream as:
  ```rust
  quote!([a + b, #bar])
  ```

Then we can implement `expand!` as a proc macro:

```rust
#[proc_macro]
pub async fn expand(input: TokenStream) -> TokenStream {
    let (input, name, output) = parse_input(input);

    let expansion = ExpansionBuilder::from_tokens(input);
    let result = expansion.expand().await.unwrap();

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
  `#[derive(Foo)]` or the author of `derive_foo`.
* If there isn't such a macro, the compiler might report a missing definition.

Both of these issues are handled by the compiler keeping track of the fact that
`#[some_other_eager_attr_macro]` is being expanded "inside" a macro derive
context, and leaving the helper attribute `#[foo_helper]` in-place.

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

### Hygiene

Eager expansion is orthogonal to macro hygiene. Hygiene information is
associated with tokens in token streams, and fresh hygiene contexts will be
automatically generated by the compiler for iteratively eagerly expanded
macros.

### Expansion order

The compiler makes no guarantees about the order in which procedural macros get
expanded, except that eager expansions which refer to an undefined macro cannot
be expanded until a definition appears. This adds to the long list of reasons
why a macro author or user shouldn't rely on the order of expansion when
reasoning about side-effects.

### Defining 'depth'

The method `ExpansionBuilder::set_max_depth` determines how many "layers" of
expansion will be performed by the compiler. The intent is that this will allow
proc macro authors to expand other proc macros for their side effects, as well
as "incrementally" expand decl macros to see their intermediate states.

The current "layer" of macro invocations are all the invocations that show up
in the AST. For example, in this input:

```rust
concat!("a", "b");

do_twice!(vec![17; 1]);

macro do_twice($($input:tt)*) {
    $($input)* ; $($input)*
}
```

The invocations of `concat!` and `do_nothing!` appear in the parsed AST,
whereas the invocation of `vec!` does not; the arguments to macros are always
opaque token streams.

After we expand all the macros in the current layer, we get this output:
```rust
"ab";

vec![17; 1]; vec![17; 1];

macro do_twice($($input:tt)*) {
    $($input)* ; $($input)*
}
```
Now there are two invocations of `vec!` in the current layer.

With this example in mind, we can more clearly describe `set_max_depth` as
specifying how many times to iteratively expand the current layer of
invocations.

# Design Rationale

## Why is expansion asynchronous?

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
relative paths. For example, how would a user of `bang_macro` use it to expand
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

## Prior art

Rust's macro system is heavily influenced by the syntax metaprogramming systems
of languages like Lisp, Scheme, and Racket (see discussion on the [Rust
subreddit](https://old.reddit.com/r/rust/comments/azlqnj/prior_art_for_rusts_macros/)).

In particular, Racket has very similar semantics in terms of hygiene, allowing
'use before define', and allowing macros to define macros. As an example of all
of these, the rough equivalent of this Rust code:

```rust
foo!(hello);
foo!((hello, world!));
mk_macro!(foo);

macro mk_macro($name:ident) {
    macro $name ($arg:tt) {
        println!("mk_macro: {}: {}",
            stringify!($name), stringify!($arg));
    }
}
```

Is this Racket code:

```racket
(let ()
    (foo hello)
    (foo (hello, world!))
    (mk_macro foo))

(define-syntax-rule
    (mk_macro name)
    (define-syntax-rule
        (name arg)
        (printf "mk_macro: ~a: ~a\n" 'name 'arg)))
```

And both of them print out (modulo some odd spacing from `stringify!`):

```
mk_macro: foo: hello
mk_macro: foo: (hello, world!)
```

Looking at the API that Racket exposes to offer [eager
expansion](https://docs.racket-lang.org/reference/stxtrans.html#%28def._%28%28quote._~23~25kernel%29._local-expand%29%29)
(alongside similar functions on that page), we see the following:
* Eager macros are essentially procedural macros that call one of the expansion
  methods.
* These expansion methods perform a 'best effort' expansion of their input
  (they don't produce an error if a macro isn't in scope, they just don't
  expand it).
* It's not clear how this system handles definitions introduced by eager
  expansion. Some
  [parts](https://docs.racket-lang.org/reference/stxtrans.html#%28def._%28%28quote._~23~25kernel%29._syntax-local-make-definition-context%29%29)
  of the API suggest that manual syntax context manipulation is involved.

Overall, it's not obvious that a straightforward translation of Racket's eager
macros is desirable or achievable (although it could provide inspiration for a
more fleshed-out procedural macro API). Future work should include identifying
Racket equivalents of the examples in this RFC to confirm this.

# Drawbacks

* Requires authors to opt-in to expansion, rather than somehow providing an
  ecosystem-wide solution similar to the one proposed by petrochenkov for
  [macros in inert
  attributes](https://internals.rust-lang.org/t/macro-expansion-points-in-attributes/11455).
* Exposes more of the compiler internals as an eventually stable API, which may
  make alternative compiler implementations more complicated.

# Alternatives

## Third-party expansion libraries

We could encourage the creation of a 'macros for macro authors' crate with
implementations of common macros (for instance, those in the standard library)
and make it clear that macro support isn't guaranteed for arbitrary macro calls
passed in to proc macros. This feels unsatisfying, since it fractures the macro
ecosystem and leads to very indirect unexpected behaviour (for instance, one
proc macro may use a different macro expansion library than another, and they
might return different results). This also doesn't help address macro calls in
built-in attributes.

## Global eager expansion

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

## Eager expansion invocation syntax

[RFC 1628](https://github.com/rust-lang/rfcs/pull/1628) proposes adding an
alternative invocation syntax to explicitly make the invocation eager (the
proposal text suggests `foo$!(...)`). The lang team couldn't reach
[consensus](https://github.com/rust-lang/rfcs/pull/1628#issuecomment-415617835)
on the design.

In addition to the issues discussed in RFC 1628, any proposal which marks
macros as eager 'in-line' with the invocation runs into a similar issue to the
[global eager expansion](#global-eager-expansion) suggestion, which
is that it bans certain token patterns from macro inputs.

Additionally, special invocation syntax makes macro *output* sensitive to the
invocation grammar: a macro might need to somehow 'escape' `$!` in its output
to prevent the compiler from trying to treat the surrounding tokens as an
invocation. This adds an unexpected and unnecessary burden on macro authors.

# Unresolved questions

These are design questions that would be best investigated while implementing
the proposed interface, as well as afterwards with feedback from users:
* Are there any corner-cases concerning attribute macros that aren't covered by
  treating them as two-argument proc-macros?
* How do we handle outer attributes?
* What can we learn from the eager macro systems of other languages, e.g.
  Racket?
