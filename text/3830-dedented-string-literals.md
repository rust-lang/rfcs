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

The output is formatted in an unconventional way, containing excessive leading whitespace.

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

   This makes it confusing to tell which scope the string belongs to. This is especially true when there are multile scopes involved:

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

3. The closing double-quote must be put at the beginning of the line, in order not to introduce trailing whitespace:

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

Sometimes, we are *forced* into the first option - sacrifice readability of the source.

In some cases, producing excessive whitespace will change meaning of the output.

Consider whitespace-sensitive languages such as Python or Haskell, or content which is meant to be read by people like generated Markdown - here we *can't* make a sacrifice on readabilty of the output - so our source code must become harder to understand.

But, what if we could have the best of both worlds?

### Dedented string literals

In order to solve these problems, the RFC proposes dedented string literals of the form: `d"string"`.

Common leading whitespace on each line after the closing quote in dedented string literals will be stripped at compile-time.

This allows us to have a more readable version of the above:

```rust
fn main() {
    println!(d"
        create table student(
            id int primary key,
            name text
        )
        ");
^^^^^^^^ // common leading whitespace (will be removed)
}
```

All of the above problems are gracefully solved:

1. Indentation level inside the string is the same as what is in the output.
1. It does not require escaping the first newline for it to look readable.
1. Nicely composes with raw string literal: `dr#"string"#`, in which the first newline *cannot* be escaped.
1. Indentation level of the statement is larger than the `println!` call,
   making it more obvious that the string is inside the call at a glance.
1. The closing parentheses in the SQL statement aligs with `create table`
   and is 1 level larger than `println!`.

Now, consider the example with multiple nested scopes again:

```rs
fn main() {
    {
        println!(d"
            create table student(
                id int primary key,
                name text
            )
            ");
    }
    println!(d"
        create table student(
            id int primary key,
            name text
        )
        ");
    {
        {
            println!(d"
                create table student(
                    id int primary key,
                    name text
                )
                ");
        } 
    }
}
```

It is immediately more obvious which string belongs to which scope.

## Closing quote controls the removed indentation

All of the common whitespace between each line, which has a higher indentation than the indentation of the line of closing quote (contained in the last line) is stripped.

Here are a few examples to demonstrate.

### No indentation is stripped when the closing quote has no indentation

The output is the same as what is in the source code.

This allows all lines to have a common indentation.

```rust
fn main() {
    println!(d"
        create table student(
            id int primary key,
            name text
        )
");
// no common leading whitespace = nothing to remove
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
    println!(d"
        create table student(
            id int primary key,
            name text
        )
    ");
^^^^ // common leading whitespace (will be removed)
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
    println!(d"
        create table student(
            id int primary key,
            name text
        )
        ");
^^^^^^^^ // common leading whitespace (will be removed)
}
```

The indentation of the ending double quote is 8 spaces. This common prefix of leading whitespace characters will be removed from the beginning of each line.

Prints:

```sql
create table student(
    id int primary key,
    name text
)
```

Result: **all indentation from source is stripped**.

Indenting the closing double quote further will have zero impact.
The dedentation will never remove non-whitespace characters.

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
    println!(d"
        create table student(
            id int primary key,
            name text
        )
            ");
^^^^^^^^ // common leading whitespace: 8 spaces
^^^^^^^^^^^^ // closing quote indentation: 12 spaces
}

// spaces removed from the beginning of each line = min(8, 12) = 8
```

```rs
fn main() {
    println!(d"
        create table student(
            id int primary key,
            name text
        )
                ");
^^^^^^^^ // common leading whitespace: 8 spaces
^^^^^^^^^^^^^^^^ // closing quote indentation: 16 spaces
}
// spaces removed from the beginning of each line = min(8, 16) = 8
```

```rs
fn main() {
    println!(d"
        create table student(
            id int primary key,
            name text
        )
                    ");
^^^^^^^^ // common leading whitespace: 8 spaces
^^^^^^^^^^^^^^^^^^^^ // closing quote indentation: 20 spaces
}
// spaces removed from the beginning of each line = min(8, 20) = 8
```

## Composition with other string literal modifiers, such as raw string literals and byte string literals

Dedented string literals `d"string"` are a new modifier for strings.

They are similar to byte strings `b"string"` and raw strings `r#"string"#`.

They compose with other every other string literal modifier.

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

    println!(d"
        create table {table_name}(
            id int primary key,
            name text
        )
        ");
^^^^^^^^ // common leading whitespace (will be removed)
}
```

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Any kind of string literal can turn into a "dedented" string literal if it is prefixed with a `d`:

- strings: `"string"` -> `d"string"`
- Raw strings: `r#"string"` -> `dr#"string"`
- Byte strings: `b#"string"` -> `db#"string"`
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
//^^ whitespace is removed

assert_eq!(dedented, "I am a dedented string literal!");
```

Common indentation of all lines up to, but **not including** the closing quote `"` is removed from the beginning of each line.

Indentation present *after* the double-quote is kept:

```rs
//               ↓ newline is removed
let dedented = d"
        I am a dedented string literal!   
    ";                               //^ newline is removed
//^^ whitespace is removed
//  ^^^^ indentation after the double quote is kept

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

let py = format!(dr#"
    def hello():
        print("{message}")

    hello()
    "#);
//^^ removed

let expected = "def hello():\n    print(\"Hello, world!\")\n\nhello()";
assert_eq!(py, expected);
```

By placing the ending quote earlier than the first non-whitespace character in any of the lines, you can reduce how much space is removed from the beginning of each line:

```rs
use std::io::Write as _;

let message = "Hello, world!";
let mut py = String::new();

// Note: Using `writeln!` because the final newline from dedented strings is removed. (more info later)

writeln!(py, d"
    def hello():
    ");
//^^ removed

// Note: We want to add 2 newlines here.
// - `writeln!` adds 1 newline at the end
// - An additional empty line is added
//   to insert the 2nd newline

// Remember, dedented string literals strip the last newline.
writeln!(py, dr#"
    print("{message}")

"#);
//^^ kept

write!(py, d"
hello()
            ");
//^^^^^^^^^^ No whitespace is removed here.
//           If the closing quote is after the common indentation
//           (in this case there is no common indentation at all),
//           all of the whitespace is stripped

let expected = "def hello():\n    print(\"Hello, world!\")\n\nhello()";
assert_eq!(py, expected);
```

## Rules

### Dedented string literals must begin with a newline

All dedented string literals must begin with a newline.
This newline is removed.

The following is invalid:

```rust
//         ↓ error: expected literal newline.
//           note: dedented string literals must start with a literal newline
//           help: insert a literal newline here: 
let py = d"def hello():
        print('Hello, world!')

    hello()
    ";
```

Escape-code newline is not supported, it must be a literal newline:

```rust
//         ↓ error: expected literal newline, but found escaped newline.
//           note: dedented string literals must start with a literal newline
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

### Last line must be empty, and preceded by a literal newline

The line which contains the closing quote `"` must be empty, and the character before the last line must be a literal newline character.

This is invalid:

```rust
let py = d"
    def hello():
        print('Hello, world!')

    hello()";
//         ^ error: expected literal newline
//           note: in dedented string literals, the line
//                 which contains the closing quote must be empty
```

Neither is using an escaped newline `\n` instead of the literal newline:

```rust
let py = d"
    def hello():
        print('Hello, world!')

    hello()\n";
//         ^ error: expected literal newline, but found escaped newline
//           note: in dedented string literals, the line
//                 which contains the closing quote must be empty
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

Benefits the above rules bring include:

- The above rules make all dedented string literals you'll find in Rust consistent.
- It allows easily changing the indentation level without having to insert a newline sometimes.
- It gives the ability for us to tell a regular string literal from a dedented string literal at a glance.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

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

> [!NOTE]
> 
> Whitespace escape characters such as `\t`, `\r` and `\n` are treated as literal code when present in the content of the dedented string, therefore the normal dedentation rules apply to them.
>
> This does not apply to `\n` after the opening quote, nor the `\n` before the line containing the closing quote. In this case escaping the newline is not allowed, it has to be a literal newline. (As described previously.)

1. The opening line (the line containing the opening quote `"`)
    - Must only contain a literal newline character after the `"` token
    - This newline is removed.
1. The closing line (the line containing the closing quote `"`)
    - Must contain only whitespace before the closing quote
    - This whitespace is the *closing indentation*.
    - The closing indentation is removed.
1. The character immediately before the closing line must be a literal newline character.
    - This newline is removed.
1. The *common indentation* is calculated.

   It is the largest amount of leading whitespace shared by all non-empty lines.

1. For each non-empty line, remove the smallest amount of leading whitespace that satisfies:

    - `min(common indentation, closing indentation)`

    What this means is:
    - Even if a line is indented by more than the closing indentation
    - Only the amount equal to the closing indentation, or less, will be removed.
    - Never more than the line actually has.

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
^^^^ // common leading whitespace (will be removed)

    "hello\nworld"
);

// This example has the same whitespace as the previous example.
// However, here we make use of whitespace escape characters
//
// This might make code more confusing, so one of the future-possibilities
// is to have a warn-by-default lint to disallow these characters in dedented strings.
assert_eq!(
    d"
\thello\n••\n\tworld
    ",

    "hello\nworld"
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
    ",
````

# Drawbacks
[drawbacks]: #drawbacks

- Contributes to the increase of string literal modifiers by adding a new variant.

  While at the moment the variety of string literal modifiers is small, it is worth to think about the implications of exponential increase of them.

  Currently, Rust has 7 types of string literals. This RFC will increase that to 13, because each string literal can be prefixed with a `d` to make it dedented.

  In the future Rust might get additional types of "string modifiers", and each combination will need to
  be accounted for.

- Increases complexity of the language. While it builds upon existing concepts, it is yet another thing for people to learn.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Design

### The choice of `d"string"` specifically

The syntax of `d"string"` is chosen for the following reasons:

- Fits with existing string modifiers, such as `b"string"`, `r#"string"#"` and `c"string"`
- Composes with existing string modifiers: `db"string"`, `dc"string"`, `dr#"string"#`, and `dbr#"string"#`. 
- Does not introduce a lot of new syntax. Dedented string literals can be explained in terms of existing language features.
- The acronym `d` for `dedent` is both clear, and not taken by any of the other string modifiers.
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

### Requirement of first and final newline

As mentioned earlier in the RFC:

- There must be a literal newline present directly after the opening quote `"`.
- There must be a literal newline present directly before the line containing the closing quote `"`.

Having this as a hard requirement will make usages of dedented string literals more consistent.

Consider the following which is invalid:

```rs
fn main() {
    // ERROR
    println!(d"create table student(
            id int primary key,
            name text
        )
        ");
}
```

- The `d"` and `create` in the first `d"create` not being separated by whitespace makes it harder to understand where the code begins. They have to be mentally separated.
- Additionally, indentation of the `create` does not align with what it will look like in the output, making it less obvious, which we would like to aviod. Therefore it is a **hard error** to not have a literal newline there.

The following is also incorrect, as there is no newline before the line containing the closing quote:

```rs
fn main() {
    println!(d"
        create table student(
            id int primary key,
            name text
        )"); // ERROR
}
```

- Having the closing quote **always** be on its own line makes it more obvious to the reader from which column onwards leading indentation will be removed.
- In the example above, it is not immediately clear where that would be from.
- It easy to modify the common indentation level of the string in the future, as you do not have to create a new line.

### Allowing the closing line to be indented more than previous lines

Having the quote be indented further than the first non-whitespace character in the
string content is allowed:

```rs
fn main() {
    println!(d"
        create table student(
            id int primary key,
            name text
        )
                ");
}
```

Reason: turning this into a syntax error is too strict, when it can be auto-fixed by tooling like `rustfmt`:

```rs
fn main() {
    println!(d"
        create table student(
            id int primary key,
            name text
        )
        ");
}
```

## Differences from RFC 3450

The [RFC #3450: Propose code string literals](https://github.com/rust-lang/rfcs/pull/3450) is similar to this one, however this RFC is different and this section explains why.

Differences:

- #3450 uses `h` as the modifier instead of `d`.

    proposes using `h` as acronym for [Here document](https://en.wikipedia.org/wiki/Here_document).

    The term is likely to be less known, and may raise confusion.

    Additionally, here documents are more associated with "code blocks". While this feature is useful for code blocks, it is not just for them.

    While the `d` mnemonic for **dedent** clearly describes what actually happens to the strings.

- #3450 allows to write an *info string*, like in markdown.

    It proposes the ability to write:

    ```rs
    let sql = d"sql
        SELECT * FROM table;
        ";
    ```

    With the `sql` not affecting the output, but can aid in syntax highlighting and such.

   1. This is not necessary, as at the moment you can add a block comment next to the string, which syntax highlighters can use *today* to inject whatever language is specified.
 
         ```rs
         let sql = /* sql */ "SELECT * FROM table;";
         ```
 
   2. Is considered out of scope for this RFC to consider.
 
         It would be a backward-compatible change to make for a future RFC, if it's desired.

   3. [Expression attributes](https://github.com/rust-lang/rust/issues/15701) are likely to be more suitable for this purpose. (not part of this RFC)

         ```rs
         let sql = #[editor::language("sql")] "SELECT * FROM table;";
         ```

- RFC #3450 makes the "code strings" always end with a newline, with the ability to prepend a minus before the closing quote in order to remove the final newline.

    However, in this RFC the following:

    ```rs
    print!(d"
        a
        ");
    ^^^^ // common leading whitespace (will be removed)
    ```

    Prints: `a`

    **Without** an ending newline.

    In order to add a newline at the end, you have to add a newline in the source code:

    ```rs
    print!(d"
        a

        ");
    ^^^^ // common leading whitespace (will be removed)
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
        ^^^^ // common leading whitespace (will be removed)
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
        ^^^^ // common leading whitespace (will be removed)
        url = "http://localhost:8080",
        mime = "application/json",
    }
    ```

    Both snippets print:

    ```
    GET http://localhost:8080
    Accept: application/json
    ```

    Note that `eprintdoc!` does not remove the final line, that's why we use `eprintln` instead of `eprint`.

- `indoc!`: Dedents the passed string.

    Before:

    ```rs
    indoc! {r#"
        def hello():
            print("Hello, world!")

        hello()
    "#}
    ^^^^ // common leading whitespace (will be removed)
    ```

    With dedented string literals:

    ```rs
    dr#"
        def hello():
            print("Hello, world!")

        hello()

    "#
    ^^^^ // common leading whitespace (will be removed)
    ```

    Both snippets evaluate to:

    ```py
    def hello():
        print("Hello, world!")

    hello()
    ```

    Note that `indoc!` does not remove the final line, that's why we add an additional newline after `hello()`.

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
text!(d"
    GET {url}
    Accept: {mime}
")
^^^^ // common leading whitespace (will be removed)
```

The language feature works with any user-defined macros that pass their arguments to `format_args!` under the hood.

#### Improved compile times

Having dedented strings as a language feature, instead of relying on a macro provided by a crate could reduce compile time.

- Users do not have to compile the crate *or* its dependencies.
- There is no need for procedural macro expansion to take place in order to un-indent the macro. This step happens directly in the compiler.

## Use a built-in macro instead

What about using a compiler built-in macro like `dedent!("string")` instead of a language-built in string modifier such as `d"string"`?

### Advantages

- Will likely have similar performance to the literal itself.

### Disadvantages

#### The macro will be unable to capture variables from the surrounding scope

One of the major benefits of having dedented string literals is that you'll be able to use them in formatting macros:

```rs
let message = "Hello, world!";

// `{message}` is interpolated
let py = format!(dr#"
    def hello():
        print("{message}")

    hello()
    "#);
//^^ removed

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
  writeln!(w, "  \
        <!-- <link rel=\"shortcut icon\" href=\"{rel}favicon.ico\"> -->\
    \n</head>\
    \n<body>\
    \n  <div class=\"body\">\
    \n    <h1 class=\"title\">\
    \n      {h1}\
    \n      <span class=\"nav\">{nav}</span>\
    \n    </h1>")
```

Into a dedented string literal:

```rust
  writeln!(w, dr#"
      <!-- <link rel="shortcut icon" href="{rel}favicon.ico"> -->
    </head>
    <body>
      <div class="body">
        <h1 class="title">
          {h1}
          <span class="nav">{nav}</span>
        </h1>
    "#);
```

The above conversion is elegant for these reasons:
- It is a simple modification by prepending `d` before the string literal
- All of the escaped sequences are removed, the whitespace removal is taken care of by the dedented string literal
- Since we can now use a raw string, we no longer have to escape the quotes
- Notably: **All of the interpolated variables continue to work as before**.

With a dedented string *macro*, it's a much more involved process. The above will fail to compile because strings expanded from macros cannot capture variables like that.

The problem being that we have to re-write all of the captured variables to pass them to the `writeln!` and not the dedented string itself:

```rust
  writeln!(w,
    dedent!(r#"
          <!-- <link rel="shortcut icon" href="{}favicon.ico"> -->
        </head>
        <body>
          <div class="body">
            <h1 class="title">
              {}
              <span class="nav">{}</span>
            </h1>
        "#),
    rel,
    h1,
    nav)
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

This limits yet again where the dedented strings count be used.

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
   println!(d"
       create table student(
           id int primary key,
           name text
       )
       ");
}
```

With a `dedent!` built-in macro:

```rs
fn main() {
   println!(
       dedent!("
           create table student(
               id int primary key,
               name text
           )
           ")
       );
}
```

With [postfix macros](https://github.com/rust-lang/rfcs/pull/2442), the situation would be better:

```rs
fn main() {
   println!("
       create table student(
           id int primary key,
           name text
       )
       ".dedent!());
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
- _Python_ - [multiline strings](https://docs.python.org/3/library/textwrap.html) using triple-quotes
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

What should happen if we have tabs (represented by `→`) and literal spaces (represented by `•`) mixed together?

```rust
let py = d"
→→→→def hello():
→→→→••••print('Hello, world!')

•→••hello()
→→••";
```

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
    println!(d"
    create table student(
        id int primary key,
        name text
    )
    ");
^^^^ // common leading whitespace (will be removed)
}
```

It could be automatically formatted by adding additional leading indentation, in order to align it with the surrounding source code:

```rust
fn main() {
    println!(d"
        create table student(
            id int primary key,
            name text
        )
        ");
^^^^^^^^ // common leading whitespace (will be removed)
}
```

This would never modify the output, but make the source code more pleasant - and bring more automation and consistency to the Rust ecosystem.

With regular string literals, this isn't possible - as modifying the whitespace in the string changes the output.

## `clippy` lint to convert strings into dedented string literals

There could be a lint which detects strings which could be written clearer as dedented string literals.

## `rustc` warn-by-default lint to disallow whitespace escape characters

In the following example:

```rs
assert_eq!(
    d"
\thello\n\t\n\tworld
\t",
//^^ common leading whitespace (will be removed)

    "hello\nworld"
);
```

Using escaped whitespace characters is the same as if the characters were written literally. (in the *content* of the string. This excludes the requirement of a **literal** newline after the double quote and before the line of the closing quote).

This is confusing, and might not work in the way people expect it to work. A warn-by-default lint could be added to disallow `\n`, `\t` and `\r` in dedented strings. (Or for instance, only allow `\t` anytime after the stripped indentation)
