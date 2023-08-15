- Feature Name: `unified_string_literals`
- Start Date: 2023-08-12
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Rust 2021 has two forms of string literals:
- **String literals** are delimited by a pair of quotation marks and support using escape sequences to represent represent non-printing characters as well as string delimiters (the double quote) and the escape character (the backslash itself).

```rust
let message = "Hello, \"John\""; // Hello, "John"
```

- **Raw string literals** are prefixed by an `r` and delimited by a pair of quotation marks surrounded by matching sets of up to 255 pound signs (`#`). Backslashes within the string are not parsed as the beginning of an escape sequence, but are passed through literally. Quotation marks only terminate the string if followed by the number of pound signs that preceded the opening quotation mark.

```rust
let js = r#"function hello() { console.log("Hello, world!"); }"#; 
// function hello() { console.log("Hello, world!"); }
```

This RFC proposes to unify the syntax of these two forms, supporting both the use of escape sequences and avoiding the need to escape backslashes and quotation marks. This proposal also uses the new syntax to improve format string ergonomics, reducing the need for double-brace escapes. We propose to introduce the new syntax and then remove the existing _raw string literal_ syntax in a future edition. We also propose to apply the same unified syntax to _bytes string literals_ and _C string literals_.

To avoid confusion with legacy code and education materials, we propose new terminology for the new syntax proposed here: _guarded string literal_. This will refer to a string literal like `#"content here"#`, where there are an equal amount of `#` before the opening quotation mark and after the closing quotation mark, but without the `r` prefix that existing _raw string literals_ use. String literals without guarding will be referred to as _bare string literals_.

# Motivation
[motivation]: #motivation

The main purpose for raw string literals is to avoid the need to escape backslashes, like those in Regex syntax (`Regex::new(r"\w+")`). But because quotation marks are often used in user interfaces and in various forms of code, people will reach for raw string literals just for the quotation-mark feature. 

But there are many use cases where someone wants to avoid escaping quotation marks or backslashes, but would still like to use other escape sequences (and therefore can't use raw strings):

<!-- swift already has this string syntax so provides better highlighting -->

1. null-terminated strings
```swift
let foo = #"first line: "Hello, World!" \0"#;
```

2. byte strings
```swift
let bytes = b#"some stuff in another encoding: "\xF7\x84" "#;
```

3. splitting a long string literal across lines
```swift
let long = #"\
Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do \
eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut \
enim ad minim veniam, quis nostrud exercitation ullamco laboris \
nisi ut aliquip ex ea commodo consequat.\
"#;
```

In addition to making the syntax more flexible, there is substantial benefit to the language having only one form of string literal syntax. This reduces the combinatorial effect of having the `r` prefix for raw and also `b`, `c`, etc for different string literal output types. It may also improve how to language syntax is taught to beginners, who now do not need learn what the `r` stands for.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## String Literals

In Rust, basic string literals are delimited with just quotation marks: `"I am a string"`. Special characters are escaped with a backslash character: `\`. This way you can add any character to your string, even unprintable ones and ones that you don't know how to type. If you want a literal backslash, escape it with another one: `\\`. If you want to have a quotation mark within your string, you can escape it with a backslash as well: `"\""`.

```rust
fn main() {
    // You can use escapes to write bytes by their hexadecimal values...
    let byte_escape = "I'm writing \x52\x75\x73\x74!";
    println!("What are you doing\x3F (\\x3F means ?) {}", byte_escape);

    // ...or Unicode code points.
    let unicode_codepoint = "\u{211D}";
    let character_name = "\"DOUBLE-STRUCK CAPITAL R\"";

    println!("Unicode character {} (U+211D) is called {}",
                unicode_codepoint, character_name );


    let long_string = "String literals
                        can span multiple lines.
                        The linebreak and indentation here ->\
                        <- can be escaped too!";
    println!("{}", long_string);
}
```

Sometimes there are just too many characters that need to be escaped or it's just much more convenient to write a string out as-is. This is where guarded string literals come into play.

Guarded string literals are just like normal string literals, but with a number of pound signs (`#`) before the opening quotation mark and a matching amount after the closing quotation mark. Escape sequences within the string must also have that number of `#` directly after the opening backslash.

```swift
fn main() {
    let raw_str = #"Escapes don't work here: \x3F \u{211D}"#;
    println!("{}", raw_str); // Escapes don't work here: \x3F \u{211D}

    let escapes = #"Unless they have the pound sign: \#x3F \#u{211D}"#;
    println!("{}", escapes); // Unless they have the pound sign: ? ℝ

    // With the #s, quotes don't need to be escaped
    let quotes = #"And then I said: "There is no escape!""#;
    println!("{}", quotes); // And then I said: "There is no escape!"

    // If you need "# in your string, just use more #s in the delimiter.
    // You can use up to 255 #s.
    let longer_delimiter = ###"A string with "# in it. And even "##!"###;
    println!("{}", longer_delimiter); // A string with "# in it. And even "##!
}
```

Want a string that's not UTF-8? (Remember, `str` and `String` must be valid UTF-8). Or maybe you want an array of bytes that's mostly text? Byte strings to the rescue!

```swift
// Byte strings can have byte escapes
let escaped = b"\x52\x75\x73\x74 as bytes";

// Guarded byte strings work just like guarded strings
let raw_bytestring = b#"\u{211D} passed through literally here"#;
let raw_bytestring = b#"but "\#x52" is escaped into "R" here"#;

let quotes = b#"You can also use "fancier" formatting, \#
                like with normal guarded strings"#;
```

## Formatting Placeholders

Rust has several macros (such as `format!`, `println!`, and `write!`) which accept "format strings". These strings have "placeholders" which are delimited by curly braces. In order to output a literal curly brace, one must double up the curly braces.

```rust
let x = 5;
let y = 10;

println!("x = {x} and y + 2 = {}", y + 2); // x = 5 and y + 2 = 12

let js = format!("function bar() {{ {} }}", "/* function body */");
// function bar() { /* function body */ }
```

Sometimes there are just too many curly braces that need to be escaped or it's just much more convenient to write a string out as-is. This is where guarded string literals come into play.

In guarded format strings, the opening `#` sequence must be placed directly before every placeholder. Any other curly braces will be passed through literally.

```rust
let js = format!(#"function bar() { #{} }"#, "/* function body */");
// function bar() { /* function body */ }
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## String literals

A _string literal_ opens with fewer than 256 of the character `U+0023` (`#`) (the _guarding prefix_) and a `U+0022` (double-quote) character. The _string body_ can contain any sequence of Unicode characters and is terminated only by another `U+0022` (double-quote) character, followed by the guarding prefix. A string literal started with one or more `U+0023` (`#`) characters is called a _guarded string literal_, but one without any opening `U+0023` (`#`) characters is a _bare string literal_.

All Unicode characters contained in the _string body_ represent themselves, the characters `U+0022` (double-quote) or `U+005C` (`\`) only hold special meaning when followed by at least as many `U+0023` (`#`) characters as were used to start the string literal (zero for bare string literals). A `U+005C` (`\`) followed by the guarding prefix is interpreted as an _escape start_.

Line-breaks are allowed in string literals. A line-break is either a newline (`U+000A`) or a pair of carriage return and newline (`U+000D`, `U+000A`). Both byte sequences are normally translated to `U+000A`, but as a special exception, when an escape start occurs immediately before a line break, then the line break character(s), and all immediately following (`U+0020`), `\t` (`U+0009`), `\n` (`U+000A`) and `\r` (`U+000D`) characters are ignored.

_Supported escapes are unchanged from those currently in the reference, aside from requiring the guard prefix as explained above._

### String literal examples
```swift
"foo"; #"foo"#;                    // foo
"\"foo\""; #""foo""#;              // "foo"

"foo #\"# bar";
##"foo #"# bar"##;                 // foo #"# bar

"\x52"; #"\#x52"#; "R"; #"R"#;     // R
"\\x52"; #"\x52"#;                 // \x52
```

## Byte string literals

A _byte string literal_ opens with the prefix `b`, followed by fewer than 256 of the character `U+0023` (`#`) (the _guarding prefix_) and a `U+0022` (double-quote) character. The _byte string body_ can contain any sequence of ASCII characters and is terminated only by another `U+0022` (double-quote) character, followed by the guarding prefix. A byte string literal started with one or more `U+0023` (`#`) characters is called a _guarded byte string literal_, but one without any opening `U+0023` (`#`) characters is a _bare byte string literal_.

All ASCII characters contained in the _string body_ represent themselves, the characters `U+0022` (double-quote) or `U+005C` (`\`) only hold special meaning when followed by at least as many `U+0023` (`#`) characters as were used to start the string literal (zero for bare string literals). A `U+005C` (`\`) followed by the guarding prefix is interpreted as an _escape start_.

_Supported escapes are unchanged from those currently in the reference, aside from requiring the guard prefix as explained above._

### Byte string literal examples
```swift
b"foo"; b#"foo"#;                     // foo
b"\"foo\""; b#""foo""#;               // "foo"

b"foo #\"# bar";
b##"foo #"# bar"##;                   // foo #"# bar

b"\x52"; b#"\#x52"#; b"R"; b#"R"#;    // R
b"\\x52"; b#"\x52"#;                  // \x52
```

## C string literals

Identical to string literals except they accept any byte sequence except Nul bytes.

_Supported escapes are unchanged from those specified in RFC 3348 (union of string literals and byte string literals), aside from requiring the guard prefix as explained above._

## Format placeholders

A _format string_ is a string literal used as an argument to the `format_args!` family of macros. Format strings use curly braces surrounding an optional set of format parameters as _placeholders_ for variable substitution.

Format placeholders start with the string literal guarding prefix followed by a single `{`, then an optional sequence of format parameters, and are terminated by a single `}`.

Bare string literals used as format strings can output the literal characters `{` and `}` by preceding them with the same character. For example, the `{` character is escaped with `{{` and the `}` character is escaped with `}}`. However, the body of a _guarded format string_ is literally passed through (following the processing of escape sequences discussed above), the sequences `{` (except when preceded by the guarding prefix), `}` (except when terminating a placeholder), `{{`, and `}}` do not have any special meaning.

```rust
format!("Hello, {}!", "world");    // => Hello, world!
format!("Hello {{}}");             // => Hello {}
format!("{{ Hello");               // => { Hello

format!(#"Hello, #{}!"#, "world"); // => Hello, world!
format!(#"Hello {}"#);             // => Hello {}
format!(#"{ Hello"#);              // => { Hello
```

### Implementation note

Format macros will need to have some way to detect the string guarding prefix. One way to achieve this is to inspect the `span` of the string literal token:
- do no further processing if it begins with an `r`
- count the number of leading `#`s

Currently, third-party procedural macros must process string literals manually anyways (or delegate to a library like `syn`), so there's little change needed. However, the compiler may choose to optimize the `format_args!` builtin by storing the guarding prefix (or just the length) in the string literal token data.

# Drawbacks
[drawbacks]: #drawbacks

One drawback of this change is syntax churn. When the old string literal syntax is removed is a future edition, crates planning on upgrading will need to update all of their raw string literals. Luckily, the upgrade path is quite simple (often just removing the `r` prefix) and can be fully automated (`cargo fix`). This kind of syntax evolution is exactly what editions are for.

Another drawback is ecosystem support. When the new syntax is introduced, macro libraries that deal with string literals (such as `ufmt` and `indoc`) will need to be updated to support it. The new format string behavior (using `#{}` in guarded string literals) is likely to require more intensive changes.

A small but notable drawback is that guarded string literals require one more character to express the same behavior as a raw string literal with no `#` prefix.

```rust
#"a raw string"#;
// vs
r"a raw string";
```

However, this is likely irrelevant since it saves a character over a raw string literal with any amount of `#`s:

```rust
#"a raw string"#;
// vs
r#"a raw string"#;
```

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

One possible alternative is to add just the `#`-guarding behavior to normal string literals, but without the change to escape sequences and format strings.
```swift
#"This has "\x52" inside it"#;    // This has "R" inside it
format!(#"Hello, {}!"#, "world"); // => Hello, world!
format!(#"Hello {{}}"#);          // => Hello {}
```
However, this has disadvantages compared to our proposal:
- must still escape backslashes
- must still escape curly braces in format strings

These make it specifically painful for usage with code, which commonly contains braces and backslash escape sequences:
```rust
let filename = "statistics.xlsx";
format!(r#"
function path() {{
  return "C:\\\\Users\\\\John\\\\Documents\\\\{filename}";
}}
"#)
```
vs
```
let filename = "statistics.xlsx";
format!(#"
function path() {
  return "C:\\Users\\John\\Documents\\#{filename}";
}
"#)
```

Also, since there is no prefix associated with the guarded literal form, this composes better with C-string and byte-string literals. There's no need to remeber the prefix order - `rb` or `br`, and escapes are needed even more often in these types of literals, since typing out an arbitrary byte sequence can be impossible.

# Prior art
[prior-art]: #prior-art

- Swift proposal SE-0200: [_Enhancing String Literals Delimiters to Support Raw Text_](https://github.com/apple/swift-evolution/blob/main/proposals/0200-raw-string-escaping.md)
- Original Rust raw string literal RFC: [_Syntax for raw string literals_](https://github.com/rust-lang/rust/issues/9411)
- My pre-RFC where **jrose** suggested the Swift design: [_Extend Hash-sequences to all String Literals_](https://internals.rust-lang.org/t/pre-rfc-extend-hash-sequences-to-all-string-literals/19300)
- Discussion on Zulip: [_Swift string literals_](https://rust-lang.zulipchat.com/#narrow/stream/213817-t-lang/topic/Swift.20string.20literals)

This design is based on the raw string literal syntax of the Swift language. Their syntax was actually based on Rust's raw string literal syntax, which was itself an iterative improvement based on Python. Swift's design is also an iterative but novel improvement, and would likely have been chosen by Rust if it had existed at the time raw string literals were originally added to Rust.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- In an escape sequence, should the `#` come before or after the `\`?

# Future possibilities
[future-possibilities]: #future-possibilities

This syntax also composes well with "code strings": multiline, indent-normalized string literals.

```swift
const CODE: &str = 
    m#"
    function path() {
      return "C:\\Users\\John\\Documents\\";
    }
    "#;
```
