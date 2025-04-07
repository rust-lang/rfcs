- Feature Name: `input macros`
- Start Date: 2025-04-02)
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC propose the addition of macros and some functions that can be used to 
read input from the user in a more ergonomic way like the 
[Input built-in function in Python](https://peps.python.org/pep-3111/). 

With this initiative we can build a small interactive programs that reads input 
from standard input and writes output to standard output is well-established as 
a simple and fun way of learning and teaching Rust as a new programming 
language. 

```rust
println!("Please enter your name: ");
let possible_name: Result<String, _> = input!(); // This could fail for example 
                                                 // if the user closes the input 
                                                 // stream

// Besides we can show a message to the user
let possible_age: Result<u8, _> = input!("Please enter your age: "); 
                                        // This could fail for example if the 
                                        // user enters a string instead of a 
                                        // number in the range of u8

// And yes, this is a result so we can handle errors like this
let lastname = input!("Please enter your lastname: ")
                .expect("The lastname is required"); 
                    // This could fail for example if the 
                    // user enters a empty string

// --- Other way to use the macro ---

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

let price: Price = input!("Please introduce a price: ")?; 
        // This could fail for example if the input is reading from a pipe and 
        // we delete the file whose descriptor is being read meanwhile the 
        // program is running

```

In these examples I show many ways to use the `input!` macro.

In this macro we think that EOF is error case, so we return a `Result` with the 
error type being the error that caused the EOF. This is because is easily to 
handle the error for something new and we can mantain a similar behavior.

However we can use besides:

```rust
let name: Option<String> = try_input!("Please introduce a price: ")?;
```

For example, that in this we can handle the error in a different way.
If we get a EOF we can return `None` and handle it in a different way but it's 
not exactly a error, it's a different case, EOF is valid but doesn't have a 
value, a way to represent this, that is why we use a `Option`.

**DISCLAIMER**: The behavior of the `input!` to me is the most intuitive, but I 
think that the `try_input!` could be useful in some cases to be correct with the 
error handling. We can change the name of the macro `try_input!` or delete it if 
we think that is not necessary. It's just a idea, I'm open to suggestions.

The behaviour of the `try_input!` is the same as the `input!` but the return 
type is `Result<Option<T>, InputError>`. The `InputError` is a enum that 
contains the error that could be returned by the `input!` macro. It was thinking 
thanks to the 
[commet of Josh Tripplet](https://github.com/rust-lang/rfcs/pull/3196#issuecomment-972915603) 
in [a previous RFC](https://github.com/rust-lang/rfcs/pull/3196). And to be 
honest yes, I think that is a good idea to have a way to handle the EOF, EOF is 
not exactly a error but it's a behaviour not too friendly usually because is a 
subtle distinction, is not exactly an edge case, but it's a less common scenario 
that people often overlook at first.

# Motivation
[motivation]: #motivation

This kind of macros could be useful for beginners and reduce the barrier to 
entry for new Rustaceans. It would also make the language more friendly and help 
with the cognitive load of learning a new language.

The second chapter in the book talk about to make a guessing game, and in this 
chapter we can see how to read input from the user, but it is not too friendly 
and is not too easy to understand. It's really complex to explain to someone of 
high level what is a `Buffer` for give you a example, so in this case we can use 
the `input!` macro to make it easier.

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


- Explaining the feature largely in terms of examples.
- Explaining how Rust programmers should *think* about the feature, and how it 
  should impact the way they use Rust. It should explain the impact as 
  concretely as possible.
- If applicable, provide sample error messages, deprecation warnings, or 
  migration guidance.
- If applicable, describe the differences between teaching this to existing Rust 
  programmers and new Rust programmers.
- Discuss how this impacts the ability to read, understand, and maintain Rust 
  code. Code is read and modified far more often than written; will the proposed 
  feature make code easier to maintain?

For implementation-oriented RFCs (e.g. for compiler internals), this section 
should focus on how compiler contributors should think about the change, and 
give examples of its concrete impact. For policy RFCs, this section should 
provide an example-driven introduction to the policy, and explain its impact in 
concrete terms.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

```rs
/// A macro that:
/// - reads **one line** from stdin (as `String` by default),
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
        $crate::read_input_from(&mut ::std::io::stdin().lock(), None)
    }};
    ($($arg:tt)*) => {{
        $crate::read_input_from(
            &mut ::std::io::stdin().lock(),
            Some(format_args!($($arg)*))
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
) -> Result<T, InputError<T::Err>>
where
    R: BufRead,
    T: FromStr,
    T::Err: std::fmt::Display + std::fmt::Debug,
{
    if let Some(prompt_args) = prompt {
        print!("{}", prompt_args);
        // Always flush so the user sees the prompt immediately
        io::stdout().flush().map_err(InputError::Io)?;
    }

    let mut input = String::new();
    let bytes_read = reader.read_line(&mut input).map_err(InputError::Io)?;
    
    let trimmed = input.trim_end_matches(['\r', '\n'].as_ref());

    // If 0, that's EOF — return Eof error
    if trimmed == 0 {
        return Err(InputError::Eof);
    }
    trimmed.parse::<T>().map_err(InputError::Parse)
}

/// A unified error type indicating either an I/O error, a parse error, or EOF.
#[derive(Debug, PartialEq)]
pub enum InputError<E> {
    /// An I/O error occurred (e.g., closed stdin).
    Io(io::Error),
    /// Failed to parse the input into the desired type.
    Parse(E),
    /// EOF encountered (read_line returned 0).
    Eof,
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

This is the technical portion of the RFC. Explain the design in sufficient 
detail that:

- Its interaction with other features is clear.
- It is reasonably clear how the feature would be implemented.
- Corner cases are dissected by example.

The section should return to the examples given in the previous section, and 
explain more fully how the detailed proposal makes those examples work.

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

This is the default behavior in Python and C# for example, these cases trim
newlines by default. In Go, the `bufio.Reader.ReadString()` function
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

- Why is this design the best in the space of possible designs?
- What other designs have been considered and what is the rationale for not choosing them?
- What is the impact of not doing this?
- If this is a language proposal, could this be done in a library or macro instead? Does the proposed change make Rust code easier or harder to read, understand, and maintain?

# Prior art
[prior-art]: #prior-art

Python has [input()], Ruby has [gets], C# has `Console.ReadLine()`
... all of these return a string read from standard input.

Some behaviors are different, for example:
- Python's `input()` returns a string, but if the input is EOF
  it raises an `EOFError` exception.
- Ruby's `gets` returns `nil` if the input is EOF.
- C#'s `Console.ReadLine()` returns `null` if the input is EOF.
- Go's `bufio.Reader.ReadString()` returns an error if the input is EOF.



Discuss prior art, both the good and the bad, in relation to this proposal.
A few examples of what this can include are:

- For language, library, cargo, tools, and compiler proposals: Does this feature exist in other programming languages and what experience have their community had?
- For community proposals: Is this done by some other community and what were their experiences with it?
- For other teams: What lessons can we learn from what other communities have done here?
- Papers: Are there any published papers or great posts that discuss this? If you have some relevant papers to refer to, this can serve as a more detailed theoretical background.

This section is intended to encourage you as an author to think about the lessons from other languages, provide readers of your RFC with a fuller picture.
If there is no prior art, that is fine - your ideas are interesting to us whether they are brand new or if it is an adaptation from other languages.

Note that while precedent set by other languages is some motivation, it does not on its own motivate an RFC.
Please also take into consideration that rust sometimes intentionally diverges from common language features.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

> What parts of the design do you expect to resolve through the RFC process
> before this gets merged?

Should the function additionally be added to `std::prelude`, so that beginners
can use it without needing to import `std::io`?

- What parts of the design do you expect to resolve through the RFC process before this gets merged?
- What parts of the design do you expect to resolve through the implementation of this feature before stabilization?
- What related issues do you consider out of scope for this RFC that could be addressed in the future independently of the solution that comes out of this RFC?

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

Think about what the natural extension and evolution of your proposal would
be and how it would affect the language and project as a whole in a holistic
way. Try to use this section as a tool to more fully consider all possible
interactions with the project and language in your proposal.
Also consider how this all fits into the roadmap for the project
and of the relevant sub-team.

This is also a good place to "dump ideas", if they are out of scope for the
RFC you are writing but otherwise related.

If you have tried and cannot think of any future possibilities,
you may simply state that you cannot think of anything.

Note that having something written down in the future-possibilities section
is not a reason to accept the current or a future RFC; such notes should be
in the section on motivation or rationale in this or subsequent RFCs.
The section merely provides additional information.

Bullshit:
https://html.spec.whatwg.org/multipage/timers-and-user-prompts.html#dom-prompt-dev