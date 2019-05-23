- Feature Name: `eager_macro_expansion`
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

Currently, if a proc macro author defines a useful macro `useful!` but hasn't
exposed it as a token-manipulating function, and another proc macro author
wants to use `useful!` within their own proc macro, they can't: they can *emit
an invocation* of `useful!`, but they can't *inspect the result* of that
invocation. Eager expansion would allow this kind of macro-level code sharing.

# Detailed design

## Design constraints

The current behaviour of macro expansion has features which make macros
intuitive to use even in complicated cases, but which constrain what a
potential eager expansion API should look like. These mostly revolve around
_delayed definitions_. Consider this example:

```rust
macro mk_macro ($macro_name:ident) {
    macro $macro_name {}
}

hello!();

mk_macro!(hello);
```

The invocation of `hello!` and the invocation that defines `hello!`
(`mk_macro!(hello)`) could be anywhere in relation to each other within a
project. In order to make the behaviour in this case as unsurprising as
possible, Rust delays the attempted expansion of `hello!` until it has a
candidate definition - that is, the compiler defers expanding `hello!` until it
expands `mk_macro!`.

We can emphasise this "delayed definition" expansion behaviour with another
example:

```rust
macro id ($($input:tt)*) {
    $($input)*
}

id!(id!(id!(mk_macro!(hello))));

hello!();
```

Here, the invocation of `hello!` can't proceed until after _four other_ macro
expansions: the three invocations of `id!` that are "hiding" the invocation of
`mk_macro!`, and then the invocation of `mk_macro!` itself.

## A silly example
What does this constraint mean for our API design? Say we have a proc macro
that needs to eagerly expand its input, imaginatively named `my_eager_pm!`,
which is defined something like this:

```rust
#[proc_macro]
fn my_eager_pm(input: TokenStream) -> TokenStream {
    // This is the magic we need to add in this RFC.
    //                     vvvvvvvvvvvvvvvvvvvvvvv
    let expansion_result = somehow_expand_macro_in(input);
    let count = count_the_tokens_in(expansion_result);
    quote! {
        println!("Number of tokens in output = {}", #count);
        #expansion_result
    }.into()
}
```

The idea here is that if we have some invocation `foo!()` which expands into `a
b c` (three tokens), then `my_eager_pm!(foo!())` expands into:

```rust
// We can only get this number by expanding `foo!()` and
// looking at the result.
// -----------------------------------------v
println!("Number of tokens in output = {}", 3);

// The result of expanding `foo!()`.
a b c
```

Now, we can combine `my_eager_pm!` with the "delayed definition" example from
earlier:

```rust
my_eager_pm!(hello!());
mk_macro!(hello);
```

If we want to maintain the nice properties that we've shown for _non-eager_
delayed definitions, then it's obvious what we _want_ to happen:

1. We expand `mk_macro!(hello)`. Afterwards, the compiler sees a definition for
   `hello!`.
2. We expand `my_eager_pm!(hello!())`. As part of this, we expand `hello!()`.

How does the compiler know to expand `mk_macro!` before trying to expand
`my_eager_pm!`? We might be tempted to suggest simple rules like "always expand
declarative macros before procedural ones", but that doesn't work:

```rust
my_eager_pm!(hello!());
my_eager_pm!(mk_macro!(hello));
```

Now the compiler needs to figure out which of these two calls to `my_eager_pm!`
to expand first.

## Lazy eager expansion

Given that the compiler today is already doing all this work to figure out what
it can expand and when, why don't we let proc macros defer to it? If a proc
macro wants to expand an invocation `foo!()`, but the compiler doesn't have a
definition for `foo!` yet, why not have the proc macro just _wait_? We can do
that by providing something like this:

```rust
pub struct ExpansionBuilder(..);

impl ExpansionBuilder {
    pub fn from_tokens(tokens: TokenStream) -> Result<Self, ParseError>;
    pub fn expand(self) -> Future<Result<TokenStream, ExpansionError>>;
}
```

Using this, we would implement our hypothetical `my_eager_pm!` like this:

```rust
#[proc_macro]
fn my_eager_pm(input: TokenStream) -> TokenStream {
    let expansion_result = ExpansionBuilder::from_tokens(input)
        .unwrap() // Ignore the parse error, if any.
        .somehow_wait_for_the_future_to_be_ready()
        .unwrap(); // Ignore the expansion error, if any.

    let count = count_the_tokens_in(expansion_result);
    quote! {
        println!("Number of tokens in output = {}", #count);
        #expansion_result
    }.into()
}
```

Now it doesn't matter what order the compiler tries to expand `my_eager_pm!`
invocations; if it tries to expand `my_eager_pm!(foo!())` before `foo!` is
defined, then the expansion will "pause" until such a definition appears. 

## Semantics

Currently, the compiler performs iterative expansion of invocations, keeping
track of unresolved expansions and revisiting them when it encounters new
definitions (this is the process that lets "delayed definitions" work, as
discussed [earlier](#design-constraints)).

In order to support the "lazy eager expansion" provided by the
`ExpansionBuilder` API, we make the compiler also track "waiting" expansions
(expansions started with `ExpansionBuilder::expand` but which contain
unresolved or unexpanded macro invocations).

We extend the existing rules for determining when a macro name is unresolvable
with an additional check for _deadlock_ among waiting expansions. This
handles cases like the following:

```rust
my_eager_macro!(mk_macro!(foo); bar!());
my_eager_macro!(mk_macro!(bar); foo!());
```

In this case, the eager expansions within each invocation of `my_eager_macro!`
depend on a definition that will only be available once the other invocation
has finished expanding. Since neither expansion can make progress, we should
report an error along the lines of: 

```
Error: can't resolve eager invocation of `bar!` because the definition is in an
       unexpandable macro
|   my_eager_macro!(mk_macro!(foo); bar!());
|                                   --------
|                                   Invocation of `bar!` occurs here.
|
|   my_eager_macro!(mk_macro!(bar); foo!());
|   ^^^^^^^^^^^^^^^ This macro can't be expanded because it needs
|                   to eagerly expand `foo!`, which is defined in an
|                   unexpandable macro.
|
|   my_eager_macro!(mk_macro!(bar); foo!());
|                   -------------- Definition of `bar` occurs here.
```

Notice that this error message would appear after as much expansion progress as
possible. In particular, the compiler would have expanded `mk_macro!(bar)` in
order to find the possible definition of `bar!`, and hence notice the deadlock.

## Path resolution

When eagerly expanding a macro, the invocation may use a _relative path_. For
example:

```rust
mod foo {
    my_eager_pm!(super::bar::baz!());
}

mod bar {
    macro baz () {};
}
```

When a macro invocation is eagerly expanded, to minimize surprise the path
should be resolved against the location of the surrounding invocation (in this
example, we would resolve the eager invocation `super::bar::baz!` against the
location `mod foo`, resulting in `mod bar::baz!`).

A future feature may allow expansions to be resolved relative to a different
path.

## Hygiene bending

Proc macros can use "hygiene bending" to modify the hygiene information on
tokens to "export" definitions to the invoking context. Normally, when a macro
creates a new identifier, the identifier comes with a "hygiene mark" which
prevents the usual macro hygiene issues. For example, if we have this
definition:

```rust
macro make_x() {
    let mut x = 0;
}
```

Then we can follow through a simple expansion. We start here:
```rust
make_x!() // Hygiene mark A
make_x!() // Hygiene mark A
x += 1;   // Hygiene mark A
```

Then after expanding `make_x!()`, we have:
```rust
let mut x = 0; // Hygiene mark B (new mark from expanding `make_x!`)
let mut x = 0; // Hygiene mark C (each expansion gets a new mark)
x += 1;        // Hygiene mark A (the original mark)
```

And of course the result is an error with the expected message "could not
resolve `x`".

Using the [`Span` API](https://doc.rust-lang.org/proc_macro/struct.Span.html)
on token streams, a proc macro can modify the hygiene marks on its output to
match that of the call site (in our example, this means we can define a proc
macro `export_x!` where the output tokens would also have hygiene mark A).

It's not clear how this should interact with eager expansion. Consider this
example:

```rust
my_eager_pm! {
    export_x!();
    x += 1;
}
x += 1;
```

When `export_x!` produces tokens with spans that match the "call site",
what should the call site be? Recalling the [definition of
`my_eager_pm!`](#a-silly-example), we expect the output to look something like
this:

```rust
println!("..."); // Hygiene mark B (new mark from `my_eager_pm!`) 
let mut x = 0;   // Hygiene mark X ("call site" mark for `export_x!`)
x += 1;          // Hygiene mark A (the original mark)
```

What should `X` be? What behaviour would be the least surprising in general?

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
implementation needs to account for the fact that a given macro invocations
could resolve to different definitions during expansion, if care isn't taken
(see [appendix B](#appendix-b)). In fact, expansions can be mutually-dependent
*between* nested eager macros (see [appendix C](#appendix-c)).

A guiding principle here is that, as much as possible, the result of eager
expansion shouldn't depend on the *order* that macros are expanded. This makes
expansion resilient to changes in the compiler's expansion process, and avoids
unexpected and desirable behaviour like being source-order dependent.
Additionally, the existing macro expansion process *mostly* has this property
and we should aim to maintain it.

A correct but simple implementation should be forwards-compatible with the
behaviour described in the appendices (perhaps by producing an error whenever
such a situation is detected).

# Prior art
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

# Rationale and alternatives
The primary rationale is to make procedural and attribute macros work more
smoothly with other features of Rust - mainly other macros.

## Alternative: mutually-recursive macros
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
* What can we learn from other language's eager macro systems, e.g. Racket?

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
  allow the first invocation to continue with its expansion.
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
the situation where an eager macro "changes" the definition of a macro (by
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
Should expand into this (indented for clarity):
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
counterpart. That is, if we expand this invocation:
```rust
eager_append_hello! {
    macro foo() {};
    foo!();
    concat!("a", "b");
}
```
`eager_append_hello!` first expands the input using `ExpansionBuilder`, with the intermediate
result:
```rust
    macro foo() {};
    "ab";
```
It then wraps the expanded input in `append_hello!`, and returns the result:
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

<a id="appendix-b-intermediate-syntax"></a>
Before we continue, we're going to need some kind of notation for an expansion
that's not currently complete. Let's say that if an invocation of `foo!` is
waiting on the expansion of some tokens `a b c`, then we'll write that as:

```rust
waiting(foo!) {
    a b c
}
```

We'll let our notation nest: if `foo!` is waiting for some tokens to expand,
and those tokens include some other eager macro `bar!` which is in turn waiting
on some other tokens, then we'll write that as:

```rust
waiting(foo!) {
    a b c
    waiting(bar!) {
        x y z
    }
    l m n
}
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
    foo!();         // foo-outer
    waiting(eager_append_world!) {
        foo!();     // foo-middle
        eager_append_hello! {
            foo!(); // foo-inner
            macro foo() {};
        }
    }
    ```
* The compiler tries to expand the tokens that `eager_append_world!` is waiting
  on (these are the tokens inside the braces after `waiting`). The `foo!`
  invocations still can't be resolved, so the compiler expands
  `eager_append_hello!`. The result is:
    <a id="ambiguous-expansion-choices"></a>
    ```rust
    foo!();         // foo-outer
    waiting(eager_append_world!) {
        foo!();     // foo-middle
        waiting(eager_append_hello!) {
            foo!(); // foo-inner
            macro foo() {};
        }
    }
    ```

At this point, we have several choices. When we described the
[semantics](#semantics) of this new `ExpansionBuilder` API, we talked about
_delaying_ expansions until their definitions were available, but we never
discussed what to do in complicated situations like this, where there are
several candidate expansions within several waiting eager expansions.

As far as the compiler can tell, there are three invocations of `foo!` (the
ones labelled `foo-outer`, `foo-middle`, and `foo-inner`), and there's a
perfectly good definition `macro foo()` for us to use.

### Outside-in
* Say we expand the invocations in this order: `foo-outer`, `foo-middle`,
  `foo-inner`.  Using the "currently available" definition of `foo!`, these all
  become empty token streams and the result is:
    ```rust
    waiting(eager_append_world!) {
        waiting(eager_append_hello!) {
            macro foo() {};
        }
    }
    ```
* Now that `eager_append_hello!` has no more expansions that it needs to wait
  for, it can make progress. It does what we [described
  earlier](#eager-append-definition), and wraps its expanded input with
  `append_hello!`:
    ```rust
    waiting(eager_append_world!) {
        append_hello! {
            macro foo() {};
        }
    }
    ```
* The next expansions are `append_hello!` within `eager_append_world!`, then
  then `append_world!`, and the result is:
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
  point, `eager_append_hello!` can make progress and wrap the result in
  `append_hello!`. If it does so, the result is:
    ```rust
    foo!();    // foo-outer
    waiting(eager_append_world!) {
        foo!() // foo-middle
        append_hello! {
            macro foo() {};
        }
    }
    ```
* At this point, the definition of `foo!` is 'hidden' by `append_hello!`, so neither
  `foo-outer` nor `foo-middle` can be resolved. The next expansion is `append_hello!`,
  and the result is:
    ```rust
    foo!();    // foo-outer
    waiting(eager_append_world!) {
        foo!() // foo-middle
        macro foo() {
            println!("hello");
        };
    }
    ```
* Here, we have a similar choice to make between expanding `foo-outer` and
  `foo-middle`.  If we expand `foo-outer` with the 'current' definition of
  `foo!`, it becomes `println!("hello");`. Instead, we'll continue 'inside-out'
  and fully expand `foo-middle` next.  For simplicity, we'll write the result
  of expanding `println!("hello");` as `<println!("hello");>`. The result is:
    ```rust
    foo!();    // foo-outer
    waiting(eager_append_world!) {
        <println!("hello")>;
        macro foo() {
            println!("hello");
        };
    }
    ```
* `eager_append_world!` is ready to make progress, so we do that:
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
eager expansion is potentially recursive, and involves expanding the 'leaves'
before backtracking, hints that we should favour the 'inside-out' expansion
order.

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
`eager_append_world!`](#eager-append-definition) from appendix B. We're also
going to re-use our makeshift syntax for representing [incomplete
expansions](#appendix-b-intermediate-syntax).

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
We can skip ahead to the case where both of the eager macros are `waiting` to
make progress:
```rust
waiting(eager_append_world!) {
    waiting(eager_append_hello!) {
        id!(macro foo() {}); // id-inner
        bar!();              // bar-inner
    };
    id!(macro bar() {});     // id-outer
    foo!();                  // foo-inner
};
foo!();                      // foo-outer
bar!();                      // bar-outer
```

Hopefully you can convince yourself that there's no way for
`eager_append_hello!` to finish expansion without expanding `id-outer` within
`eager_append_world!`, and there's no way for `eager_append_world!` to finish
expansion without expanding `id-inner` within `eager_append_hello!`; this means
we can't *just* use the 'inside-out' expansion order that we looked at in
[appendix B](#appendix-b).

## A solution
A few simple rules let us make progress in this example while recovering the
desired 'inside-out' behaviour discussed [earlier](#inside-out).

Assume that the compiler associates each `ExpansionBuilder::expand` with an
*expansion context* which tracks macro invocations and definitions that appear
within the expanding tokens. Additionally, assume that these form a tree: if an
eager macro expands another eager macro, as above, the 'inner' definition scope
is a child of the outer definition scope (which is a child of some global
'root' scope).

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
        eager_append_world = {
            Definitions = [],
            Invocations = [
                "id-outer",
                "foo-inner",
            ],
            Child-Contexts = {
                eager_append_hello = {
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
* The expansion associated with a call to `ExpansionBuilder::expand` can only
  use a definition that appears in its own context, or its parent context (or
  grandparent, etc).
* The expansion associated with a call to `ExpansionBuilder::expand` is
  'complete' once its context has no invocations left. At that point the
  resulting tokens are returned via the pending `Future` and the context is
  destroyed.

Notice that, under this rule, both `id-outer` and `id-inner` are eligible for
expansion. After we expand them, our tokens will look like this:
```rust
waiting(eager_append_world!) {
    waiting(eager_append_hello!) {
        macro foo() {};
        bar!();         // bar-inner
    };
    macro bar() {};
    foo!();             // foo-inner
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
        eager_append_world = {
            Definitions = [
#               A new definition!
#               vvvvvvvvvvv
                "macro bar",
            ],
            Invocations = [
                "foo-inner",
            ],
            Child-Contexts = {
                eager_append_hello = {
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
waiting(eager_append_world!) {
    waiting(eager_append_hello!) {
        macro foo() {};
    };
    macro bar() {};
    foo!();             // foo-inner
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
        eager_append_world = {
            Definitions = [
                "macro bar",
            ],
            Invocations = [
                "foo-inner",
            ],
            Child-Contexts = {
                eager_append_hello = {
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

Our second rule kicks in now that `eager_append_hello!` has no invocations. We
'complete' the expansion by returning the relevant tokens to the still-waiting
expansion of `eager_append_hello!` via the `Future` returned by
`ExpansionBuilder::expand`. Then `eager_append_hello!` wraps the resulting
tokens in `append_hello!`, resulting in this expansion state:
```rust
waiting(eager_append_world!) {
    append_hello! {
        macro foo() {};
    };
    macro bar() {};
    foo!();             // foo-inner
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
        eager_append_world = {
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
the compiler should error with something along the lines of the deadlock error
suggested in the section on [semantics](#semantics).

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
