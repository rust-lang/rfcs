- Feature Name: dbg_macro
- Start Date: 2018-03-13
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

Add a `dbg!($expr)` macro to the prelude (so that it doesn’t need to be imported)
that prints its argument with some metadata (source code location and stringification)
before returning it.

This is a simpler and more opinionated counter-proposal
to [RFC 2173](https://github.com/rust-lang/rfcs/pull/2173).


# Motivation
[motivation]: #motivation

Sometimes a debugger may not have enough Rust-specific support to introspect some data
(such as calling a Rust method), or it may not be convenient to use or available at all.
“`printf` debugging” is possible in today’s Rust with:

```rust
println!("{:?}", expr);
```

This RFC improves some aspects:

* The `"{:?}",` part of this line is boilerplate that’s not trivial to remember
  or even type correctly.
* If the expression to be inspected is part of a larger expression,
  it either needs to be duplicated (which may add side-effects or computation cost)
  or pulled into a `let` binding which adds to the boilerplate.
* When more than one expression is printed at different places of the same program,
  and the formatting itself (for example a plain integer)
  doesn’t indicate what value is being printed,
  some distinguishing information may need to be added.
  For example: `println!("foo = {:?}", x.foo());`

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

To inspect the value of a given expression at run-time,
it can be wrapped in the `dbg!` macro to print the value to `stderr`,
along with its source location and source code:

```rust
fn foo(n: usize) {
    if let Some(_) = dbg!(n.checked_sub(4)) {
        /*…*/
    }
}

foo(3)
```
```
[example.rs:2] n.checked_sub(4) = None
```

This requires the type of the expression to implement the `std::fmt::Debug` trait.
The value is returned by the macro unchanged.


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

In `src/libstd/lib.rs`, `dbg` is added to the `#[macro_reexport]` attribute on `extern crate core`.

The macro below is added to `src/libcore/macros.rs`,
with a doc-comment based on the [Guide-level explanation][guide-level-explanation] of this RFC.

```rust
#[macro_export]
macro_rules! dbg {
    ($expr:expr) => {
        match $expr {
            expr => {
                eprintln!("[{}:{}] {} = {:#?}", file!(), line!(), stringify!($expr), &expr);
                expr
            }
        }
    }
}

```

The use of `match` over let is similar to the implementation of `assert_eq!`.
It [affects the lifetimes of temporaries](
https://stackoverflow.com/questions/48732263/why-is-rusts-assert-eq-implemented-using-a-match#comment84465322_48732525).

# Drawbacks
[drawbacks]: #drawbacks

Adding to the prelude should be done carefully.
However a library can always define another macro with the same name and shadow this one.

# Alternatives
[alternatives]: #alternatives

- See [RFC 2173](https://github.com/rust-lang/rfcs/pull/2173) and discussion there.

- This RFC does not offer users control over the exact output being printed.
  This is because a use of this macro is intended to be run a small number of times
  before being removed.
  If more control is desired (for example logging in an app shipped to end users),
  other options like `println!` or the `log` crate remain available.

- If the macro accepts more than one expression (returning a tuple),
  there is a question of what to do with a single expression.
  Returning a one-value tuple `($expr,)` is probably unexpected,
  but *not* doing so creates a discontinuty in the macro’s behavior as things are added.
  With only one expression accepted, users can still pass a tuple expression
  or call the macro multiple times.

- Printing could be disabled when `cfg!(debug_assertions)` is false to reduce runtime cost
  in release build.
  However this cost is not relevant if uses of `dbg!` are removed before shipping
  to any form of production (where the `log` crate might be better suited)
  and deemed less important than the ability to easily investigate bugs
  that only occur with optimizations.
  (Which [do happen](https://github.com/servo/servo/issues/19519)
  and can be a pain to debug.)

- Any detail of the formatting can be tweaked. For example, `{:#?}` or `{:?}`?

# Prior art
[prior-art]: #prior-art

Many languages have a construct that can be as terse as `print foo`.

# Unresolved questions
[unresolved]: #unresolved-questions

Unbounded bikeshedding.
