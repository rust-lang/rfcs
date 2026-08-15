- Feature Name: `dedented_string_literals`
- Start Date: 2025-06-05
- RFC PR: [rust-lang/rfcs#3830](https://github.com/rust-lang/rfcs/pull/3830)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Add dedented string literals: `d"string"`.

With:

```rs
let sql = d"
    create table student(
        id int primary key,
        name text
    )
    ";
```

Being equivalent to:

```rs
let sql = "\
create table student(
    id int primary key,
    name text
)";
```

# Motivation
[motivation]: #motivation

Problem: Embedding formatted text in Rust's string literals forces us to make a choice:

- Sacrifice readability of the output
- Sacrifice readability of the source code

## Introduction

### Sacrifice readability of the output

In order to print the following:

```sql
create table student(
    id int primary key,
    name text
)
```

The initial attempt might look as follows:

```rust
fn main() {
    println!("
        create table student(
            id int primary key,
            name text
        )
    ");
}
```

Which outputs (using `^` to mark the beginning of a line, and `·` to mark a leading space):

```sql
^
^········create table student(
^············id int primary key,
^············name text
^········)
^····
^
```

The output is formatted in an unconventional way, containing excessive leading indentation.

The alternative allows for a sane output, but at the cost of making the code less readable:

### Sacrifice readability of the source code

In order for the output to be more sensible, we must sacrifice readability of the source code:

```rust
fn main() {
    println!(
        "\
create table student(
    id int primary key,
    name text
)");
}
```

The above example would output the expected:

```sql
create table student(
    id int primary key,
    name text
)
```

But the improvement in output comes at a cost:

1. We now have to escape the first newline:

   ```diff
   fn main() {
       println!(
   +       "\
   create table student(
       id int primary key,
       name text
   )");
   }
   ```

   This is not possible to do in raw strings, so the output ends up looking even worse for them, with indentation of the outer SQL statement being larger in the source code than the inner statement:

   ```diff
   fn main() {
       println!(
   +       r#"create table student(
       id int primary key,
       name text
   )"#);
   ```

2. The SQL statement does not have any indentation in reference to the surrounding code.

   This is contrary to how we would normally write code, with indentation 1 level deeper than the surrounding.

   ```diff
   fn main() {
       println!(
           "\
   +create table student(
   +   id int primary key,
   +   name text
   +)");
   }
   ```

   This makes it confusing to tell which scope the string belongs to. This is especially true when there are multiple scopes involved:

   ```rs
   fn main() {
       {
           println!(
               "\
   create table student(
      id int primary key,
      name text
   )");
       }
       println!(
           "\
   create table student(
      id int primary key,
      name text
   )");
       {
           {
               println!(
                   "\
   create table student(
      id int primary key,
      name text
   )");
            
           } 
       }
   }
   ```

   All of the strings end up on the same level, despite them being in different scopes.

3. The closing double-quote must be put at the beginning of the line, in order not to introduce trailing horizontal whitespace:

   ```diff
   fn main() {
       println!(
           "\
   create table student(
       id int primary key,
       name text
   +)");
   }
   ```

As you can see, we have to choose one or the other. In either case we have to give something up.

Another way to format the above would be the following:

```rs
fn main() {
    println!(concat!(
        "create table student(\n",
        "    id int primary key,\n",
        "    name text,\n",
        ")\n",
    ));
}
```

The above:
- Is formatted nicely by `rustfmt`
- Produces the correct output

However, it looks very noisy.
- Each line ends with an escaped `\n`.
- Requires double-quotes around each line.
- This does not allow for interpolations, such as `{variable_name}` as the format string is expanded by a macro.

Sometimes, we are *forced* into the first option - sacrificing readability of the source.

In some cases, producing excessive whitespace will change meaning of the output.

Consider whitespace-sensitive languages such as Python or Haskell, or content which is meant to be read by people like generated Markdown - here we *can't* make a sacrifice on readabilty of the output - so our source code must become harder to understand.

But, what if we could have the best of both worlds?

### Dedented string literals

In order to solve these problems, the RFC proposes dedented string literals of the form: `d"string"`.

Common leading indentation on each line after the opening quote in dedented string literals will be stripped at compile-time.

This allows us to have a more readable version of the above:

```rust
fn main() {
    println!(
        d"
        create table student(
            id int primary key,
            name text
        )
        "
    );
^^^^^^^^ // common leading indentation (will be removed)
}
```

All of the above problems are gracefully solved:

1. Indentation level inside the string is the same as what is in the output.
1. It does not require escaping the first newline for it to look readable.
1. Nicely composes with raw string literal: `dr#"string"#`, in which the first newline *cannot* be escaped.
1. Indentation level of the statement is larger than the `println!` call,
   making it more obvious that the string is inside the call at a glance.
1. The closing parentheses in the SQL statement aligs with `create table`.

Now, consider the example with multiple nested scopes again:

```rs
fn main() {
    {
        println!(
            d"
            create table student(
                id int primary key,
                name text
            )
            "
        );
    }
    println!(
        d"
        create table student(
            id int primary key,
            name text
        )
        "
    );
    {
        {
            println!(
                d"
                create table student(
                    id int primary key,
                    name text
                )
                "
            );
        } 
    }
}
```

It is immediately more obvious which string belongs to which scope.

## Closing quote controls the removed indentation

From the column containing the closing quote `"`, common leading horizontal whitespace is stripped from each line.

Here are a few examples to demonstrate.

### No indentation is stripped when the closing quote has no indentation

The output is the same as what is in the source code.

This allows all lines to have a common indentation.

```rust
fn main() {
    println!(
        d"
        create table student(
            id int primary key,
            name text
        )
"
// no common leading indentation = nothing to remove
    );
}
```

In the above example, the closing quote is on the very first character. Common indentation is not stripped at all.

Prints: 

```sql
        create table student(
            id int primary key,
            name text
        )
```

Outcome: **No indentation is removed. Output contains 2 levels of indentation. Source contains 2 levels of indentation**.

### Strip 1 level of indentation

In order to strip the first level of indentation, the ending quote is aligned to the `println!` call.

```rust
fn main() {
    println!(
        d"
            create table student(
                id int primary key,
                name text
            )
        "
^^^^^^^^ // common leading indentation (will be removed)
    );
}
```

The indentation of the closing double quote is 4 spaces. The 4 spaces will be removed from each line.

Prints:

```sql
    create table student(
        id int primary key,
        name text
    )
```

Outcome: **1 indentation level in the output, 1 indentation level has been stripped from the source**.

### Strip *all* indentation

All indentation can be stripped by placing the closing double quote on the same level as content of the dedented string literal:

```rust
fn main() {
    println!(
        d"
        create table student(
            id int primary key,
            name text
        )
        "
^^^^^^^^ // common leading indentation (will be removed)
    );
}
```

The indentation of the ending double quote is 8 spaces. This common prefix of leading horizontal whitespace characters will be removed from the beginning of each line.

Prints:

```sql
create table student(
    id int primary key,
    name text
)
```

Result: **all indentation from source is stripped**.

Indenting the closing double quote further will have zero impact.
The dedentation will never remove non-horizontal-whitespace characters.

Each of the following **examples** print:

```sql
create table student(
    id int primary key,
    name text
)
```

**Examples**:

```rs
fn main() {
    println!(
        d"
        create table student(
            id int primary key,
            name text
        )
            "
^^^^^^^^ // common leading indentation: 8 spaces
^^^^^^^^^^^^ // closing quote indentation: 12 spaces
    );
}

// spaces removed from the beginning of each line = min(8, 12) = 8
```

```rs
fn main() {
    println!(
        d"
        create table student(
            id int primary key,
            name text
        )
                "
^^^^^^^^ // common leading indentation: 8 spaces
^^^^^^^^^^^^^^^^ // closing quote indentation: 16 spaces
    );
}
// spaces removed from the beginning of each line = min(8, 16) = 8
```

```rs
fn main() {
    println!(
        d"
        create table student(
            id int primary key,
            name text
        )
                    "
^^^^^^^^ // common leading indentation: 8 spaces
^^^^^^^^^^^^^^^^^^^^ // closing quote indentation: 20 spaces
    );
}
// spaces removed from the beginning of each line = min(8, 20) = 8
```

## Composition with other string literal modifiers, such as raw string literals and byte string literals

Dedented string literals `d"string"` are a new modifier for strings.

They are similar to byte strings `b"string"` and raw strings `r#"string"#`.

They compose with others like every other string literal modifier.

To be precise, the RFC introduces 6 new types of string literals:

- Dedented string literal: `d"string"`
- Dedented raw string literal: `dr#"string"#`
- Dedented byte string literal: `db"string"`
- Dedented byte raw string literal: `dbr#"string"#`
- Dedented C string literal: `dc"string"`
- Dedented C raw string literal: `dcr#"string"#`

The `format_args!` macro, and by extension all wrapper macros that pass arguments to `format_args!` under the hood - also accept dedented string literals:

```rs
fn main() {
    let table_name = "student";

    println!(
        d"
        create table {table_name}(
            id int primary key,
            name text
        )
        "
^^^^^^^^ // common leading indentation (will be removed)
    );
}
```

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Any kind of string literal can turn into a "dedented" string literal if it is prefixed with a `d`:

- strings: `"string"` -> `d"string"`
- Raw strings: `r#"string"#` -> `dr#"string"#`
- Byte strings: `b"string"` -> `db"string"`
- ...and others...

> [!NOTE]
>
> The above list is a slight simplification.
> There are a few rules that apply to dedented string literals which we will get to shortly.

An example comparing regular `"string"`s and dedented `d"string"`s:

```rust
let regular = "
    I am a regular string literal.
    ";

// All of the visible whitespace is kept.
assert_eq!(regular, "\n    I am a regular string literal.\n    ");

//               ↓ newline is removed
let dedented = d"
    I am a dedented string literal!   
    ";                           //^ newline is removed
//^^ indentation is removed

assert_eq!(dedented, "I am a dedented string literal!");
```

Common indentation of all lines up to, but **not including** the closing quote `"` is removed from the beginning of each line.

Indentation present *after* the double-quote is kept:

```rs
//               ↓ newline is removed
let dedented = d"
        I am a dedented string literal!   
    ";                               //^ newline is removed
//^^ indentation is removed
//  ^^^^ horizontal whitespace after the double quote is kept

assert_eq!(dedented, "    I am a dedented string literal!");
```

Dedented string literals make it easy to embed multi-line strings that you would like to keep formatted according to the rest of the code:

```rs
let py = d"
    def hello():
        print('Hello, world!')

    hello()
    ";
//^^ removed

let expected = "def hello():\n    print('Hello, world!')\n\nhello()";
assert_eq!(py, expected);
```

They compose with all string literals, such as c strings `c"string"`, raw strings, `r#"string"#` and byte strings `b"string"`:

```rs
// dedented raw string
let py = dr#"
    def hello():
        print("Hello, world!")

    hello()
    "#;
//^^ removed

let expected = "def hello():\n    print(\"Hello, world!\")\n\nhello()";
assert_eq!(py, expected);
```

You can use them in formatting macros, such as `println!`, `write!`, `assert_eq!`, `format_args!` and similar:

```rs
let message = "Hello, world!";

let py = format!(
    dr#"
    def hello():
        print("{message}")

    hello()
    "#
//^^ removed
);

let expected = "def hello():\n    print(\"Hello, world!\")\n\nhello()";
assert_eq!(py, expected);
```

By placing the closing quote `"` earlier than the first non-horizontal-whitespace character in any of the lines, you can reduce how much indentation is removed from each line:

```rs
use std::io::Write as _;

let message = "Hello, world!";
let mut py = String::new();

// Note: Using `writeln!` because the final newline from dedented strings is removed. (more info later)

writeln!(
    py,
    d"
    def hello():
    "
//^^ removed
);

// Note: We want to add 2 newlines here.
// - `writeln!` adds 1 newline at the end
// - An additional empty line is added
//   to insert the 2nd newline

// Remember, dedented string literals strip the last newline.
writeln!(
    py,
    dr#"
    print("{message}")

"#
//^^ kept
);

write!(
    py,
    d"
hello()
            "
);
//^^^^^^^^^^ No indentation is removed here.
//           If the closing quote is after the common indentation
//           (in this case there is no common indentation at all),
//           all of the common indentation is stripped

let expected = "def hello():\n    print(\"Hello, world!\")\n\nhello()";
assert_eq!(py, expected);
```

## Rules

### Dedented string literals must begin with an end-of-line character (EOL)

All dedented string literals must begin with an EOL.
This EOL is removed.

The following is invalid:

```rust
//         ↓ error: expected literal EOL
//           note: dedented string literals must start with a literal EOL
//           help: insert a literal newline here: 
let py = d"def hello():
        print('Hello, world!')

    hello()
    ";
```

Escaped EOL such as an escaped newline (`\n`), it must be a literal EOL:

```rust
//         ↓ error: expected literal EOL, but found escaped newline.
//           note: dedented string literals must start with a literal EOL
let py = d"\ndef hello():
        print('Hello, world!')

    hello()
    ";
```

This is the correct syntax for the first line:

```rust
// OK
let py = d"
    def hello():
        print('Hello, world!')

    hello()
    ";
```

### Last line must be empty, and preceded by a literal EOL

The line which contains the closing quote `"` can only contain horizontal whitespace, and the character before the last line must be a literal EOL.

This is invalid:

```rust
let py = d"
    def hello():
        print('Hello, world!')

    hello()";
//         ^ error: expected literal EOL
//           note: in dedented string literals, the line
//                 which contains the closing quote can
//                 only contain horizontal whitespace
```

Neither is using an escaped EOL (e.g. escaped newline `\n`) instead of the literal EOL:

```rust
let py = d"
    def hello():
        print('Hello, world!')

    hello()\n";
//         ^ error: expected literal EOL, but found escaped newline `\n`
//           note: in dedented string literals, the line
//                 which contains the closing quote can
//                 only conatin horizontal whitespace
```

This is the correct syntax for the last line:

```rust
let py = d"
    def hello():
        print('Hello, world!')

    hello()
    ";
// OK
```

Both outputs will not contain EOL at the end, since the literal EOL is stripped.

If you'd like to have a trailing EOL, you can insert a literal newline at the end (or any other EOL):

```rust
let py = d"
    def hello():
        print('Hello, world!')

    hello()

    ";
// OK
```

You can also use an escaped newline. This is fine, because the string still ends with a literal EOL (which cannot be escaped):

```rust
let py = d"
    def hello():
        print('Hello, world!')

    hello()\n
    ";
// OK
```

Benefits the above rules bring include:

- The above rules make all dedented string literals you'll find in Rust consistent.
- It allows easily changing the indentation level without having to insert an EOL sometimes.
- It gives the ability for us to tell a regular string literal from a dedented string literal at a glance.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Terms used

We use these terms throughout the RFC and they are explained in detail in this section.

- **Whitespace** is as defined in the [reference](https://doc.rust-lang.org/reference/whitespace.html) as any character with the [`Pattern_White_Space`](https://www.unicode.org/reports/tr31/) unicode property
- **Horizontal whitespace** is spaces or tabs. These are the only `Pattern_White_Space` characters that are *horizontal* space per [UAX#31](https://www.unicode.org/reports/tr31/#Contexts_for_Ignorable_Format_Controls)
- **EOL (end-of-line) character** is any "end of line" character as classified in [`UAX#R3a-1`](https://www.unicode.org/reports/tr31/#R3a-1)
- **Indentation** is one or more **horizontal whitespace** at the beginning of a line
- An **empty line** only consists of literal horizontal whitespace

A "newline" is used as an example of a specific EOL character, however any other valid EOL character can be used.

## String Literals

6 new [string literal](https://doc.rust-lang.org/reference/tokens.html#characters-and-strings) types:

Note: **Literal newlines** (*not* escaped newlines: `\n`) are represented with `\ln` for the purpose of the explanation.

|                                              | Example         | `#`&nbsp;sets[^nsets] | Characters  | Escapes             |
|----------------------------------------------|-----------------|------------|-------------|---------------------|
| Dedented String                   | `d"\ln EXAMPLE \ln"`       | 0          | All Unicode | [Quote](#quote-escapes) & [ASCII](#ascii-escapes) & [Unicode](#unicode-escapes) |
| Dedented Raw string           | `dr#"\ln EXAMPLE \ln"#`    | <256       | All Unicode | `N/A`                                                      |
| Dedented Byte string         | `db"\ln EXAMPLE \ln"`      | 0          | All ASCII   | [Quote](#quote-escapes) & [Byte](#byte-escapes)                               |
| Dedented Raw byte string | `dbr#"\ln EXAMPLE \ln"#`   | <256       | All ASCII   | `N/A`                                                      |
| Dedented C string               | `dc"\ln EXAMPLE \ln"`      | 0          | All Unicode | [Quote](#quote-escapes) & [Byte](#byte-escapes) & [Unicode](#unicode-escapes)   |
| Dedented Raw C string       | `dcr#"\ln EXAMPLE \ln"#`   | <256       | All Unicode | `N/A`                                                                            |

## Interaction with macros

- `format_args!` and wrapper macros such as `println!` can accept dedented string literals: `format!(d"...")`.
- `concat!` accepts dedented strings, just like it accepts raw strings. Each dedented string passed to `concat!` is dedented before concatenation.
- The `literal` macro fragment specifier accepts all of the 6 new string literals.

## Algorithm for dedented strings

1. The opening line (the line containing the opening quote `"`)
    - *May* contain 1 or more horizontal whitespace characters. (*trailing* horizontal whitespace)
    - These horizontal whitespace characters are removed.
    - Must only contain a literal EOL after the `"` token
    - This EOL is removed.
1. The closing line (the line containing the closing quote `"`)
    - Must contain only horizontal whitespace before the closing quote
    - This horizontal whitespace is the *closing indentation*.
    - The closing indentation is removed.
1. The character immediately before the closing line must be a literal EOL.
    - This EOL is removed.
1. The *common indentation* is calculated.

   It is the largest amount of leading horizontal whitespace shared by all non-empty lines.

1. For each line, remove the smallest amount of leading horizontal whitespace that satisfies:

    - `min(common indentation, closing indentation)`

    What this means is:
    - Even if a line is indented by more than the closing indentation
    - Only the amount equal to the closing indentation, or less, will be removed.
    - Never more than the line actually has.

### Treatment of literal whitespace escapes: `\t`, `\r` and `\n`

#### On the line containing the closing quote

- Only horizontal whitespace is allowed before the closing quote.
- Escapes are not permitted even if they are escapes for horizontal whitespace (e.g. a tab escape `\t`), because escapes are processed after dedenting, so they are not yet horizontal whitespace when the line with the closing quote is processed.

#### In the content of the string

The escaped characters `\t`, `\r` and `\n` are treated as regular characters for the purposes of dedentation.

So the following:

```rs
println!(
    d"
    \ta
    \tb
    \tc
        " // the indent here is a tab
);
```

Prints, with each indentation being **1 tab**:

```
    a
    b
    c
```

The indentation is not removed, because common indentation in this example is 0. (closing indentation is 1 tab).

Escaped characters at the beginning of the string are interpreted as any other character, and **not** horizontal whitespace.

After the dedentation is calculated, the escapes then expand into their literal counterparts.

### Edge Cases

> [!NOTE]
>
> `•` denotes a space.

````rs
// the whitespace at the start of non-empty lines is not part
// of the calculation for "common indentation"
// amongst non-empty lines
//
// remove the smallest amount of leading whitespace
assert_eq!(
    d"
••••hello
••
••••world
    ",
^^^^ // common leading indentation (will be removed)

    "hello\n\nworld"
);

// line consisting of only spaces is allowed

// However, nothing is removed because the:

// > common indentation of all non-empty lines

// is 0 here. (all lines are empty)

// so min(0, x) = 0 -> remove 0 characters
assert_eq!(
    d"
••••••••
    ",

    "••••••••"
);

// no whitespace removed either
assert_eq!(
    d"
••••••••
",

    "••••••••"
);

// explanation:
//
// Initially we have:
//
// ```rust
// let _ = d"
//
// ";
// ```
//
// The literal newline directly after the opening `"` is removed. We get:
//
// ```rust
// let _ = "
// ";
// ```
//
// The literal newline directly before the line containing
// the closing `"` is removed. We get:
//
// ```rust
// let _ = "";
// ```
//
// An empty string.
assert_eq!(
    d"

",

    ""
);

// error: Expected a literal newline character
//        before the line containing the closing quote
//
// note: The literal newline character after the opening quote
//       is removed in all cases
#[expect_compile_error]
let _ = d"
    ";
````

## Treatment of special unicode characters

The invisible whitespace characters `U+200E` (left-to-right mark) and `U+200F` (right-to-left mark) cannot appear anywhere inside the indentation to be stripped from a line.

When the compiler encounters these characters, it offers to place them directly *after* the stripped indentation.

Invalid example, `◀` represents `U+200F` and `▶` represents `U+200E`:

```rust
let py = d"
 ◀  def hello():
  ▶     print('Hello, world!')

    hello()\n
    ";
//^^ error: U+200E cannot appear in the stripped indentation
//   help: place them after the stripped indentation
//^^ error: U+200F cannot appear in the leading indentation
//   help: place them after the stripped indentation
```

It should be fixed as follows:

```rust
let py = d"
    ◀def hello():
        ▶print('Hello, world!')

    hello()\n
    ";
// OK
```

The above example is valid because the invisible characters `U+200F` and `U+200E` after the indentation which will be remain in the output, while the indentation of 4 spaces will be stripped from each line.

## Mixed spaces and tabs

In all examples of this RFC, we only assume that the common indentation of each line (to be stripped) and indentation of the closing quote of the dedented string uses the same character (either literal tabs, or literal spaces)

Mixing these character in a way that is ambiguous is disallowed, and will error. For instance, in the following example with literal tabs (represented by `→`) and literal spaces (represented by `•`) mixed together:

```rust
// error: ambiguous spaces mixed with tabs
let py = d"
→→→→def hello():
→→→→••••print('Hello, world!')

•→••hello()
→→••";
```

The above program is rejected due to ambiguity. There is no single "common indentation" that is the same on each line.

Mixing spaces and tabs in a way such that the common indentation matches, *even if* the indentation consists of both spaces and tabs is allowed:

```rust
let py = d"
→••→•def hello():
→••→•••••print('Hello, world!')

→••→hello()
→••→";
```

The above is equivalent to:

```rust
let py = "\
•def hello():
•••••print('Hello, world!')

hello()";
```

Common indentation is `→••→`, which is stripped from each line.

Empty lines can safely be mixed with either spaces or tabs, as they do not count for the purposes of dedentation.

# Drawbacks
[drawbacks]: #drawbacks

- While the reference specifies `r` as ["not processing any escapes"](https://doc.rust-lang.org/reference/tokens.html#raw-string-literals), users are less likely familiar with the exact definition and more familiar with the name and the effect: it leaves the string as-is.

  This can feel contradictory to `d` which is a specific form of modifying the string content and so a `dr""` could read as something that should be a compilation error.

- The more string literal modifiers that are stacked on each other, more work is needed to decipher it and can feel a bit too foreign

- Contributes to the increase of string literal modifiers by adding a new variant.

  While at the moment the variety of string literal modifiers is small, it is worth to think about the implications of exponential increase of them.

  Currently, Rust has 7 types of string literals. This RFC will increase that to 13, because each string literal can be prefixed with a `d` to make it dedented.

  In the future Rust might get additional types of "string modifiers", and each combination will need to
  be accounted for.

- Increases complexity of the language. While it builds upon existing concepts, it is yet another thing for people to learn.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Design

### The choice of the letter `d` for "dedent"

When picking a single letter for this feature, we want:

- A letter that represents a mnemonic
- The mnemonic should make sense
- And be memorable

The RFC picks `d` as a mnemonic for "dedent".

- Dedentation is a simple atomic operation which removes the indentation of the string
- The transformation is always a dedentation

  If there is no indentation, removing the it is still accurately described as a "dedentation" because the nothing is removed.
- It might help make the acronym more memorable by thinking about the `d` as "**d**eleting" the indentation.

#### Why not `u` for "unindent"

Confusion can arise due to the way this string prefix has been used in other languages:

- In Python 2, `u` is a prefix for Unicode strings
- In C++, `u` is used for UTF-16 strings

The goal a single-letter acronym hopes to accomplish is to be memorable and make sense.
It can be argued that the word "Unindent" is more complex than the word "Dedent":

- Unindent contains a negation, consisting of two "parts": **un** + **indent**. Undoing an indentation.
- Dedent represents an atomic operation, which is removal of indentation and is a synonym to unindent.

Using a negated word can be considered to be less desireable, because in order to undo the negation we have to perform an extra "step" when thinking about it.

Consider that instead of a negated `if` condition:

```rs
if !string.is_empty() {
   walk()
} else {
   run()
}  
```

Writing the non-negated version first is often clearer:

```rs
if string.is_empty() {
   run()
} else {
   walk()
}  
```

Using a word with a lower cognitive complexity may make it easier to think about and more memorable.

#### Why not `i` for "indent"

Indent is the opposite of dedent. It could make sense, but from a completely different perspective.

The question is, which one do we value more:

- A word that describes what the string looks like in the source code.
- A word that describes the transformation that the string goes through when it is evaluated.

"Indent" describes what the string looks like in the source code:

```rs
fn main() {
    let table_name = "student";

    println!(
        d"
        create table {table_name}(
            id int primary key,
            name text
        )
        "
    );
}
```

But it does not describe the transformation that it goes through:

```sh
create table student(
    id int primary key,
    name text
)
```

When the string is evaluated, the indentation is removed. It is **dedented**.

In the source code, the string is **indented**.

- When viewing the string from the source code, the indentation is obvious.

  However, it is *not* obvious what will happen to the string when it is evaluated. "Dedent" can be clearer in this regard, as we already have 1 piece of information and the word "dedent" brings us the other piece.

- The string may not always be considered to be indented:

   ```rs
   let _ = d"
   hello world
   ";
   ```

  In the above example, there is no indentation for the strings. It would be inaccurate to describe the string as having indentation.

  Once the string is evaluated, it is accurate to describe the removal of the non-existing indentation as still "dedenting" the string.

#### Why not `m` for "multi-line"

- Dedented string literals do not necesserily represent a multi-line string:

```rs
let _ = d"
hello world
";
```

The above is equivalent to:

```rs
let _ = "hello world";
```

Confusion could arise, as people expect it to evaluate to a string spanning multile lines.

#### Why not `h` for "heredoc"

RFC #3450 uses `h` as the modifier instead of `d`, as an acronym for [Here document](https://en.wikipedia.org/wiki/Here_document).

- The term is likely to be less known, and may raise confusion, especially amongst
  those that don't know what it is.
- Here documents are more associated with "code blocks", which may associate an "info string"
  with them (such as in markdown). This RFC does not propose an info string.

While the feature this RFC proposes (dedented string literals) are useful for code
blocks, it is not just for them.

### The choice of the form `d"string"`

The syntax of `d"string"` is chosen for the following reasons:

- Fits with existing string modifiers, such as `b"string"`, `r#"string"#"` and `c"string"`
- Composes with existing string modifiers: `db"string"`, `dc"string"`, `dr#"string"#`, and `dbr#"string"#`. 
- Does not introduce a lot of new syntax. Dedented string literals can be explained in terms of existing language features.
- Adding a single letter `d` before a string literal to turn it into a dedented string literal is an incredibly easy modification.
- Rust reserves space for additional string modifiers.

  Adding this feature does not require a new edition, as it is backwards-compatible for editions later than Edition 2024, as the syntax has been [reserved](https://doc.rust-lang.org/edition-guide/rust-2024/reserved-syntax.html) since this edition.

The choice for `d` to come before all other modifiers is not arbitrary.

Consider `dbr` and all possible alternatives:

1. `dbr`: dedented byte raw string
1. `bdr`: byte dedented raw string
1. `brd`: byte raw dedented string

The first example reads in the most natural manner. The other two don't.

<!--
    NOTE: I would personally have preffered drb = detended raw byte string, as 'raw byte' reads more naturally than 'byte raw'.
    But since this is already in the language, we can't change it.
-->

### Requirement of first and final EOL

As mentioned earlier in the RFC:

- There must be a literal newline present directly after the opening quote `"`.
- There must be a literal newline present directly before the line containing the closing quote `"`.

Having this as a hard requirement will make usages of dedented string literals more consistent.

Consider the following which is invalid:

```rs
fn main() {
    // ERROR
    println!(
        d"create table student(
            id int primary key,
            name text
        )
        "
    );
}
```

- The `d"` and `create` in the first `d"create` not being separated by whitespace makes it harder to understand where the code begins. They have to be mentally separated.
- Additionally, indentation of the `create` does not align with what it will look like in the output, making it less obvious, which we would like to aviod. Therefore it is a **hard error** to not have a literal EOL.

The following is also incorrect, as there is no EOL before the line containing the closing quote:

```rs
fn main() {
    println!(
        d"
        create table student(
            id int primary key,
            name text
        )" // ERROR
    );
}
```

- Having the closing quote **always** be on its own line makes it more obvious to the reader from which column onwards leading indentation will be removed.
- In the example above, it is not immediately clear where that would be from.
- It easy to modify the common indentation level of the string in the future, as you do not have to create a new line.

### The choice of not ending with an EOL

Dedented string literals do not end with an EOL.

The following:

```rs
fn main() {
    print!(
        d"
        create table student(
            id int primary key,
            name text
        )
        "
    );
}
```

Prints, *without* a newline at the end:

```sh
create table student(
    id int primary key,
    name text
)
```

In order to add a final newline, insert a newline (literal "\n" or escaped `\n`) (or any EOL) at the end:

```rs
fn main() {
    print!(
        d"
        create table student(
            id int primary key,
            name text
        )

        "
    );
}
```

Removing the final EOL is consistent with removing the initial EOL.

The line containing the opening quote `"` and the line containing the closing quote `"` can be considered to be fully exempt from the output.

If this *wasn't* the behaviour:
- It would make less sense to remove the EOL from the beginning, but not from the end.
- Dedented strings would always end with a EOL
- ..But how do you opt-out of the EOL?

  Using a special syntax, like closing with a `-"` (as a different RFC proposes) would be too special-cased, it wouldn't fit in with the rest of the language.

  It would be confusing for those that want to end the dedented string with a `-`.

Removing *both* the EOL at the start and the end is consistent, and allows maximum flexibility whilst not making additional trade-offs such as having to introduce new special syntax to exclude the EOL.

### Allowing the closing line to be indented more than previous lines

Having the quote be indented further than the first non-horizontal-whitespace character in the
string content is allowed:

```rs
fn main() {
    println!(
        d"
        create table student(
            id int primary key,
            name text
        )
                "
    );
}
```

Reason: turning this into a syntax error is too strict, when it can be auto-fixed by tooling like `rustfmt`:

```rs
fn main() {
    println!(
        d"
        create table student(
            id int primary key,
            name text
        )
        "
    );
}
```

## Differences from RFC 3450

The [RFC #3450: Propose code string literals](https://github.com/rust-lang/rfcs/pull/3450) is similar to this one, however this RFC is different and this section explains why.

Differences:

- #3450 uses `h` as the modifier instead of `d`. Explained [earlier](#why-not-h-for-heredoc)

- #3450 allows to write an *info string*, like in markdown.

    It proposes the ability to write:

    ```rs
    let sql = d"sql
        SELECT * FROM table;
        ";
    ```

    With the `sql` not affecting the output, but can aid in syntax highlighting and such.
 
   1. Is considered out of scope for this RFC to consider.
 
         It would be a backward-compatible change to make for a future RFC, if it's desired.

   1. [Expression attributes](https://github.com/rust-lang/rust/issues/15701) are likely to be more suitable for this purpose. (not part of this RFC)

         ```rs
         let sql = #[editor::language("sql")] "SELECT * FROM table;";
         ```

- RFC #3450 makes the "code strings" always end with an EOL, with the ability to prepend a minus before the closing quote in order to remove the final EOL.

    However, in this RFC the following:

    ```rs
    print!(
        d"
        a
        "
    ^^^^ // common leading indentation (will be removed)
    );
    ```

    Prints: `a`

    **Without** an ending newline.

    In order to add a newline at the end, you have to add a newline in the source code:

    ```rs
    print!(
        d"
        a

        "
    ^^^^ // common leading indentation (will be removed)
    );
    ```

    The above prints:

    ```
    a  
    ```

    **With** a newline.

    Additionally, finishing with `-"` instead of `"` is not seen anywhere in the language, and would not fit in.

## Use a crate instead

What are the benefits over using a crate, such as `indoc`?

1. Having dedented strings as a language feature allows them to be used in Rust snippets
   and examples where said examples would not otherwise have a dependency on the crate.

   This makes the feature more discoverable.

2. Dedented strings are a "nice-to-have", if they were a core language feature they would likely be used
   much more, but as this functionality is currently only available in a crate, it is unlikely people
   would want to add a dependency just for dedented strings, especially for one-off usecases.

3. No need to know about the specific crate, which most projects may not depend on.

   Learn the feature once, and use it anywhere.

4. Reduce the entry barrier to contribution to projects

   Crates may be hesitant in adding a dependency on a dedented string crate because it would
   be *yet another* thing for contributors to learn and be aware of.

### Crate macros

The [`indoc`](https://crates.io/crates/indoc) crate is similar to the feature this RFC proposes.

The macros the crate exports help create dedented strings:

- `eprintdoc!`
- `formatdoc!`
- `indoc!`
- `printdoc!`
- `writedoc!`

These macros would no longer be necessary, as the dedented string literals compose with the underlying macro call. (Dedented strings can be passed to `format_args!`).

The benefits of replacing these, and similar macros with language features are described below.

#### Reduces the proliferation of macros

Macros can make code harder to understand. They can transform the inputs in arbitrary ways. Contributors have to learn them, increasing the entry barrier for a new project.

For the above reason, projects may be hesitant to use crates that provide this as it would make contributing harder.

The dedent macros will be possible to replace using the dedented string literals proposed in this RFC. Examples, using the `indoc` crate's macros specifically:

- `eprintdoc!`: Calls `eprint!` under the hood, dedenting the passed string.

    Before:

    ```rs
    eprintdoc! {"
            GET {url}
            Accept: {mime}
        ",
        ^^^^ // common leading indentation (will be removed)
        url = "http://localhost:8080",
        mime = "application/json",
    }
    ```

    With dedented string literals:

    ```rs
    eprintln! {
        d"
            GET {url}
            Accept: {mime}
        ",
        ^^^^ // common leading indentation (will be removed)
        url = "http://localhost:8080",
        mime = "application/json",
    }
    ```

    Both snippets print:

    ```
    GET http://localhost:8080
    Accept: application/json
    ```

    Note that `eprintdoc!` does not remove the final EOL, that's why we use `eprintln` instead of `eprint`.

- `indoc!`: Dedents the passed string.

    Before:

    ```rs
    indoc! {r#"
        def hello():
            print("Hello, world!")

        hello()
    "#}
    ^^^^ // common leading indentation (will be removed)
    ```

    With dedented string literals:

    ```rs
    dr#"
        def hello():
            print("Hello, world!")

        hello()

    "#
    ^^^^ // common leading indentation (will be removed)
    ```

    Both snippets evaluate to:

    ```py
    def hello():
        print("Hello, world!")

    hello()
    ```

    Note that `indoc!` does not remove the final EOL, that's why we add an additional newline after `hello()`.

As a bonus, not only does it unify many macros under a single language feature.

It also allows us to trivially create new macros that automatically make use of the feature in a backwards-compatible way.

Take for instance the `text!` macro exported from the `iced` crate:

```rs
macro_rules! text {
    ($($arg:tt)*) => {
        $crate::Text::new(format!($($arg)*))
    };
}
```

In order to ergonomically supply a dedented string to it, one needs to re-create the macro:

```rs
macro_rules! textdoc {
    ($($arg:tt)*) => {
        iced::text!(formatdoc!($($arg)*))
    };
}
```

That's not a problem for *this* example, however with more involved macros such as [ones from the `log` crate](https://docs.rs/log/0.4.27/src/log/macros.rs.html#165-186) it becomes a problem. 

With this RFC, re-implementing the macros is not going to be necessary anymore, as you can just pass in the dedented string literals:

```rs
text!(
    d"
    GET {url}
    Accept: {mime}
"
^^^^ // common leading indentation (will be removed)
)
```

The language feature works with any user-defined macros that pass their arguments to `format_args!` under the hood.

#### Improved compile times

Having dedented strings as a language feature, instead of relying on a macro provided by a crate could reduce compile time.

- Users do not have to compile the crate *or* its dependencies.
- There is no need for procedural macro expansion to take place in order to un-indent the macro. This step happens directly in the compiler.

## Use a built-in macro instead

What about using a compiler built-in macro like `dedent!("string")` instead of a language built-in string modifier such as `d"string"`?

### Advantages

- Will likely have similar performance to the literal itself.

### Disadvantages

#### The macro will be unable to capture variables from the surrounding scope

One of the major benefits of having dedented string literals is that you'll be able to use them in formatting macros:

```rs
let message = "Hello, world!";

// `{message}` is interpolated
let py = format!(
    dr#"
    def hello():
        print("{message}")

    hello()
    "#
//^^ removed
);

let expected = "def hello():\n    print(\"Hello, world!\")\n\nhello()";
assert_eq!(py, expected);
```

In the above example, the variable `message` is captured and used directly in the `format!` macro call.

However, this feature would not be possible with a `dedent!` macro.

Consider the following code:

```rs
fn main() {
    let foo = "foo";
    let bar = "bar";

    let x = format!(concat!("{foo}", "bar"));
}
```

It attempts to create a string `{foo}bar` which is passed to `format!`. Due to limitations, it does not compile:

```
error: there is no argument named `foo`
 --> src/main.rs:5:21
  |
5 |     let x = format!(concat!("{foo}", "bar"));
  |                     ^^^^^^^^^^^^^^^^^^^^^^^
  |
  = note: did you intend to capture a variable `foo` from the surrounding scope?
  = note: to avoid ambiguity, `format_args!` cannot capture variables when the format string is expanded from a macro

error: could not compile `dedented` (bin "dedented") due to 1 previous error
```

Importantly:

> to avoid ambiguity, `format_args!` cannot capture variables when the format string is expanded from a macro

A `dedent!` macro would have the same limitation: Namely the string is created from the expansion of a macro.

The problem with `dedent!` is that we expect it to be largely used with formatting macros such as `format!` and `println!` to make use of string interpolation.

Implementing dedented string literals as a macro will significantly limit their functionality.

Consider a conversion from a regular string literal that prints some HTML:

```rust
writeln!(
  w,
  "  <!-- <link rel=\"shortcut icon\" href=\"{rel}favicon.ico\"> -->\
  \n</head>\
  \n<body>\
  \n  <div class=\"body\">\
  \n    <h1 class=\"title\">\
  \n      {h1}\
  \n      <span class=\"nav\">{nav}</span>\
  \n    </h1>"
)
```

Into a dedented string literal:

```rust
writeln!(
    w,
    dr#"
      <!-- <link rel="shortcut icon" href="{rel}favicon.ico"> -->
    </head>
    <body>
      <div class="body">
        <h1 class="title">
          {h1}
          <span class="nav">{nav}</span>
        </h1>
    "#
);
```

The above conversion is elegant for these reasons:
- It is a simple modification by prepending `d` before the string literal
- All of the escaped sequences are removed, the indentation removal is taken care of by the dedented string literal
- Since we can now use a raw string, we no longer have to escape the quotes
- Notably: **All of the interpolated variables continue to work as before**.

With a dedented string *macro*, it's a much more involved process. The above will fail to compile because strings expanded from macros cannot capture variables like that.

The problem being that we have to re-write all of the captured variables to pass them to the `writeln!` and not the dedented string itself:

```rust
writeln!(
    w,
    dedent!(
        r#"
          <!-- <link rel="shortcut icon" href="{}favicon.ico"> -->
        </head>
        <body>
          <div class="body">
            <h1 class="title">
              {}
              <span class="nav">{}</span>
            </h1>
        "#
    ),
    rel,
    h1,
    nav
);
```

Which is unfortunate.

It might lead users to choose not to use this feature.

#### Limits macros

Macro fragment specifier `$lit: literal` is able to accept dedented string literals.

However, it won't be able to accept string literals created from a macro.

Today, the following code:

```rs
macro_rules! foo {
    ($lit:literal) => {{}};
}

fn main() {
    foo!(concat!("foo", "bar"));
}
```

Fails to compile:

```rs
error: no rules expected `concat`
 --> src/main.rs:6:10
  |
1 | macro_rules! foo {
  | ---------------- when calling this macro
...
6 |     foo!(concat!("foo", "bar"));
  |          ^^^^^^ no rules expected this token in macro call
  |
note: while trying to match meta-variable `$lit:literal`
 --> src/main.rs:2:6
  |
2 |     ($lit:literal) => {{}};
  |      ^^^^^^^^^^^^

error: could not compile `dedented` (bin "dedented") due to 1 previous error
```

A `dedent!()` macro will have the same restriction.

This limits yet again where the dedented strings could be used.

#### Consistency

It would be inconsistent to have dedicated syntax for raw string literals `r#"str"#`, but be forced to use a macro for dedented string literals.

The modifiers `b"str"` and `r#"str"#` are placed in front of string literals.

They do *no* allocation, only transforming the string at compile-time.

We do not use macros like `byte!("str")` or `raw!("str")` to use them, so having to use `dedent!("str")` would feel inconsistent.

Dedentation also happens at compile-time, transforming the string literal similar to how raw string literals `r#"str"#` do.

However, macros like `format!("{foo}bar")` allocate. That's one of reasons why there are no `f"{foo}bar"` strings. In Rust, allocation is explicit.

Someone learning about dedented strings, and learning that they're accessible as a macro rather than a string modifier similar to how `r#"string"#` is, may incorrectly assume that the reason why dedented strings require a macro is because allocation happens, and Rust is explicit in this regard.

And when they learn about the actual behaviour, it will be surprising.

#### Wrapping the string in a macro call causes an additional level of indentation

With dedented string literals:

```rs
fn main() {
    println!(
        d"
        create table student(
            id int primary key,
            name text
        )
        "
    );
}
```

With a `dedent!` built-in macro:

```rs
fn main() {
   println!(
       dedent!(
            "
            create table student(
                id int primary key,
                name text
            )
            "
       )
   );
}
```

With [postfix macros](https://github.com/rust-lang/rfcs/pull/2442), the situation would be better:

```rs
fn main() {
    println!(
        "
        create table student(
            id int primary key,
            name text
        )
        ".dedent!()
    );
}
```

However, since that RFC currently [does not](https://github.com/rust-lang/rfcs/pull/2442#issuecomment-2567115172) look like it will be included anytime soon, the ergonomics of this feature should not be blocked on postfix macros.

#### Composability

Dedented string literal modifier `d` composes with *all* existing string literal modifiers.

Converting a string literal into a dedented string literal is simple, just add a `d` and fix the compile errors if necessary.

If dedented strings were accessible as a macro `dedent!()` instead, this would be a harder transformation to do - because you now have to wrap the whole string in parenthesis and write `dedent!`.

## Impact of *not* implementing this RFC

- The Rust ecosystem will continue to rely on third-party crates like `indoc` that provide dedented string literals which only work with the macros provided by the crate.

  Composing them with macros from a different crate may not always be ergonomic.
- Examples and snippets of Rust code that would otherwise not depend on any dependency will not benefit from dedented string literals.
- Crates that would otherwise benefit from the feature, but do not consider it worth enough to add an additional dependency for, will not benefit from dedented string literals.

# Prior art
[prior-art]: #prior-art

In other languages:

- _Java_ - [text blocks](https://openjdk.java.net/jeps/378) using triple-quotes.
- _Kotlin_ - [raw strings](https://kotlinlang.org/docs/strings.html#raw-strings) using triple-quotes and `.trimIndent()`.
- _Scala_ - [multiline strings](https://docs.scala-lang.org/overviews/scala-book/two-notes-about-strings.html)
  using triple-quotes and `.stripMargin`.
- _C#_ - [Raw string literals](https://learn.microsoft.com/en-us/dotnet/csharp/language-reference/tokens/raw-string)
- _Python_ - [multiline strings](https://docs.python.org/3/library/textwrap.html) using triple-quotes and [`inspect.cleandoc`](https://docs.python.org/3/library/inspect.html#inspect.cleandoc)
  to avoid escaping and `textwrap.dedent`.
- _Jsonnet_ - [text blocks](https://jsonnet.org/learning/tutorial.html) with `|||` as a delimiter.
- _Bash_ - [`<<-` Heredocs](https://pubs.opengroup.org/onlinepubs/9699919799/utilities/V3_chap02.html#tag_18_07_04).
- _Ruby_ - [`<<~` Heredocs](https://www.rubyguides.com/2018/11/ruby-heredoc/).
- _Swift_ - [multiline string literals](https://docs.swift.org/swift-book/LanguageGuide/StringsAndCharacters.html#ID286)
  using triple-quotes - strips margin based on whitespace before closing
  delimiter.
- _Nix_ - [indented strings](https://nix.dev/manual/nix/2.29/language/string-literals.html)
- _Scala_ - [stripMargin](https://www.scala-lang.org/api/2.12.7/scala/collection/immutable/StringLike.html#stripMargin:String)
- _PHP_  - `<<<` [heredoc/nowdoc](https://wiki.php.net/rfc/flexible_heredoc_nowdoc_syntaxes#closing_marker_indentation)
  The indentation of the closing marker dictates the amount of whitespace to
  strip from each line.
- _JavaScript_ - [Proposal String Dedent](https://github.com/tc39/proposal-string-dedent)
- _MoonBit_ - [Multi-line Strings](https://docs.moonbitlang.com/en/latest/language/fundamentals.html#string)
- _Haskell_ - [Multi-line Strings](https://ghc.gitlab.haskell.org/ghc/doc/users_guide/exts/multiline_strings.html)

In the Rust ecosystem:

- [`dedent`](https://docs.rs/dedent/0.1.1/dedent/macro.dedent.html)
- [`textwrap-macros`](https://docs.rs/textwrap-macros/0.3.0/textwrap_macros/macro.dedent.html)
- [`indoc`](https://docs.rs/indoc/latest/indoc/)

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None

# Future possibilities
[future-possibilities]: #future-possibilities

## More string modifiers

At some point, Rust might gain new types of string modifiers. Such as `o"string"` which would create a `String`, for example. (only speculative)

Supporting these new hypothetical string modifiers means that the interaction between all possible string modifiers needs to be taken into account.

Each new string modifier could *double* the variety of string literals, possibly leading to combinatorial explosion.

## `rustfmt` support

Formatting tooling such as `rustfmt` will be able to make modifications to the source it previously would not have been able to modify, due to the modifications changing output of the program.

If indentation of the dedented string does not match the surrounding code:

```rust
fn main() {
    println!(
        d"   // 4 trailing spaces here
        create table student(
            id int primary key,
            name text
        )
            "
^^^^^^^^ // common leading indentation (will be removed)
    );
}
```

It could be automatically formatted by adding additional leading indentation, in order to align it with the surrounding source code:

```rust
fn main() {
    println!(
        d" // 0 trailing spaces here (stripped)
        create table student(
            id int primary key,
            name text
        )
        "
^^^^^^^^ // common leading indentation (will be removed)
    );
}
```

This would never modify the output, but make the source code more pleasant - and bring more automation and consistency to the Rust ecosystem.

With regular string literals, this isn't possible - as modifying the whitespace in the string changes the output.

## `clippy` lint to convert strings into dedented string literals

There could be a lint which detects strings which could be written clearer as dedented string literals.

## `rustc` warn-by-default lint to disallow whitespace escape characters

As explained in the [reference level explanation](#reference-level-explanation), using escapes `\t`, `\n` and `\r` is allowed.

Their behaviour might be surprising, so it is worth to consider a warn-by-default lint for them.
