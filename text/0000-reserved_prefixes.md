- Feature Name: reserved_prefixes
- Start Date: 2021-03-31
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Beginning with the 2021 edition, in all contexts, reserve the syntax `ident#foo` and `ident"foo"`, as a way of future-proofing against future language changes.

# Motivation
[motivation]: #motivation

In [RFC 2151](https://rust-lang.github.io/rfcs/2151-raw-identifiers.html), the language syntax was expanded to allow identifiers to optionally be prefixed with `r#`, to ease migrating code when new keywords are introduced. Conversely, [RFC 3098](https://github.com/rust-lang/rfcs/pull/3098) (still under discussion as of this writing) is proposing to allow keywords to be prefixed with `k#`, as an unobtrusive way to introduce new keywords without requiring any migration effort or edition-level coordination.

In almost all circumstances these are frictionless additions; there is no place in the basic Rust grammar that would conflict with productions of the form `foo#bar`. However, there is a wrinkle with regard to macros. Consider the following code:
```rust
macro_rules! demo {
    ( $a:tt ) => { println!("one token") };
    ( $a:tt $b:tt $c:tt ) => { println!("three tokens") };
}

demo!(a#foo);
demo!(r#foo);
demo!(k#foo);
```

Prior to Rust 1.30 and the stabilization of raw identifiers (RFC 2151), the above code would have produced the following compiler error:
```
error: found invalid character; only `#` is allowed in raw string delimitation: f
 --> tokens.rs:8:7
  |
8 | demo!(r#foo);
  |       ^^
```

The `r#` prefix for raw identifiers was chosen because it exploited a quirk of the parser, which prevented any code containing `r#foo` from compiling due to the parser believing that it was processing a raw string literal.

After Rust 1.30 , it prints the following:
```
three tokens
one token
three tokens
```

If RFC 3098 were accepted, it would print the following:
```
three tokens
one token
one token
```

This would be a breaking change, which is why RFC 3098 is currently aiming to be implemented across an edition. However, the time-sensitivity of that RFC could be obviated if the language merely guaranteed that such space was syntactically available. Therefore, this RFC proposes merely reserving such syntactic space, without attaching any semantic meaning to it, to accommodate both the "raw keywords" proposal and any other future changes that would benefit.

As further motivation, the notion of reserving "syntactic space" as an aid to backwards-compatibility is an idea that has precedence from other languages. [C reserves large swathes of the identifier space](https://www.gnu.org/software/libc/manual/html_node/Reserved-Names.html) for its own use, most notably identifiers that begin with `_` or `__`. Likewise, Python reserves all identifiers of the form `__foo__` for special use by the language.

In contrast to Python or C, reserving syntax via `#` rather than `_` is much less of an imposition on ordinary users, because `#` is not a valid character in identifiers. The only contexts in which this change would be observable is within macros: `foo#bar` would now lex as one token rather than three. As such, the above code would produce the following when compiled on the 2021 edition:
```
one token
one token
one token
```

Note that this syntactic reservation is whitespace-sensitive: any whitespace to either side of the intervening `#` will cause three tokens to be produced rather than one. This provides a simple migration path for anyone who would be impacted by this change; they would need only change their macro invocations from `foo!(bar#qux)` to any of `foo!(bar # qux)`, `foo!(bar# qux)`, or `foo!(bar #qux)`.

This RFC goes beyond merely reserving the prefix `k#` and reserves all identifiers directly preceding the `#`. This has the following benefits:

1. It increases the amount of leeway for future language changes that might wish to use this space (e.g. a hypothetical mechanism for edition-specific keywords might be written as `edition2015#use`).
2. It has symmetry with the existing notion of [literal suffixes](https://doc.rust-lang.org/reference/tokens.html#suffixes).
3. It avoids complicating the grammar and parser with bespoke concepts.

Finally, this RFC also proposes that this same syntactic reservation be applied to string literals (including raw string literals), whose syntax was the original inspiration for this design space. Once again, this reservation would be mostly unobservable by end-users and would only manifest in code using macros like so:

```rust
macro_rules! demo {
    ( $a:tt ) => { println!("one token") };
    ( $a:tt $b:tt ) => { println!("two tokens") };
    ( $a:tt $b:tt $c:tt $d:tt ) => { println!("four tokens") };
}

demo!(br"foo");
demo!(bar"foo");
demo!(bar#"foo"#);
```

Prior to the 2021 edition, this produces:
```
one token
two tokens
four tokens
```

Following the 2021 edition, all would be lexed as one token. Once again, whitespace could be inserted to mitigate any breakage.

The motivation in this case, aside from the symmetry with prefixed identifiers, would be to leave open the design space for additional string literal prefixes other than `b"` and `r"`, e.g. hypothetical format string literals `f"`, `String` literals `s"`, `CString` literals `c"`,  `OsString` literals `o"`, UTF-16 literals `w"`, user-overloadable string literals `x"`, etc., and any sensible combinations of these.

# Guide-level explanation
When writing macros that accept token trees as inputs, be aware that certain grammatical productions which have no meaning in Rust are nonetheless accepted by the grammar, as they represent "reserved space" for future Rust language development. This behavior can be observed by macros that consume token trees. In particular, anything of the form `<identifier>#<identifier>` or `<identifier>"<string contents>"` will be consumed by a macro as a single token tree. This is in contrast to, for example, `<identifier>@<identifier>`, which would be consumed as three token trees, as `@` does not indicate reserved syntax in the way that `#` does. These single tokens rely on the absence of whitespace, so a macro invocation can use `<identifier> # <identifer>` (note the spaces) as a way to consume individual tokens adjacent to a `#`.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

New tokenizer rules are introduced:

> RESERVED_IDENTIFIER : IDENTIFIER_OR_KEYWORD<sub>Except `r`</sub> `#` IDENTIFIER_OR_KEYWORD
>
> RESERVED_STRING_LITERAL : IDENTIFIER_OR_KEYWORD RAW_BYTE_STRING_LITERAL | IDENTIFIER_OR_KEYWORD BYTE_STRING_LITERAL | IDENTIFIER_OR_KEYWORD<sub>Except `b`</sub> RAW_STRING_LITERAL | IDENTIFIER_OR_KEYWORD<sub>Except `b`, `r`, `br`</sub> STRING_LITERAL

Any use of a reserved identifier or reserved string literal is a compilation error.

The use of "identifier" in this document should be taken to refer to the definition of "identifier" that is in use by Rust as of the 2021 edition. At the time of this writing, the `non_ascii_idents` feature is not yet stabilized, but is on track to be. If `non_ascii_idents` is stabilized before the 2021 edition, then the syntactic reservations that take place in the 2021 edition will include things like `über#foo`. However, if `non_ascii_idents` is *not* stabilized before the 2021 edition, then any subsequent stabilization of `non_ascii_idents` would need to take care to *not* expand the reservations in this RFC, and instead defer that task to the next edition.

An edition migration will be implemented that looks for `ident#ident` or `ident"string"` within macro calls and inserts whitespace to force individual tokenization.

# Drawbacks
[drawbacks]: #drawbacks

* Complicates macro tokenizing rules.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

* Reserve only `ident#foo` and not `ident"foo"`. The former has a concrete RFC that would benefit from this, but the latter is currently just aspirational.
* Instead of `ident`, reserve only `[a-z]+` or `[a-z]`. However, it would probably be even more surprising if `x#foo` were considered one token and `X#foo` or `xx#foo` were considered three.
* Instead of `ident`, reserve prefixes that permit any sequence of identifier continuation characters. This would allow things like preceding digits, e.g. `4#foo`.
* In addition to adding reserved prefixes to string literals, add them to numeric literals as well: `ident#1234`, `ident#56.78`.

#  Unresolved questions
[unresolved-questions]: #unresolved-questions

* Is the automatic migration possible to implement? I'm unclear if rustfmt, and hence rustfix, is capable of peering inside of macro invocations.

# Prior art
[prior-art]: #prior-art

* [C: Reserved names](https://www.gnu.org/software/libc/manual/html_node/Reserved-Names.html)
* [Python: Reserved classes of identifiers](https://docs.python.org/3/reference/lexical_analysis.html#reserved-classes-of-identifiers)
