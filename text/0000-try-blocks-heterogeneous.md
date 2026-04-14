- Feature Name: `try_blocks_heterogeneous`
- Start Date: 2026-04-12
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

## Summary
[summary]: #summary

[RFC 3721](https://github.com/rust-lang/rfcs/pull/3721) implemented default support for homogeneous `try {...}` blocks, where all `?`s return the same error type. This RFC aims to provide support for explicit annotation of the returned error type from a `try {...}` block.

## Motivation
[motivation]: #motivation

> I'm a bit concerned about this change. Applications and libraries often use crates like `thiserror` to automatically group errors. For example, I often write something like
>
> ```rust
> #[derive(Error)]
> enum MyError {
>     #[error("Failed to parse config: {0}")]
>     InvalidConfig(#[from] serde::Error),
>     #[error("Failed to connect to server: {0}")]
>     ServerConnectionFailed(#[from] io::Error),
>     ...
> }
> ```
>
> which I then use as
>
> ```rust
> fn example() -> Result<(), MyError> {
>     let config = parse_config()?; // ? promotes serde::Error to MyError
>     let server = connect_to_server(server.url)?; // ? promotes io::Error to MyError
>     // ...
> }
> ```
>
> With this change, this approach would stop working in `try` blocks.
>
> ~ [purplesyringa commenting on #3721](https://github.com/rust-lang/rfcs/pull/3721#issuecomment-2466852085)

Currently there is no way to get the following example to compile, as the compiler is unable to safely determine the correct types returned from the try blocks, and no notation is available for the user to specify the type:

```rust
#![feature(try_blocks)]

use std::num::ParseIntError;

#[derive(Debug)]
struct Error1;

#[derive(Debug)]
struct Error2;

impl From<ParseIntError> for Error1 {
    fn from(_: ParseIntError) -> Self {
        Self
    }
}

impl From<ParseIntError> for Error2 {
    fn from(_: ParseIntError) -> Self {
        Self
    }
}

impl From<Error1> for Error2 {
    fn from(_: Error2) -> Self {
        Self
    }
}

impl From<Error2> for Error1 {
    fn from(_: Error2) -> Self {
        Self
    }
}

fn err1(s: &str) -> Result<i32, Error1> {
    Ok(s.parse()?)
}

fn err2(s: &str) -> Result<i32, Error2> {
    Ok(s.parse()?)
}

fn heterogeneous_into_exists() {
    let x = try { err1("1")? + err2("2")? };
    let y = try { err2("1")? + err1("2")? };
    assert_eq!(x.unwrap(), y.unwrap());
}
```

The initial experimental approach to provide a prrof-of-concept introduced the (**deliberate placeholder**) syntax `try bikeshed ... {...}` in [PR #149489](https://github.com/rust-lang/rust/pull/149489).

>_For the remainder of this RFC we will continue with `bikeshed` to allow for examples which work on current nightly with `#![feature(try_blocks_heterogeneous)]`._
>
>_See open questions and [try bikeshed: What should the syntax be?](https://github.com/rust-lang/rust/issues/154128) for consideration of possible target syntax._

This would allow the above example to become:

```rust
fn heterogeneous_into_exists() {
    let x = try bikeshed Result<_, Error1> { err1("1")? + err2("2")? };
    let y = try bikeshed Result<_, Error1> { err2("1")? + err1("2")? };
    assert_eq!(x.unwrap(), y.unwrap());
}
```

and for cases where no direct `Into` relationship exists, or is needed, via a common third error type:

```rust
use std::{error::Error, fmt::Display};
impl Error for Error1 {}
impl Display for Error1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error1")?;
        Ok(())
    }
}

impl Error for Error2 {}
impl Display for Error2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error2")?;
        Ok(())
    }
}

fn heterogeneous_into_anyhow() {
    let x = try bikeshed anyhow::Result<_> { err1("1")? + err2("2")? };
    let y = try bikeshed anyhow::Result<_> { err2("1")? + err1("2")? };
    assert_eq!(x.unwrap(), y.unwrap());
}
```

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

_Assuming the explanation for try blocks is implemented as per RFC 3712, which contains:_

> This behaviour is what you want in the vast majority of simple cases.  In particular,
> it always works for things with just one `?`, so simple things like `try { a? + 1 }`
> will do the right thing with minimal syntactic overhead.  It's also common to want
> to group a bunch of things with the same error type.  Perhaps it's a bunch of calls
> to one library, which all use that library's error type.  Or you want to do
> [a bunch of `io` operations](<https://github.com/rust-lang/rust/blob/d6f3a4ecb48ead838638e902f2fa4e5f3059779b/>> > compiler/rustc_borrowck/src/nll.rs#L355-L367) which all use `io::Result`.  Additionally, `try` blocks work with
> `?`-on-`Option` as well, where error-conversion is never needed, since there is only `None`.
>
> It will fail to compile, however, if not everything shares the same error type.
> Suppose we add some formatting operation to the previous example:
>
> ```rust,edition2021,compile_fail
> let pair_result = try {
>     let a = std::fs::read_to_string("hello")?;
>     let b = std::fs::read_to_string("world")?;
>     let c: i32 = b.parse()?;
>     (a, c)
> };
> ```
>
> The compiler won't let us do that:
>
> ```text
> error[E0308]: mismatched types
>   --> src/lib.rs:14:32
>    |
>    |     let c: i32 = b.parse()?;
>    |                           ^ expected struct `std::io::Error`, found struct `ParseIntError`
>    = note: expected enum `Result<_, std::io::Error>`
>               found enum `Result<_, ParseIntError>`
> note: return type inferred to be `Result<_, std::io::Error>` here
>   --> src/lib.rs:14:32
>    |
>    |     let a = std::fs::read_to_string("hello")?;
>    |                                             ^
> ```
>
> ~~For now, the best solution for that mixed-error case is the same as before: to refactor it to a function.~~

_replace the final sentence with ..._

> While it may be obvious, or even irrelevant, to you which error type `pair_result` could potentially have, the compiler has no way to know this.
>
> Just like in other situations where the compiler cannot safely infer the exact type to use, you must annotate the block with a valid error type. We've already mentioned that `Result` automatically converts between error types where a suitable implementation of `Into` exists and you can leverage this and write:
>
> ```rust
> let pair_result = try bikeshed Result<_, PairError> {
>   let a = std::fs::read_to_string("hello")?;
>   let b = std::fs::read_to_string("world")?;
>   let c: i32 = b.parse()?;
>   (a, c)
> };
> ```
>
> As long as you have defined a suitable error:
>
> ```rust
> enum PairError {
>     IoError(Box<io::Error>),
>     ParseError(Box<num::ParseIntError>),
> }
> 
> impl From<io::Error> for PairError {
>     fn from(e: io::Error) -> Self {
>         Self::IoError(Box::new(e))
>     }
> }
> 
> impl From<num::ParseIntError> for PairError {
>     fn from(e: num::ParseIntError) -> Self {
>         Self::ParseError(Box::new(e))
>     }
> }
> ```
>
> Of course, there are crates available to simplify this if you do not want or need to create your own custom error type.

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This is the technical portion of the RFC. Explain the design in sufficient detail that:

- Its interaction with other features is clear.
- It is reasonably clear how the feature would be implemented.
- Corner cases are dissected by example.

The section should return to the examples given in the previous section, and explain more fully how the detailed proposal makes those examples work.

## Drawbacks
[drawbacks]: #drawbacks

Why should we _not_ do this?

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- Why is this design the best in the space of possible designs?
- What other designs have been considered and what is the rationale for not choosing them?
- What is the impact of not doing this?
- If this is a language proposal, could this be done in a library or macro instead? Does the proposed change make Rust code easier or harder to read, understand, and maintain?

## Prior art
[prior-art]: #prior-art

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

## Unresolved questions
[unresolved-questions]: #unresolved-questions

- [ ] What should the syntax be? See [Issue #154128](https://github.com/rust-lang/rust/issues/154128) for discussion of alternatives (`:`, `->`, `as`, ...)
- [ ] What type should be annotated? This should probably be the full type, with optional inferance, as currently implemented for `bikeshed`, but see [Issue #154127](https://github.com/rust-lang/rust/issues/154127) for discussion.

## Future possibilities
[future-possibilities]: #future-possibilities

### Allow inference via function return type

For cases such as

```rust
fn heterogeneous_via_return_type() -> Result<(), Error1> {
    let x = try { err1("1")? + err2("2")? }?;
    let y = try { err2("1")? + err1("2")? };
    assert_eq!(x, y?);
    Ok(())
}
```

where the errors involved all implement `Into<Error1>`

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
