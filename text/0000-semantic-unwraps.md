- Feature Name: `semantic-unwraps`
- Start Date: 2024-10-03
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC proposes `Result::{todo, unreachable}` and `Option::{todo, unreachable}` functions which work like `.unwrap()` but imply different semantic reasons for the unwrapping.

`.todo()` implies that error handling still needs to be done and that unwrapping here is a temporary thing that should be fixed in the future. This is analogous to `todo!()`.

`.unreachable()` implies that we are unwrapping because we believe the `Err` or `None` case *cannot* occur. This is analogous to `unreachable!()`.

In short, this allows users to preserve semantic information about why they are unwrapping. This proves extremely useful in prototype code, and in distinguishing "unreachable unwraps" and "todo unwraps" from "panic unwraps".

As an example:

```rs
// unwrap is still used for the analogous-to-panic!() case
TcpListener::bind(&addr).unwrap();

// we're panicking because error handling is not implemented yet.
// this use case is common in prototype applications.
let int: i32 = input.parse().todo();
let arg2 = std::env::args().nth(2).todo();

// these error states are unreachable.
// this use case is common in static declarations.
NonZeroU32::new(10).unreachable();
Regex::new("^[a-f]{5}$").unreachable();
```

# Motivation
[motivation]: #motivation

`.unwrap()` is semantically overloaded in rust. It finds itself used for three significantly different reasons:

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

- I completely assert that the `None/Err` case here cannot happen

```rs
// e.g.
// this cannot fail
NonZeroU32::new(10).unwrap();
Regex::new("^[a-f]{5}$").unwrap();
```

### What's wrong with this?

Users find themselves using `.unwrap()` for these different reasons, but the semantic reason *why* unwrapping was done is not stored in the source code.
Some users write comments near unwraps to justify them, but this is easy to forget or fall out of sync with the codebase.

In my experience, the `unwrap as todo!()` paradigm is much more common in application rust.

This problem becomes more pronounced when one wants to go back over the code and fix all of those "todo unwraps".
It's not easy to figure out whether the `.unwrap()` has a good reason for being there, or is simply a result of a hastily-written MVP.

While in terms of actual program execution, nothing is different (the program will panic), the source code itself doesn't necessarily track the information *why* that panic was placed there.

### Prior art

We already have prior art for "different kinds of panics" in the form of `todo!()` and `unreachable!()`. These macros are used frequently in rust and I'm not aware of anyone considering them a bad API.

This gives the method names `.todo()` and `.unreachable()` a good justification, and they already map to a commonly-used feature.

### What do we get then?

```rs
// unwrap is still used for the analogous-to-panic!() case
TcpListener::bind(&addr).unwrap();

// we're panicking because error handling is not implemented yet.
let int: i32 = input.parse().todo();
let arg2 = std::env::args().nth(2).todo();

// these error states are unreachable.
NonZeroU32::new(10).unreachable();
Regex::new("^[a-f]{5}$").unreachable();
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

## `Result::unreachable()`

`Result::unreachable()` Returns the value in `Ok`, consuming the result.

This function will panic if the value is an `Err`, **with a panic message indicating that the error case should not be reachable.** This is analogous to `unreachable!()`.

This function may be preferred to `Result::unwrap()` for cases where the error case cannot happen. `Result::unreachable()` makes it clearer that this case is not expected to happen.

### Example

```rust
// The error state here cannot be reached.
let my_address: std::net::IpAddr = "127.0.0.1".parse().unreachable();
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

## `Option::unreachable()`

`Option::unreachable()` Returns the value in `Some`, consuming the option.

This function will panic if the value is `None`, **with a panic message indicating that the None case should not be reachable.** This is analogous to `unreachable!()`.

This function may be preferred to `Option::unwrap()` for cases where None cannot happen. `Option::unreachable()` makes it clearer that this case is not expected to happen.

### Example

```rust
// The error state here cannot be reached.
let amount_of_crabs = NonZeroU32(12).unreachable();
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
        todo!("Error handling not implemented: {err:?}")
      }
    }
  }

  pub const fn unreachable(self) -> T {
    match self {
      Ok(t) => t,
      Err(err) => {
        unreachable!("Error case should not be reachable: {err:?}")
      }
    }
  }
}

impl Option<T> {
  pub const fn todo(self) -> T {
    match self {
      Some(t) => t,
      None => {
        todo!("None handling not implemented")
      }
    }
  }

  pub const fn unreachable(self) -> T {
    match self {
      Some(t) => t,
      None => {
        unreachable!("None case should not be reachable")
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

We could call these methods `.unwrap_todo()`, `.unwrap_unreachable()` instead, which might make it more obvious that these things panic. However, I'm conscious that these names are rather long, and having to write out `.unwrap_todo()` in prototype code is unlikely to catch on as a result.

I don't think there's any good reason to choose names other than `todo` and `unreachable`, as they already exist prominently in rust.

### What about `.expect()`?

We do already have `{Option, Result}::expect` which serves a similar-ish purpose of "unwrap with a reason".

I argue that this doesn't necessarily map onto the semantics of `todo` or `unreachable`, Especially `todo`.

While this feature can be emulated with `.expect("todo")` and `.expect("unreachable")`, this is frustrating to type, easy to typo, harder to grep for and cannot be highlighted nicely by an IDE.

### Should this take an argument?

`.expect()` takes an argument which allows more info to be strapped on about the panic.

I don't think `.todo` taking an argument would be good as it makes the code harder to write, plus, I don't see what you'd ever write there.

It would maybe be useful to strap a reason on to `.unreachable` (why do you think its unreachable?), but this seems infrequently useful and makes the normal case ("this is self-evidently unreachable") more annoying to work with.

# Prior art
[prior-art]: #prior-art

The names `todo` and `unreachable` have prior art in the `todo!()` and `unreachable!()` macros, respectively.

The concept of "semantic panics" has been in rust since 1.0.0, and this feature is widely used across the ecosystem.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

In general, this is a relatively small addition in terms of "new" things, as it's comprised entirely of existing concepts in rust.

### Error Message Names

At the moment I've just ripped the names from `todo!()` and `unreachable!()`.

I'm a little skeptical of the use of the term `None Handling` in `None Handling not implemented` in `Option::todo()`. I don't think that term is used anywhere else in rust and it doesn't look too great.

There's probably a better phrasing for that.

### Constness

It would be ideal if these functions are `const`. However, I understand that `const-unwrap` is not stable yet.

# Future possibilities
[future-possibilities]: #future-possibilities

## `::unimplemented()`

People might want `.unimplemented`. I think introducing this might be confusing and unecessary.

I don't see `unimplemented!()` used anywhere near as much as `todo!()` or `unreachable!()`, and I don't see `unimplemented!()` ("i am deliberately not handling this") mapping over to `Option` or `Result`.

It would make sense to add this to complete a mapping between those macros and Result/Option, but it doesn't seem like a very important addition.

## `::unreachable_unchecked()`

This has a macro counterpart in `unreachable_unchecked!()`, but I don't really see the point of adding it here. This is already achieved understandably with `unwrap_unchecked()`.

There's no harm in adding it, but it doesn't seem as important.
