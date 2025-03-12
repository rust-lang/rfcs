- Feature Name: N/A
- Start Date: 2018-05-04
- RFC PR: [rust-lang/rfcs#2436](https://github.com/rust-lang/rfcs/pull/2436)
- Rust Issue: N/A

# Summary
[summary]: #summary

This RFC defines an official Rust style guide. The style is a specification for the default behaviour of [Rustfmt](https://github.com/rust-lang-nursery/rustfmt), is required for official Rust projects (including the compiler and standard libraries), and is recommended for all Rust projects. The style guide in this RFC only covers the formatting of code, it does not have any recommendations about how to write idiomatic or high quality Rust code.

The formatting guidelines in the style guide have been decided on in the [formatting RFC process](https://github.com/rust-lang/rfcs/blob/master/text/1607-style-rfcs.md) by the style team. The guidelines were [extensively debated](https://github.com/rust-lang-nursery/fmt-rfcs/issues?utf8=%E2%9C%93&q=is%3Aissue) and this RFC is the result of that consensus process. I would like to discourage re-opening debate on the guidelines themselves here. Please limit discussion to the presentation and application of the guide, omissions from the guide, and issues which were missed in the formatting RFC process.

Thanks to the style team for their work on the guidelines: Brian Anderson, Jorge Aparicio, Nick Cameron, Steve Klabnik, Nicole Mazzuca, Scott Olson, and Josh Triplett.

# Motivation
[motivation]: #motivation

Formatting code is a mostly mechanical task which takes both time and mental effort. By using an automatic formatting tool, a programmer is relieved of this task and can concentrate on more important things.

Furthermore, by sticking to an established style guide (such as this one), programmers don't need to formulate ad hoc style rules, nor do they need to debate with other programmers what style rules should be used, saving time, communication overhead, and mental energy.

Humans comprehend information through pattern matching. By ensuring that all Rust code has similar formatting, less mental effort is required to comprehend a new project, lowering the bar to entry for new developers.

Thus, there are productivity benefits to using a formatting tool (such as rustfmt), and even larger benefits by using a community-consistent formatting, typically by using a formatting tool's default settings.

## Options

Rustfmt has many options for customising formatting. The behaviour of those options is outside the scope of this RFC. We recommend that users do not configure Rustfmt using the available options and use the default settings. The reason for doing so is consistency in code formatting across the ecosystem - this lowers the bar for developers to move from one project to another because they don't need to get used to reading a new style of formatting.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

See the [guide text](https://doc.rust-lang.org/nightly/style-guide/).

The style guide formerly lived in the RFC repo, since it was an appendix to this RFC. The style guide has now been moved to the `rust-lang/rust` repository, as of RFC 3309. Amendments to the style guide go through the FCP process.



# Drawbacks
[drawbacks]: #drawbacks

One can level some criticisms at having a style guide:

* it is bureaucratic, gives developers more to worry about, and crushes creativity,
* there are edge cases where the style rules make code look worse (e.g., around FFI).

However, these are heavily out-weighed by the benefits.


# Rationale and alternatives
[alternatives]: #alternatives

Many alternative formatting guidelines were discussed in the [formatting RFC process](https://github.com/rust-lang-nursery/fmt-rfcs/issues?utf8=%E2%9C%93&q=is%3Aissue). The guiding principles behind that process are outlined in [that repo](https://github.com/rust-lang-nursery/fmt-rfcs#guiding-principles).

A possible alternative to this style of style guide would be to try and provide a complete and exhaustive specification, such that if any two tools correctly implemented the specification, they would always format code in the same style. However, this would be a massive undertaking and of limited value (it would permit projects to move easily from one tool to another, but since the tools would be so constrained, there would be little benefit in making a second tool).

We could also not have a written style guide and state that the output of Rustfmt is the official Rust style, however, that would not have permitted the community input that the formatting RFC process facilitated, and would not give a good way to judge breaking changes in Rustfmt.


# Prior art
[prior-art]: #prior-art

Rust has [API design guidelines](https://rust-lang-nursery.github.io/api-guidelines/); an early version ('the Rust style guide') contained both formatting and API design guidelines.

Some language have official style guides (e.g., [Python](https://www.python.org/dev/peps/pep-0008/) and [Kotlin](https://kotlinlang.org/docs/reference/coding-conventions.html#formatting)). For those that do not, several unofficial guides usually appear, for example, there are several style guides for C++, such as [Google's](https://google.github.io/styleguide/cppguide.html) and [Mozilla's](https://developer.mozilla.org/en-US/docs/Mozilla/Developer_guide/Coding_Style).


# Unresolved questions
[unresolved]: #unresolved-questions


