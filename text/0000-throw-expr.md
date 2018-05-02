- Feature Name: `throw_expr`
- Start Date: 2018-04-30
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

Introduce diverging `throw expr` expressions, typed at `!`,
which either `break` to the closest `try { .. }` if there is one, or if not,
`return` from the `fn` or closure. The expression form `throw expr` is supported
on edition 2018 onwards. This also means that `throw` is reserved as a keyword.

# Motivation
[motivation]: #motivation

## Highlighting the *"unhappy path"*

[RFC 243]: https://github.com/rust-lang/rfcs/blob/master/text/0243-trait-based-exception-handling.md#-operator

[RFC 243] writes regarding the `?` operator that:

> The `?` operator itself is suggestive, syntactically lightweight enough
> to not be bothersome, and lets the reader determine at a glance where an
> exception may or may not be thrown.

This RFC takes the moral of that story to heart and agrees that you should be
able to *"determine at a glance where an exception may or may not be thrown"*.
However, currently, when you write logic such as:

```rust
if condition {
    return Err(Foo)
} else {
    return Ok(Bar)
}

// other stuff...
```

[indeed suggests]: https://github.com/rust-lang/rfcs/blob/master/text/0243-trait-based-exception-handling.md#throw-and-throws

the unhappy path is not particularly differentiated from the happy path in
terms of syntax. By introducing `throw`, as RFC 243 [indeed suggests],
we can improve readability with distinct syntax:

```rust
if condition {
    throw Foo
} else {
    return Ok(Bar)
}

// other stuff...
```

In this case, it is also syntactically terser and conveys semantic intent better.

As a hypothetical extension, this can be further improved upon the future
with `Ok`-wrapping `try fn`s. With those you could then write:

```rust
if condition {
    throw Foo
} else {
    return Bar
}

// other stuff...
```

## Terser code in generic contexts

The terser syntax discussed in the previous section matters even more when
writing in a generic setting where the resulting type may not be `Result<T, E>`.
If the `R: Try` is held generic, then we currently have to write:

```rust
if condition {
    return Try::from_error(Foo)
}

// other stuff...
```

By introducing `throw`, this can be made significantly terser:

```rust
if condition {
    throw Foo;
}

// other stuff...
```

## Uniform support for early unhappy returns to functions and `try { .. }`
[uniform-bail-macro]: #uniform-support-for-early-unhappy-returns-to-functions-and-try---

[`bail!`]: https://docs.rs/failure/0.1.1/src/failure/macros.rs.html#25-36

Consider the macro [`bail!`] in the [`failure`] crate. It is defined as:

```rust
macro_rules! bail {
    ($e:expr) => {
        return Err($crate::err_msg($e));
    };
    ($fmt:expr, $($arg:tt)+) => {
        return Err($crate::err_msg(format!($fmt, $($arg)+)));
    };
}
```

The use of `return Err` creates a problem. There's no way for `bail!` to
perform an early "return" to the closest `try { .. }` block.
To make this possible, we can change the definition of `bail!` to:

```rust
macro_rules! bail {
    ($e:expr) => {
        throw $crate::err_msg($e);
    };
    ($fmt:expr, $($arg:tt)+) => {
        throw $crate::err_msg(format!($fmt, $($arg)+));
    };
}
```

With this new definition, we've gained two abilities:
1. `bail!` now works with any `R: Try<Error = Error>` and not just `Result<T, E>`.
2. `bail!` can short circuit to the nearest `try { .. }` as well as to the
   nearest `fn` (if no enclosing `try { .. }` exists).

## The existence of `bail!` shows there's a need

As discussed in the previous section and in [the prior art][prior-art-failure],
the [`failure`] and [`error-chain`] each define a `bail!` macro which has the
same role as `throw` would. This shows that there's a need for `throw`.

## Increasing familiarity

As discussed in the [prior-art], a super majority of all programming languages
(N = 41, TIOBE = 60.361%, PYPL = 83.99%) have a construct for throwing / raising
exceptions.

Introducing `throw` increases familiarity and makes the language easier to learn
for programmers who for example know Java. This can both facilitate adoption of
Rust and make the language more readable for those programmers.

For multi-lingual folks, the cognitive dissonance when switching back and forth
between languages can also be reduced.

## Erasing dichotomies 

[erasing dichotomies]: https://doc.rust-lang.org/stable/book/second-edition/print.html#people-who-value-speed-and-stability

Part of what Rust has been about is [erasing dichotomies].
We say:
+ Performance without unsafety.
+ Concurrency without fear.
+ Speed and ergonomics.

We have also started erasing the dichotomy between the correctness benefits
of *errors as values* and the ergonomics benefits of implicit exceptions by
introducing `expr?` and `try { .. }`. By introducing `throw expr` we can
further erase parts of this dichotomy.

## The time to reserve is now

At the time of writing, a new Rust edition 2018 is being prepared.
The opportunity to reserve new keywords is now. 
The next opportunity will be years from now.
Therefore, postponing the reservation of `throw` as a keyword,
because the proposal is not exactly the final design we end up with,
would be a mistake, assuming we wish to introduce `throw` at some point.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This RFC introduces `throw` expressions. If before this RFC you wrote:

```rust
if condition {
    return Err(error)
}
```

you can instead now write:

```rust
if condition {
    throw error;
}
```

This also works for `try { .. }` expressions. If you write:

```rust
enum MyError { Foo, Bar }

fn foo() -> Result<(), MyError> {
    let foo: Result<(), MyError> = try {
        if true {
            throw MyError::Foo;
        }

        println!("after throw");
    };

    println!("after try");
    foo
}
```

then `"after try"` will be printed but `"after throw"` will not.

Note that if you change the return type of `foo` to `Result<(), OtherError>` and
instead write:
```rust
fn foo() -> Result<(), OtherError> {
    let foo: Result<(), MyError> = try {
        if true {
            throw OtherError::Bar;
        }
    };

    foo
}
```

then a type error will be raised instead of `throw`ing the `OtherError::Bar`
to the enclosing function `foo`. See the section on [exceptional syntax][exceptional_syntax] for a discussion on this.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Reserving a keyword

**A keyword `throw` is reserved in edition 2018.**
This implies that users must use the raw identifier `r#throw` when they want
to refer to `throw` in an edition-2015 crate from an edition-2018 crate.

## Grammar

We introduce the token `THROW` which lexes `"throw"`.

[`expr`]: https://github.com/rust-lang/rust/blob/79252ff4e25d82f9fe856cb66f127b79cdace163/src/grammar/parser-lalr.y#L1381

[`expr_nostruct`]: https://github.com/rust-lang/rust/blob/79252ff4e25d82f9fe856cb66f127b79cdace163/src/grammar/parser-lalr.y#L1507

[`nonblock_expr`]: https://github.com/rust-lang/rust/blob/79252ff4e25d82f9fe856cb66f127b79cdace163/src/grammar/parser-lalr.y#L1443

We modify the grammar of Rust by adding, to [`expr`], [`expr_nostruct`], and [`nonblock_expr`], the following (directly after `YIELD expr`):

```ebnf
| THROW expr
```

## Semantics

An expression of the form `throw expr`, where [`expr`] is a meta-variable for
some expression, is a diverging expression typed at `!`.

The semantics of `throw expr` is defined as:
`break 'to Try::from_error(expr)` where `'to` is the closest `try { .. }`
or the enclosing `fn` or closure if the `throw expr` is not within a `try { .. }`
expression.

## General properties

This property should hold:

1. `try { throw err; expr } ≡ try { throw err }`

Assuming `err` and the `?` operator performs no side effects, these should:

2. `¬(err ≡ ok) -> ¬(try { throw err; ok }? ≡ ok)`, conversely:
3. `err ≡ ok -> try { throw err; ok }? ≡ ok`

# Drawbacks
[drawbacks]: #drawbacks

## Erasing the syntactic component of *"errors as values"*

One of the oft touted benefits of Rust is that *"errors are just values"*.
This idea has two components:
+ The semantic - The important aspects of this are that:
    - You know from the type, such as `Result<T, E>` whether the exceptional
      case `E` can occur for an expression of type `expr: Result<T, E>`.
      Therefore, if you have an expression `expr: usize`,
      there can't be any exceptions there.
    - You know exactly what kind of errors there can be.
    - Errors must be handled.
+ The syntactic.
  
  In these case, what is considered important is that successes and errors
  values are **explicitly** wrapped.
  In the case of `Result<T, E>`, this is done with the data constructors
  `Ok(..)` and `Err(..)`.
  Pattern matching is also done **explicitly** with `if let` or `match`
  on the variants of `Result<T, E>`.

While this RFC does not change the semantic component as *"errors as values"*,
it does moves us even further away from the syntactic component.
This has been happening ever since the `try!(expr)` macro was introduced which
was then changed to `expr?` and thus even more syntactically lightweight.

The next development was to then introduce `try { .. }` (previously `catch {..}`).
The current (as of 2018-04-29) implementation of `try { .. }` performs
`Ok`-wrapping. This change removes the success side of the syntactic component.
Introducing `throw` expressions then removes the failure side.

For those who prefer to keep the syntactic component,
this RFC presents a step in the wrong direction.

A mitigating factor is that while these people will need to read `throw` in
other's code, they can still use `Err(..)` in their own projects.

## More association with exceptions

Beyond `try { .. }`, this RFC further associates Rust with exceptional
terminology. Some people take the view that since Rust does not use
exception handling in the common mechanism that languages such as Java use,
using exceptional terminology is misleading and bad for learning.

While we have considered the semantic differences between Rust and languages
like Java as a drawback, we have mostly considered the familiarity beneficial.

## Edition breakage

As always, when a new real keyword is introduced, there is some degree of breakage.
This is a drawback of this proposal. A mitigating factor is that the degree is
believed to be small.

## *"But what about the success case?"*

If we introduce `throw expr`, then an obvious question becomes:
\- *"How do you do an early return with Ok-wrapping?"*
Not being able to perform an early return for the success case could
be considered a drawback. Although that could also be considered letting
the perfect be the enemy of the good.

A strategy for introducing early-return on success with Ok-wrapping **could**
be to introduce `try fn` and / or to let `return` perform an early return
to the nearest `try { .. }` block and Ok-wrap.
In the latter case, the user loses the ability to `return` to the function.
This could be solved with `break 'fn expr`.
One problem with modifying what `return` means in `try { .. }` could be that
people are so used to `return` always returning from the current function
that it could become quite confusing.

# Rationale and alternatives
[alternatives]: #rationale-and-alternatives

## The keyword can't be contextual

Whatever the keyword is, it can't be contextual (unless a macro is used..).
To see why, let's define a unit struct named exactly as the keyword.

```rust
struct throw;
```

If we just want to throw a simple expression such as `throw 42;`,
then it all works out well.
If we however modify this slightly and wrap the expression `42` in a new scope,
we get `throw { 42 }`. This is still fine and unambiguous.

Let's now consider the expression `{}` which is typed at `()`,
which we see in this example:
```rust
let x: () = {};
```

If we instead write `throw {}`, that is equivalent to `throw ()`,
then this conflicts with the struct literal `throw {}`.
Therefore, while the probability of `throw {}` being written is low,
we still have an ambiguity and the keyword can't be contextual.

## The keyword shouldn't be contextual

[keyword policy]: https://paper.dropbox.com/doc/Keyword-policy-SmIMziXBzoQOEQmRgjJPm
[permalink]: https://gist.github.com/Centril/4c82c19b3cb02cc565622a37d1591785

A recent [keyword policy] ([permalink]), adopted by the language team,
decided that moving forward, keywords for new features in new editions
should be real keywords instead of being contextual. The main motivation
for this was to optimize for maintenance (and reduce technical debt).

## Choice of keyword
[choice of keyword]: #choice-of-keyword

We consider the following keywords:

+ `throw`
+ `raise`
+ `fail`
+ `die`
+ `error`

We dismiss `die` because we don't want to refer to killing the process.

A reason to reject `error` specifically is that `std::error` exists as a module,
`std::io::IntoInnerError::error` is also a function that would be broken by
introducing `error` as a keyword. This keyword is also the only one that is
used in the standard library as an identifier.

It could be considered a benefit of `fail` that it is close to the `failure`
crate's `bail!` macro. However; the RFC author considers this to mainly be
a drawback because it is easy to mistake one for the other.
Meanwhile `error` and `fail` are too tied to failure and error handling.
However, `fail` fits nicely with `try { .. }` as you might *"try but fail"*
at doing something in English.
The expression form `fail expr` should also be as immediately understandable
as `throw expr` would be.

Since the words `die`, `error`, and `fail` are much less commonly used than
the keywords `throw` and `raise` we move on to consider the latter two.

The main benefit of `throw` is that while it is familiar in languages with
exception handling, it is less indicative of exceptions. It makes sense to say
*"throw a ball"* and it works with the "re-throw" framing of the `?` operator.
One problem with `throw` however could be that if you can *throw* something then
there might be an expectation that you can *catch* something, and so it may
obligate us to introduce the `try { .. } catch { .. }` form which we may not
want to do.

In the RFC authors view, `raise` + a noun is used less often.
However, one benefit of `raise` is that while you can `throw` something
vertically and horizontally, you usually only `raise` something upwards.
This is good because if we consider nested `try { .. }` expressions or scopes,
then `raise` moves "up".

Considering crates named as the keyword we find that there isn't a crate named
`raise`, there is a crate named [`throw`](https://crates.io/crates/throw).
This crate would not be directly usable on the new edition,
but it has zero reverse dependencies.
With respect to `fail`, it does exist as a crate, which has reverse dependencies.
However, all of those transitive dependencies are written by the same author.

Searching for `raise` with sourcegraph indicates a very
small number of uses as an identifier.
For `fail`, we find slightly more hits than `raise` on sourcegraph.
Meanwhile, sourcegraph identifies 3 cases with `throw` where breakage would
occur which is fewer than for `raise` and `fail`.
To sum up, none of the considered keywords would result in much breakage.

Since both `raise` and `throw` are equally long (5 characters), the extent of
breakage is very low, and it is hard to make decisive arguments in either way,
we turn to popularity.
In the [summary][summary-of-data] of the prior art in other languages,
`throw` is more than twice as popular in languages with such a concept than
`raise`. `throw` also exists in approximately twice as many languages.
Therefore, if we go with popularity alone, we should chose `throw`,
and so we do that.

## A built-in macro

In one alternative design `throw expr` is a macro `throw!(expr)`.

### The case against it

This design is less good because:

+ It is less readable and ergonomic.
  In particular, a writer has to track the closing parenthesis `)`.

+ It is less consistent.

  We have moved away from introducing important functionality such as this
  via built-in macros.
  An example of this is the macro `try!` which later became dedicated syntax with
  the `?` operator. The macro then became deprecated.

  While the [async and `await` RFC](https://github.com/rust-lang/rfcs/pull/2394)
  does introduce `await!` as a macro under the Stroustrup rule,
  the RFC does introduce `await` as a keyword.
  This is indicative of that the final syntax will indeed not be `await!`
  Indeed, the RFC has this to say on the matter:
  > Though this RFC proposes that `await` be a built-in macro,
  > we'd prefer that some day it be a normal control flow construct.

+ It is less familiar.
  Most languages use the `throw expr` syntax instead of
  using `throw(expr)` and especially `throw!(expr)`.

+ It gives us freedom and peace of mind.
  By reserving a new keyword, we can use `throw` in other contexts.
  It is difficult to see what those other contexts would be right now,
  but we should expect the unexpected.

### The case for it

+ It breaks nothing.
  While `throw` as a keyword will cause some small degree of breakage,
  a `throw!` macro will break nothing and can therefore be implemented
  on edition 2015. However, we do want to encourage switching to edition 2018.

## Desugaring to a trait
[desugaring]: #desugaring-to-a-trait

The current specification of `throw expr`'s semantics is that it
works with the `Try` trait by wrapping `expr` as `Try::from_error(expr)`
and then short circuits to the closest `try { .. }` or `fn`.

While the short circuiting behaviour should not change, we can consider alternate
semantics of how the expression is converted into the partiality monad (such as
`Result<T, E>`) or container.

Let's consider `throw expr` desugaring to a dedicated trait. Two designs come
to mind in this space.

### The associated model

In this design, we use an associated type.
This means that there can only be one `Error` type implementer of `Throw`.

```rust
pub trait Throw {
    type Error;

    fn from_error(error: Self::Error) -> Self;
}

impl<T, E> Throw for Result<T, E> {
    type Error = E;

    fn from_error(error: E) -> Self {
        Err(error)
    }
}

struct NoneError;

impl<T> Throw for Option<T> {
    type Error = NoneError;

    fn from_error(error: NoneError) -> Self {
        None
    }
}
```

### The relational model

In this design, we instead use a type parameter.
This means that there can be many implementers of `Error` per implementer of
`Throw`. This model is more flexible.

```rust
pub trait Throw<Error> {
    fn from_error(error: Error) -> Self;
}

impl<T, E> Throw<E> for Result<T, E> {
    fn from_error(error: E) -> Self {
        Err(error)
    }
}

impl<T> Throw<()> for Option<T> {
    fn from_error(_: ()) -> Self {
        None
    }
}
```

### The desugaring of `throw;`
[bare_throw]: #the-desugaring-of-throw

In the case of `Option<T>`,
the error variant contains zero extra bits of information as we see from the
definition of `Option<T>`:

```rust
pub enum Option<T> {
    None, // We only know that there was nothing, but not the reason why.
    Some(T),
}
```

Therefore, it is redundant from a user's perspective to say `throw ();`.
This becomes especially problematic if you have to say `throw NoneError;`.
Instead, we would like to say `throw;` and let the compiler fill in `NoneError`.

While this is not part of the current proposal, to not lock in a specific design,
we could add that later. In this space, we consider three designs:

#### Using `Default`

In this design, we leverage the `Default` trait to fill in the error and desugar
`throw;` to `throw Default::default()`. This mechanism is flexible and works
with both `Option<T>` as well as `Result<T, E>` where you can fill in the
default error reason (assuming there is one).
This also works with both the associated and relational models discussed above.

This design leverages the fact that many types implement the `Default` trait
which is a good thing.

#### Using a dedicated `DefaultThrow` trait

We can define a trait `DefaultThrow`:

```rust
pub trait DefaultThrow {
    fn default_throw() -> Self;
}
```

The expression `throw` then uses `DefaultThrow::default_throw()` directly and
does not go via `Try` or the `Throw` traits discussed above.

The primary benefit of this design is that it is maximally flexible.
You don't even need to implement `Throw` and can simply implement `DefaultThrow`
for your exception carrying type.

This design is perhaps too flexible however.
The combination of `Throw` and `Default` is simpler to implement.
Since there may not be comparatively many types that implement `Throw`,
using `DefaultThrow` could still be worthwhile.

### Using `Throw<SpecialType>`

We could desugar `throw;` to `throw SpecialType;`
where `SpecialType` is some unit type, which could be `()`.
For this mechanism to work well for `Result<T, E>`, specialization is needed.
This design can further be augmented with `X: From<SpecialType>, Throw<X>`.

## `yield throw expr` as a special construct?
[yield_throw]: #yield-throw-expr-as-a-special-construct

Let's first consider the expression `return (return)`.
Since `return` is a diverging computation, the second `return` will never be reached.
Therefore, `return (return)` is semantically equivalent to just `return`.
A Rust compiler understands this and correctly warns us with:

```rust
warning: unreachable expression
```

Let's now consider `yield` and `throw`. There are two ways to nest them:

### `throw yield x` or `throw (yield x)`

If we consider this to be `return Err(yield x)`, then we have
that:
```rust
let y: () = yield x;
let r: Result<?T, ()> = Err(y);
return r
```

We see that `throw yield x` has reasonable and productive semantics,
even if its not readable, where you return `()` the error.

### `yield throw x` or `yield (throw x)`

If we simply see this as `yield (return Err(x))` then we get back the
`unreachable` warning from before. In this case, `yield throw x` is not useful
beyond `throw x` as the `yield` is unreachable. Therefore, we could repurpose
`yield throw x` as a special construct (and a separate AST node..).

### A special construct

The question naturally becomes:
Should we repurpose `yield throw x`, and if so, what would `yield throw x` mean?

One possible meaning is to desugar `yield throw x` simply as:
`yield Try::from_error(x)`. This could be useful if you were yielding
a sequence of results and wanted to yield the error case.

However, there may be code generation hazards for macros to differentiate
`yield throw x` and `yield (throw x)`. To some the behaviour may also be confusing.
Perhaps, reusing `throw` for yielding errors should be considered to stretch
the keyword `throw` too far since it does not leverage the intuition about `throw`
from other programming languages.

This RFC does not propose `yield throw x` to be introduced as a special
construct since the proposal is intentionally conservative to begin with.
However, it is worth considering at some later point.

## Doing nothing

As usual, we have the choice of doing nothing.
Some motivation for why we might want to do nothing is discussed in the section
on [drawbacks]. In particular, this proposal edges us ever closer to *exceptional
terminology* and erasing the syntactic component of *errors as values*.

# Prior art
[prior-art]: #prior-art

## This mechanism exists in many languages

[PYPL]: http://pypl.github.io/PYPL.html
[TIOBE]: https://www.tiobe.com/tiobe-index/

All of the languages listed below have either a built in expression or statement
form for the equivalent of `throw` expressions, or they have library functions
which are widely used in the community.

Note that this is not necessarily a complete list! It is only a best effort to
compile the prior art. The categorization of functional, c++-family languages
is also fuzzy. In some cases, such as in Haskell, more than one word is used.
When this is the case, it is noted and the duplication is discounted from
the total [TIOBE] and [PYPL] shares.

### Summary of data
[summary-of-data]: #summary-of-data

+ **There are 26 languages with `throw`,** while there are 14 languages with
`raise`.
+ According to [TIOBE]:
    + **`throw` takes up 48.005% of the share,** while `raise` takes up 11.701%.
    + **60.361% of the share belongs to a language with some form of keyword.**
+ According to [PYPL]:
    + **`throw` takes up 58.80% of the share,** while `raise` takes up 27.74%.
    + **83.99% of the share belongs to a language with some form of keyword.**
+ Other keywords are also noted below but do not have a notable share.

### `throw` (N = 26, TIOBE = 48.005%, PYPL = 58.80%)

#### Functional (N = 5, TIOBE = 0.604%, PYPL = 0.31%)

+ Haskell -
  [[Control.Exception]](http://hackage.haskell.org/package/base-4.11.1.0/docs/Control-Exception.html#v:throw),
  [[Control.Monad.Except, mtl]](http://hackage.haskell.org/package/mtl-2.2.2/docs/Control-Monad-Except.html#v:throwError)
    - *note:* these are library functions but it is counted since this is about
      familiarity and not whether a concept is built in or not.
- [purescript](https://github.com/purescript/purescript-exceptions/blob/740d3f9bca0d6635e3a8be43ea082962e937cd01/src/Control/Monad/Eff/Exception.purs#L85-L88)
+ [Elixir](https://elixir-lang.org/getting-started/try-catch-and-rescue.html)
    - *note:* both `throw` and `raise` exist.
+ [Erlang](http://erlang.org/doc/reference_manual/expressions.html#id84473)
+ [Clojure](https://clojuredocs.org/clojure.core/throw)

#### C++ family (N = 7, TIOBE = 28.754%, PYPL = 32.24%)

+ [C++](http://en.cppreference.com/w/cpp/language/throw)
+ [D](https://tour.dlang.org/tour/en/basics/exceptions)
+ [C#](https://docs.microsoft.com/en-us/dotnet/csharp/language-reference/keywords/throw)
+ [Java](https://docs.oracle.com/javase/tutorial/essential/exceptions/throwing.html)
+ [Scala](http://tutorials.jenkov.com/scala/exception-try-catch-finally.html#throw-exception)
+ [Kotlin](https://kotlinlang.org/docs/reference/exceptions.html)
+ [Groovy](http://groovy-lang.org/semantics.html)

#### Javascript family (N = 3, TIOBE = 3.717%, PYPL = 10.14%)

+ [JavaScript](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Statements/throw)
+ [TypeScript](https://www.typescriptlang.org/docs/handbook/release-notes/typescript-2-0.html)
+ [ActionScript](https://help.adobe.com/en_US/ActionScript/3.0_ProgrammingAS3/WS5b3ccc516d4fbf351e63e3d118a9b90204-7ed1.html#WS5b3ccc516d4fbf351e63e3d118a9b90204-7ec5)

#### SQL (N = 1, TIOBE = 0.400%, PYPL = Unlisted)

+ [Transact-SQL](https://docs.microsoft.com/en-us/sql/t-sql/language-elements/throw-transact-sql?view=sql-server-2017)

#### Other (N = 10, TIOBE = 14.530%, PYPL = 16.11%)

+ [Dart](https://www.dartlang.org/guides/language/language-tour#throw)
+ [PHP](http://php.net/manual/en/language.exceptions.php)
+ [Visual Basic](https://docs.microsoft.com/en-us/dotnet/visual-basic/language-reference/statements/throw-statement)
+ [Objective C](https://developer.apple.com/library/content/documentation/Cocoa/Conceptual/Exceptions/Tasks/RaisingExceptions.html#//apple_ref/doc/uid/20000058-BBCCFIBF)
    - *note:* both `@throw` and `raise` exist. Only counted once in total share.
+ [Swift](https://developer.apple.com/library/content/documentation/Swift/Conceptual/Swift_Programming_Language/ErrorHandling.html)
+ [Julia](https://docs.julialang.org/en/stable/manual/control-flow/#Exception-Handling-1)
+ [Powershell](https://docs.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_throw?view=powershell-6)
+ [Apex](https://developer.salesforce.com/page/An_Introduction_to_Exception_Handling)
+ [RPG](http://devnet.asna.com/documentation/Help102/AVR/_HTML/THROW.htm)
+ [SAS](http://support.sas.com/documentation/cdl/en/sclref/59578/HTML/default/viewer.htm#a001273995.htm)

### `raise` (N = 14, TIOBE = 11.701%, PYPL = 27.74%)

#### ML derivatives (N = 5, TIOBE = Unlisted, PYPL = Unlisted)

+ [OCaml](https://ocaml.org/learn/tutorials/error_handling.html)
+ [F#](https://docs.microsoft.com/en-us/dotnet/fsharp/language-reference/exception-handling/the-raise-function)
+ [Standard ML](https://learnxinyminutes.com/docs/standard-ml/)
+ [F*](https://www.fstar-lang.org/tutorial/)
+ [Idris](http://docs.idris-lang.org/en/latest/effects/simpleeff.html#exceptions)
    - *note:* see note for Haskell.

#### SQL (N = 2, TIOBE = 1.173%, PYPL = Unlisted)

+ [PL/SQL](https://docs.oracle.com/cd/B28359_01/appdev.111/b28370/raise_statement.htm#LNPLS01337)
+ [Postgres](https://www.postgresql.org/docs/9.3/static/plpgsql-errors-and-messages.html)

#### Other (N = 7, TIOBE = 11.701%, PYPL = 27.74%)

+ [Python](https://docs.python.org/3/tutorial/errors.html#raising-exceptions)
+ [Ruby](http://rubylearning.com/satishtalim/ruby_exceptions.html)
+ [Objective C](https://developer.apple.com/library/content/documentation/Cocoa/Conceptual/Exceptions/Tasks/RaisingExceptions.html#//apple_ref/doc/uid/20000058-BBCCFIBF)
+ [Delphi](http://www.delphibasics.co.uk/RTL.asp?Name=raise)
+ [Elixir](https://elixir-lang.org/getting-started/try-catch-and-rescue.html)
+ [Tcl](https://www.tcl.tk/man/tcl/TkCmd/raise.htm)
+ [ABAP](https://help.sap.com/doc/abapdocu_751_index_htm/7.51/en-US/abapraise_exception_class.htm)

### `fail` (N = 1, TIOBE = 0.221%, PYPL = 0.31%)

+ [Haskell](http://hackage.haskell.org/package/base-4.11.1.0/docs/Prelude.html#v:fail)
    - *note:* see previous note for Haskell. Only counted once in total share.

### `die` (N = 1, TIOBE = 1.527%, PYPL = 0.78%)

+ [Perl](http://perldoc.perl.org/functions/die.html)

### `error` (N = 2, TIOBE = 0.378%, PYPL = 0.37%)

+ [Koka](https://www.rise4fun.com/koka/tutorialcontent/guide#h21)
+ [Lua](http://www.lua.org/manual/5.3/manual.html#pdf-error)

## Crates: [`failure`] and [`error-chain`]
[prior-art-failure]: #crates-failure-and-error-chain

[`failure`]: https://docs.rs/failure/0.1.1/failure/
[`error-chain`]: https://docs.rs/error-chain/0.11.0/error_chain/

The [`failure`] crate has a macro [`bail!`] that plays the same role as `throw` would.
In essence, `bail!(x)` boils down to:

```rust
return Err(format_err!(x))
```

The [`error-chain`] crate also has a `bail!(expr)` macro which amounts to:

```rust
return Err(expr.into());
```

## *Paper: Exceptional Syntax*
[exceptional_syntax]: #paper:-exceptional-syntax

[Exceptional Syntax]: https://www.microsoft.com/en-us/research/wp-content/uploads/2016/02/exceptionalsyntax.pdf

[lambda calculus]: https://en.wikipedia.org/wiki/Lambda_calculus

[1]: #exceptional_syntax_jfp

In the paper *[Exceptional Syntax]* [[1]] a [lambda calculus] is discussed
where one can `raise` exceptions.

One interesting difference between the relationship of `try` and `throw` in
this proposal and the paper is the last rule in Fig. 1:

```
            M ↑ E
------------------------------  E ∉ H
(try x ⇐ M in P unless H) ↑ E
```

This rule means that if the expression `M` raises the exception `E`
but the handler in `H` does not handle the exception,
then `try x ⇐ M in P unless H` raises `E`.

We could optionally mimic the same behaviour in Rust with respect to
`try { .. }` and functions by saying that if `expr?` raises `e : E`
but `try { .. }` does not capture `E`, then `e` is instead raised to
the next `try { .. }` until one block is found which supports the
exception type `E`.

If no block supports it, meaning that the enclosing function does not support it,
then the type checker rejects the function and raises a type error.

For `throw e`, the nearest supporting `try { .. }`, or the enclosing function,
is chosen with the same logic as for `expr?`.

These semantics could be added after introducing a simpler version of the
`throw e` expression form since it would accept strictly more programs as
well typed. However, one drawback of the more complex semantics is potentially
that programs involving `throw expr` and `expr?` become harder to reason about.

<a name="exceptional_syntax_jfp">\[1\]</a>
Nick Benton and Andrew Kennedy. 2001. Exceptional syntax. <br/>
J. Funct. Program. 11, 4 (July 2001), 395-410.<br/>
DOI = <http://dx.doi.org/10.1017/S0956796801004099>

# Unresolved questions
[unresolved]: #unresolved-questions

The [choice of keyword] should be finalized during the RFC period.

Post RFC period and during stabilization,
we have the following unresolved question:

- How will the trait(s) look that backs up the `throw` operator?
  See the section on [desugaring] for a discussion on possibilities.

- Allow `throw;`?
  See the section on [the possible semantics of `throw;`][bare_throw] for
  a discussion on possibilities.

- Should `From` conversions be involved?

- Should `throw` only be permitted inside `try { .. }`?
  Doing this would have two main drawbacks:
  1. `throw` can't be used in normal functions and thus the usefulness of
     `throw` diminishes significantly.
  2. `throw` can't act as double-duty primitive for the `bail!` macro
     as discussed in the [motivation][uniform-bail-macro].

- What is the relationship between `throw` and `yield`?
  See the [rationale][yield_throw] for a discussion.

- Should `throw 'label expr` be supported?
  This would be akin to `break 'label expr` but use `Error`-wrapping.
  It is unclear how often the need for this would arise,
  but the consistency with `break` could be one benefit.

Answering many of these question, and more, will likely require another RFC
to finalize the design once we have more experience.
