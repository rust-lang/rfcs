# Semicolon Items

- Feature Name: `semicolon_items`
- Start Date: 2018-06-17
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

The semicolon (`;`) is now accepted as an item, permitting a user to write
`struct Foo {};`, among other things. The `item` fragment specifier in a
`macro_rules` matcher will match `;`.

# Motivation
[motivation]: #motivation

## Leftover semicolons when refactoring

Various common refactorings often leave behind extraneous semicolons, such as:

* Replacing a closure with a `fn` item:
  
  ```rust
  let add_two = |x| x + 2;
  
  // becomes:
  fn add_two(x: u32) -> u32 {
      x + 2
  };
  ```
  
* Adding a field to what was formerly a unit struct:

  ```rust
  struct UnitExpr;
  
  // becomes:
  struct UnitExpr {
      span: Span,
  };
  ```

* Changing a tuple struct into a braced struct:

  ```rust
  struct Foo(String, usize);

  // becomes:
  struct Foo {
      bar: String,
      baz: usize,
  };
  ```
 
The error emitted by the compiler in these circumstances is never
indicative of a bug, but nonetheless disturbs the flow of writing
and causes the user to have to wait and recompile.
During that time, the user's train of thought can easily be lost.

## Improving consistency with items in `fn` bodies

[compiletest_rs]: https://github.com/laumann/compiletest-rs/blob/master/src/runtest.rs#L2585-L2660

Incidentally, these extraneous semicolons are currently permitted on items
defined inside of `fn` bodies, as a consequence of the fact that `;` in
isolation is a valid *statement* (though not an item).
The following, slightly modified example is from [compiletest_rs].

```rust
fn read2_abbreviated(mut child: Child) -> io::Result<Output> {
    use std::mem::replace;
    use read2::read2;

    const HEAD_LEN: usize = 160 * 1024;
    const TAIL_LEN: usize = 256 * 1024;

    enum ProcOutput {
        Full(Vec<u8>),
        Abbreviated {
            head: Vec<u8>,
            skipped: usize,
            tail: Box<[u8]>,
        }
    };

    impl ProcOutput {
        ...
    };

    ...
}
```

By permitting semicolons as items outside of `fn` bodies,
a modicum of consistency in the Rust's grammar can be gained.

## A frequent editing mistake

For the authors of this RFC, who have written Rust code for many years,
it still sometimes happens that they mistakenly place a semicolon after
`struct Name {}`. As previously mentioned, such trivialities needlessly
disrupt the editing process.

## Retaining a uniform style

To retain as uniform of a style possible in the Rust community,
this RFC proposes that `rustfmt`, Rust's code formatting tool,
should remove extraneous `;` since they are most likely left over
while editing or as a frequently made mistake.
No possibility of configuring this behavior of `rustfmt` is proposed at this time.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Simply put, the token `;` is now accepted as an item.
This means that you are allowed to write all of the following:

```rust
struct Foo {
    bar: Baz
}; // <-- NOTE
```

You can also put a `;` after `enum`, `trait`, `impl`, `fn`, `union`,
and `extern` items.

A `macro_rules` matcher using the `item` fragment will now also accept
`;` as an item. For example, given:

```rust
macro_rules foo! {
    ($x: item) => { .. }
}
```

you may write:

```rust
foo!(;)
```

It's important to note that while `;` is now accepted where `item`s are,
this is not intended as the recommended style, but only to improve the
consistency in Rust's grammar, as well as writing flow.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

1. The token `;` is accepted as a valid item, both in the language syntax
and by the `item` macro 'fragment specifier'. As an example, `struct Foo {};`
is therefore in the language accepted by a Rust compiler.

2. `rustfmt` will remove any extraneous `;` items.

# Drawbacks
[drawbacks]: #drawbacks

The language accepted by a Rust parser is made somewhat more complicated;
this can have some minor effect on parsing performance and will also
complicate the language users will need to understand.
However, we believe that nothing new is actually necessary to be learned
as a redundant `;` in an example such as `struct Foo {};` should already
be intuitive for someone who knows `struct Foo {}`.

# Rationale and alternatives
[alternatives]: #rationale-and-alternatives

## Do nothing

As always, if we conclude that the motivation is not enough,
we can elect to do nothing.

## Improve error messages further

An idea due to [@joshtripplet](https://github.com/joshtriplett) is to improve
the error messages given by `rustc` instead of accepting redundant `;`s.

The current error message when writing `struct Foo {};` is:

```rust
error: expected item, found `;`
 --> src/main.rs:1:14
  |
1 | struct Foo {};
  |              ^ help: consider removing this semicolon
```

This error message is already quite good, giving the user actionable
information. However, the specific case could be improved by recognizing it
specially and saying something like *"don't put a `;` after a `struct { ... }`"*
(same for `union`s and `enum`s). This idea is being discussed in [issue #51603](https://github.com/rust-lang/rust/issues/51603).

However, this does not solve the problem when refactoring,
and neither does it enhance writing flow nor make the grammar more consistent.

# Prior art
[prior-art]: #prior-art

Rust is unusual in that it is a semicolon-terminated language,
yet it has very few kinds of statements that are not expressions in disguise.
This makes direct comparisons to other languages difficult.  

There is however some prior art:

* C++11 [added support](http://en.cppreference.com/w/cpp/language/declarations)
for `;` as an "empty declaration," where prior specifications of the language did not.

* GHC, the Glasgow Haskell Compiler, accepts `;` at the top level.
For example, the following is accepted by GHC:

  ```haskell
  module Foo where
  ;
  ```

# Unresolved questions
[unresolved]: #unresolved-questions

* Does this create parsing ambiguities anywhere?
