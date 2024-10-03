- Feature Name: `option-result-todo`
- Start Date: 2024-10-03
- RFC PR: [rust-lang/rfcs#3706](https://github.com/rust-lang/rfcs/pull/3706)
- Rust Issue: [rust-lang/rust#3706](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC proposes `Result::todo` and `Option::todo` functions which work like `.unwrap()` but imply different semantic reasons for the unwrapping.

`.todo()` implies that error handling still needs to be done and that unwrapping here is a temporary thing that should be fixed in the future. This is analogous to `todo!()`.

In short, this allows users to preserve semantic information about why they are unwrapping. This proves extremely useful in prototype code, and in distinguishing "todo, add error handling here" from "no, i actually want to panic in this case.".

As an example:

```rs
// unwrap is still used for the analogous-to-panic!() case
TcpListener::bind(&addr).unwrap();

// we're panicking because error handling is not implemented yet.
// this use case is common in prototype applications.
let int: i32 = input.parse().todo();
let arg2 = std::env::args().nth(2).todo();
```

# Motivation
[motivation]: #motivation

`.unwrap()` is semantically overloaded in rust. It finds itself used for two significantly different reasons:

- If this is `None/Err`, our program is in a bad state and we want to exit (missing config files, missing resource files, some external invariant not upheld)

```rs
// e.g.
// our web server should just die if it can't open a tcp socket
TcpListener::bind(&addr).unwrap();
```

- I haven't added `None/Err` handling here yet (prototype code)

```rs
// e.g.
// parse user input as an int,
// can't be bothered handling bad input right now
let int: i32 = input.parse().unwrap();
// my CLI needs a second argument to do anything useful,
// i should handle this properly later
let arg2 = std::env::args().nth(2).unwrap();

```

### What's wrong with this?

Users find themselves using `.unwrap()` for these different reasons, but the semantic reason *why* unwrapping was done is not stored in the source code.
Some users write comments near unwraps to justify them, but this is easy to forget or fall out of sync with the codebase.

In my experience, the `unwrap as todo!()` paradigm is much more common in application rust.

This problem becomes more pronounced when one wants to go back over the code and fix all of those "todo unwraps".
It's not easy to figure out whether the `.unwrap()` has a good reason for being there, or is simply a result of a hastily-written MVP.

While in terms of actual program execution, nothing is different (the program will panic), the source code itself doesn't necessarily track the information *why* that panic was placed there.

### Prior art

We already have prior art for "different kinds of panics" in the form of `todo!()`. This macro is used frequently in Rust and I'm not aware of anyone considering them a bad API.

This gives the method name `.todo()` a good justification, as it already maps to a commonly-used feature.

### What do we get then?

```rs
// unwrap is still used for the analogous-to-panic!() case
TcpListener::bind(&addr).unwrap();

// we're panicking because error handling is not implemented yet.
let int: i32 = input.parse().todo();
let arg2 = std::env::args().nth(2).todo();
```

And now the semantic reason for "why" we're panicking is preserved!

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## `Result::todo()`

`Result::todo()` returns the value in `Ok`, consuming the result.

This function will panic if the value is an `Err`, **with a panic message indicating that error handling is not implemented yet here.** This is analogous to `todo!()`.

This function may be preferred to `Result::unwrap()` for prototype code where error handling should be implemented later.

### Example

```rust
// Error handling is not implemented here yet. This is quick and dirty prototype code.
let my_file: Vec<u8> = std::fs::read("file.txt").todo();
```

## `Option::todo()`

`Option::todo()` returns the value in `Some`, consuming the option.

This function will panic if the value is `None`, **with a panic message indicating that None handling is not implemented yet here.** This is analogous to `todo!()`.

This function may be preferred to `Option::unwrap()` for prototype code where handling should be implemented later.

### Example

```rust
// None handling is not implemented here yet. This is quick and dirty prototype code.
let arg2 = std::env::args().nth(2).todo();
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Since this is just a regular function composed of very regular parts of rust, I don't think there are many technical questions to be answered.

This would be implemented by adding two features on `Result` and `Option`.

I imagine an implementation would look vaguely like this:

```rs
impl Result<T, E> where E: fmt::Debug {
  pub const fn todo(self) -> T {
    match self {
      Ok(t) => t,
      Err(err) => {
        todo!("Error not handled: {err:?}")
      }
    }
  }
}

impl Option<T> {
  pub const fn todo(self) -> T {
    match self {
      Some(t) => t,
      None => {
        todo!("None not handled")
      }
    }
  }
}
```

# Drawbacks
[drawbacks]: #drawbacks

### Obvious ones

It's more functionality in the standard library which isn't ideal.

It might cause some code churn/documentation churn if merged, as documentation now has other ways to indicate "exercise-for-the-reader" error handling.

### Does `.todo()` encourage bad habits?

I think a decent argument against this RFC is that `.todo()` encourages lazy error handling in rust.

I'd argue that we already have this in rust with `.unwrap()`. And in fact, the current state of things is *more* dangerous.
In my experience, people will always elide error handling in prototype code, and the current state of affairs conflates that temporary panicking with permanent/intentional panicking.

With a theoretical `.todo()`, you can defer error handling later in your software lifecycle, but it's always possible to see *where* your lazy error handling is.

What I'm saying is -- this doesn't encourage lazy error handling, it just makes it less easy to forget.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

### This could be done with ResultExt

Then everyone could import a crate that does this for them. However, I think there's value
in standardising this in rust, as its an extremely common use case. Plus, if standardised, IDEs and Rust-Analyzer could treat it better.

For example, `#[clippy::todo]` could highlight `.todo()`s and so on.

### Other Method Names

We could call this method `.unwrap_todo()` instead, which might make it more obvious that this will panic. However, I'm conscious that these names are rather long, and having to write out `.unwrap_todo()` in prototype code is unlikely to catch on as a result.

I don't think there's any good reason to choose names other than `todo`. They already exist prominently in Rust.

### What about `.expect()`?

We do already have `{Option, Result}::expect` which serves a similar-ish purpose of "unwrap with a reason".

I argue that this doesn't necessarily map as nicely onto the semantics of `todo`.

While this feature can be emulated with `.expect("todo")`, this is frustrating to type, easy to typo, harder to grep for and cannot be highlighted nicely by an IDE.

### Should this take an argument?

`.expect()` takes an argument which allows more info to be strapped on about the panic.

I don't think `.todo` taking an argument would be good as it makes the code harder to write, plus, I don't see what you'd ever write there.

# Prior art
[prior-art]: #prior-art

The name `todo` has prior art in the `todo!()` macro, in which it means the exact same thing.

The concept of "semantic panic" has been in rust since 1.0.0, and this feature is widely used across the ecosystem.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

In general, this is a relatively small addition in terms of "new" things, as it's comprised entirely of existing concepts in rust.

### Panic Message Names

At the moment I've just ripped the panic messages from `todo!()`.

There may be better phrasing.

### Constness

It would be ideal if these functions are `const`. However, I understand that `const-unwrap` is not stable yet.

# Future possibilities
[future-possibilities]: #future-possibilities

## `::unreachable()`

The initial RFC contained `.unreachable()` alongside `.todo()`. However, I think `.unreachable()` is not of much value, and is *significantly more contentious* than `.todo()`.

`.unreachable()` can be done with `.expect("REASON WHY THIS CANNOT HAPPEN")` with little downside. In fact, I struggle to come up with a convincing argument why adding a method for this would help things.

`.expect()` is a lot nicer for these things; you can provide a reason why you believe the error not to occur. Plus, `.unreachable()` is a confusing name.

The same is true for `unimplemented` and `unreachable_unchecked` equivalents. Both already map decently onto `expect` and `unwrap_unchecked`, respectively.
