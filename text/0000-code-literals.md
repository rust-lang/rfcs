- Feature Name: code_literals
- Start Date: 2023-06-18
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Add a new kind of multi-line string literal for embedding code which
plays nicely with `rustfmt`.

# Motivation
[motivation]: #motivation

  - Embedding code as a literal string within a Rust program is often
    necessary. A prominent example is the `sqlx` crate, which
    has the user write SQL queries as string literals within the program.
  - Rust already supports several kinds of multi-line string literal,
    but none of them are well suited for embedding code.

    1.  Normal string literals, eg. `"a string literal"`. These can be
        written over multiple lines, but require special characters
        to be escaped. Whitespace is significant within the literal,
        which means that `rustfmt` cannot fix the indentation of the
        code block. For example, beginning with this code:

        ```rust
        if some_condition {
            do_something_with(
                "
                a nicely
                indented code
                string
                "
            );
        }
        ```

        If the indentation is changed, such as by removing the
        conditional, then `rustfmt` must re-format the code like so:

        ```rust
        do_something_with(
            "
                a nicely
                indented code
                string
                "
        );
        ```

        To do otherwise would be to change thange the value of
        the string literal.
    
    2.  Normal string literals with backslash escaping, eg.
        ```rust
        "
        this way\
        whitespace at\
        the beginning\
        of lines can\
        be ignored\
        "
        ```

        This approach still suffers from the need to escape special
        characters. The backslashes at the end of every line are
        tedious to write, and are problematic if whitespace is
        meaningful within the code. For example, if python code
        was being embedded, then the indentation would be lost.
        Finally, although `rustfmt` could in principle reformat
        these strings, in practice doing so in a reasonable way
        is complicated and so this has never been enabled by default.
    
    3.  Raw string literals, eg. `r#"I can use "s!"#`

        This solves the problem of special characters, but suffers
        from the same inability to be reformatted, and the trick
        of using an `\` at the end of each line cannot be applied
        because escape characters are not recognised.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

In addition to string literals and raw string literals, a third type
of string literal exists: code string literals.

```rust
    let code = ```
        This is a code string literal

        I can use special characters like "" and \ freely.

            Indentation is preserved *relative* to the indentation level
            of the first line.

    It is an error for a line to have "negative" indentation (ie. be
    indented less than the indentation of the opening backticks) unless
    the line is empty.
        ```;
```

`rustfmt` will automatically adjust the indentation of the code string
literal as a whole to match the surrounding context, but will never
change the relative indentation within such a literal.

Anything directly after the opening backticks is not considered
part of the string literal. It may be used as a language hint or
processed by macros (similar to the treatment of doc comments).

```rust
let sql = ```sql
    SELECT * FROM table;
    ```;
```

Similar to raw string literals, there is no way to escape characters
within a code string literal. It is expected that procedural macros
would build upon code string literals to add support for such
functionality as required.

If it is necessary to include triple backticks within a code string
literal, more than three backticks may be used to enclose the
literal, eg.

```rust
let code = ````
    ```
````;
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

A code string literal will begin and end with three or more backticks.
The number of backticks in the terminator must match the number used
to begin the literal.

The value of the string literal will be determined using the following
steps:

1.  Start from the first newline after the opening backticks.
2.  Take the string exactly as written until the closing backticks.
3.  Remove equal numbers of spaces or tabs from every non-empty line
    until the first character of the first non-empty line is neither
    a space nor a tab, or until every line is empty.
    Raise a compile error if this could not be done
    due to a "negative" indent or inconsistent whitespace (eg. if
    some lines are indented using tabs and some using spaces).

Here are some edge case examples:

```rust
    // Empty string
    assert_eq!(```foo
    ```, "");

    // Newline
    assert_eq!(```

    ```, "\n");

    // No terminating newline
    assert_eq!(```
        bar```, "bar");

    // Terminating newline
    assert_eq!(```
        bar
    ```, "bar\n");

    // Preserved indent
    assert_eq!(```
    if a:
        print(42)
    ```, "if a:\n    print(42)\n");

    // Relative indent
    assert_eq!(```
            if a:
                print(42)
    ```, "if a:\n    print(42)\n");

    // Relative to first non-empty line
    assert_eq!(```


            if a:
                print(42)
    ```, "\n\nif a:\n    print(42)\n");
```

The text between the opening backticks and the first newline is
preserved within the AST, but is otherwise unused.

# Drawbacks
[drawbacks]: #drawbacks

The main drawback is increased complexity of the language:

1. It adds a new symbol to the language, which was not previously used.
2. It adds a third way of writing string literals.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

There is lots of room to bike-shed syntax.
If there is significant opposition to the backtick syntax, then an
alternative syntax such as:
```
code"
    string
"
```
could be used.

Similarly, the use of more than three backticks may be unpopular.
It's not clear how important it is to be able to nest backticks
within backticks, but a syntax mirroring raw string literals could
be used instead, eg.
```
`# foo
    string
#`
```

There is also the question of whether the backtick syntax would
interfere with the ability to paste Rust code snippets into such
blocks. Experimentally, markdown parsers do not seem to have any
problems with this (as demonstrated in this document).

# Prior art
[prior-art]: #prior-art

The proposed syntax is primarily based on markdown code block syntax,
which is widely used and should be familiar to most programmers.


# Unresolved questions
[unresolved-questions]: #unresolved-questions

- None

# Future possibilities
[future-possibilities]: #future-possibilities

-   Macro authors could perform further processing
    on code string literals. These macros could add support for string
    interpolation, escaping, etc. without needing to further complicate
    the language itself.

-   Procedural macros could look at the text following the opening triple
    quotes and use that to influence code generation, eg.

    ```rust
    query!(```postgresql
        <query>
    ```)
    ```

    could parse the query in a PostgreSQL specific way.

-   Code literals could be used by crates like `html-macro`
    or `quote` to provide better surface syntax and faster
    compilation.

-   Code literals could be used with the `asm!` macro to avoid
    needing a new string on every line.
