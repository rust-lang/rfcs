- Feature Name: `unprefixed_guarded_strings`
- Start Date: 2024-03-24
- RFC PR: [rust-lang/rfcs#3593](https://github.com/rust-lang/rfcs/pull/3593)
- Tracking Issue: [rust-lang/rust#123735](https://github.com/rust-lang/rust/issues/123735)

# Summary
[summary]: #summary

Beginning with the 2024 edition, reserve the syntax `#"foo"#`, as a way of future-proofing against future language changes.

# Motivation
[motivation]: #motivation

[RFC 3101](https://github.com/rust-lang/rfcs/blob/master/text/3101-reserved_prefixes.md) reserved, among other things, all ident-prefixed strings like `ident"foo"` and `ident##"foo"##`. Despite these prefixes not conflicting with basic Rust grammar, reserving various prefixes avoids future macro breakage.

Reserving all identifier prefixes covers a large swath of future possibilities, but one edge case was not included: unprefixed "guarded" string literals.

```rust
// Basic string literal
"bar";
// Prefixed string literal
r"foo";
// Prefixed guarded string literal
r#"foo"#;
// Unprefixed guarded string literal
#"foo"#; // not yet reserved
```

[RFC 3475](https://github.com/rust-lang/rfcs/pull/3475) proposes to use this syntax for a new kind of string literal, reserving the syntax in Edition 2024. However, it is unlikely that RFC will be merged before Edition 2024. It could be declined, leaving that syntax for an entirely different proposal. In order to enable usage of this syntax in the future without waiting for the next edition boundary, we propose reserving `#"foo"#` syntax independently in this RFC.

Just like in RFC 3101, we must reserve this syntax across an edition boundary to avoid breaking macros. This reservation would be mostly unobservable by end-users and would only manifest in code using macros like so:

```rust
macro_rules! demo {
    ( $a:tt ) => { println!("one token") };
    ( $a:tt $b:tt $c:tt ) => { println!("three tokens") };
}

demo!("foo");
demo!(r#"foo"#);
demo!(#"foo"#);
```

Prior to the 2024 edition, this produces:
```
one token
one token
three tokens
```

Following the 2021 edition, `#"foo"#` would become a compiler error.

Note that this syntactic reservation is whitespace-sensitive: any whitespace to either side of the intervening `#` will allow this code to compile. This provides a simple migration path for anyone who would be impacted by this change; they would need only change their macro invocations from `foo!(#"qux"#)` to `foo!(# "qux" #)` or `foo!(# "qux"#)`. It is possible to automate this mechanical migration via rustfix.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When designing DSLs via macros that take token trees as inputs, be aware that certain syntactic productions which have no meaning in Rust are nonetheless forbidden by the grammar, as they represent "reserved space" for future language development. In addition to the `<identifier>#<identifier>`, `<identifier>"<string contents>"`, `<identifier>'<char contents>'`, and `<identifier>#<numeric literal>` forms reserved in Edition 2021, `#"<string contents>"` (with any number of leading `#`) is reserved for future use by the language.

Note that this syntax relies on the absence of whitespace, so a macro invocation can use `# "<string contents>"` (note the space) as a way to consume string literal tokens adjacent to a `#`.

Putting it all together, this means that the following are valid macro invocations:

* `foo!("qux")`
* `foo!("qux"#)`
* `foo!(r#"qux"#)`
* `foo!(# "qux")`

...but the following are invalid macro invocations:

* `foo!(#"qux")`
* `foo!(#"qux"#)`
* `foo!(####"qux"####)`

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation


New tokenizing rules are introduced:

> RESERVED_GUARDED_STRING_LITERAL : `#`<sup>+</sup> STRING_LITERAL

When compiling under the Rust 2024 edition (as determined by the edition of the current crate), any instance of the above produces a tokenization error.

An edition migration may be implemented that looks for `#"string"#`, etc. within macro calls and inserts whitespace to force proper tokenization.

What follows are some examples of suggested error message templates:
```
error: invalid string literal
 --> file.rs:x:y
  |
1 | foo!(#"qux"#);
  |      ^^^^^^^ help: try using whitespace here: `# "qux" #`
  |
  = note: unprefixed guarded string literals are reserved for future use
```

# Drawbacks
[drawbacks]: #drawbacks

* Complicates macro tokenizing rules.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- Just merge [RFC 3475: Unified String Literals](https://github.com/rust-lang/rfcs/pull/3475) instead. That RFC is a strict superset of this RFC.

# Prior art
[prior-art]: #prior-art

* [RFC 3101: Reserved prefixes in the 2021 edition](https://github.com/rust-lang/rfcs/blob/master/text/3101-reserved_prefixes.md)
* [Swift: Extended String Delimiters](https://docs.swift.org/swift-book/documentation/the-swift-programming-language/stringsandcharacters/#Extended-String-Delimiters)
