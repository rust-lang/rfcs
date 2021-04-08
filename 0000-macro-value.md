- Feature Name: `macro_value`
- Start Date: 2021-03-27
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Add a new macro_rules matcher, `$name:value`, with identical semantics to that of a function capture.

# Motivation
[motivation]: #motivation

Value arguments to function-like macros are tricky to deal with.
While macro_rules macros don't suffer from the common and most egregious pitfalls of C-style preprocessor macros,
such as misnested brackets and operator precedence,
using an `$:expr` capture more than once still evaluates the expression more than once,
duplicating side effects.

Additionally,
we have the additional wrinkle of the lifetime and drop timing of temporaries complicating matters further,
if your intent is to write a macro invocation with equivalent-to-function-call semantics.
Suffice to say,
`let arg = $arg;` has the incorrect drop behavior,
and the current best practice is to instead expand to

```rust
match ( $arg0, $arg1, ) {
    ( arg0, arg1, ) => { /* macro body */ }
}
```

instead.
We can simplify this and make getting the correct behavior easier on macro authors.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

(in a section explaining `macro_rules` matchers:)

While `$:expr` is good for capturing an expression and copying that expression into the macro-expanded code,
it does exactly that: it duplicates the captured expression to every expansion of the capture.
If, for example, you wrote the trivial `min!` macro,

```rust
macro_rules! min {
    ( $a:expr, $b:expr ) => {
        if $a <= $b { $a } else { $b }
    };
}
```

then both `$a` and `$b` are evaluated twice,
once at each expansion point,
as opposed to a single time,
as would be the case if [`min` were a function](https://doc.rust-lang.org/std/cmp/fn.min.html).
If you want the arguments to the macro to be evaluated a single time,
as if they were simple function arguments,
you can use the `$:value` matcher:

```rust
macro_rules! min {
    ( $a:value, $b:value ) => {
        if $a <= $b { $a } else { $b }
    };
}
```

This time,
`$a` and `$b` are evaluated a single time upon invoking the macro,
and each expansion of the capture refers to the same value,
just like function arguments.

(no need to mention temporary lifetimes in the guide.)

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

A new macro matching mode,
`$:value`,
is added.
It captures the same grammar,
has the same follow set,
and can be expanded in the same positions as `$:expr`.

A `macro_rules` macro capturing an expression as `$:value` can only be used in expression position,
not any other position
(item, type, etc.).
As such,
extra information is provided to the compiler that it MAY use for nicer error messages.
(When expanding an expression-position-only macro in item position,
 the current 1.51 rustc says
 "the usage of `mac!` is _likely_ invalid in item context"
 (emphasis mine),
 which could be strengthened if all macro arms capture `$:value`.)

For a given capture `$name:value`,
the captured expression is evaluated a single time upon entry into the macro expansion,
whether `$name` is mentioned in the macro expansion zero, one, or any number times.
Every expansion of `$name` within the macro expansion refers to the name of the temporary where the captured expression was evaluated.
If more than one `$:value` capture is present,
they are evaluated from left to right.
The intent is that this has identical semantics to that of a function argument capture.

A compiler MAY implement this by expanding to a `match` expression,
similar to the following:

```rust
macro_rules! mac! {
    /* other arms */
    ( /* other captures */ $name:value /* other captures */ ) => {
        /* macro body */
    };
    /* other arms */
}
// "desugars" to
macro_rules! mac! {
    /* other arms */
    ( /* other captures */ $name:expr /* other captures */ ) => {
        match $name {
            name => {
                /* macro body, $name replaced with name (hygienically) */
            }
        }
    };
    /* other arms */
}
```

but the compiler is expected to also handle the case where `$:value` is inside of a macro repetition,
which cannot be directly implemented by just a desugaring of the `macro_rules!` invocation.

To illustrate (one of) the differences in lifetimes, consider temporary lifetime extension:

```rust
macro_rules! mlet {
    ( $e:expr ) => {{
        let e = $e;
        e
    }};
}

macro_rules! mmatch {
    ( $e:expr ) => {
        match $e {
            e => e,
        }
    };
}

fn mfn<T>(e: T) -> T {
    e
}

struct Wrap(&'static str);
impl Drop for Wrap {
    fn drop(&mut self) {}
}

fn a() { let _a = mlet!  (&Wrap("a")); } // borrowck error
fn b() { let _b = mmatch!(&Wrap("b")); } // compiles
fn c() { let _c = mfn    (&Wrap("c")); } // compiles
```

plus there are other differences with drop timing of temporaries,
especially with macros used in statement position rather than expression position.
Using `:value` gives the known behavior of a function argument,
rather than "however you used it."

# Drawbacks
[drawbacks]: #drawbacks

For one, `$:value` is another thing that has to be learned to write effective `macro_rules!` macros.
However, it replaces the `match` trick, so the author believes this comes out nuetral.

Of course, it expands the language, where a code solution is sufficient, if not elegant.

Additionally,
this would be the first `macro_rules!` macro where a capture isn't plain token (tree) substitution.
If the `match` trick is actually fully equivalent to function argument semantics,
(the author believes this to be true but hasn't exhaustively shown this,)
then tools such as `cargo expand` (`-Z unpretty`) can use it as the macro expansion.
But still, this moves `macro_rules!` away from just being token (tree) substitution,
and gives `macro_rules!` a superpower that isn't available to function-like proc macros.

# Alternatives
[alternatives]: #alternatives

Of course,
we could just not do this,
and `macro_rules!` authors can just use the `match` trick.
More importantly, though,
there are two other discussed features that could address the same problem space:

## `$:place`
[macro_place]: #macro-place

For exposition, we use
`macro_rules! m { ( $x:value ) => ( &$x ); }`
to explain functionality.

`$:value` as described above generates a new _named_ temporary for the captured value,
to match the behavior of function arguments exactly.
That is,
`m!(array[0])` would call `Deref::deref` and return a reference to a copy of the deref'd to value
(which would be dropped immediately, causing a borrowck error if bound)
exactly matching the behavior of a function, which would fail typeck.

An alternative semantic, which I call `$:place`,
captures the _place_ in this situation, not the _value_.
That is, `m!(array[0])` would call `Deref::deref` and return that reference directly.
If `m!` were defined as `=> ( &mut $x );`,
then `Deref::deref_mut` would be called.
If the capture is expanded only once,
this behaves identically to an `$:expr` capture,
except for the evaluation timing of side effects.

However, `$:place` adds a new concept to Rust,
that of capturing a _place_ directly.
This is entirely impossible in surface Rust today.
This form of capturing may be more intuitive to macro authors
(who are already used to and use similar behavior from `$:expr`).
However,
as this is much more complicated on the implementation side than a simple `$:value`,
and `$:value` offers most of the benefit of `$:place` without the extra implementation complexity,
`$:place` is just offered here as an alternative.

## `macro fn`
[macro_fn]: #macro-fn

Another possibility that's been discussed is `macro fn`.
Basically, these would be `fn`, and have the semantics of `fn`,
but be duck typed (like macros) and semantically copy/pasted into the calling scope.
This is basically the exact feature that "`macro_rules!` with function-like captures" is trying to serve,
except for one important thing: a `macro fn` is (potentially) still a `fn` in that it has one fixed airity,
and can't be overloaded like a macro can.
Basically, `macro fn` is asking for "macros 2.0,"
which is still desirable, but still a _long_ ways off.
`$:value` offers a small improvement in the status quo without adding a completely new system into the language.

# Rationale
[rationale]: #rationale

`$:value` simplifies the authoring of `macro_rules` macros,
as authors now no longer need to learn and remember to use the `match` trick to bind macro value arguments,
and instead can just use the `$:value` matcher to get function-argument semantics.
Thus, while adding to the semantics provided by the Rust compiler,
it reduces the needed complexity to write correct `macro_rules!`.

Additionally,
it is impossible to have a repetition of expr captures that has function-argument like drop timing through use of the `match` trick alone,
as it requires knowing the airity of the captures ahead of time to name each capture.
`$:value` directly unlocks properly and fully variadic macros that act like function calls with respect to temporary lifetimes.

# Prior art
[prior-art]: #prior-art

Kotlin's [`inline fn`](https://kotlinlang.org/docs/inline-functions.html) behave similarly to (typechecked) macros.
Semantically, `inline fn` is inlined into the call site,
allowing things like `return`/`break`/`continue`ing from the calling scope.
Otherwise, `inline fn` behaves semantically like a function call.

[POV-Ray's macros](http://www.povray.org/documentation/3.7.0/r3_3.html#r3_3_2_8_3)
are also interesting for comparison.

The author knows of no other languages with macro-like functionality
that isn't just textual or lexical replacement.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Would `$:value` or `$:place` be more desirable to have in the language?
  How much more complicated _would_ `$:place` be to implement?
- How exactly should `$:value`(/`$:place`) be treated by `cargo expand` (`-Z unpretty`)?
  - `$:value` can probably just use the `match` trick,
    reducing semantic loss to just hygiene/name clashes,
    which is already expected.
  - `$:place` is a big unknown,
    and probably the best it can do is a `$:value` match
    by-value, by-ref, or by-ref-mut as appropriate.
- Do we want to (can we) provide similar "correctness shortcuts" to proc macros,
  rather than leaving them to rely on the `match` trick?

# Future possibilities
[future-possibilities]: #future-possibilities

If `$:value` is accepted now, `$:place` could be accpeted later if deemed necessary.
`$:place` is usable in all the places `$:value` would be used
(and saves a semantic move/copy before optimization,)
but is much more complicated for its small additional benefit,
so `$:value` is preferred by this RFC.
Matching function semantics,
requiring `&mut $expr` at the call site,
isn't necessarily a bad thing,
and macro authors can still just accept `$:expr`.

[RFC #2442](https://github.com/rust-lang/rfcs/pull/2442) proposes `postfix.macros!()`,
which captures its receiver with `$:value` semantics.
Having `$:value` available in `macro_rules!` would smooth the on-ramp for explaining `postfix.macros!()`.
However, this RFC is _not_ about postfix macros, and stands on its own merit.
