- Feature Name: `inputln`
- Start Date: 2021-11-16
- RFC PR: [rust-lang/rfcs#3196](https://github.com/rust-lang/rfcs/pull/3196)

# Summary
[summary]: #summary

Add an `inputln` convenience function to `std::io` to read a line from standard
input and return a `std::io::Result<String>`.

# Motivation
[motivation]: #motivation

Building a small interactive program that reads input from standard input and
writes output to standard output is well-established as a simple and fun way of
learning and teaching a new programming language.  Case in point the chapter 2
of the official Rust book is [Programming a Guessing Game], which suggests the
following code:

[Programming a Guessing Game]: https://doc.rust-lang.org/book/ch02-00-guessing-game-tutorial.html

```rs
let mut guess = String::new();

io::stdin()
    .read_line(&mut guess)
    .expect("Failed to read line");
```

While the above code is perfectly clear to everybody who already knows Rust, it
can be quite overwhelming for a beginner because it confronts them with three
new concepts at once: mutability, borrowing and the `Result` type. Didactically
it would be better if these concepts could be introduced one at a time.  This
RFC therefore proposes the introduction of a `std::io::inputln` function so
that the above example could be simplified to just:

```rs
let guess = io::inputln().expect("Failed to read line");
```

This would allow for a more graceful introduction to Rust. Letting beginners
experience the exciting thrill of running their own first interactive Rust
program, without being confronted with mutability and borrowing straight away.
While mutability and borrowing are very powerful concepts, Rust does not force
you to use them when you don't need them. The examples we use to teach Rust to
complete beginners should reflect that.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

`std::io::inputln()` is a convenience wrapper around `std::io::Stdin::read_line`.
The function allocates a new String buffer for you, reads a line from standard
input and trims the newline (`\n` or `\r\n`).  The result is returned as a
`std::io::Result<String>`. When the input stream has reached EOF a
`std::io::Error` of kind `ErrorKind::UnexpectedEof` is returned.

If you are repeatedly reading lines from standard input and don't need to
allocate a new String for each of them you should be using
`std::io::Stdin::read_line` directly instead, so that you can reuse an existing
buffer.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

```rs
use std::io::{Error, ErrorKind, Write};

pub fn inputln() -> std::io::Result<String> {
    std::io::stdout().flush()?; // because print! doesn't flush

    let mut input = String::new();

    if std::io::stdin().read_line(&mut input)? == 0 {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "EOF while reading a line",
        ));
    }

    if input.ends_with('\n') {
        input.pop();
        if input.ends_with('\r') {
            input.pop();
        }
    }
    Ok(input)
}
```

The newline trimming behavior is the same as of `std::io::BufRead::lines`.

# Drawbacks
[drawbacks]: #drawbacks

* Can lead to unnecessary buffer allocations in Rust programs when developers
  don't realize that they could reuse a buffer instead. This could potentially
  be remedied by a new Clippy lint.

* `println!` and `writeln!` are both macros, so Rust programmers might out of
  habit try to call `inputln!()`. This should however not pose a big hurdle if
  `std::io::inputln` is added to `std::prelude` because in that case `rustc`
  already provides a helpful error message:

  ```
    error: cannot find macro `inputln` in this scope
    --> src/main.rs:13:5
     |
  13 |     inputln!();
     |     ^^^^^^^
     |
     = note: `inputln` is in scope, but it is a function, not a macro
   ```

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Why should the function trim newlines?

We assume that the returned string will often be processed with `String::parse`,
for example it is likely that developers will attempt the following:

```rs
let age: i32 = io::inputln()?.parse()?;
```

If `inputln()` didn't trim newlines the above would however always fail since
the `FromStr` implementations in the standard library don't expect a trailing
newline. Newline trimming is therefore included for better ergonomics, so that
programmers don't have to remember to add a `trim()` call whenever they want to
parse the returned string. In cases where newlines have to be preserved the
underlying `std::io::Stdin::read_line` can be used directly instead.

## Why should the function handle EOF?

If the function performs newline trimming, it also has to return an error when
the input stream reaches EOF because otherwise users had no chance of detecting
EOF. Handling EOF allows for example a program that attempts to parse each line
as a number and prints their sum on EOF to be implemented as follows:

```rs
fn main() -> std::io::Result<()> {
    let mut sum = 0;
    loop {
        match inputln() {
            Ok(line) => sum += line.parse::<i32>().expect("not a number"),
            Err(e) if e.kind() == ErrorKind::UnexpectedEof => break,
            Err(other_error) => {
                return Err(other_error);
            }
        }
    }
    println!("{}", sum);
    Ok(())
}
```

## Why should the function flush standard output before reading?

Users are bound to display prompts with the `print!` macro, for example they
might do:

```rs
print!("enter your name: ");
let name = io::inputln()?;
```

The `print!` macro however does not flush standard output, meaning if `inputln`
wouldn't flush standard output, the above code would read from standard input
without printing anything. `inputln` should therefore flush stdout to spare
users from walking into this pitfall.

The overhead this poses is negligible. Flushing standard output is effectively
a no-op if there's no buffered data. While flushing standard output can fail if
the file descriptor has been closed, the assumption in Rust programs generally
is that standard output is always both open and writable. For example the
`print!` and `println!` macros panic when their writing to standard output
fails, and [since 1.48.0 Rust reopens the standard file
descriptors](https://github.com/rust-lang/rust/pull/75295) with `/dev/null`
when they are closed on startup. While it has been suggested to [add a method
that closes standard output to the standard
library](https://github.com/rust-lang/rust/issues/40032), the proposal also
elaborated that standard output would immediately be reopend with `/dev/null`
to uphold that very assumption.

## Why should the function be implemented as a function instead of a macro?

If the function were implemented as a macro it could take an optional `prompt`
argument and only flush standard output when it is actually needed.

This function might however very well be the first time a Rust beginner
encounters the `Result` type, so it should really be impemented as an actual
function, so that it has a clear signature:

```rs
pub fn inputln() -> std::io::Result<String>
```

As opposed to a macro for which `rustdoc` would show something like:

```rs
macro_rules! prompt {
    () => { ... };
    ($($args : tt) +) => { ... };
}
```

which is not at all helpful for a beginner trying to understand what's going on.

## What is the impact of not doing this?

A higher chance of Rust beginners getting overwhelmed by mutability and borrowing.

# Prior art
[prior-art]: #prior-art

Python has [input()], Ruby has [gets], C# has `Console.ReadLine()`
... all of these return a string read from standard input.

[input()]: https://docs.python.org/3/library/functions.html#input
[gets]: https://ruby-doc.org/docs/ruby-doc-bundle/Tutorial/part_02/user_input.html

Other standard libraries additionally:

* accept a prompt to display to the user before reading from standard input
  (e.g. Python and Node.js)

* provide some functions to parse multiple values of specific data types
  into ovariables (e.g. C's `scanf`, C++, Java's `Scanner`)

Python's `input()` function can additionally make use of the GNU readline
library and Node.js' [readline](https://nodejs.org/api/readline.html) interface
provides a history and TTY keybindings as well.  The function suggested in this
RFC does not include such high-level features, these are better left to crates,
such as [`rustyline`](https://crates.io/crates/rustyline).

While scanning utilities could also be added to the Rust standard library, how
these should be designed is less clear, as well as whether or not they should be
in the standard library in the first place. There exist many well established
input parsing libraries for Rust that are only a `cargo install` away. The same
argument does not apply to `inputln()` ... beginners should be able to get
started with an interactive Rust program without having to worry about
mutability, borrowing or having to install a third-party library.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

> What parts of the design do you expect to resolve through the RFC process
> before this gets merged?

The name of the function is up to debate. `read_line` would also be an obvious
choice because the function wraps `std::io::Stdin::read_line`. Since the
function however additionally performs newline trimming and yields an error for
EOF (as opposed to returning an `Ok` variant), naming it the same might mislead
users into thinking that the function does not have these subtle differences.
In particular because there is precedent for convenience functions that share
the name of their underlying function to also behave the same
(`std::io::read_to_string` and `std::fs::read_to_string` both wrap
`Read::read_to_string` without processing the string or introducing additional
error sources).

Should the function additionally be added to `std::prelude`, so that beginners
can use it without needing to import `std::io`?

> What related issues do you consider out of scope for this RFC that could be
> addressed in the future independently of the solution that comes out of this RFC?

I consider the question whether or not scanning utilities should be added to the
standard library to be out of the scope of this RFC.

# Future possibilities
[future-possibilities]: #future-possibilities

Once this RFC is implemented:

* The Chapter 2 of the Rust book could be simplified
  to introduce mutability and borrowing in a more gentle manner.

* Clippy might also introduce a lint to tell users to avoid unnecessary
  allocations due to repeated `inputln()` calls and suggest
  `std::io::Stdin::read_line` instead.

With this addition Rust might lend itself more towards being the first
programming language for students.
