- Start Date: 2014-11-02
- RFC PR:
- Rust Issue:

# Summary

> "A designer knows he has achieved perfection not when there is nothing left
> to add, but when there is nothing left to take away."
>
> -- Antoine de Saint-Exupery


This RFC proposes replacing the `loop` keyword with `while true`.  `while true`
is the common idiom for an infinite loop in most other languages, which makes
it easier for people to read and write Rust.  A separate keyword is superflous,
and may cause people to question Rust's design or the compiler's smarts.

The proposed implementation is to simply recognize the tokens `while true` in
place of the token `loop` and keep the existing internal loop handling
unchanged.  This is a simple change that can be done before Rust 1.0.  Unifying
the internal handling of both loop types is specifically not proposed, as that
is an implementation detail that can be changed at any later time.

# Motivation

The primary justification for the existince of `loop` seems to be that it
helped compiler reasoning, since a `loop` will always be executed at least
once.  Thus, code like this is valid:

```{rust}
fn main() {
    let x;
    loop { x = 1i; break; }
    println!("{}", x)
}
```

whereas currently the equivalent with `while true` does not compile (`x` is
possibly uninitialized).

However, it is just as easy for the compiler to recognize the tokens `while
true` and apply the existing infinite loop reasoning.  That makes `loop`
superfluous.  No other mainstream C-family language has a keyword for an
infinite loop ([C][^1], [C++][^2], [Go][^3], [D][^4], [Java][^5], [C#][^6]).
The addition of a keyword for this special case seems unwarranted without
strong reason.  Removing it yields a cleaner, smaller, simpler, more familiar
language.

Removal was proposed in March 2014 in Github [issue
\#12975](https://github.com/rust-lang/rust/issues/12975); the only real
objection was that `while` didn't support labeled breaks at the time, but this
has since been fixed in [issue
\#12643](https://github.com/rust-lang/rust/issues/12643) as of August 2014.

Last and certainly least, it frees up the variable name `loop` for use by
newbies everywhere.


[^1]: https://en.wikipedia.org/wiki/C_syntax#Reserved_keywords
[^2]: http://en.cppreference.com/w/cpp/keyword
[^3]: https://golang.org/ref/spec#Keywords
[^4]: http://dlang.org/lex.html
[^5]: https://en.wikipedia.org/wiki/List_of_Java_keywords
[^6]: http://msdn.microsoft.com/en-us/library/x53a06bb.aspx


# Detailed design

This RFC proposes removing the `loop` keyword, recognizing `while true` in its
place, and replacing all uses in the compiler and standard libraries.

For now, this RFC proposes only recognizing the literal two-token sequence
`while true` as an infinite loop.  Recognizing compile-time constant
expressions that evaluate to `true` in the `while` condition is a
backward-compatible enhancement that can and should be added at any time.

It looks like `syntax::parse::parser::Parser.parse_while_expr()` could be extended
to recognize `true` following `while` in the same way it currently recognizes
`while let`, and the existing `parse_loop_expr()` could be called in that case.

If this RFC is accepted, there is the question of how to roll out the change.
The keyword could be completely removed all at once, giving an error with a
hint, but this would break a lot of code and make a lot of people unhappy.
Alternatively we could have a grace period where we recognize both `loop` and
`while true` for infinite loop, issuing a deprecation warning in the former
case.  The work could be done in phases:

1. Modify compiler to recognize both `loop` and `while true` for infinite loop; deprecate `loop`
2. Update documentation
3. Update compiler and standard libraries use `while true`
4. Remove `loop` from compiler, leave hint to use `while true`

# Drawbacks

This will require typing an additional six keystrokes for every infinite loop
written in Rust from now until infinity.  An experiment was performed to
measure the impact on productivity.  Typing `loop` 10 times took an average
of 1.13 +/- 0.22 sec, while typing `while true` took an average of 1.57 +/-
0.28 sec. This is an increase of 0.44 sec, or 1.4X, statistically significant
at the p = 0.0005 level by a one-tailed unpaired t-test.  Such a significant
drain on programmer productivity should be weighed very un-seriously when
considering adoption of this RFC.

This will also add six bytes to every occurence of an infinite loop in Rust
source.  As of the 2014-11-01 nightly revision, the Rust source contains an
estimated 400 occurrences of the `loop` keyword*, or about 1 in every 11 of the
~4400 Rust source files.  Changing these to `while true` would add about 2400
bytes to the Rust source, an increase of 0.009% on the 26 MB source
distribution.  It is difficult to estimate how the impact on download and
compile times will compare to the imapct on programmer time.

The largest drawback is breaking existing code.  However, as seen above, the
`loop` keyword is fairly rare.  The change is very small, completely contained,
and can be done mostly mechanically. A script could be provided to assist
people in migrating their code.  Breakage can be mitigated with a deprecation
warning before full removal.

\* `$ find . -name '*.rs' | xargs sed -e 's#//.*##g'  | grep -wc loop`

# Alternatives

The alternative is to leave the `loop` keyword in, freezing this design wart
for all time, and dealing with the steady stream of newbies on Reddit and IRC
asking "Why the separate keyword?" long past the time when anyone can remember
the need for it.  Do we really want to drive a Rusty
[auto](http://www.seebs.net/faqs/c-iaq.html#question-1.8)?

# Unresolved questions

If approved, the main question is whether to remove the keyword in one fell
swoop, giving an error with a hint, or allow a grace period where the compiler
accepts both and issues a deprecation warning.  If a grace period is desired,
then the question is how long.  3-4 weeks seems like a reasonable balance
between letting more people find out gently, and still allowing time for
migration between the hard removal and Rust 1.0.

