- Start Date: 2014-07-21
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Unify and sweeten the syntax for attributes and macros using the `@foo` notation.

# Motivation

Currently, attributes and macros/syntax extensions are both conceptually macros: they are user-definable syntactic extensions that transform token trees to token trees. However, their syntaxes are quite different: at present, attributes use `#[attr()]`, while macros use `macro!()`. By switching to a uniform syntax, `@attr()` and `@macro()`, we can emphasize their similarity and save syntactic space. At the same time, we reduce the verbosity of attributes and make them more like other languages by using the `@` notation.

The `!` and `#` notation take up syntactic space that we may want to use elsewhere. For example, RFC #204 suggests using `!` for a type assertion.

At least the following languages use `@` notation for attributes: Java (annotations), Python (decorators), D, Dart (metadata), Scala (annotations), and Swift. Languages have generally chosen either `@` or `[]` (brackets) to represent attributes; we cannot choose the latter because of ambiguity with array literals.

Julia uses `@` for macros.

Objective-C uses `@` to indicate special notation not in the C language. Since Objective-C is not a macro expander but is a full-fledged compiler, this is not directly analogous. But, in the author's opinion, the `@` sigil has a similar feel in Objective-C.

# Detailed design

The following syntactic changes occur:

* `#[inline]` → `@inline`
* `#[inline(never)]` → `@inline(never)`
* `#[deprecated="May discolor some fabrics"]` → `@deprecated="May discolor some fabrics"`
* `println!("Hello {}", "Niko")` → `@println("Hello {}", "Niko")`
* `vec!["spam", "eggs", "bacon"]` → `@vec["spam", "eggs", "bacon"]`
* `bitflags! { flags Flags: u32 ... }` → `@bitflags { flags Flags: u32 ... }`

Parsing is slightly complicated because, where an item is expected, the parser does not know whether an item macro or an attribute is next after parsing the leading `@`, identifier, and `(`. Therefore, the parser parses a series of parenthesis-delimited token trees in these cases, and looks at the next token following the `)` to determine whether it parsed an item macro or an attribute. If the next token is `;`, it considers what it just parsed an item. Otherwise, it reinterprets what it just parsed as an attribute.

# Drawbacks

* The beauty of `@` vis-à-vis `#`/`!` is in the eye of the beholder.

* The complication of parsing increases the complexity of the language somewhat and may affect syntax highlighting. (However, this is mitigated to some degree because macros and attributes are already difficult to syntax highlight as a result of their free-form syntax.)

# Alternatives

There are innumerable other syntaxes one could consider. Unadorned brackets for attributes, C#-style (e.g. `[inline(never)]`), does not seem possible to reconcile with our array syntax.

The impact of not doing this is that the current syntax will remain.

# Unresolved questions

* `@deprecated="foo"` may be ugly. Should we do anything about this? One possibility is to switch to `@deprecated("foo")`, which is more consistent anyhow.
