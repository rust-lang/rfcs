- Feature Name: `macro_metavar_expr`
- Start Date: 2021-01-23
- RFC PR: [rust-lang/rfcs#3086](https://github.com/rust-lang/rfcs/pull/3086)
- Rust Issue: [rust-lang/rust#83527](https://github.com/rust-lang/rust/issues/83527)

# Summary
[summary]: #summary

Add new syntax to declarative macros to give their authors easy access to
additional metadata about macro metavariables, such as the index, length, or
count of macro repetitions.

# Motivation
[motivation]: #motivation

Macros with repetitions often expand to code that needs to know or could
benefit from knowing how many repetitions there are, or which repetition is
currently being expanded.  Consider the example macro used in the guide to
introduce the concept of macro repetitions: building a vector, recreating the
`vec!` macro from the standard library:

```
macro_rules! vec {
    ( $( $x:expr ),* ) => {
        {
            let mut temp_vec = Vec::new();
            $(
                temp_vec.push($x);
            )*
            temp_vec
        }
    };
}
```

This would be more efficient if it could use `Vec::with_capacity` to
preallocate the vector with the correct length.  However, there is no standard
facility in declarative macros to achieve this, as there is no way to obtain
the *number* of repetitions of `$x`.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The [example `vec` macro definition in the guide][guide-vec] could be made
more efficient if it could use `Vec::with_capacity` to pre-allocate a vector
with the correct capacity.  To do this, we need to know the number of
repetitions.

[guide-vec]: https://doc.rust-lang.org/book/ch19-06-macros.html#declarative-macros-with-macro_rules-for-general-metaprogramming

Metadata about metavariables, like the number of repetitions, can be accessed
using **metavariable expressions**.  The metavariable expression for the
count of the number of repetitions of a metavariable `x` is `${count(x)}`, so
we can improve the `vec` macro as follows:

```
#[macro_export]
macro_rules! vec {
    ( $( $x:expr ),* ) => {
        {
            let mut temp_vec = Vec::with_capacity(${count(x)});
            $(
                temp_vec.push($x);
            )*
            temp_vec
        }
    };
}
```

The following metavariable expressions are available:

| Expression                 | Meaning    |
|----------------------------|------------|
| `${count(ident)}`          | The number of times `$ident` repeats in total. |
| `${count(ident, depth)}`   | The number of times `$ident` repeats at up to `depth` nested repetition depths. |
| `${index()}`               | The current index of the inner-most repetition. |
| `${index(depth)}`          | The current index of the nested repetition at `depth` steps out. |
| `${length()}`              | The length of the inner-most repetition. |
| `${length(depth)}`         | The length of the nested repetition at `depth` steps out. |
| `${ignore(ident)}`         | Binds `$ident` for repetition, but expands to nothing. |
| `$$`                       | Expands to a single `$`, for removing ambiguity in recursive macro definitions. |

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Metavariable expressions in declarative macros provide expansions for
information about metavariables that are otherwise not easily obtainable.

This is a backwards-compatible change as both `$$` and `${ .. }` are not
currently accepted as valid.

The metavariable expressions added in this RFC are concerned with declarative
macro metavariable repetitions, and obtaining the information that the
compiler knows about the repetitions that are being processed.

## Count

The `${count(x)}` metavariable expression shown in the `vec` example in the
previous section counts the number of repetitions that will occur if the
identifier is used in a repetition at this depth.  This means that in a macro
expansion like:

```
    ${count(x)} $( $x )*
```

the expression `${count(x)}` will expand to an unsuffixed integer literal
equal to the number of times the `$( $x )*` repetition will repeat.  For
example, if the metavariable `$x` repeats four times then it will expand to
the integer literal `4`.

If repetitions are nested, then an optional depth parameter can be used to
limit the number of nested repetitions that are counted.  For example, a macro
expansion like:

```
    ${count(x, 1)} ${count(x, 2)} ${count(x, 3)} $( a $( b $( $x )* )* )*
```

The three values this expands to are the number of outer-most repetitions (the
number of times `a` would be generated), the sum of the number of middle
repetitions (the number of times `b` would be generated), and the total number
of repetitions of `$x`.

## Index and length

Within a repetition, the `${index()}` and `${length()}` metavariable
expressions give the index of the current repetition and the length of the
repetition (i.e., the number of times it will repeat). The index value ranges
from `0` to `length - 1`, and the expanded values are unsuffixed integer
literals so they are also suitable for tuple indexing.

For nested repetitions, the `${index()}` and `${length()}` metavariable
expressions expand to the inner-most index and length respectively.
If the `depth` parameter is specified, then the metavariable expression
expands to the index or length of the surrounding nested repetition, counting
outwards from the inner-most repetition.  The expressions `${index()}` and
`${index(0)}` are equivalent.

For example in the expression:

```
    $( a $( b $( c $x ${index()}/${length()} ${index(1)}/${length(1)} ${index(2)}/${length(2)} )* )* )*
```

the first pair of values are the index and length of the inner-most
repetition, the second pair are the index and length of the middle
repetition, and the third pair are the index and length of the outer-most
repetition.

## Ignore

Sometimes it is desired to repeat an expansion the same number of times as a
metavariable repeats but without actually expanding the metavariable.  It may
be possible to work around this by expanding the metavariable in an expression
like `{ $x ; 1 }`, where the expanded value of `$x` is ignored, but this
is only possible if what `$x` expands to is valid in this kind of expression.

The `${ignore(ident)}` metavariable acts as if `ident` was used for the purposes
of repetition, but expands to nothing.  This means a macro expansion like:

```
    $( ${ignore(x)} a )*
```

will expand to a sequence of `a` tokens repeated the number of times that `x` repeats.

## Dollar dollar

Since metavariable expressions always apply during the expansion of the macro,
they cannot be used in recursive macro definitions.  To allow recursive macro
definitions to use metavariable expressions, the `$$` expression expands to a
single `$` token.

This is also necessary for unambiguously defining repetitions in nested
macros.  For example, this resolves [issue 35853], as the example in
that issue can be expressed as:

```
macro_rules! foo {
    () => {
        macro_rules! bar {
            ( $$( $$any:tt )* ) => { $$( $$any )* };
        }
    };
}

fn main() { foo!(); }
```

[issue 35853]: https://github.com/rust-lang/rust/issues/35853

## Larger example

For a larger example of these metavariable expressions in use, consider the
following macro that operates over three nested repetitions:

```
macro_rules! example {
    ( $( [ $( ( $( $x:ident )* ) )* ] )* ) => {
        counts = (${count(x, 1)}, ${count(x, 2)}, ${count(x)})
        nested:
        $(
            indexes = (${index()}/${length()})
            counts = (${count(x, 1)}, ${count(x)})
            nested:
            $(
                indexes = (${index(1)}/${length(1)}, ${index()}/${length()})
                counts = (${count(x)})
                nested:
                $(
                    indexes = (${index(2)}/${length(2)}, ${index(1)}/${length(1)}, ${index()}/${length()})
                    ${ignore(x)}
                )*
            )*
        )*
    };
}
```

Given this input:
```
    example! {
        [ ( A B C D ) ( E F G H ) ( I J ) ]
        [ ( K L M ) ]
    }
```

The macro would expand to:
```
    counts = (2, 4, 13)
    nested:
        indexes = (0/2)
        counts = (3, 10)
        nested:
            indexes = (0/2, 0/3)
            counts = (4)
            nested:
                indexes = (0/2, 0/3, 0/4)
                indexes = (0/2, 0/3, 1/4)
                indexes = (0/2, 0/3, 2/4)
                indexes = (0/2, 0/3, 3/4)
            indexes = (0/2, 1/3)
            counts = (4)
            nested:
                indexes = (0/2, 1/3, 0/4)
                indexes = (0/2, 1/3, 1/4)
                indexes = (0/2, 1/3, 2/4)
                indexes = (0/2, 1/3, 3/4)
            indexes = (0/2, 2/3)
            counts = (2)
            nested:
                indexes = (0/2, 2/3, 0/2)
                indexes = (0/2, 2/3, 1/2)
        indexes = (1/2)
        counts = (1, 3)
        nested:
            indexes = (1/2, 0/1)
            counts = (3)
            nested:
                indexes = (1/2, 0/1, 0/3)
                indexes = (1/2, 0/1, 1/3)
                indexes = (1/2, 0/1, 2/3)
```


# Drawbacks
[drawbacks]: #drawbacks

This adds additional syntax to the language, that program authors must learn
and understand.  We may not want to add more syntax.

The author believes it is worth the overhead of new syntax, as even though
there exist workarounds for obtaining the information if it's really needed,
these workarounds are sometimes difficult to discover and naive
implementations can significantly harm compiler performance.

Furthermore, the additional syntax is limited to declarative macros, and its
use should be limited to specific circumstances where it is more understandable
than the alternatives.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This RFC proposes a modest but powerful extension to macro syntax that makes
it possible to obtain information that the compiler already knows, but
requires inefficient and complex techniques to obtain in the macro.

The original proposal was for a shorter syntax to provide the count of
repetitions: `$#ident`.  During discussions of this syntax, it became clear
that it was not obvious as to which number this referred to: the count of
repetitions at this level, or the length of the current repetition.  It also
does not provide a way to discover counts or lengths for other repetition
depths.  There was also interest in being able to discover the index of the
current repetition, and the `#` character had been used in similar proposals
for that.  There was some reservation expressed for the use of the `#` token
because of the cognitive burden of another sigil, and its common use in the
`quote!` macro.

The meaning of the `depth` parameter in `index` and `count` originally
counted inwards from the outer-most nesting.  This was changed to count
outwards from the inner-most nesting so that expressions can be copied
to a different nesting depth without needing to change them.

This RFC proposes using `${ ... }` as the delimiter for metavariable
expressions.  Available alternatives are:
* `$[ ...  ]`, e.g.: `$[count(value)]`
* `$:`, e.g. `$:count(value)`
* `$@`, e.g. `$@count(value)`
* `$!`, e.g. `$!count(value)`
* Another sigil, although `#` should be avoided to avoid clashes with the
  `quote!` macro.

## Why not a proc macro or built-in macro?

To avoid extending the language with new syntax, we could consider writing
something that looks like a macro invocation, such as `count!(value)`, which
would be implemented as a procedural macro or built-in to the compiler.

While this is compelling from a language simplicity perspective, it creates
some problems due to the way macro expansions are processed.  During macro
transcription, other macro invocations are not evaluated, so in the macro:

```
macro_rules! example {
    ($($x:ident),*) => count!(x)
}
```

During transcription, `example!(a, b, c)` would expand to `count!(x)`.  At this
point, the knowledge of the metavariable `x` and its repetition is lost, and
no procedural macro or built-in macro would be able to work out the count.

To workaround this we would need to re-expand the repetition
(`count!($($x),*)`, forcing the `count!` macro to re-parse and count the
repetitions.  This is additional unnecessary work that this RFC seeks to
address.

Another way to think of metavariable expressions is as "macro transcriber
directives". You can think of the macro transcriber as performing the
following operations:

* `$var` => the value of `var`
* `$( ... ) ...` => a repetition

This RFC adds two more:

* `${ directive(args) }` => a special transcriber directive
* `$$` => `$`

We could special-case certain macro invocations like `count!` during
transcription, but that feels like a worse solution. It would make it harder
to understand what the macro transcriber is going to do with arbitrary code
without remembering all of the special macros that don't work like other
macros.

# Prior art
[prior-art]: #prior-art

Declarative macros with repetition are commonly used in Rust for things that
are implemented using variadic functions in other languages.  Usually these
other languages provide mechanisms for finding the number of variadic
arguments, and it is a notable limitation that Rust does not.

Scripting languages, like Bash, which use `$var` for variables, often use
similar `${...}` syntax for values based on variables: for example `${#var}`
is used for the length of `$var`.  This means `${...}` expressions should not
seem too weird to developers familiar with these scripting languages.

A proposal for counting sequence repetitions was made in [RFC 88].  That RFC
proposed several options for additional syntax, however the issue was
postponed to after the 1.0 release.  This RFC addresses the needs of RFC 88,
and also goes further, as it proposes a more general syntax useful for more
than just counting repetitions, such as obtaing the index of the current
repetition.  Since the generated values are integer literals, it also
addresses the ability to index tuples in repetitions (using `tup.${index()}`),
which was noted as an omission in RFC 88.  It's also not possible to implement
efficiently as a procedural macro, as the procedural macro would not have
access to the repetition counts without generating a sequence and then
counting it again.

[RFC 88]: https://github.com/rust-lang/rfcs/pull/88

# Unresolved questions
[unresolved-questions]: #unresolved-questions

No unresolved questions at present.

While more expressions are possible, expressions beyond those defined in this RFC are out-of-scope.

# Future possibilities
[future-possibilities]: #future-possibilities

The metavariable expression syntax (`${...}`) is purposefully generic, and may
be extended in future RFCs to anything that may be useful for the macro
expander to produce.

The syntax `$[...]` is still invalid, and so remains available for any other
extensions which may come in the future and don't fit in with metavariable
expression syntax.  Additionally, any symbol after `$` is also invalid, so
other sequences, such as `$@`, are available.
