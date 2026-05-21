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

Providing an easy way to build self-contained docs that work for non-docs.rs readers is
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

    #![doc(syntax="+tex_math_dollars")]

In a future edition, we may enable it by default. If you need to turn it off, add this line to your crate root.

    #![doc(syntax="-tex_math_dollars")]

The only supported values for `doc(syntax)` are `"+tex_math_dollars"` and `"-tex_math_dollars"`,
but we left the option open for future extensions.

When this feature is enabled, equations are wrapped in single or double `$` dollar signs.

    $$\sum_{i=0}^N x_i$$

The result looks like this:

> $$\sum_{i=0}^N x_i$$

A detailed comparison between our syntax and KaTeX's can be found
[here](https://tmke8.github.io/math-core/comparison.html).

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

### Disabling and enabling math syntax

The `doc(syntax="+tex_math_dollars"|"-tex_math_dollars")` attribute enables and disables
support for parsing `$`-delimited TeX math in Rustdoc's Markdown.

Obviously, you can't set both of them at the same time on a single item.
If neither of them are set,
rustdoc will use the edition-specific default (which is currently to disable it).

This attribute can be set on any item that accepts doc comments.
The syntax is inherited from the syntactic parent item.
So, to list a few examples:

```rust
// lib.rs

//! This is *not* math syntax: $x$

#[doc(syntax="-tex_math_dollars")]
pub mod bar;
#[doc(syntax="+tex_math_dollars")]
pub mod baz;

#[doc(syntax="+tex_math_dollars")]
/// This *is* math syntax: $x$
pub struct Foo;

impl Foo {
    /// This is *not* math syntax: $x$
    pub fn foo() {}
}

#[doc(syntax="+tex_math_dollars")]
impl Foo {
    /// This *is* math syntax: $x$
    pub fn bar() {}
}

#[doc(syntax="+tex_math_dollars")]
/// <https://github.com/rust-lang/rust/blob/29155a4cd6/src/librustdoc/visit_ast.rs#L454>
pub fn wizzywig() {
    #[doc(syntax="-tex_math_dollars")] //~ WARN doc syntax can only be declared on items that actually appear in documentation
    pub struct WizzyWig;

    impl Foo {
        /// This *is* math syntax: $x$
        pub fn baz() {}
    }
}
```

```rust
// bar.rs
#![doc(syntax="+tex_math_dollars")] //~ ERROR only one doc syntax can be declared on a single item
```

```rust
// baz.rs
//! This *is* math syntax
```

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
we can't run a [one-time batch converter job][] over old crates.io crates.

This class of problem has come up when [intra-doc links were designed][],
when [pulldown-cmark was last updated][],
when [hoedown was replaced with pulldown-cmark in the first place][],
and when [anyone proposes replacing Markdown with something else][]
that has a "principled extension" system.

[CommonMark spec]: https://spec.commonmark.org/0.31.2/#characters-and-lines
[one-time batch converter job]: https://github.blog/engineering/a-formal-spec-for-github-markdown/#the-migration
[intra-doc links were designed]: https://github.com/rust-lang/rust/issues/54191
[pulldown-cmark was last updated]: https://github.com/rust-lang/rust/pull/121659#issuecomment-1992752820
[hoedown was replaced with pulldown-cmark in the first place]: https://internals.rust-lang.org/t/what-to-do-about-pulldown-and-commonmark/5115
[anyone proposes replacing Markdown with something else]: https://internals.rust-lang.org/t/rustdoc-restructuredtext-vs-markdown/356

### Verbosity or breakage as side effect

From the perspective of 99% of doc authors who didn't want to write a math span in the first place,
false positives that mangle their generated docs are a nasty papercut.
Having to read English text without any spaces caused by failing to escape the dollar signs sucks
(though not as bad as [accidentally triggering a link refdef][],
as [the result][example rendering of a mistake] might still be legible).
The LaTeX math syntax is forgiving enough that English prose is often syntactically valid LaTeX,
so Rustdoc invalid LaTeX warnings can't catch every unescaped dollar sign.

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

### Why LaTeX math in markdown, specifically?

I would like to avoid the annoying scenario where Rustdoc deploys a complicated, special purpose language,
then the community moves on to some new, incompatible language,
and we’re stuck maintaining it ourselves because of the stability promise.

There are a lot of special-purpose technical notations that we might theoretically want to support,
but LaTeX-math-in-markdown is special, for two reasons:

- [Lindy effect][]: LaTeX is an established standard that is not going anywhere any time soon.
- There is more than one implementation of the subset of LaTeX that we need.

The pull request I've been working on uses [math-core][], but, if that implementation turns out
to be problematic, we could pivot to another one, like [pulldown-latex][],
or [katex run in quick-js][] [^1].

[Lindy effect]: https://en.wikipedia.org/wiki/Lindy_effect
[math-core]: https://github.com/tmke8/math-core
[pulldown-latex]: https://github.com/carloskiki/pulldown-latex
[katex run in quick-js]: https://docs.rs/katex/latest/katex/

[^1]:
    I would prefer not to do *that*, because it's slow and seems to have poor error reporting,
    but, if we can't achieve good-enough feature support any other way, it's an option.

<table>
<caption>Comparison with other math equation formats</caption>
<thead>
<tr>
  <th>Name</th>
  <th>Release date</th>
  <th>Rust implementation</th>
  <th>Implementation-agnostic specification</th>
  <th>Markdown embedding standard</th>
  <th>Easy to type on a keyboard</th>
  <th>Popularity</th>
</tr>
</thead>
<tbody>
<tr>
  <th>LaTeX</th>
  <td>1984</td>
  <td>✅</td>
  <td>❎</td>
  <td>✅</td>
  <td>✅</td>
  <td>High</td>
</tr><tr>
  <th>Typst</th>
  <td>2019</td>
  <td>✅</td>
  <td>❎</td>
  <td>❎</td>
  <td>✅</td>
  <td>Moderate</td>
</tr><tr>
  <th>AsciiMath</th>
  <td>2009</td>
  <td><a href="https://docs.rs/asciimath-rs/latest/asciimath_rs/">✅</a></td>
  <td><a href="https://asciimath.org/index-mathml.html#syntax">✅</a></td>
  <td>❎</td>
  <td>✅</td>
  <td>Moderate</td>
</tr><tr>
  <th>UnicodeMath</th>
  <td>2006</td>
  <td><a href="https://murrayiii.github.io/UnicodeMathML/playground/">❎</a></td>
  <td><a href="https://www.unicode.org/notes/tn28/">✅</a></td>
  <td>✅</td>
  <td>❎</td>
  <td>I would need access to <a href="https://support.microsoft.com/en-us/office/linear-format-equations-using-unicodemath-and-latex-in-word-2e00618d-b1fd-49d8-8cb4-8d17f25754f8">Microsoft Office</a> feature analytics to answer this question.</td>
</tr>
</tbody>
</table>

### Why use a Rust library, specifically, for the implementation?

If your LaTeX syntax doesn't parse,
you'll get a warning at Rustdoc compile time,
just like you do with broken intra-doc links:

    error: unknown command "\frobnicate"
      --> $DIR/basic.rs:6:5
       |
    LL | //! $\frobnicate{2}$
       |     ^^^^^^^^^^^^^^^^
       |
    note: the lint level is defined here
      --> $DIR/basic.rs:2:9
       |
    LL | #![deny(rustdoc::invalid_math)]
       |         ^^^^^^^^^^^^^^^^^^^^^
    
    error: aborting due to 1 previous error

That's harder if the engine is written in a different
language than the compiler.

### Why target MathML

[MathML Core][] is supported by [most browsers][caniuse mathml],
but with a [few known bugs][mathml bugs] that the math-core library tries to work around.
It is a strict accessibility improvement over rendering SVG, for example.

[MathML Core]: https://www.w3.org/TR/mathml-core/
[caniuse mathml]: https://caniuse.com/mathml
[mathml bugs]: https://github.com/tmke8/math-core/issues/209

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

We should also look into [subsetting](https://github.com/tmke8/math-core#font-subsetting) the font. 


## Future possibilities
[future-possibilities]: #future-possibilities

### Extending `doc(syntax)`

The name `tex_math_dollars` is deliberately the same [name and syntax used by Pandoc](https://pandoc.org/MANUAL.html#extension-tex_math_dollars) for this feature.
If we add more syntactic features, we can follow the same pattern. An example of how to build on this:

```ebnf
(* if no language is supplied, the default is "rustdoc_markdown", so "+tex_math_dollars" is synonymous with "rustdoc_markdown+tex_math_dollars" *)
doc syntax = [ language ], { ( "+" | "-" ), extension } ;
(* "commonmark" enables no extensions (you can, of course, add them)
  "rustdoc_markdown" is synonymous with `commonmark+intra_doc_links+doctests+smart+pipe_tables+strikeout+footnotes+task_lists"
  "gfm" is synonymous with `commonmark+smart+tex_math_dollars+pipe_tables+strikeout+footnotes+task_lists+emoji+tex_math_gfm+alerts+autolink_bare_uris+yaml_metadata_block"
  "typst" doesn't support any extensions except "intra_doc_links" and "doctests", so, for example, `doc(syntax="typst+pipe_tables")` is an error *)
language = "commonmark" | "rustdoc_markdown" | "gfm" | "typst" ;
extension = 
   "intra_doc_links"
 | "doctests"
 | "smart"
 | "pipe_tables"
 | "strikeout"
 | "footnotes"
 | "task_lists"
 | "tex_math_dollars"
 | "emoji"
 | "tex_math_gfm"
 | "alerts"
 | "autolink_bare_uris"
 | "yaml_metadata_block" ;
```

If a parent and child element both have `doc(syntax)` attributes, they aren't merged. The child just overrides the parent.

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

### Custom macros

Some implementations, such as [Pandoc's `latex_macros` extension][],
let you write LaTeX macro definitions directly in Markdown.
For example, these two doc comments are equivalent:

```rust
#[doc(syntax="+tex_math_dollars+latex_macros")]
/// \newcommand{\tuple}[1]{\langle #1 \rangle}
///
/// $\tuple{a, b, c}$
fn foo_bar() {}

#[doc(syntax="+tex_math_dollars+latex_macros")]
/// $\langle a, b, c \rangle$
fn foo_bar() {}
```

Existing users of rustdoc and katex already write custom functions.
This isn't a niche feature.

[Pandoc's `latex_macros` extension]: https://pandoc.org/MANUAL.html#extension-latex_macros

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
