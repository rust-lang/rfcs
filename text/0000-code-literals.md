- Feature Name: code_literals
- Start Date: 2023-06-18
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Add a new kind of multi-line string literal for embedding code which
plays nicely with `rustfmt` and doesn't introduce unwanted whitespace
into multi-line string literals.

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

  - The existing string literals introduce extra unwanted whitespace
    into the literal value. Even if that extra whitespace does not
    semantically affect the nested code, it results in ugly output
    if the code is ever logged (such as might happen when logging
    SQL query executions).

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

In addition to string literals and raw string literals, a third type
of string literal exists: code string literals.

```rust
    let code = ```
        This is a code string literal

        I can use special characters like "" and \ freely.

            Indentation is preserved *relative* to the indentation level
            of the terminating triple backticks.

    It is an error for a line to have "negative" indentation (ie. be
    indented less than the final triple backticks) unless
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

In order to suppress the final newline, the literal may instead be
closed with `!``` `, eg.

```rust
let code = ```
    Text with no final newline
    !```;
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

A code string literal will begin and end with three or more backticks.
The number of backticks in the terminator must match the number used
to begin the literal.

The value of the string literal will be determined using the following
steps:

1.  Measure the whitespace indenting the closing backticks. If a
    non-whitespace character (other than a single `!`) exists before
    the closing backticks on the same line, then issue a compiler error.
2.  Take the lines *between* (but not including) the opening and
    closing backticks exactly as written.
3.  Remove exactly the measured whitespace from each line. If this
    cannot be done, then issue a compiler error.
4.  If the string was terminated with `!``` `, then remove the
    final newline.

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
        bar
        !```, "bar");

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

    // Relative to closing backticks
    assert_eq!(```


            if a:
                print(42)
    ```, "\n\n        if a:\n            print(42)\n");
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

## A list of all options regarding syntax

### Quote style

  - :heavy_check_mark: **3+N backticks**
    ```rust
    let _ = ```
        some code
        ```;
    ```

  - **3+N double-quotes**
    ```rust
    let _ = """
        some code
        """;
    ```

  - **3+N single quotes**
    ```rust
    let _ = '''
        some code
        ''';
    ```

  - **Word prefix + N+1 hashes**
    ```rust
    let _ = code#"
        some code
        "#;
    ```

  - **Single character prefix + N+1 hashes**
    ```rust
    let _ = m#"
        some code
        "#;
    ```
    (note: `c` is already reserved for C strings)

### Indentation rules

  - :heavy_check_mark: **Relative to closing quote + retain final newline**

    Benefits:
      - Allows every possible indentation to be represented.
      - Simple rule.
      - The value of the string is obvious and intuitive.

    Drawbacks:
      - It is not possible to represent strings without a trailing newline.

  - **Relative to closing quote + remove final newline**

    Benefits:
      - Allows every possible indentation to be represented.
      - Simple rule.
      - Strings without a final newline can be represented.

    Drawbacks:
      - There are two ways to represent the empty string.
      - It is unintuitive that two empty lines between quotes results in
        a single newline.

  - **Relative to first non-empty line**

    Benefits:
      - Simple rule.
      - The value of the string is obvious and intuitive.
      - Strings without a final newline can be represented.

    Drawbacks:
      - Some indentations cannot be represented.

  - **Relative to least indented line**

    Benefits:
      - Simple rule.
      - The value of the string is obvious and intuitive.
      - Strings without a final newline can be represented.

    Drawbacks:
      - Some indentations cannot be represented.

### Modifications

  - :heavy_check_mark: **Language hint directly following opening quote**

    This is intended to allow extra information (eg. language) to be
    conveyed by the programmer to macros and/or their IDE. For example:
    ```rust
    let _ = ```sql
        SELECT * FROM table;
        ```;
    ```
    Here, an intelligent IDE could apply syntax highlighting to the nested
    code block, knowing that the code is SQL.

  - :heavy_check_mark: **Annotation on closing quote to remove trailing
    newline**

    For indentation rules where the final quote must appear on
    its own line and there is no way to represent a string without
    a trailing newline, a modification character could be used.

    For example:
    ```rust
    let _ = ```
        no trailing newline
        !```;
    ```

    Or (the less serious suggestion of)...
    ```rust
    let _ = ```
        no trailing newline
        🚫```;
    ```

    This could be used with any quote style and is unambiguous because
    nothing can otherwise appear on the same line prior to the closing
    quote.

# Prior art
[prior-art]: #prior-art

The proposed quote style is primarily based on markdown code block syntax,
which is widely used and should be familiar to most programmers. This is
also where the language hint comes from.

The indentation rules are borrowed from [Perl's "Indented Here-docs"](https://perldoc.perl.org/perlop#EOF) and [PHP's "Heredoc" syntax](https://www.php.net/manual/en/language.types.string.php#language.types.string.syntax.heredoc)

The [`indoc` crate](https://docs.rs/indoc/latest/indoc/) exists to remove
leading indentation from multiline string literals. However, it cannot
help with the reformatting done by `rustfmt`, and is generally not understood
by IDEs.


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
