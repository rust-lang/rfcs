
- Feature Name: reserved_prefixes
- Start Date: 2021-03-31
- RFC PR: [rust-lang/rfcs#3101](https://github.com/rust-lang/rfcs/pull/3101)
- Rust Issue: [rust-lang/rust#84978](https://github.com/rust-lang/rust/issues/84978)

# Summary
[summary]: #summary

Beginning with the 2021 edition, reserve the syntax `ident#foo`, `ident"foo"`, `ident'f'`, and `ident#123`, as a way of future-proofing against future language changes.

# Motivation
[motivation]: #motivation

In [RFC 2151](https://rust-lang.github.io/rfcs/2151-raw-identifiers.html), the language syntax was expanded to allow identifiers to optionally be prefixed with `r#`, to ease migrating code when new keywords are introduced. Conversely, [RFC 3098](https://github.com/rust-lang/rfcs/pull/3098) (still under discussion as of this writing) is proposing to allow keywords to be prefixed with `k#`, as an unobtrusive way to introduce new keywords without requiring any migration effort or edition-level coordination.

In almost all circumstances these are frictionless additions; there is no place in the basic Rust grammar that would conflict with productions of the form `foo#bar`. However, there is a minor wrinkle with regard to macros. Consider the following code:
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

The `r#` prefix for raw identifiers was originally chosen because it exploited a quirk of the parser, which prevented any code containing `r#foo` from compiling due to the parser believing that it was processing a raw string literal.

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

This would be a breaking change, which is why RFC 3098 is currently aiming to be implemented across an edition. However, the time-sensitivity of that RFC could be obviated if the language merely guaranteed that such space was syntactically available. Therefore this RFC proposes reserving such syntactic space, without attaching any semantic meaning to it, to accommodate both the "raw keywords" proposal or any other future language changes that would benefit.

The notion of reserving "syntactic space" as an aid to backwards-compatibility is an idea that has precedence from other languages. [C reserves large swathes of the identifier space](https://www.gnu.org/software/libc/manual/html_node/Reserved-Names.html) for its own use, most notably identifiers that begin with `_` or `__`. Likewise, Python reserves all identifiers of the form `__foo__` for special use by the language.

In contrast to Python or C, reserving syntax via `#` rather than `_` is much less of an imposition on ordinary users, because `#` is not a valid character in Rust identifiers. The only contexts in which this change would be observable is within macros: `foo!(bar#qux)` would now fail to lex (a.k.a. tokenize). As such, the above code would produce the following compilation error (wording TBD) when upgrading to the 2021 edition:
```
error: unknown prefix on identifier: a#
 --> tokens.rs:7:7
  |
7 | demo!(a#foo);
  |       ^^ help: try using whitespace here: `a # foo`
  |
  = note: prefixed identifiers are reserved for future use
```

Note that this syntactic reservation is whitespace-sensitive: any whitespace to either side of the intervening `#` will allow this code to compile. This provides a simple migration path for anyone who would be impacted by this change; they would need only change their macro invocations from `foo!(bar#qux)` to any of `foo!(bar # qux)`, `foo!(bar# qux)`, or `foo!(bar #qux)`. It is possible to automate this mechanical migration via rustfix.

Rather than try to guess what prefixes it might be useful to reserve, this RFC reserves *all* [identifiers](https://doc.rust-lang.org/reference/identifiers.html) directly preceding a `#`. This has the following benefits:

1. It increases the amount of leeway for future language changes that might wish to use this space (e.g. a hypothetical mechanism for edition-specific keywords might be written as `edition2015#use`).
2. It has symmetry with the existing notion of [literal suffixes](https://doc.rust-lang.org/reference/tokens.html#suffixes).
3. It avoids complicating the grammar and parser with bespoke concepts.

Finally, this RFC also proposes that this same syntax be reserved for string literals (`ident"foo"`), char literals (`ident'f'`), and numeric literals (`ident#123`). Once again, this reservation would be mostly unobservable by end-users and would only manifest in code using macros like so:

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

Following the 2021 edition, these would become compiler errors. Once again, whitespace could be (automatically) inserted to mitigate any breakage.

The motivation here, aside from the symmetry with prefixed identifiers and [literal suffixes](https://doc.rust-lang.org/reference/tokens.html#suffixes), would be to leave open the design space for new literal prefixes along the lines of the existing `b"` and `r"` prefixes. Some hypothetical examples (not necessarily planned features or planned syntax): format string literals `f"`, `String` literals `s"`, `CString` literals `c"`,  `OsString` literals `o"`, UTF-16 literals `w"`, user-overloadable string literals `x"`, etc.

There is one subtle note to this reservation: because raw string literals and string literals tokenize differently, any prefix ending in `r` will tokenize as a raw string literal would tokenize, and any prefix not ending in `r` will tokenize as a non-raw string literal would tokenize. This is considered acceptable in that it is assumed that new prefixes on these literals will be "compositional" in nature, in the same sense that `b` and `r` on string literals compose today, and thus it will be natural and intentional to compose any such prefix with `r` in order to achieve raw string semantics when desired. However, any hypothetical *non*-compositional prefix would need to be chosen carefully in order to achieve its desired tokenization

# Guide-level explanation
When designing DSLs via macros that take token trees as inputs, be aware that certain syntactic productions which have no meaning in Rust are nonetheless forbidden by the grammar, as they represent "reserved space" for future language development. In particular, anything of the form `<identifier>#<identifier>`, `<identifier>"<string contents>"`, `<identifier>'<char contents>'`, and `<identifier>#<numeric literal>` is reserved for exclusive use by the language; these are called *reserved prefixes*.

Unless a prefix has been assigned a specific meaning by the language (e.g. `r#async`, `b"foo"`), Rust will fail to tokenize when encountering any code that attempts to make use of such prefixes. Note that these prefixes rely on the absence of whitespace, so a macro invocation can use `<identifier> # <identifier>` (note the spaces) as a way to consume individual tokens adjacent to a `#`.

Putting it all together, this means that the following are valid macro invocations:

* `foo!(r#async)`,
* `foo!(b'x')`
* `foo!(bar # qux)`,
* `foo!(bar #123)`
* `foo!(bar# "qux")`

...but the following are invalid macro invocations:

* `foo!(bar#async)`
* `foo!(bar#123)`
* `foo!(bar"qux")`

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

New tokenizing rules are introduced:

> RESERVED_IDENTIFIER : IDENTIFIER_OR_KEYWORD<sub>Except `r`</sub> `#` IDENTIFIER_OR_KEYWORD
>
> RESERVED_BYTE_LITERAL : IDENTIFIER_OR_KEYWORD BYTE_LITERAL
>
> RESERVED_CHAR_LITERAL : IDENTIFIER_OR_KEYWORD<sub>Except `b`</sub> CHAR_LITERAL
>
> RESERVED_RAW_BYTE_STRING_LITERAL : IDENTIFIER_OR_KEYWORD RAW_BYTE_STRING_LITERAL
>
> RESERVED_BYTE_STRING_LITERAL : IDENTIFIER_OR_KEYWORD BYTE_STRING_LITERAL
>
> RESERVED_RAW_STRING_LITERAL : IDENTIFIER_OR_KEYWORD<sub>Except`b`</sub> RAW_STRING_LITERAL
>
> RESERVED_STRING_LITERAL : IDENTIFIER_OR_KEYWORD<sub>Except `b`, `r`, `br`</sub> STRING_LITERAL
>
> RESERVED_NUMERIC_LITERAL : IDENTIFIER_OR_KEYWORD `#` (INTEGER_LITERAL | FLOAT_LITERAL)

When compiling under the Rust 2021 edition (as determined by the edition of the current crate), any instance of the above produces a tokenization error.

The use of "identifier" in this document proactively refers to whatever definition of "identifier" is in use by Rust as of the 2021 edition. At the time of this writing, the `non_ascii_idents` feature is not yet stabilized, but is on track to be. If `non_ascii_idents` is stabilized before the 2021 edition, then the syntactic reservations that take place in the 2021 edition will include things like `über#foo`. However, if `non_ascii_idents` is *not* stabilized before the 2021 edition, then any subsequent stabilization of `non_ascii_idents` would need to take care to *not* expand the reservations in this RFC, and instead defer that task to the next edition.

An edition migration may be implemented that looks for `ident#ident`, `ident"string"`, etc. within macro calls and inserts whitespace to force proper tokenization.

What follows are some examples of suggested error message templates:
```
error: unknown prefix on identifier: bar#
 --> file.rs:x:y
  |
1 | bar#qux;
  | ^^^^
  |
```
```
error: unknown prefix on identifier: bar#
 --> file.rs:x:y
  |
1 | foo!(bar#qux);
  |      ^^^^ help: try using whitespace here: `bar # qux`
  |
  = note: prefixed identifiers are reserved for future use
```
```
error: unknown prefix on string literal: bar
 --> file.rs:x:y
  |
1 | foo!(bar"qux");
  |      ^^^ help: try using whitespace here: `bar "qux"`
  |
  = note: prefixed string literals are reserved for future use
```

# Drawbacks
[drawbacks]: #drawbacks

* Complicates macro tokenizing rules.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

* Reserve only prefixed identifiers, and not prefixed literals. The former has a concrete RFC that would benefit from this, but the latter is currently just aspirational.
* Instead of `ident`, reserve only `[a-z]+` or `[a-z]`. However, reserving only `[a-z]` would force future language extensions to use exclusively pithy single-letter syntax, even for features that may not be common enough to warrant such abbreviated syntax. Reserving identifiers in this space provides more flexibility for future language design, without impacting Rust programs.
* Instead of `ident`, reserve only `[a-zA-Z_][a-zA-Z0-9_]*`, the set of ASCII-only identifiers. This would cover the space future Rust language design extensions are likely to use. However, the explanation of the reserved space would require presenting a distinct concept separate from the definition of identifiers. In addition, reserving only ASCII identifiers seems unlikely to provide a benefit to future Rust programs.
* Instead of `ident`, reserve prefixes that permit any sequence of identifier continuation characters. This would allow things like preceding digits, e.g. `4#foo`.

#  Unresolved questions
[unresolved-questions]: #unresolved-questions

* How to manage the API `proc_macro::TokenStream::from_str`, which does not take any edition information? ([raised here](https://github.com/rust-lang/rfcs/pull/3101#issuecomment-832686934))

# Prior art
[prior-art]: #prior-art

* [C: Reserved names](https://www.gnu.org/software/libc/manual/html_node/Reserved-Names.html)
* [Python: Reserved classes of identifiers](https://docs.python.org/3/reference/lexical_analysis.html#reserved-classes-of-identifiers)
* [Ada: Based literals](archive.adaic.com/standards/83lrm/html/lrm-02-04.html#2.4.2)
* [Emacs Calc: Integers](https://www.gnu.org/software/emacs/manual/html_mono/calc.html#Integers)

