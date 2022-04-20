- Feature Name: `macro_value`
- Start Date: 2021-03-27
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Add a new macro_rules matcher, `$name:value`, with similar semantics to that of a function argument.

# Motivation
[motivation]: #motivation

Value arguments to function-like macros are tricky to deal with.
While macro_rules macros don't suffer from the most common pitfalls of C-style preprocessor macros,
such as misnested brackets and operator precedence, using an `$:expr` capture more than once
still evaluates the expression more than once, duplicating side effects.

Additionally,
we have the additional wrinkle of the lifetime and drop timing of temporaries complicating matters further,
if your intent is to write a macro invocation with equivalent-to-function-call semantics.
Suffice to say, `let arg = $arg;` has the incorrect drop behavior,
and the current best practice is to expand to

```rust
match ( $arg0, $arg1, ) {
    ( arg0, arg1, ) => { /* macro body */ }
}
```

instead.
We can simplify this and make getting the correct behavior easier on macro authors.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

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

then both `$a` and `$b` are evaluated twice, once at each expansion point, as opposed to a single time,
as would be the case if [`min` were a function](https://doc.rust-lang.org/std/cmp/fn.min.html).
If you want the arguments to the macro to be evaluated a single time, as if they were simple function arguments,
you can use the `$:value` matcher:

```rust
macro_rules! min {
    ( $a:value, $b:value ) => {
        if $a <= $b { $a } else { $b }
    };
}
```

This time,
`$a` and `$b` are evaluated a single time when control flow enters the expanded macro,
and each expansion of the capture refers to the same value,
just like function arguments.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

A new macro matching mode, `$:value`, is added.
It captures the same grammar, has the same follow set,
and can be expanded in the same positions as `$:expr`.

A `macro_rules` macro capturing an expression as `$:value` can only be used in expression 
or expression statement position, not any other position (item, type, etc.).
As such, extra information is provided to the compiler that it MAY use for nicer error messages.
(When expanding an expression-position-only macro in item position,
 `rustc` 1.51 says "the usage of `mac!` is _likely_ invalid in item context" (emphasis mine),
 which could be strengthened if all macro arms capture `$:value`.)

The exact semantics of `$:value` capture is determined by use of `k#autoref`,
as introduced by [RFC #2442, "Simple postfix macros"](https://github.com/rust-lang/rfcs/pull/2442).
A `k#autoref` binding invokes the same compiler machinery currently used by closure captures
to determine whether to use the `ref`, `ref mut`, or by-value binding mode.

For a given capture `$name:value`,
the captured expression is evaluated a single time upon entry into the macro expansion,
whether `$name` is mentioned in the macro expansion zero, one, or any number times.
Whether the place value of `$name` is bound in `k#autoref` mode.

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
        match k#autoref $name {
            __name => {
                /* macro body, $name hygienically replaced with __name */
            }
        }
    };
    /* other arms */
}
```

but the compiler is expected to also handle the case where `$:value` is inside of a macro repetition,
which cannot be directly implemented by just a desugaring of the `macro_rules!` invocation, rather
requiring this "desugaring" to occur at macro expansion time.

The use of `match` in the desugaring ensures that temporary lifetimes last until the end of the
expression; a deguaring based on `let` would end temporary lifetimes too early.

To illustrate this, see the following example:

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

# Drawbacks
[drawbacks]: #drawbacks

For one, `$:value` is another thing that has to be learned to write effective `macro_rules!` macros.
However, it replaces the `match` trick, so the author believes this comes out neutral.
If RFC #2442 is accepted, this just exposes the machinery used by `$:self` to any macro capture,
though it does still slightly increase the burden of the compiler to support multiple such captures,
such as within macro repetition, and not just a single receiver expression.

This RFC, like RFC #2442, does not propose exposing `k#autoref` to user code. As such, this gives
`macro_rules!` macros a superpower (access to `$:value`) which is not available to proc macros.

Additionally, as written, this RFC precludes the use of `$:value` macro binders in statement macros.
Specifically, any `let` bindings in the macro would be dropped at the end of the macro, rather than
living to be dropped at the end of the containing scope. An alternative wording which works for
statement macros would be interesting, but much more complicated, as it cannot just rely on `match`
to define semantics, and would likely have to expand differently in statement and expression position.[^1]

[^1]: Here's a draft attempt at defining such: when expanded in expression position, use the `match`
definition. When expanded in statement position, start with `let k#autoref __name = $name;` and end
with `drop(__name);`. However, this likely does not handle temporary lifetimes properly, which
should extend the temporary to the end of the macro invocation (`drop(__name)`) and no further.

# Alternatives
[alternatives]: #alternatives

A previous iteration of this RFC had `$:value` always bind by-value (matching the behavior of
function arguments directly) and suggested `$:place` as a future extension to provide
`k#autoref`-like behavior. With `k#autoref` much better specified than the old `$:place`,
this RFC now suggests `$:value` to always have `k#autoref` behavior. To get the always-by-value
behavior, macro authors can introduce a `if false { drop($name) }` call to force `$name` to be by-value.
However, it might be useful to have `$:value` always bind by-value, and a separate macro binder
provide the `k#autoref` binding behavior.

Of course, we could just not do this, and `macro_rules!` authors can just use the `match` trick.
More importantly, though, there's two discussed features that could address the same problem space:

## `macro fn`
[macro_fn]: #macro-fn

Another possibility that's been discussed is `macro fn`.
Basically, these would be `fn`, and have the semantics of `fn`,
but be duck typed (like macros) and semantically copy/pasted into the calling scope.
This is basically the exact feature that this RFC is trying to serve, except for two key things:

- A `macro fn` is (potentially) still a `fn` in that it has one fixed airity, and can't be overloaded like a macro can.
- `$:value` uses `k#autoref` binding, but `macro fn`, to match function arguments, would likely use by-move binding.

Basically, `macro fn` is asking for "macros 2.0", which is still desirable, but still a _long_ ways off.
`$:value` offers a small improvement in the status quo without adding a completely new system into the language.

## `k#autoref`
[k_autoref]: #k_autoref

Just expose `k#autoref` behavior directly! For any case that doesn't put `$:value` in a repetition,
this would allow macro authors to just ask for the context dependent binding mode directly, and
would allow macro authors to solve the statement position macros question on their own.

It would still require macro authors to know *when* they need to use `k#autoref` and the `match` trick,
but most of the actual functionality exposed by this RFC would still be available to macro authors.

# Rationale
[rationale]: #rationale

`$:value` simplifies the authoring of `macro_rules` macros,
as authors now no longer need to learn and remember to use the `match` trick to bind macro value arguments,
and instead can just use the `$:value` matcher to get the desired semantics.
Thus, while adding to the semantics provided by the Rust compiler,
it reduces the needed complexity to write correct `macro_rules!`.

This also extends the `k#autoref` binding behavior introduced by RFC #2442 to any macro binder.

Additionally, it is impossible to have a repetition of expr captures that has function-argument like
drop timing through use of the `match` trick alone, as it requires knowing the airity of the
captures ahead of time to name each capture. `$:value` directly unlocks properly and fully variadic
macros that act like function calls with respect to temporary lifetimes.

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

- Do we want to expose `k#autoref` so that proc macros can use this behavior?
- Unknown unknowns.

# Future possibilities
[future-possibilities]: #future-possibilities

[RFC #2442](https://github.com/rust-lang/rfcs/pull/2442) proposes `postfix.macros!()`,
which captures its receiver with `$:value` semantics.
Having `$:value` available in `macro_rules!` would smooth the on-ramp for explaining `postfix.macros!()`.
However, this RFC is _not_ about postfix macros, and stands on its own merit.
