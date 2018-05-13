- Feature Name: escape_block
- Start Date: 2018-05-12
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

Add an `escape {}` block that allows limiting the `?` propagation
scope, and pause error handling adjustments in favor of in-ecosystem
experimentation.

# Motivation
[motivation]: #motivation

After the introduction of `?` there has been a lot of discussion about
the future direction of Rust error handling semantics and syntax. The
new system is closer to the syntax known from exception based languages.

The error handling proposals are currently introduced in a step-by-step
fashion. Things which have been proposed, either via RFCs or other
discussions with language team members, include:

* `try`/`catch` blocks for catching and handling errors.
* `throw` or `fail` for raising errors to some catching scope.
* `pass` or `success` for success value propagation.
* `?` also raising towards some catching scope.
* Auto-converting final expression results in `Ok` or `Some`.

This RFC proposes to add a built-in `escape` blockto the langauge
that restricts `?` propagation space and implicitly carries an
`'escape` label, allowing the above to be implemented by macros.

This allows us to pause the current push for error handling adjustments
and experiment with and innovate error handling extensions as macros
before they are finalized in the language. With this, we can gain
experience with exception-like syntax and its semantics as a whole
instead of having to consider them one-by-one.

Note: This does not preclude adding things like `try` and `throw`. Most
notably, the expected keywords can still be reserved as is planned.

The feature will also provide scoped early-exiting for all other kinds
of macro based control flow, or to composably build early-exiting
functionality for uses such as generating parsing code.

The open questions in the error problem space that can be iterated by
this strategy include:

* If catching blocks should auto-convert their result or not.
* Should throwing facilities be auto-converting the thrown values.
* Should there be failure/success throwing facilities.
* Should special type hinting be available.
* Should auto conversions convert to success or failure values.

There is relevant prior discussion about error handling:

* [RFC: Reserve `try` for `try { .. }` block expressions][reserve-try]
* [Tracking issue for `try` blocks][reserve-try-tracking]
* [RFC: `throw` expressions][throw-expressions]
* [Trait-based exception handling][trait-based]
* [Issue for Auto-Wrapping][auto-wrapping]

This RFC aims to allow the crates ecosystem to provide these kinds of
control flow additions. This means that it is possible to have stable,
opt-in functionality for the community to experiment with.

It is the belief of the RFC author that experimentation and iteration
outside of core will allow error handling to develop as a whole picture,
instead of individual features. It also gives a chance for features to
develop as general functionality instead of being specific to error
handling.

The outlined strategy follows the basic feature development formula:

* Introduce facilities to allow the ecosystem to provide the control
  flow.
* Have crates that can be safely used by stable user code and have time
  to settle in their semantics.
* (Optional) Take the most common control flow uses and provide them
  as macros in `std`.
* Take the syntax and semantics that have turned out really to be
  very common and introduce them as core syntax, if there is enough need
  for them to be a part of the core language.

This will tell us:

* Do we need the functionality to be in core, is it wide-spread enough?
* Do we need it to be built-in syntax, do we have requirements that can
  only be satisfied with syntax instead of macros?
* Are any of the facilities useful for things other than error handling?
* What is the extent of real-life readability improvements?
* Is additional functionality such as auto-conversion an advantage or
  a hindrance?
* Are there solutions to the question of default-to-ok/default-to-error.

The `escape` block is a good building block for the additional syntax
because it doesn't have open questions about semantics.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

An `escape {}` block limits the propagation of values via `?` to
it instead of the surrounding function scope. It also adds an implicit
`'escape` label to allow targeting the block with
[break-with-value][break-with-value]. When there is no surrounding
`escape` block, the `'escape` label targets the surrounding function.

There is no auto-conversion.

## Examples of direct feature use

### Limiting Error Propagation

```rust
let result: Result<_, MyError> = escape {
    let item_a = calc_a()?;
    let item_b = calc_b()?;
    Ok(combine(item_a, item_b)?)
};
```

### Optional operations in sequences

```rust
let final: Option<_> = escape {
    let mut sum = 0;
    for item in items {
        sum += item.get_value()?;
    }
    Some(sum)
};
```

### Searching for an item

```rust
let item = escape {
    for item in items {
        let data = verify(item)?;
        if matches(data) {
            break 'escape Ok(data);
        }
    }
    Err(MyError::NotFound)
};
```

## Examples of implementable macros

Note: The examples use `catch` instead of `try` as identifier to
avoid confusion with the planned `try {}` blocks, the existing `try!`
macro, and because it was easier to come up with descriptive names
in the `catch_*` format given it is a big list of possible macros.

### A catching block with final-result conversion

```rust
macro_rules! catch {
    ($($body:tt)*) => {
        escape { ::std::ops::Try::from_ok({ $($body)* }) }
    }
}

let result = catch! { a? + b? };
```

### An error throwing macro with final-result conversion

```rust
macro_rules! throw {
    ($value:expr) => {
        break 'escape ::std::ops::Try::from_error($value)
    }
}

fn open(path: &Path) -> Result<File, Error> {
    match File::open(path) {
        Ok(file) => Ok(file),
        Err(io_err) => {
            log("IO error occured");
            throw!(io_error);
        },
    }
}
```

### Finalising a block with a success value

```rust
macro_rules! final {
    ($value:expr) => {
        break 'escape ::std::ops::Try::from_ok($value)
    }
}

let value: Option<_> = catch! {
    if let Some(cached) = cache.get(&id) {
        final!(cached);
    }
    let new = calc();
    cache.insert(id, new);
    new
};
```

### Other possibilities for macros

Note: The names and syntax aren't final suggestions and merely serve as
illustration. The list is also likely not exhaustive.

```rust
// catch without conversion
catch_plain! { ... }

// catch with preset Result<_, _> hint
fallible! { ... }

// catch with preset Option<_> hint
optional! { ... }

// catch with error as converted final result
attempt! { ... }

// catch with preset Result<_, Error>
catch_error!(Error { ... })

// catch with preset Result<Success, Failure>
catch_result!(Success or Failure { ... })

// catch with preset Option<Value>
catch_option!(Value { ... })

// throwing without conversion
catch_plain!(...)

// providing a final value without conversion
final_plain!(...)

// optionally throwing an Option<Error>
throw_if_some!(...)

// Finalizing an optional value
final_if_some!(...)

// Inline mapping of an error
handle!(do { ... } match err { ... })

// Inline mapping and hinting of an error
handle!(do { ... } match err: Error { ... })

// Special case for failure crate removing need to type hint errors
catch_failure! { ... }
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The required adjustments are:

* Disallow manually specifying `'escape` labels.
* Make function scopes imply `'escape`.
* Add `escape {}` blocks to explicitly specify `'escape` scopes.
* Make `?`target the closest `'escape`.

# Drawbacks
[drawbacks]: #drawbacks

* This could possibly halt or redirect exception like error handling
  syntax.
* It requires a keyword reservation.
* The resulting macros cannot be called `try! { ... }` as that would
  directly conflict with the current `try!` macro semantics.
* The exception-like syntax would require opt-in via a dependency until
  things have settled enough for it to be in core.

# Rationale and alternatives
[alternatives]: #alternatives

## Use `try {}` blocks instead

We could introduce `try` blocks and give an implicit `'try` label to
them. This has a couple of disadvantages:

* This RFC is intended to allow to find what semantics `try` blocks
  (and others) should have. This is hard to do when it is built upon
  the thing it simulates, as the underlying blocks can't have semantics
  adjusted without breaking the control flow built on top.

* Due to the above, `try` blocks would need to be unstable while the
  control flow semantics and syntax settles. This would also require
  nightly for all experimentation, while the `escape` based solution can
  independently be stabilized allowing experimentation and development
  in the stable Rust ecosystem.

## Do Nothing

If nothing is done, the possibilities seem to be:

* Go forward with the planned syntax additions.
* Decide not to stabilize any special error handling syntax.

This proposal aims to provide a compromise allowing:

* The additional syntax to be available on stable for use if desired.
* Collection of more big-picture experience before finalising semantics.
* Alternatives and edge cases (type hinting?) to be researched more
  widely.

# Prior art
[prior-art]: #prior-art

There is prior art with regard to the path of going-through-the-ecosystem
stabilisation, even in relation to error handling:

* `match result { ..., Err(err) => return Err(err) }` becoming `try!`
  becoming `?`.
* error use being iterated via `error-chain` and then `failure`.

# Unresolved questions
[unresolved]: #unresolved-questions

## As usual, the name

Should this be named `escape`?

An alternative might be, amusingly, `catch`. This would provide a nice
symmetry:

* `catch` would only be about catching things propagated by `?`
  without additional semantics.
* `try` is an error handling variant of `catch` providing auto-Ok
  wrapping.

The previous `try`/`catch` discussions can probably provide for
additional good ideas for names.


[break-with-value]: rust-lang/rfcs#2046
[reserve-try]: rust-lang/rfcs#2388
[reserve-try-tracking]: rust-lang/rust#50412
[auto-wrapping]: rust-lang/rust#41414
[throw-expressions]: rust-lang/rfcs#2426
[trait-based]: https://github.com/rust-lang/rfcs/blob/master/text/0243-trait-based-exception-handling.md
[trait-based-break]: https://github.com/rust-lang/rfcs/blob/master/text/0243-trait-based-exception-handling.md#early-exit-from-any-block

