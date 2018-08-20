- Feature Name: `undisambiguated_expr_generics`
- Start Date: 2018-08-20
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

Make disambiguating generic arguments in expressions with `::` optional, allowing generic arguments to be specified without `::`. This makes the following valid syntax:

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

The requirement to write `::` before generic arguments in expressions is an unexpected corner case in the language, violating the principle of least surprise. There were historical reasons for its necessity in the past, acting as a disambiguator for other uses of `<` and `>` in expressions. However, the syntax is no longer ambiguous with uses of the comparison operators and it is possible to parse generic arguments in expressions without disambiguation. Making `::` optional in expressions takes a step towards eliminating an oddity in the Rust syntax, making it more uniform and less confusing (e.g. [1](https://users.rust-lang.org/t/why-cant-i-specify-type-parameters-directly-after-the-type/2365), [2](https://users.rust-lang.org/t/type-parameter-syntax-when-defining-vs-calling-functions/15037), [3](https://github.com/rust-lang/book/issues/385), [4](https://www.reddit.com/r/rust/comments/73pm5e/whats_the_rationale_behind_for_type_parameters/), [5](https://matematikaadit.github.io/posts/rust-turbofish.html)) to beginners.

There have been two historical reasons to require `::` before generic arguments in expressions.

## Syntax ambiguity
Originally, providing generic arguments without `::` meant that some expressions were ambiguous in meaning.

```rust
// Take the following:
a < b > ( c );
// Is this a generic function call..?
a<b>(c);
// Or a chained comparison?
(a < b) > (c);
```

However, chained comparisons are now banned in Rust: the previous example results in an error:

```rust
a < b > ( c ); // error: chained comparison operators require parentheses
```

This means that syntax is no longer ambiguous and we can determine whether `<` is a comparison operator or the start of a generic argument list during parsing.

## Performance
Since the ambiguity resolution, the main concern regarding allowing `::` to be omitted was the potential performance implications. Although by the time we reach the closing angle bracket `>` we know whether we're parsing a comparison or a generic argument list, when we initially encounter `<`, we are not guaranteed to know which case we're parsing. In order to solve this problem, we need to first start parsing a generic argument list and then backtrack if this fails (or use a parser that can deal with ambiguous grammars). We generally prefer to avoid backtracking, as it can be slow. However, up until now the concern with using backtracking for `<` disambiguation was purely theoretical, without any empirical testing to validate it.

[A recent experiment](https://github.com/rust-lang/rust/pull/53511) to allow generic arguments without `::` disambiguation [showed no performance regressions](https://github.com/rust-lang/rust/pull/53511#issuecomment-414172984) using the backtracking technique. This indicates that in existing codebases, allowing `::` to be omitted is unlikely to lead to any performance regressions.

Similarly, the performance implications of deleting all occurrences of `::` (and simply using generic arguments directly) [also showed no performance regressions](https://github.com/rust-lang/rust/pull/53511#issuecomment-414360849). This is likely to be due to the relative uncommonness of providing explicit generic arguments and using comparison operators in the cases of ambiguous prefixes, relative to typical codebases.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

To explicitly pass generic arguments to a type, value or method, you may write the lifetime, type and const arguments in angle brackets (`<` and `>`) directly after the expression.

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

There's an initial implementation in https://github.com/rust-lang/rust/pull/53511, upon which the implementation can be based. The parser will now attempt to parse generic argument lists without `::`, falling back on attempting to parse a comparison if that fails.

The feature will initially be gated (e.g. `#![feature(undisambiguated_expr_generics)]`). However, note that the parser changes will be present regardless of whether the feature is enabled or not, because feature detection occurs after parsing. However, because it has been shown that there are little-to-no performance regressions when modifying the parser and without taking advantage of `::` optionality, this should not be a problem.

An allow-by-default lint `disambiguated_expr_generics` will be added to suggest removing `::` when the feature is enabled. Obviously this is undesirable in most existing codebases, as the number of linted expressions is likely to be large, but could be useful for new codebases and in the future.

Note that, apart from for those users who explicitly increase the level of the lint, no steps are taken to discourage the use of `::` at this stage.

# Drawbacks
[drawbacks]: #drawbacks

The primary drawback is the potential for performance regressions due to backtracking. However, empirical evidence ([1](https://github.com/rust-lang/rust/pull/53511#issuecomment-414172984) and [2](https://github.com/rust-lang/rust/pull/53511#issuecomment-414360849)) suggests this should not be a problem. Although it is probable that a pathological example could be constructed that does result in poorer performance, such an example would not be representative of typical Rust code and therefore is not helpful to seriously consider. Backtracking is already used for some cases in the parser.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

If we want to allow `::` to be omitted, there are two solutions:
- Backtracking, as suggested here.
- Using a parser for nondeterministic grammars, such as GLL.

Although using a more sophisticated parser would come with its own advantages, it's an overly complex solution to this particular problem. Backtracking seems to work well in typical codebases and provides an immediate solution to the problem.

Alternatively we could continue to require `::`. This would ensure there would be no performance implications, but would leave the nonconformal and surprising syntax in place.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None.
