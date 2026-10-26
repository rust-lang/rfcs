- Feature Name: input_macros
- Start Date: 2025-04-19
- RFC PR: [rust-lang/rfcs#3799](https://github.com/rust-lang/rfcs/pull/3799)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC proposes the addition of macros or some functions that can be used to 
read input from the user in a more ergonomic way, similar to the 
[Input built-in function in Python](https://peps.python.org/pep-3111/). 

With this initiative, we can build a small interactive programs that reads input 
from standard input and writes output to standard output. This is 
well-established as a simple and fun way of learning and teaching Rust as a 
new programming language. 

```rust
print!("Please enter your name: ");
let possible_name: Result<String, _> = input!(); // This could fail, for example 
                                                 // if the user closes the input 
                                                 // stream

// Besides we can show a message to the user
let possible_age: Result<u8, _> = input!("Please enter your age: "); 
                                        // This could fail, for example if the 
                                        // user enters a string instead of a 
                                        // number in the range of u8

// And yes, this is a result so we can handle errors like this
let lastname: String = input!("Please enter your lastname: ")
                        .expect("The lastname is required"); 
                            // This could fail for example if the 
                            // user enters a empty string

// --- Another way to use the macro ---

struct Price {
    currency: String,
    amount: f64,
}

impl FromStr for Price {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split_whitespace().collect();
        if parts.len() != 2 {
            return Err("String must have two parts".to_string());
        }
        let currency = parts[0].to_string();
        let amount = parts[1].parse().unwrap();
        Ok(Price { currency, amount })
    }
}

let price: Price = input!("Please enter a price: ")?; 
        // This could fail for example if the input is reading from a pipe and 
        // we delete the file whose descriptor is being read while the
        // program is running

```

The examples above demonstrate several ways to use the `input!` macro.

In this macro, reaching EOF is consider an error case, so we return a `Result` 
with an error type indicating the cause of the EOF. This approach makes error 
handling straightforward and maintains consistent behavior.

Alternatively, the following can be used:

```rust
let name: Option<Price> = try_input!("Please enter a price: ")?;
```

In this case, EOF is not treated as an error, but as a valid case represented by
`None`. This allows handling EOF differently from other errors, since EOF 
indicates the absence of a value rather than an error condition. This is why the 
macro returns an `Option` type.

**Note**: The behavior of the `input!` is intended to be intuitive, but the 
`try_input!` may be useful in cases where more nuanced error handling is 
required. The name `try_input!` is provisional and open to change or removal 
based on further discussion and feedback.

The behaviour of `try_input!` is similar to `input!` but its return 
type is `Result<Option<T>, InputError>`. The `InputError` is a enum that 
contains the error that could be returned by the `input!` macro. This design was 
inspired by 
[commet of Josh Tripplet](https://github.com/rust-lang/rfcs/pull/3196#issuecomment-972915603) 
on [a previous RFC](https://github.com/rust-lang/rfcs/pull/3196). Handling EOF 
as a distinct case (rather than an error) allows for more flexible error 
handling, as EOF is not always an error but rather a less common scenario that 
can be overlooked.

# Motivation
[motivation]: #motivation

These macros could be especially useful for beginners, reducing the barrier to 
entry for new Rustaceans. They would also make the language more approachable 
and help lower the cognitive load when learning Rust.

For example, the second chapter of the Rust book introduces a guessing game and 
demonstrates how to read input from the user. The current approach is not very 
beginner-friendly and can be difficult to explain, especially concepts like 
buffers. Using the `input!` macro would simplify this process and make it more 
accessible.

This functionality is highly requested by the community, we can find as 
evidenced a [PR in Rust](https://github.com/rust-lang/rust/pull/75435) with 
many comments and a lot of discussion about this topic. We can also find
other [RFC](https://github.com/rust-lang/rfcs/pull/3183), 
[another RFC](https://github.com/rust-lang/rfcs/pull/3196) and
many issues in the Rust repository that discuss this topic.
In addition, this is not a new idea, as we can find similar topics in 
[internal.rust-lang discussions](https://internals.rust-lang.org/t/pre-rfc-input-macro/527).

This RFC aims to provide a solution to this problem. 

Like many others, I would like to have a simple way to read input from the user
without having to deal with the complexities of the standard library. 

This RFC proposes to allow us a more graceful introduction to Rust, not only a 
utility function. It aims to make the language more approachable and
friendly for new users of the language.

The idea behind creating a new macro comes from these discussions.
In many of these discussions, examples are shown of possible implementations, but
most of them use macros to implement this functionality.
The idea could be implemented as a function, too, actually [an old poll](https://strawpoll.com/zxds5jye6/results)
shows that the majority wants the feature, the specific implementation is less 
important.
The implementation of this RFC is a macro, but it could be changed to a function
if the community prefers that way or we could have both options.

This poll was mentioned in [this comment](https://github.com/rust-lang/rfcs/pull/3183#issuecomment-979421461)
on a previous RFC. (Thanks to [undersquire](https://github.com/undersquire))

Besides, the idea of this implementation is solve too the problem of parsing 
some types for example in other languages we have functions like `nextInt()` in 
Java which is a function that reads an integer from the input stream and returns 
it. We don't have this in Rust, we have to use the `read_line` function and then 
trim the string and parse it into the type that we want. 
And this is a very common use case, we sometime read a string from the input
but we want to parse it into a number or a struct.

If we repeat this process many times in our code, we have a lot of boilerplate
code that we can avoid with this macro. 

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Explaining the idea is not a rabbit hole, it's a very basic idea.
- We add a new macro `input!` that can be used to read input from the user in a 
  more friendly way. This macro returns a `Result<T, InputError>`.
- We add a new macro `try_input!` that can be used to read input from the user 
  in a more friendly way but with a different behaviour, this macro return a 
  `Result<Option<T>, InputError>`.
- In both cases we must to accept a type `T` where `T` is a type who implement 
  `FromStr` trait, so we can convert the input to the type that we want.
- We must to specify the `InputError` type who must to be a enum that have three 
  variants:
    - `EOF` that is the error that we get when we reach the end of the input 
      stream.
    - `Parse(e)` that is the error that we get when we can't parse the input to 
      the type that we want, `e` is the equivalent to a variable which type is 
      `FromStr::Err`.
    - `Io(e)` that is the error that we get when we have a IO error, `e` is the 
      equivalent to a variable which type is `std::io::Error`.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

```rs
use std::{fmt::Arguments, io::{self, BufRead, Write}, str::FromStr};

/// A macro that:
/// - optionally prints a prompt (with `print!`).
/// - reads **one line** from stdin.
/// - returns `Err(InputError::Eof)` if EOF is encountered.
/// - returns `Err(InputError::Parse(e))` if the input cannot be parsed.
/// - returns `Err(InputError::Io(e))` if an IO error occurs.
///
/// # Usage:
/// ```no_run
/// // No prompt
/// let text: String = input!().unwrap();
///
/// // With prompt
/// let name: String = input!("Enter your name: ").unwrap();
///
/// // Formatted prompt
/// let user = "Alice";
/// let age: String = input!("Enter {}'s age: ", user).unwrap();
/// ```
#[macro_export]
macro_rules! input {
    () => {{
        $crate::read_input_from(
            &mut ::std::io::stdin().lock(),
            None,
            $crate::PrintStyle::Continue,
        )
    }};
    ($($arg:tt)*) => {{
        $crate::read_input_from(
            &mut ::std::io::stdin().lock(),
            Some(format_args!($($arg)*)),
            $crate::PrintStyle::Continue
        )
    }};
}

/// A macro that:
/// - prints the prompt on its own line (with `println!`),
/// - then reads one line,
/// - returns `Err(InputError::Eof)` if EOF is encountered.
/// - returns `Err(InputError::Parse(e))` if the input cannot be parsed.
/// - returns `Err(InputError::Io(e))` if an IO error occurs.
/// - otherwise parses into `String`.
///
/// # Usage:
/// ```no_run
/// let line: String = inputln!("What's your favorite color?").unwrap();
/// ```
#[macro_export]
macro_rules! inputln {
    () => {{
        $crate::read_input_from(
          &mut ::std::io::stdin().lock(), 
          None, 
          $crate::PrintStyle::NewLine
        )
    }};
    ($($arg:tt)*) => {{
        $crate::read_input_from(
          &mut ::std::io::stdin().lock(), 
          None, 
          $crate::PrintStyle::NewLine
        )
    }};
}

/// A single function that:
/// 1. Optionally prints a prompt (and flushes).
/// 2. Reads one line from the provided `BufRead`.
/// 3. Returns `Err(InputError::Eof)` if EOF is reached.
/// 4. Parses into type `T`, returning `Err(InputError::Parse)` on failure.
/// 5. Returns `Err(InputError::Io)` on I/O failure.
pub fn read_input_from<R, T>(
    reader: &mut R,
    prompt: Option<Arguments<'_>>,
    print_style: PrintStyle,
) -> Result<T, InputError<T::Err>>
where
    R: BufRead,
    T: FromStr,
    T::Err: std::fmt::Display + std::fmt::Debug,
{
    if let Some(prompt_args) = prompt {
        match print_style {
            PrintStyle::Continue => {
                // Use print! for no newline
                print!("{}", prompt_args);
            }
            PrintStyle::NewLine => {
                // Use println! for adding a newline
                println!("{}", prompt_args);
            }
        }
        // Always flush so the user sees the prompt immediately
        io::stdout().flush().map_err(InputError::Io)?;
    }

    let mut input = String::new();
    let bytes_read = reader.read_line(&mut input).map_err(InputError::Io)?;
    
    // If 0, that's EOF — return Eof error
    if bytes_read == 0 {
        return Err(InputError::Eof);
    }

    let trimmed = input.trim_end_matches(['\r', '\n'].as_ref());
    trimmed.parse::<T>().map_err(InputError::Parse)
}

/// A unified error type indicating either an I/O error, a parse error, or EOF.
#[derive(Debug)]
pub enum InputError<E> {
    /// An I/O error occurred (e.g., closed stdin).
    Io(io::Error),
    /// Failed to parse the input into the desired type.
    Parse(E),
    /// EOF encountered (read_line returned 0).
    Eof,
}

/// Defines how the prompt should be printed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrintStyle {
    /// Print the prompt without a trailing newline (uses `print!`).
    Continue,
    /// Print the prompt with a trailing newline (uses `println!`).
    NewLine,
}

impl<E: std::fmt::Display + std::fmt::Debug> std::fmt::Display for InputError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InputError::Io(e) => write!(f, "I/O error: {}", e),
            InputError::Parse(e) => write!(f, "Parse error: {}", e),
            InputError::Eof => write!(f, "EOF encountered"),
        }
    }
}

impl<E: std::fmt::Display + std::fmt::Debug> std::error::Error for InputError<E> {}
```

Another thing to consider is that the `input!` and `inputln!` macros just 
delegate to the `read_input_from` function, which is the core of the
implementation. This function is generic over the reader type and the type
that we want to parse. Besides that, we have a `PrintStyle` enum that is used to
determine how to print the prompt. The `PrintStyle` enum has two variants:
- `PrintStyle::Continue` that is used to print the prompt without a trailing 
  newline.
- `PrintStyle::NewLine` that is used to print the prompt with a trailing
  newline.
It allow us to use the same function for both macros and to have a more
ergonomic way to print the prompt.

You can find the implementation in this [repository](https://github.com/Phosphorus-M/input-lib) or you can install it from crates.io with:

```sh
cargo install input-lib
```

# Drawbacks
[drawbacks]: #drawbacks

* Can lead to unnecessary buffer allocations in Rust programs when developers
  don't realize that they could reuse a buffer instead. This could potentially
  be remedied by a new Clippy lint.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives


## Why should the macro trim newlines?

We assume that the returned string will often be processed with 
`FromStr::from_str`, for example it is likely that developers will attempt the 
following:

```rs
let age: i32 = input!()?;
```

If `input()` didn't trim newlines the above would however always fail since
the `FromStr` implementations in the standard library don't expect a trailing
newline. Newline trimming is therefore included for better ergonomics, so that
programmers don't have to remember to add a `trim` or a `trim_end_matches` call 
whenever they want to parse the returned string. In cases where newlines have to 
be preserved the underlying `std::io::Stdin::read_line` can be used directly 
instead.

This is the default behavior in Python and C# for give you a example, these 
cases trim newlines by default. In Go, the `bufio.Reader.ReadString()` function
does not trim newlines, but the `bufio.Scanner` type does. The `bufio.Scanner`
type is the one that is used in the Go standard library for reading input
from the console. The `bufio.Scanner` type is a higher level abstraction.

## Why should the function handle EOF?

If the function performs newline trimming, it also has to return an error when
the input stream reaches EOF because otherwise users had no chance of detecting
EOF. Handling EOF allows for example a program that attempts to parse each line
as a number and prints their sum on EOF to be implemented as follows:

```rs
fn main() -> Result<(), InputError<ParseFloatError>> {
    let mut sum = 0;
    loop {
        let result: Result<f64, _> = input!();
        match result {
            Ok(mut prices) => sum += prices,
            Err(InputError::Eof) => {
                println!("Oh no, EOF!");
                break;
            }
            Err(e) => {
                println!("Error: {:?}", e); // e could be a ParseFloatError
                                            // or an IO error
                return Err(e);
            }
        }
    }
    println!("{}", sum);
    Ok(())
}
```

## What is the impact of not doing this?
If this RFC is not accepted, the current way of reading input from the user
will remain the same. This means that new Rustaceans will have to learn how to
use the `std::io::Stdin::read_line` function and how to handle the `Buffer`
and the `String` types. This can be a barrier to entry for new Rustaceans and
it can make the language less friendly. 

This RFC was presented as a pre-RFC at a [Rust Argentina meetup](https://rust-lang.ar/p/2025-april/), 
where it received positive feedback, particularly from attendees new to Rust 
(many with backgrounds in NodeJS, Python, and Go). They found the current 
approach to reading input in Rust complex and not very user-friendly, and were 
enthusiastic about the proposed macros.

They were not too happy with the current way of reading input from the user. 
They think that it was too complex and not too friendly. 

The presentation can be found [here](https://youtu.be/CjZq93pzOkA?t=4080) (in 
Spanish)
And yes, the guy with the black shirt with the Rust logo is me (👋).

## Could this be done in a library or macro instead? 

Well yes, but I think that this is a good idea to have it in the standard
library. I think that this is a good idea to have it in the standard library
because it is a common use case and it is a good idea to have it in the
standard library to make the language more friendly.

Another way that we could consider this is added like a pseudo oficial library 
like `rand` which is not in the standard library but is recommended by the Rust 
team and the oficial documentation as the book does it.

# Prior art
[prior-art]: #prior-art

Python has `input()`, Ruby has `gets`, C# has `Console.ReadLine()`
... all of these return a string read from standard input.

Some behaviors are different, for example:
- Python's `input()` returns a string, but if the input is EOF
  it raises an `EOFError` exception.
- Ruby's `gets` returns `nil` if the input is EOF.
- C#'s `Console.ReadLine()` returns `null` if the input is EOF.
- Go's `bufio.Reader.ReadString()` returns an error if the input is EOF.
- Java's `readln` returns an `null` if the input is EOF.

Maybe a thing to have in mind is that in JavaScript we have the `prompt` function
but this function in the case of the browser enable a little dialog when we 
select cancel this dialog we receive a `null` value.
This is a [specification in JavaScript](https://html.spec.whatwg.org/multipage/timers-and-user-prompts.html#dom-prompt-dev).

# Unresolved questions
[unresolved-questions]: #unresolved-questions

> What parts of the design do you expect to resolve through the RFC process
> before this gets merged?

Should the function additionally be added to `std::prelude`, so that beginners
can use it without needing to import `std::io`?

# Future possibilities
[future-possibilities]: #future-possibilities

Once this RFC is implemented:

* The Chapter 2 of the Rust book could be simplified
  to introduce mutability and borrowing in a more gentle manner.

  In the current version of the book, the code looks like this:
  ```rust
  let mut guess = String::new();

  io::stdin()
      .read_line(&mut guess)
      .expect("Failed to read line");

  let guess: u32 = guess.trim().parse().expect("Please type a number!");

  println!("You guessed: {guess}");
  ```

  With the new macros, it could be simplified to:  

  ```rust
  let guess: u8 = input!().expect("Please type a number!");
  println!("You guessed: {guess}");
  ```

* Clippy might also introduce a lint to tell users to avoid unnecessary
  allocations due to repeated `inputln()` calls and suggest
  `std::io::Stdin::read_line` instead.

With this addition Rust might lend itself more towards being the first
programming language for students.
