- Feature Name: `undisambiguated_generics`
- Start Date: 2018-09-14
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

Make disambiguating generic arguments in expressions with `::` optional, allowing generic arguments
to be specified without `::` (making the "turbofish" notation no longer necessary).
This makes the following valid syntax:

```rust
struct Nooper<T>(T);

impl<T> Nooper<T> {
    fn noop<U>(&self, _: U) {}
}

fn id<T>(t: T) -> T {
    t
}

fn main() {
    id<u32>(0u32); // ok
    let n = Nooper<&str>(":)"); // ok
    n.noop<()>(()); // ok
}
```

# Motivation
[motivation]: #motivation

The requirement to write `::` before generic arguments in expressions is an unexpected corner case
in the language, violating the principle of least surprise. There were historical reasons for its
necessity in the past, acting as a disambiguator for other uses of `<` and `>` in expressions.
However, now the ambiguity between generic arguments and comparison operators has been reduced to a
single edge case that is very unlikely to appear in Rust code (and has been demonstrated to occur in
[none of the existing crates](https://github.com/rust-lang/rust/pull/53578#issuecomment-421475443)
in the Rust ecosystem as of 2018-09-14). Making `::` optional in expressions takes a step towards
eliminating an oddity in the Rust syntax, making it more uniform and less confusing (e.g.
[1](https://users.rust-lang.org/t/why-cant-i-specify-type-parameters-directly-after-the-type/2365),
[2](https://users.rust-lang.org/t/type-parameter-syntax-when-defining-vs-calling-functions/15037),
[3](https://github.com/rust-lang/book/issues/385),
[4](https://www.reddit.com/r/rust/comments/73pm5e/whats_the_rationale_behind_for_type_parameters/),
[5](https://matematikaadit.github.io/posts/rust-turbofish.html)) to beginners.

There have been two historical reasons to require `::` before generic arguments in expressions.

## Syntax ambiguity
Originally, providing generic arguments without `::` meant that some expressions were ambiguous in
meaning.

```rust
// Take the following:
a < b > ( c );
// Is this a generic function call..?
a<b>(c);
// Or a chained comparison?
(a < b) > (c);
```

However, chained comparisons are [now banned in Rust](https://github.com/rust-lang/rfcs/pull/558):
the previous example results in an error.

```rust
a < b > ( c ); // error: chained comparison operators require parentheses
```

This syntax is therefore no longer ambiguous and we can determine whether `<` is a comparison
operator or the start of a generic argument list during parsing.

There is, however, one case in which the syntax is currently ambiguous.

```rust
// The following:
(a < b, c > (d));
// Could be a generic function call...
( a<b, c>(d) );
// Or a pair of comparisons...
(a < b, c > (d));
```

Ultimately, this case does not seem occur naturally in Rust code. A
[Crater run on over 20,000 crates](https://github.com/rust-lang/rust/pull/53578#issuecomment-421475443)
determined that no crates regress if the ambiguity is resolved in favour of a generic expression
rather than tuples of comparisons of this form. We propose that resolving this ambiguity in favour
of generic expressions to eliminate `::` is worth this small alteration to the existing parse.

## Performance
Apart from parsing ambiguity, the main concern regarding allowing `::` to be omitted was the
potential performance implications. Although by the time we reach the closing angle bracket `>` we
know whether we're parsing a comparison or a generic argument list, when we initially encounter `<`,
we are not guaranteed to know which case we're parsing. To solve this problem, we need to
first start parsing a generic argument list and then backtrack if this fails (or use a parser that
can deal with ambiguous grammars). We generally prefer to avoid backtracking, as it can be slow.
However, up until now, the concern with using backtracking for `<`-disambiguation was purely
theoretical, without any empirical testing to validate it.

[A recent experiment](https://github.com/rust-lang/rust/pull/53511) to allow generic arguments
without `::`-disambiguation [showed no performance regressions](https://github.com/rust-lang/rust/pull/53511#issuecomment-414172984)
using the backtracking technique. This indicates that in existing codebases, allowing `::` to be
omitted is unlikely to lead to any performance regressions.

Similarly, the performance implications of deleting all occurrences of `::` (and simply using
generic arguments directly)
[also showed no performance regressions](https://github.com/rust-lang/rust/pull/53511#issuecomment-414360849).
This is likely to be due to the relative uncommonness of providing explicit generic arguments and
using comparison operators in the cases of ambiguous prefixes, relative to typical codebases.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

To explicitly pass generic arguments to a type, value or method, you may write the lifetime, type
and const arguments in angle brackets (`<` and `>`) directly after the expression. (Note that the
"turbofish" notation is no longer necessary.)

```rust
struct Nooper<T>(T);

impl<T> Nooper<T> {
    fn noop<U>(&self, _: U) {}
}

fn id<T>(t: T) -> T {
    t
}

fn main() {
    id<u32>(0u32);
    let n = Nooper<&str>(":)");
    n.noop<()>(());
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

An initial implementation is present in https://github.com/rust-lang/rust/pull/53578, upon which the
implementation may be based. The parser will now attempt to parse generic argument lists without
`::`, falling back on attempting to parse a comparison if that fails.

The feature will initially be gated (e.g. `#![feature(undisambiguated_generics)]`). However,
note that the parser changes will be present regardless of whether the feature is enabled or not,
because feature detection occurs after parsing. However, because it has been shown that there are
little-to-no performance regressions when modifying the parser and without taking advantage of `::`
optionality, this should not be a problem.

When `undisambiguated_generics` is not enabled, the parser modifications will allow us to
provide better diagnostics: specifically, we'll be able to correctly suggest (in a
machine-applicable manner) using `::` whenever the user has actually typed undisambiguated generic
arguments. The current diagnostic suggestions suggesting the use of `::` trigger whenever there are
chained comparisons, which has false positives and does not provide a fix suggestion.

An allow-by-default lint `disambiguated_generics` will be added to suggest removing `::` when
the feature is enabled. This is undesirable in most existing codebases, as the number of
linted expressions is likely to be large, but could be useful for new codebases and in the future.

Note that, apart from for those users who explicitly increase the level of the lint, no steps are
taken to discourage the use of `::` at this stage (including in tools, such as rustfmt).

# Drawbacks
[drawbacks]: #drawbacks

The primary drawback is that resolving ambiguities in favour of generics means changing the
interpretation of `(a<b, c>(d))` from a pair of tuples to a generic function call. However, in
practice, this has been demonstrated
([1](https://github.com/rust-lang/rust/pull/53578#issuecomment-421475443)) not to cause issues in
practice (the syntax is unnatural for Rust and is actively warned against by the compiler).

Additionally, there is potential for performance regressions due to backtracking. However,
empirical evidence ([1](https://github.com/rust-lang/rust/pull/53511#issuecomment-414172984) and
[2](https://github.com/rust-lang/rust/pull/53511#issuecomment-414360849)) suggests this should not
be a problem. Although it is probable that a pathological example could be constructed that does
result in poorer performance, such an example would not be representative of typical Rust code and
therefore is not helpful to seriously consider. Backtracking is already used for some cases in the
parser.

The other potential drawback is that other parsers for Rust's syntax (for example in external tools)
would also have to implement some form of backtracking (or similar) to handle this case. However,
backtracking is straightforward to implement in many forms of parser (such as recursive decent) and
it is likely this will not cause significant problems.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

If we want to allow `::` to be omitted, there are two solutions:
- Backtracking, as suggested here.
- Using a parser for nondeterministic grammars, such as GLL.

Although using a more sophisticated parser would come with its own advantages, it's an overly
complex solution to this particular problem. Backtracking seems to work well in typical codebases
and provides an immediate solution to the problem.

Alternatively we could continue to require `::`. This would ensure there would be no performance
implications, but would leave the nonconformal and surprising syntax in place. We could potentially
use backtracking to provide the improved diagnostic suggestions to use `::`, while still preventing
`::` from being omitted.

## Future frequency of disambiguated generic expressions
It is likely that should the
[generalised type ascription](https://github.com/rust-lang/rfcs/pull/2522) RFC be accepted and
implemented, the number of cases where generic type arguments have to be provided is reduced, making
users less likely to encounter the `::` construction. However, the
[const generics](https://github.com/rust-lang/rfcs/pull/2000) feature, currently in implementation,
is conversely likely to increase the number of cases (specifically where const generic arguments
are not used as parameters in types).

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Should we warn against the ambiguous case initially? This would be more conservative, but
considering that this pattern has not been encountered in the wild, this is probably unnecessary.
- Should `(a < b, c > d)` parse as a pair of comparisons? In the aforementioned Crater run, this
syntax was also resolved as a generic expression followed by `d` (also causing no regressions), but
we could hypothetically parse this unambiguously as a pair (though this would probably require more
complex backtracking).
