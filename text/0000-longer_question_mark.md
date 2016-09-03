- Feature Name: long_question_mark
- Start Date: 2016-09-03
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Improve code readability and language consistency by
renaming an operator, increasing its visibility.

The changes are:

 * Rename the current `?` operator to `?!`.

This RFC amends [RFC 243][243].

[243]: https://github.com/rust-lang/rfcs/pull/243

# Motivation
[motivation]: #motivation

The `?` operator is very short, and has been criticised
in the past to "hide" a return. Most times code is read,
not written. Renaming it to `?!` will make it two times
as long and therefore harder to overlook.

A single character feels too short for an operator that may return.

Function returns that are not at the bottom
(aka "implicit return") right now are only doable with the
`return` keyword and macros. Giving the `?` operator an
exclamation mark will retain the simple "return keyword or
exclamation mark" rule that code readers can use if they
want to find the places a function can return.

The current behaviour of `?` is different from other language `?`
notations, which make it work on a expression level instead of a
function level. Those languages often exceed Rust in popularity
and coders coming from those contexts will be confused by this
tiny difference in functionality.

This RFC removes the inconsistency both for coders from
different languages and for readers, and leaves space for a
future possible `?` operator that matches the other languages better.

## What use cases does it support?

The same use cases are supported for `?!` as for `?`,
but with increased readability for readers who want to know all
edge cases of code's behaviour (this is an use case as well!).

# Detailed design
[design]: #detailed-design

The `?` operator stays a language feature, but gets renamed to `?!`.

# Drawbacks
[drawbacks]: #drawbacks

The process of going through this PR and delaying stabilisation of the `question_mark` feature will
further prevent use of the `?` feature by stable Rust users.
RFC 243 has initially been proposed in 2014, and has been discussed since, for a very long time.

Expressitivity wise, writing `?` is shorter than `?!`, and it will make reading code that uses
`?!` instead of `?` harder if you want to focus on the main functionality of the code and want to
blend out edge cases like error handling.

If `?` can be overlooked easily, `?!` can be overlooked easily as well.

The new name `?!` may be mistaken for a macro.

The rename might be bikeshedding.

# Alternatives
[alternatives]: #alternatives

Alternatively, the `?` feature could be dropped completely. The general need
for non-prefix based error handling feature exists though.

One could drop `?` to wait for postfix macros which may implement postfix error
propagation on a libs level, and not inside the language, which would be overall
cleaner. But its not sure whether they will come, and when this will happen. Also,
the bad name of macros in C/C++ based languages has made macros in rust
unpopular as well, despite the design advantages. Plus extra syntax would be needed
to work with `catch` from RFC 243, that the postfix macros would then expand to.

The `?` could be made match other languages instead, making it `.map(...)` sugar.
The use case of the `?` operator that allows chained execution would still be supported
if the whole expression is wrapped inside a `try!` invocation, but futures which need a
short notation for early returns will suffer.

## What is the impact of not doing this?

If things are kept moving the way they are, with letting `?` stabilise,
it will become easier to overlook where a function returns than with `?!`.
Also, all the other arguments from the motivation section apply.

# Unresolved questions
[unresolved]: #unresolved-questions

Should there be a `?` operator that works as `.map(...)` sugar?
Which additional use cases will that cover? Will it help to prevent use of `?!`
in cases where it might disturb a reader?

Maybe the `?!` name is still too short? Should a valid identifier be chosen
instead of "?" so that if Rust gets postfix macros, the operator can be moved
into libcore?
