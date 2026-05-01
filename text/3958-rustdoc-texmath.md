- Feature Name: `rustdoc_texmath`
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: [rust-lang/rfcs#3958](https://github.com/rust-lang/rfcs/pull/3958)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

## Summary
[summary]: #summary

Add support for the de facto standard TeX-math-in-markdown syntax to Rustdoc. It’s currently implemented using the [math-core][] library, which generates MathML Core and is restricted to the subset of LaTeX that can be implemented that way. If you use unsupported syntax, you get a compiler warning from rustdoc.

[math-core]: https://docs.rs/math-core/latest/math_core/

## Motivation
[motivation]: #motivation

It would be nice if we could write complex equations in our docs.
We know that there's demand for this feature,
first of all because people have [asked for it][internals thread],
but mostly because of [crates that did it themselves][] by loading [katex.js][] with inline HTML.

As far as I know, this is the most popular way of doing that:

    [package.metadata.docs.rs]
    rustdoc-args = ["--html-in-header", "katex-header.html", "--cfg", "docsrs"]

Because only docs.rs reads that directive,
local `cargo doc` and non-Rustdoc doc readers won't see it.
There is a way to make it work in `cargo doc`,
but it [seems to be less popular][include hack].

Fixing this bug and making it easier to build self-contained docs is
the main motivation for adding built-in support for math syntax to Rustdoc,
but there are a few other quality of life improvements that come with this feature:

- We can report math syntax errors on the CLI, just like we do for intra-doc links.
- We can render math in the resulting web page without JavaScript.
  No flash of unstyled content or blocking scripts.
- Built-in TeX math doesn't require double-escaping, because the Markdown parser knows about math,
  and lets you backslash escape the dollar sign to disable it.
- Cross-crate inlining works.

[internals thread]: https://internals.rust-lang.org/t/adding-latex-support-to-rustdoc/23858
[crates that did it themselves]: https://github.com/search?q=rustdoc-args+%3D+%5B%22--html-in-header%22%2C+%22katex-header.html%22%2C+%22--cfg%22%2C+%22docsrs%22%5D+language%3Atoml&type=code
[katex.js]: https://katex.org/
[include hack]: https://github.com/search?q=%23%21%5Bdoc+%3D+include_str%21%28%22katex.html%22%29%5D+language%3Arust&type=code

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

### How to enable

To enable `$\TeX$` math syntax in rustdoc, add this line to your crate root.

    #![doc(math_syntax)]

In a future edition, we may enable it by default. If you need to turn it off, add this line to your crate root.

    #![doc(no_math_syntax)]

When this feature is enabled, equations are wrapped in single or double `$` dollar signs.

    $$\sum_{i=0}^N x_i$$

The result looks like this:

> $$\sum_{i=0}^N x_i$$

A detailed comparison between our syntax and KaTeX's can be found
[here](https://tmke8.github.io/math-core/comparison.html).

You can add custom \commands by supplying key=value pairs to the math syntax attribute:

    #![doc(math_syntax(
        // usage: $\floor{x}$
        floor=r##"\delim{\lfloor}{#1}{\rfloor}"##,
    ))]

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

### Disabling and enabling math syntax

The crate-level doc attributes `math_syntax` and `no_math_syntax` enable and disable
support for parsing `$`-delimited TeX math in Rustdoc's Markdown.

Obviously, you can't set both of them at the same time. If neither of them are set,
rustdoc will use the edition-specific default (which is currently to disable it).

The `math_syntax` attribute accepts an optional list of `key="value"` pairs for
custom macros. This is similar to the `macros` parameter that KaTeX accepts,
but the `key` is an ident that only includes the name of the macro, without the backslash
or the number of parameters. The `"value"` is a string literal with TeX-like code,
and, optionally, `#`numbered parameter placeholders.

### Writing math code in markdown

Math expressions are wrapped in `$` signs. One dollar sign means "inline" math,
and two means "display" math.

Inline math cannot have any whitespace at the start or end of its contents,
so `$1$` is a math span, but `$ 1 $` is not. Inline math spans also
can't be empty.

Display math is allowed to have space at the start,
so `$$ 1 $$` is a display math span.

Unescaped curly braces within math spans must balance,
and unescaped dollar signs can only appear between unescaped curly braces,
so `$$ 1 {$} 2 $$` is parsed as a display math span,
but `$$ 1 $ 2 $$` and `$$ { $$` are not.

### Math syntax

Within a math span, whitespaces are used for grouping and formatting.
But you can't have more than one line break in a row within a math span,
because that ends the paragraph that contains it.

Other characters are usually rendered literally, except for

- backslashes, `\`, which are the sigil for commands
- curly braces, `{` and `}`, which are used for command arguments
- dollar signs, `$`, which delimit math spans
- number signs, `#`, which are used to refer to macro parameters
- ampersands, `&`, which are used for writing matrices and tables
- circumflex, `^`, which is used for exponents
- underscore, `_`, which is used for subscript
- single quote, `'`, which becomes the prime symbol
- tilde, `~`, which becomes a rendered, non-breaking space (since ordinary spaces are used for grouping)
- percent, `%`, which mark line comments
- NUL, which is not allowed

Commands are used to write things that can't easily be typed on a keyboard,
and for complex layouts like fractions and matrices. The math-core parser
that we use implements hundreds of commands.

<I>TODO: Full list is in <https://github.com/tmke8/math-core/blob/main/crates/math-core/src/commands.rs>.
Do I need to include it all here?</I>

## Drawbacks
[drawbacks]: #drawbacks

### There is no such thing as invalid Markdown

Adding new syntax to Rustdoc's Markdown is rough,
because it's so difficult to do without causing widespread breakage.
As spelled out in the [CommonMark spec][],
"any sequence of characters is a valid CommonMark document,"
so changing anything so that it acts like a metacharacter where it didn't used to
changes the behavior of already-valid documents;
a *breaking change.*

And, unlike when GitHub redesigned their Markdown as a CommonMark dialect,
we can't run a [one-time batch converter job][] over old crates.io crates [^ghmath].

This class of problem has come up when [intra-doc links were designed][],
when [pulldown-cmark was last updated][],
when [hoedown was replaced with pulldown-cmark in the first place][],
and when [anyone proposes replacing Markdown with something else][]
that has a "principled extension" system.

[^ghmath]:
    Did GitHub run a similar batch job when they added math syntax?
    I can't think of any reason why they wouldn't, but I also can't find any proof that they did.
    It seems like it would require running the math-enabled parser over all the issue comments,
    and, if it detects math, add a backslash in front of the dollar signs.
    After all, math syntax didn't exist in GitHub Issues until they added it,
    so any detected math span is, by definition, a false positive.

[CommonMark spec]: https://spec.commonmark.org/0.31.2/#characters-and-lines
[one-time batch converter job]: https://github.blog/engineering/a-formal-spec-for-github-markdown/#the-migration
[intra-doc links were designed]: https://github.com/rust-lang/rust/issues/54191
[pulldown-cmark was last updated]: https://github.com/rust-lang/rust/pull/121659#issuecomment-1992752820
[hoedown was replaced with pulldown-cmark in the first place]: https://internals.rust-lang.org/t/what-to-do-about-pulldown-and-commonmark/5115
[anyone proposes replacing Markdown with something else]: https://internals.rust-lang.org/t/rustdoc-restructuredtext-vs-markdown/356

### Verbosity or breakage as side effect

From the perspective of 99% of doc authors who didn't want to write a math span in the first place,
false positives that mangle their generated docs are a nasty papercut.
Failing to escape the dollar signs when you needed to is not as bad as [accidentally triggering a link refdef][],
since the degraded result might still be [legible][example rendering of a mistake],
but having to read English text without any spaces sucks.
Also, the LaTeX math syntax is forgiving enough that normal text is often valid,
so Rustdoc compiler warnings won't catch every accidental match.

But if we assume that every doc author adds the escapes that they need,
this forces doc comments to have more escaped metacharacters than they used to.
This makes doc comments less easily readable in their source form,
imposing a cost on the 99% that don't want the feature in favor of the 1% who do.

This argument, if taken to its logical extreme, would imply that we should use plain text
doc comments with no extra formatting features. The downside of doing that is
similar to the downside of not offering TeX math: users who *really* want bold text deploy
[unicode crimes][] and pictures of text, which create accessibility problems.

[accidentally triggering a link refdef]: https://github.com/rust-lang/rust/issues/133150
[example rendering of a mistake]: https://tmke8.github.io/math-core/#input:H4sIAAAAAAAAEwXBwQ0AIQgEwFa2guvCQvaBSoJIhETLv5n2uMIEu6P5MM2JODsF92iVONRBGEseFmsig_79a-7ZDjYAAAA=
[unicode crimes]: https://ux.stackexchange.com/questions/118149/can-screen-readers-interpret-unicode-styles-fonts-such-as-bold-and-italics

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

### Why TeX math in markdown, specifically?

I would like to avoid the annoying scenario where Rustdoc deploys a complicated, special purpose language, then the community moves on to some new, incompatible language, and we’re stuck maintaining it ourselves because of the stability promise.

There are a lot of special-purpose technical notations that we might theoretically want to support,
but TeX-math-in-markdown is special, for two reasons:

- [Lindy effect][]: LaTeX is an established standard that is not going anywhere any time soon.
- There is more than one implementation of the subset of LaTeX that we need.

The pull request I've been working on uses [math-core][], but, if that implementation turns out
to be problematic, we could pivot to another one, like [pulldown-latex][],
or [katex run in quick-js][] [^1].
That’s not an option with, for example, Typst.

[Lindy effect]: https://en.wikipedia.org/wiki/Lindy_effect
[math-core]: https://github.com/tmke8/math-core
[pulldown-latex]: https://github.com/carloskiki/pulldown-latex
[katex run in quick-js]: https://docs.rs/katex/latest/katex/

[^1]:
    I would prefer not to do *that*, because it's slow and seems to have poor error reporting,
    but, if we can't achieve good-enough feature support any other way, it's an option.

## Prior art
[prior-art]: #prior-art

- <https://github.com/cben/mathdown/wiki/math-in-markdown>
- <https://en.wikibooks.org/wiki/LaTeX/Mathematics>
- The span parsing is based on the [math spec for commonmark-hs][],
  which is the parser used if you run `pandoc` in `gfm` mode.
- Span parsing is documented in more detail in the [math spec for pulldown-cmark][].

[math spec for commonmark-hs]: https://github.com/jgm/commonmark-hs/blob/master/commonmark-extensions/test/math.md
[math spec for pulldown-cmark]: https://pulldown-cmark.github.io/pulldown-cmark/specs/math.html

## Unresolved questions
[unresolved-questions]: #unresolved-questions

### Avoiding Hyrum's Law

There are a lot of \commands in [math-core][], and some of them are known buggy,
meaning they don't match LaTeX itself.
We don't want authors to rely on those bugs, either accidentally
or in a workaround.

Normally, we might "phase in" new commands by making them unstable first,
letting more risk-tolerant authors try it out,
then make it available to everyone else.
But math-core doesn't have an API for that.

### Font

Right now, math formulas default to Noto Sans Math.

This was chosen because it's inoffensive and fine. But it is a sans serif font face, that will usually be surrounded by serif text.


## Future possibilities
[future-possibilities]: #future-possibilities

### Undelimited environments

It's a relatively rare feature, but Jupyter Notebook and a few others support
LaTeX environments introduced with the `\begin{foo}` / `\end{foo}` syntax
without wrapping dollar signs.
Since backslashes in Markdown only have meaning when followed by punctuation,
the false positives shouldn't be that common.
Since we don't have to worry about false positives,
we can treat it like a CommonMark block construct and allow blank lines in it.

    /// Computes sum from `start` to `end` of the given function.
    ///
    /// \begin{equation}
    /// \sum_{i=start}^{end}{f(i)}
    /// \end{equation}
    fn sum(f: impl FnMut(usize) -> usize, start: usize, end: usize) -> usize {
        let mut result = 0;
        for i in start..=end {
            result += f(i);
        }
        result
    }

### Drawing and charting syntax

There are a lot of different chart formats we *could* try to support.
The tough part is that we want to support it long-term,
give error messages at compile time (if the language has a concept of errors),
and, ideally, have a specification without much churn.

- [PlantUML][] is pretty much exactly what we would want.
  But we don't want to bundle a JRE.
- The other obvious choice is [Mermaid][], because GitHub supports it.
  The upside is that it's popular and terse. The downside is that the only existing
  implementation is a JavaScript library. We could copy in the JS library and embed
  the source code into our HTML, but we wouldn't be able to give syntax errors at
  Rustdoc compile time that way.
- If we supported undelimited LaTeX environment blocks, then it would make sense to
  implement a subset of the [LaTeX drawing tools][] on top of SVG.
  The downside is that the only other implementation of these languages that I know of is [LaTeXML][], which is not written in Rust.
  The upside is that integrating with the math engine lets you
  directly include equations and `math_syntax` macros in your graphics.
- [Svgbob][] actually has a Rust implementation. Ironic, since Svgbob has no syntax errors,
  it's actually less important to have a Rust implementation than it is for the others,
  which have the possibility of an "invalid document" with errors that we would want
  to report at compile time.

[PlantUML]: https://github.com/plantuml/plantuml
[LaTeX drawing tools]: https://en.wikibooks.org/wiki/LaTeX/Introducing_Procedural_Graphics
[LaTeXML]: https://en.wikipedia.org/wiki/LaTeXML
[Mermaid]: https://github.com/mermaid-js/mermaid
[Svgbob]: https://github.com/ivanceras/svgbob
