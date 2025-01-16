- Feature Name: `unified_string_literals`
- Start Date: 2023-08-12
- RFC PR: [rust-lang/rfcs#3475](https://github.com/rust-lang/rfcs/pull/3475)
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

This RFC proposes to unify the syntax of these two forms, supporting both the use of escape sequences and avoiding the need to escape backslashes and quotation marks. This proposal also uses the new syntax to improve format string ergonomics, reducing the need for double-brace escapes. We propose to apply the same unified syntax to _bytes string literals_ and _C string literals_.

To avoid confusion with legacy code and education materials, we propose new terminology for the new syntax proposed here: _guarded string literal_. This will refer to a string literal like `#"content here"#`, where there are an equal amount of `#` before the opening quotation mark and after the closing quotation mark, but without the `r` prefix that existing _raw string literals_ use. String literals without guarding will be referred to as _bare string literals_.

# Motivation
[motivation]: #motivation

The main purpose for raw string literals is to avoid the need to escape backslashes, like those in Regex syntax (`Regex::new(r"\w+")`). But because quotation marks are often used in user interfaces and in various forms of code, people will reach for raw string literals just for the quotation-mark feature. 

But there are many use cases where someone wants to avoid escaping quotation marks or backslashes, but would still like to use other escape sequences (and therefore can't use raw strings):

<!-- swift already has this string syntax so provides better highlighting -->

1. null-terminated strings
```swift
let foo = #"first line: "Hello, World!" \#0"#;
```

2. byte strings
```swift
let bytes = b#"some stuff in another encoding: "\#xF7\#x84" "#;
```

3. splitting a long string literal across lines
```swift
let long = #"\#
Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do \#
eiusmod tempor "incididunt" ut labore et dolore magna aliqua. Ut \#
enim ad minim veniam, quis nostrud exercitation ullamco laboris \#
nisi ut aliquip ex ea commodo consequat.\#
"#;
```

In addition to making the syntax more flexible, there is substantial benefit to the language having only one form of string literal syntax. This reduces the combinatorial effect of having the `r` prefix for raw and also `b`, `c`, etc for different string literal output types. It may also improve how the language syntax is taught to beginners, who now do not need learn what the `r` stands for.

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
[guide-formatting-placeholders]: #guide-formatting-placeholders

Rust has several macros (such as `format!`, `println!`, and `write!`) which accept "format strings". These strings have "placeholders" which are delimited by curly braces. In order to output a literal curly brace, one must double up the curly braces.

```rust
let x = 5;
let y = 10;

println!("x = {x} and y + 2 = {}", y + 2); // x = 5 and y + 2 = 12

let js = format!("function bar() {{ {} }}", "/* function body */");
// function bar() { /* function body */ }
```

Sometimes there are just too many curly braces that need to be escaped or it's just much more convenient to write a string out as-is. This is where guarded string literals come into play.

In guarded format strings, the opening `#` sequence must be placed directly after the opening `{` of every placeholder. Any other curly braces will be passed through literally.

```rust
let js = format!(#"function bar() { {#} }"#, "/* function body */");
// function bar() { /* function body */ }
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## String literals

> **<sup>Lexer</sup>**\
> QUOTE_ESCAPE<sub>**N**</sub> :\
> &nbsp;&nbsp; `\` `#`<sup>N</sup> \[ `'` `"` ]
>
> ASCII_ESCAPE<sub>**N**</sub> :\
> &nbsp;&nbsp; &nbsp;&nbsp; `\` `#`<sup>N</sup> `x` OCT_DIGIT HEX_DIGIT\
> &nbsp;&nbsp; | `\` `#`<sup>N</sup> \[ `n` `r` `t` `\` `0` ]
>
> UNICODE_ESCAPE<sub>**N**</sub> :\
> &nbsp;&nbsp; `\` `#`<sup>N</sup> `u{` ( HEX_DIGIT `_`<sup>\*</sup> )<sup>1..6</sup> `}`
>
> STRING_CONTINUE<sub>**N**</sub> :\
> &nbsp;&nbsp; `\` `#`<sup>N</sup> _followed by_ \\n
>
> STRING_CONTENT<sub>**N**</sub> :\
> &nbsp;&nbsp; `#`<sup>N</sup> `"` (\
> &nbsp;&nbsp; &nbsp;&nbsp; ~( [ `"` `\` ] `#`<sup>N</sup> |  _IsolatedCR_ )\
> &nbsp;&nbsp; &nbsp;&nbsp; | QUOTE_ESCAPE<sub>N</sub>\
> &nbsp;&nbsp; &nbsp;&nbsp; | ASCII_ESCAPE<sub>N</sub>\
> &nbsp;&nbsp; &nbsp;&nbsp; | UNICODE_ESCAPE<sub>N</sub>\
> &nbsp;&nbsp; &nbsp;&nbsp; | STRING_CONTINUE<sub>N</sub>\
> &nbsp;&nbsp; )<sup>\*</sup> `"` `#`<sup>N</sup>
>
> STRING_LITERAL :\
> &nbsp;&nbsp; STRING_CONTENT<sub>0..255</sub> SUFFIX<sup>?</sup>

A _string literal_ opens with fewer than 256 of the character `U+0023` (`#`) (the _guarding prefix_) and a `U+0022` (double-quote) character. The _string body_ can contain any sequence of Unicode characters and is terminated only by another `U+0022` (double-quote) character, followed by the guarding sequence. A string literal started with one or more `U+0023` (`#`) characters is called a _guarded string literal_, but one without any opening `U+0023` (`#`) characters is a _bare string literal_.

All Unicode characters contained in the _string body_ represent themselves, the characters `U+0022` (double-quote) or `U+005C` (`\`) only hold special meaning when followed by at least as many `U+0023` (`#`) characters as were used to start the string literal (zero for bare string literals). A `U+005C` (`\`) followed by the guarding sequence is interpreted as an _escape start_.

Line-breaks are allowed in string literals. A line-break is either a newline (`U+000A`) or a pair of carriage return and newline (`U+000D`, `U+000A`). Both byte sequences are normally translated to `U+000A`, but as a special exception, when an escape start occurs immediately before a line break, then the line break character(s), and all immediately following space (`U+0020`), `\t` (`U+0009`), `\n` (`U+000A`) and `\r` (`U+000D`) characters are ignored.

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

A _byte string literal_ opens with the prefix `b`, followed by fewer than 256 of the character `U+0023` (`#`) (the _guarding prefix_) and a `U+0022` (double-quote) character. The _byte string body_ can contain any sequence of ASCII characters and is terminated only by another `U+0022` (double-quote) character, followed by the guarding sequence. A byte string literal started with one or more `U+0023` (`#`) characters is called a _guarded byte string literal_, but one without any opening `U+0023` (`#`) characters is a _bare byte string literal_.

All ASCII characters contained in the _string body_ represent themselves, the characters `U+0022` (double-quote) or `U+005C` (`\`) only hold special meaning when followed by at least as many `U+0023` (`#`) characters as were used to start the string literal (zero for bare string literals). A `U+005C` (`\`) followed by the guarding sequence is interpreted as an _escape start_.

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
[reference-format-placeholders]: #reference-format-placeholders

A _format string_ is a string literal used as an argument to the `format_args!` family of macros. Format strings use curly braces surrounding an optional set of format parameters as _placeholders_ for variable substitution.

Format placeholders start with a single `{`, followed by the string literal guarding sequence, then an optional sequence of format parameters, and are terminated by a single `}`.

Bare string literals used as format strings can output the literal characters `{` and `}` by preceding them with the same character. For example, the `{` character is escaped with `{{` and the `}` character is escaped with `}}`. However, the body of a _guarded format string_ is literally passed through (following the processing of escape sequences discussed above), the sequences `{` (except when followed by the guarding sequence), `}` (except when terminating a placeholder), `{{`, and `}}` do not have any special meaning.

```rust
format!("Hello, {}!", "world");    // => Hello, world!
format!("Hello {{}}");             // => Hello {}
format!("{{ Hello");               // => { Hello

format!(#"Hello, {#}!"#, "world"); // => Hello, world!
format!(#"Hello {}"#);             // => Hello {}
format!(#"{{ Hello"#);             // => {{ Hello

let five = 5;
format!(#"five: {#five}!"#);       // => five: 5
format!(#"five hex: {#five:#x}"#); // => five: 0x5
```

### Implementation note
[reference-format-placeholders-impl-note]: #reference-format-placeholders-impl-note

Format placeholders are not a language lexing question at all. `#"this string has {#} in it"#` is just a string literal that in any other context resolves to the string `this string has {#} in it`. It is entirely the format macro's responsibility to parse the placeholders in accordance with the guarding sequence.

When a macro is parsing a format string, it simply needs to know the prefix used:

- if `#*`, the placeholder is always `{#*}` and doubled curly braces are passed through literally
- otherwise, the placeholder is always `{}` and curly braces are escaped by doubling

Essentially, all a format macro needs from the source code is the prefix. It can use the processed content of the literal (string value) when actually parsing the formatting. This means that `#"{\#x23}"#` would be treated identically to `"{}"`, just like `"\x7B\x7D"` is today.

Currently, the `proc_macro` API does not provide a way to get the string value, so third-party procedural macros must process string literals manually, using the span (or delegate to a library like `syn`). When this new syntax is first introduced, macros which parse directly (like `indoc`) will need to be manually updated to interpret the new syntax. Because of the manual involvement, they are likely to learn about the new way format placeholders work in these strings. 

However, proc macros which use a parser library like `syn` may encounter a situation where a simple dependency bump allows their macros to treat these strings exactly the same as existing string literal forms. These proc macros will support guarded format strings with non-standard syntax, without the macro author knowing. Therefore, it is important that we perform a review of third-party format crates before stabilization.

### Interaction with `concat!`
[reference-format-placeholders-concat]: #reference-format-placeholders-concat

We propose that `concat!` always return a bare string literal. Any string literals passed to it have escape sequences processed before being concatenated into a single string literal without a guarding sequence. `concat!(#"with "inner string", escape \#n, and placeholder {#} last"#)` would resolve to the string literal `"with \"inner string\", escape \n, and placeholder {#} last"`.

Example `concat!` behavior:
```rust
fn main() {
    let x = 42;
    println!(#"{#x} {x}"#, x = x);              // => 42 {x}
    println!("{#x} {x}", x = x);                // error: invalid format string
    println!(concat!(#"{#x}"#, " {x}"), x = x); // error: invalid format string
}
```

Using a guarded format string with `concat!` will result in an "invalid format string" error, due to the `#` at the start of the placeholder. The diagnostic for this should be improved to note that `concat!` always resolves to a bare string literal:

```diff
  error: invalid format string: expected `'}'`, found `'#'`
   --> src/main.rs:4:14
    |
  5 |     println!(concat!(#"{#x}"#, " {x}"), x = x);
    |              ^^^^^^^^^^^^^^^^^^^^^^^^^ expected `'}'` in format string
    |
+   = note: `concat!` always resolves to a bare string literal
    = note: if you intended to print `{`, you can escape it using `{{`
    = note: this error originates in the macro `concat`
```

## Timeline

In Edition 2021 and earlier, `#"le string"#` resolves to three separate tokens when passed to a macro, so adding this new syntax in those editions would be a breaking change. To avoid that, we will introduce the change on the 2024 edition boundary.

### Edition 2024: Reserve the syntax

> **<sup>Lexer</sup>**\
> RESERVED_GUARDED_STRING_LITERAL :\
> &nbsp;&nbsp; RESERVED_GUARDED_STRING_CONTENT SUFFIX<sup>?</sup>
>
> RESERVED_GUARDED_STRING_CONTENT :\
> &nbsp;&nbsp; &nbsp;&nbsp; `#"` ( ~ _IsolatedCR_ )<sup>* (non-greedy)</sup> `"#`\
> &nbsp;&nbsp; | `#` RESERVED_GUARDED_STRING_CONTENT `#`

When compiling under the Rust 2024 edition (as determined by the edition of the current crate), any instance of `RESERVED_GUARDED_STRING_LITERAL` will result in a tokenization error (until the `unified_string_literals` feature is stabilized).

# Drawbacks
[drawbacks]: #drawbacks

The largest drawback is the complexity ths introduces to the grammar of string literals, on top of the various forms of string literals that already exist or are planned. This complex grammar will be difficult to implement so we should also work on exposing useful `proc_macro` APIs to help third-party macros.

Another drawback is ecosystem support. When the new syntax is introduced, macro libraries that deal with string literals (such as `ufmt` and `indoc`) will need to be updated to support it. The new format string behavior (using `{#}` in guarded string literals) is likely to require more intensive changes.

Syntax churn and ecosystem consistency may also be an minor issue, similar to how the introduction of inlined format args and let-else (and clippy lints pushing them) has gone.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

### Guarding for quotes but not escapes nor placeholders

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
format!(r#"
function path() {{
  return "C:\\\\Users\\\\John\\\\Documents\\\\{filename}";
}}
"#)
```
vs
```
format!(#"
function path() {
  return "C:\\Users\\John\\Documents\\{#filename}";
}
"#)
```

Also, since there is no prefix associated with the guarded literal form, this composes better with C-string and byte-string literals. There's no need to remember the prefix order (`rb` or `br`?), and escapes are needed even more often in these types of literals, since typing out an arbitrary byte sequence is impossible in UTF-8 Rust source code.

### Guarding for quotes and escapes but not format placeholders
[alternative-no-guarding-placeholders]: #alternative-no-guarding-placeholders

Many suggest that string guarding should not be tied to format placeholders as suggested in this proposal. Placeholders would instead work the same as they do currently in string literals and raw string literals.

Concerns:
1. Layering violation, too magical: requires the lexer to interface with libraries, but formatting should only depend on the content of the string
2. Formatting should be orthogonal to escaping: proposal would require guarding placeholders even if you only want to avoid escaping quotes or backslashes
3. Inconsistent with raw string literals: `r#"placeholder: {}"#` vs `#"placeholder: {#}"#`

Addressing these concerns:
1. Format macros already have to use the span of the literal even to just get the value of the string content. If we choose to add an API to expose the value, we can just as easily add an API for the guarding sequence. Macros have always worked at the syntax level, and Rust users generally understand that. Compared to other things macros do in the wild, this is pretty tame. 
2. In our view, format placeholders are a kind of contextual escape sequence. The definition of escape sequence is "a combination of characters that has a meaning other than the literal characters contained therein", which fits exactly. We're also only talking about a single additional `#` for each placeholder in most cases, compared to two extra characters for each pair of literal `{}` in the output string. Plus, the `#` inside the placeholder doesn't impact the ease of spotting placeholders, since users are used to a mix of controlling symbols within the braces.
3. Raw string literals are just different, and users of the language understand that. We think this behavior maintains more consistency with how the other escapes work in these strings.

There is a true trade-off here, but we think the benefit for cases with many literal braces outweights the slight detriment to the general formatting case. You pay for the double-brace escaping on every literal brace in your string (there can be many), but you only pay for the placeholder guarding when you use a placeholder.
```swift
format!(#"
function path() {{
  let custom = getCustomPath();
  if (custom) {{
    return custom;
  }} else {{
    return "C:\\Users\\John\\Documents\\{#filename}";
  }}
}}
"#)
```
vs
```swift
format!(#"
function path() {
  let custom = getCustomPath();
  if (custom) {
    return custom;
  } else {
    return "C:\\Users\\John\\Documents\\{#filename}";
  }
}
"#)
```

Guarded string literals should allow the user to avoid escaping of any kind (therefore having all text outside escape sequences pass through literally), knowing that in return they have to use the guard to close the string, in escape sequences (should they want them), and in format placeholders.

#### Specify the placeholder prefix within the format string

Using currently invalid syntax, specify the placeholder prefix independently of the string guarding sequence:

```swift
format!(#"{(#)}The natural numbers, denoted "N", are the set {#{}, #{}, ...}."#, 1, 2)
format!("{(%)}The natural numbers, denoted \"N\", are the set {%{}, %{}, ...}.", 1, 2)
// The natural numbers, denoted "N", are the set {1, 2, ...}.
```

This would be independent of the string literal syntax and more flexible (useable with bare strings, raw strings, and any other future string type).

However, it is more complex to implement and use. Independence from the string literal syntax is arguably a disadvantage, the flexibility is of limited usefulness beyond what our proposal offers, and the `"{(prefix)}` syntax is not exactly elegant.

### Split the format changes into a separate RFC

This would depend on a second RFC being incorporated before guarded strings are stabilized, or this RFC would have to require that they can't be used as format strings.

While either is possible, they would likely just draw out the process for little benefit. The first could silently break usage of the unstable feature, and the second would just be annoying. 

Tying format placeholders to the string syntax is a core part of this proposal and can't be easily separated.

### Promote format string placeholder parsing to the lexer 
[alternative-placeholder-lexer]: #alternative-placeholder-lexer

An alternative to `concat!` always resolving to a bare string literal: have the lexer record the positions of format placeholders. When multiple strings are concatenated, the placeholder positions from all are retained. Then, macros are provided an API to retrieve the indices where each placeholder starts.

Benefits:
- `concat!` "just works" with format strings.
- macros have less work to do

However, we consider this solution to be too complex for such a niche situation. It also has the drawback of requiring the expansion of the `proc_macro` API, and it doesn't solve the `syn` issue discussed above.

### `\#{}` formatting placeholder

Under this alternative, `\{` would be added as a new escape sequence resolving to a literal `{`. Format macros would look for that escape sequence in the literal span rather than using the processed string content. `concat!` could also use these escape sequences to propagate placeholders.

Disadvantages:
- Requires format macros to use the span rather than just the value + prefix
- One more character than `{#}`

### `#{}` formatting placeholder

Placing it after the opening `{` parses unambiguously. `"#{}"` is already a valid format string, but `"{#}"` can result in an format-parse error. Likewise, `#"##{}"#` would be treated as a literal `#` followed by a placeholder, but `#"{##}"#` can result in the same error.

Additionally, `#"( x = "#{x}" )"#` will actually lex as four tokens because the first `"#` closes the string:
```
#"(x = "#
{x}
")"
#
```
This would make it almost impossible to place quotes directly around a placeholder, requiring the use of an escape instead. Whereas `#"( x = "{#x}" )"#` lexes as a single string without issue.

### `#\` escape start

Putting it after (`\#n`) parses unambiguously and allows us to catch more issues at compile time. `" #\n "` is already a valid string, but `" \#n "` can result in an "unexpected escape sequence, help: remove the extra `#`" error. Likewise, `##" ###\n "##` could mean `#\n` or could be a mistaken extra `#`, wheras `##" \###n "##` can result in the same error.

# Prior art
[prior-art]: #prior-art

- Swift proposal SE-0200: [_Enhancing String Literals Delimiters to Support Raw Text_](https://github.com/apple/swift-evolution/blob/main/proposals/0200-raw-string-escaping.md)
- Original Rust raw string literal RFC: [_Syntax for raw string literals_](https://github.com/rust-lang/rust/issues/9411)
- My pre-RFC where **jrose** suggested the Swift design: [_Extend Hash-sequences to all String Literals_](https://internals.rust-lang.org/t/pre-rfc-extend-hash-sequences-to-all-string-literals/19300)
- Discussion on Zulip: [_Swift string literals_](https://rust-lang.zulipchat.com/#narrow/stream/213817-t-lang/topic/Swift.20string.20literals)

This design is based on the raw string literal syntax of the Swift language. Their syntax was actually based on Rust's raw string literal syntax, which was itself an iterative improvement based on Python. Swift's design is also an iterative but novel improvement, and would likely have been chosen by Rust if it had existed at the time raw string literals were originally added to Rust.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

Should we remove the legacy raw string syntax?
  + It can still be useful and more ergonomic when someone wants to ensure that absolutely no escapes are possible. Consider `#######"this is a \######n string"#######`, where it is difficult to tell from a glance that `\######n` is not a newline escape. `r"this is a \######n string"` makes that very clear, and is easier on the eyes.
  + But, cases like the above are quite rare. And there is overhead in keeping the raw string literal syntax around. It's hard to argue for continuing to teach raw string literals alongside both bare string literals and guarded string literals, multiplied by various literal types (`b`, `c`). If we plan on no longer teaching raw string literals, a user may have to look up what the `r` means when they come across a rare usage. Or a user may accidentally add an `r` without knowing what it means.
  + Also, cases like the above can use a longer guarding sequence than necessary to make it clear what is and is not an escape: `#########"this is a \######n string"#########`. The `needless_raw_string_hashes` clippy lint will currently trigger on that, but this can easily be changed to allow for cases that can benefit from the extra clarity extra `#`s bring.

# Future possibilities
[future-possibilities]: #future-possibilities

### Code Strings

This syntax also composes well with "code strings": multiline, indent-normalized string literals.

```swift
const CODE: &str = 
    m#"
    function path() {
      return "C:\\Users\\John\\Documents\\";
    }
    "#;
```

### `f`-strings

This proposed syntax and applies directly to `f`-strings without modification beyond adding the `f` prefix.

```rust
let count = 12;
f"count: {count}"    // => thing: 12
f#"count: {#count}"# // => thing: 12
```

`f`-strings make an even stronger case that `\#` and `{#}` escape sequences should be consistent. This would be very similar to how Swift strings work: `"\(count)"` and `#"\#(count)"#`.

### Remove raw string literal syntax

Guarded string literals make raw string literals largely redundant. We can remove them in a future edition and migrate all raw string literals to use guarded strings instead.

However, this is a controversial choice and can be decided after guarded strings are introduced. We plan to introduce a follow-up RFC for removing raw strings after this RFC is merged.
