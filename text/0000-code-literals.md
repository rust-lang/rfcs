- Feature Name: code_literals
- Start Date: 2023-06-18
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Add a new kind of multi-line string literal for embedding code which
plays nicely with `rustfmt` and doesn't introduce unwanted whitespace
into multi-line string literals.

---

**NOTE: The syntax presented here is *one possible syntax* 
in a huge space. The purpose of this RFC is to gain consensus that
such a feature would be beneficial to the language, not to settle
every possible bike-shedding decision.**

---

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

        To do otherwise would be to change the value of
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

A modifier `h` (for
[Here document](https://en.wikipedia.org/wiki/Here_document))
may be added to a string literal prefix to change how the
string is interpreted by the compiler. The effect of the `h`
modifier causes all indentation to be relative to the
closing quote:

```rust
    let code = h"
        This is a code string literal.

        I can use escape sequences like \n since the `h`
        prefix was added to a normal string literal

            Indentation is preserved *relative* to the indentation level
            of the terminating quote.

    It is an error for a line to have negative indentation (ie. be
    indented less than the final quote) unless
    the line is empty.
        ";
```

`rustfmt` will automatically adjust the indentation of the code string
literal as a whole to match the surrounding context, but will never
change the relative indentation within such a literal.

The `h` modifier will often be combined with raw string literals to
embed sections of code such as SQL:

```rust
    let code = hr#"
        This is also a code string literal

        I can use special characters like "" and \ freely.

            Indentation is still *relative* to the indentation level
            of the terminating quote.
        "#;
```

For completeness, the `h` modifier may also be combined with byte
and raw byte string literals, eg. `hb"` and `hbr#"`.

Anything directly after the opening quote is not considered
part of the string literal. It may be used as a language hint or
processed by macros (similar to the treatment of doc comments).

```rust
let sql = hr#"sql
    SELECT * FROM table;
    "#;
```

When the `h` modifier is used with a raw string literal, the same
rules as usual apply, where the number of `#` characters can be
increased if the sequence `"#` needs to appear inside the string.

In order to suppress the final newline, the literal may instead be
closed with `-" ` or `-"#` depending on the opening quote, eg.

```rust
let code = hr#"
    Text with no final newline
    -"#;
```

Aside from this `-` modifier, only whitespace may appear on the final
line prior to the closing quote.

Together, these rules ensure that every possible string can be represented
in a single canonical way, while allowing the indentation of the string
as a whole to be changed freely.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

An `h` modifier may be added to the prefix of the following string
literal types:

- String literals `h"`
- Raw string literals `hr#"`
- Byte string literals `hb"`
- Raw byte string literals `hbr#"`
- *C string literals `hc"`*
- *Raw C string literals `hcr#"`*

The `h` modifier will appear before all characters in the prefix.
This rule exists for consistency with raw byte strings, which must
be written as `br"<content>"` and not `rb"<content>"`. The choice to
have `h` come first is otherwise arbitrary and was chosen for simplicity.

The value of a string literal with the `h` modifier will be determined
using the following steps:

1.  Measure the whitespace indenting the closing quote. If a
    non-whitespace character (other than a single `-`) exists before
    the closing quote on the same line, then issue a compiler error.
2.  Take the lines *between* (but not including) the opening and
    closing quotes exactly as written.
3.  Remove exactly the measured whitespace from each non-empty line.
    If this cannot be done, then issue a compiler error. The
    whitespace must match down to the exact character sequence.
4.  If a `-` character was present immediately prior to the closing
    quote, then remove the final newline. If there was no final newline
    to remove (because the string was empty) then issue a compiler error.
5.  Interpret any escape sequences and apply any pre-processing as
    usual for the string literal type without an `h` modifier.
    For example, newlines in the file are always treated as `\n`
    even if the file is encoded with `\r\n` newlines.

Here are some edge case examples:

```rust
    // Empty string with language hint
    assert_eq!(h"foo
        ", "");

    // Newline
    assert_eq!(h"

        ", "\n");

    // No terminating newline
    assert_eq!(h"
        bar
        -", "bar");

    // Terminating newline
    assert_eq!(h"
        bar
        ", "bar\n");

    // Preserved indent
    assert_eq!(hr#"
    if a:
        print(42)
    "#, "if a:\n    print(42)\n");

    // Relative indent
    assert_eq!(hr#"
            if a:
                print(42)
            "#, "if a:\n    print(42)\n");

    // Relative to closing quote
    assert_eq!(hr#"


            if a:
                print(42)
    "#, "\n\n        if a:\n            print(42)\n");

    // Interactions with escaping rules
    assert_eq!(h"
        \"\
            foo\n
            bar
    \t
    ", "    \"foo\n\n        bar\n\t\n");
```

Any text between the opening quote and the first newline is
preserved within the AST, but is otherwise unused. It will be
referred to as a "language hint", although may also be used for other
purposes.

The "language hint" (if present) must not begin with a whitespace
character. It is recommended that editors distinguish the language hint
from the rest of the string in some way, such as by highlighting it in
a different colour.

Overall this is a backwards compatible change for editions 2021 onwards,
since edition 2021 reserved prefixes for this kind of feature:
https://doc.rust-lang.org/reference/tokens.html#reserved-prefixes.

Editions prior to 2021 will not benefit from this feature.

# Drawbacks
[drawbacks]: #drawbacks

The main drawback is increased complexity of the language:

1.  It adds a four new types of string literals given all
    the combinations.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Many possible options regarding syntax have been explored during
the life of this RFC. This section will attempt to categorize
and enumerate every variation considered. The options marked
with a :heavy_check_mark: are the variations which were chosen
to form the syntax proposed above.

## A list of all options regarding syntax

### Quote style

  - :heavy_check_mark: **Single character prefix + N hashes**
    ```rust
    let _ = hr#"
        some code
        "#;
    ```
    (note: `c` is already reserved for C strings)

  - **3+N backticks**
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

  - **Word prefix + N hashes**
    ```rust
    let _ = code#"
        some code
        "#;
    ```

### Indentation rules

  - :heavy_check_mark: **Relative to closing quote + retain final newline**

    Benefits:
      - Allows every possible indentation to be represented.
      - Simple rule.
      - The value of the string is obvious and intuitive.

    Drawbacks:
      - Requires an additional syntax to allow representing strings
        without a trailing newline.

  - **Relative to closing quote + remove final newline**

    Benefits:
      - Allows every possible indentation to be represented.
      - Simple rule.
      - Strings without a final newline can be represented.

    Drawbacks:
      - There are two ways to represent the empty string. For example:
        ```rust
        let _ = h"
            "
        ```
        And 
        ```rust
        let _ = h"

            "
        ```
        Would need to both represent the empty string. This is
        unintuitive. It also means that *two* empty lines are
        necessary to represent a single newline.

      - The common case (where the final newline does not need
        to be suppressed) is ugly and wastes vertical space:
        ```rust
        let _ = h"
            some code
        
            ";
        ```

      - Forgetting to add this ugly blank line at the end is a footgun
        when concatenating two strings:
        ```rust
        let a = h"
            if a == 1:
                return True
            ";
        let b = h"
            if b == 1:
                return False
            "
        format!("{a}{b}") == h"
            if a == 1:
                return Trueif b == 1:
                return False
            "
        ```

  - **Relative to first non-empty line**

    Benefits:
      - Simple rule.
      - The value of the string is obvious and intuitive.
      - Strings without a final newline can be represented.

    Drawbacks:
      - Some indentations cannot be represented (those
        where the first line should be indented). At least
        not without further extensions.

  - **Relative to least indented line**

    Benefits:
      - Simple rule.
      - The value of the string is obvious and intuitive.
      - Strings without a final newline can be represented.

    Drawbacks:
      - Some indentations cannot be represented (those
        where every line should be indented). At least
        not without further extensions.

### Modifications

  - :heavy_check_mark: **Language hint directly following opening quote**

    This is intended to allow extra information (eg. language) to be
    conveyed by the programmer to macros and/or their IDE. For example:
    ```rust
    let _ = h"sql
        SELECT * FROM table;
        ";
    ```
    Here, an intelligent IDE could apply syntax highlighting to the nested
    code block, knowing that the code is SQL. The string is not treated
    any differently by the compiler, it's purely there for IDEs and
    optionally procedural macros.

  - **Language hint prior to opening quote**

    Similar to above, but using syntax like the following:

    ```rust
    let _ = h_sql"
        SELECT * FROM table;
        ";
    ```
    If combined with a raw string it might look like:
    ```rust
    let _ = h_sql_r#"
        SELECT * FROM table;
        "#;
    ```
    The choice of `_` as a separator is unsatisfactory, as it is normally
    used as a *joining* character.

  - **Language hint via an expression attribute**
  
    Similar to above, but using syntax like the following:

    ```rust
    let _ = #[lang(sql)] h"
        SELECT * FROM table;
        ";
    ```

    This gets very symbol heavy when combined with raw strings:
    ```rust
    let _ =  #[lang(sql)] hr#"
        SELECT * FROM table;
        "#;
    ```

  - :heavy_check_mark: **Annotation on closing quote to remove trailing
    newline**

    For indentation rules where the final quote must appear on
    its own line and there is no way to represent a string without
    a trailing newline, a modification character could be used.

    For example:
    ```rust
    let _ = h"
        no trailing newline
        -";
    ```

    Or (the less serious suggestion of)...
    ```rust
    let _ = h"
        no trailing newline
        🚫";
    ```

    This could be used with any quote style and is unambiguous because
    nothing can otherwise appear on the same line prior to the closing
    quote.

    Having the annotation be in the string prefix is also possible
    (such as `hn"`) but this is worse because it is non-local (the
    only effect is on the last line of the string) it "uses up" a
    letter for a possible string prefix, and it makes the string
    prefix even longer than it already is.

  - **Explicit indentation markers on the closing quote**

    This modification would be useful for indentation rules which
    otherwise would now allow every possible indentation to be
    represented:

    ```rust
    let _ = h"
            This line will retain 4 characters of indentation.
        ____";
    ```

    Note that this would not be needed in the currently proposed
    scheme, since it can already represent every indentation level.

# Prior art
[prior-art]: #prior-art

The indentation rules are borrowed from [Perl's "Indented Here-docs"](https://perldoc.perl.org/perlop#EOF) and [PHP's "Heredoc" syntax](https://www.php.net/manual/en/language.types.string.php#language.types.string.syntax.heredoc)

The [`indoc` crate](https://docs.rs/indoc/latest/indoc/) exists to remove
leading indentation from multiline string literals. However, it cannot
help with the reformatting done by `rustfmt`, and is generally not
understood by IDEs. It also cannot distinguish between "real" whitespace
in the final, and whitespace introduced by escape sequences.

The "language hint" is based on markdown code block syntax.

See also https://learn.microsoft.com/en-us/dotnet/csharp/programming-guide/strings/#raw-string-literals .


# Unresolved questions
[unresolved-questions]: #unresolved-questions

- None

# Future possibilities
[future-possibilities]: #future-possibilities

-   Macro authors could perform further processing
    on code string literals. These macros could add support for string
    interpolation, escaping, etc. without needing to further complicate
    the language itself.

-   Procedural macros could look at the text following the opening
    quotes and use that to influence code generation, eg.

    ```rust
    query!(h"postgresql
        <query>
        ")
    ```

    could parse the query in a PostgreSQL specific way.

-   Code literals could be used by crates like `html-macro`
    or `quote` to provide better surface syntax and faster
    compilation.

-   Code literals could be used with the `asm!` macro to avoid
    needing a new string on every line.
