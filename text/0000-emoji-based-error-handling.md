- Feature Name: emoji_based_error_handling
- Start Date: 2016-04-01
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)
- RFC Authors: [@dbrgn][DB], [@dns2utf8][SS], [@rnestler][RN]


# Summary
[summary]: #summary

This RFC proposes to implement emoji based error handling in Rust.


# Motivation
[motivation]: #motivation

Emoji have become an universal way of expressing emotions. Error handling in is
generally a very emotional topic in software development. Therefore error
handling and the use of emoji are a perfect match.

Furthermore, after the introduction of the question mark postfix operator in
[RFC 243][1] (Trait-based exception handling), a lot of people raised concerns
that error handling is not visible enough in the code anymore. Using colorful
emoji to denote error handling would certainly make it obvious that errors are
being handled.


# Detailed design
[design]: #detailed-design

**Description**

The cornerstones of Rust error handling are two enums: `Result` and `Option`. A
`Result<T, E>` may either be `Success<T>` or `Err(E)` while an `Option(T)` may
be either `Some(T)` or `None`.

```rust
enum Result<T, E> {
    Ok(T),
    Err(E),
}
enum Option<T> {
    Some(T),
    None,
}
```

We suggest that these error types are complemented by emoji based alternatives.

A `Result` is an expression of hope for a certain outcome (:pray:, U+1F64F). If
the operation succeeds, developers party (:beers:, U+1F37B), but sometimes shit
happens (:poop:, U+1F4A9).

```rust
enum 🙏<T, E> {
    🍻<T>,
    💩<E>,
}
```

On the other hand, an `Option` is an expression of uncertainty (:interrobang:,
U+203D).  Something might be there (:raising_hand:, U+1F64B) or it might not be
(:x:, U+274C).

```rust
enum ‽<T> {
    🙋<T>,
    ❌,
}
```

**Matching**

These emoji can be matched as usual:

```rust
fn i_am_a_teapot(which_kind: ‽<TeaType>) -> 🙏<Tea, String> {
    match which_kind {
        🙋(kind) => 🍻(kind.brew()),
        ❌ => 💩("Not specified which kind of tea to brew."),
    }
}
```

**Unwrapping and Macros**

Another often used error handling mechanism is the `try!` macro. We suggest
using the :muscle: emoji to bring across the notion of strength or trust. Yes
we try!

When the developer is too lazy to do proper error handling, he/she may also use
the `unwrap()` method. We suggest replacing it with the see-no-evil emoji
(:see_no_evil:).

Last but not least, when all else fails, code may panic (:scream:).

```rust
if let 🙋(coffee) = brew() {
    println!("Drinking {} coffee", coffee.rate_taste().🙈());
    💪!(coffee.drink());
} else {
    😱!("406 - WE'RE OUT OF COFFEE!!1");
}
```

**Feature Gate**

As with other newly proposed features, we propose adding a feature gate for emojified error handling:

```rust
#[feature(emojification)]
```

**Implementation**

The Rust grammar already fully supports UTF8. There are two ways to implement this:

- Modify the `IDENTIFIER` grammar to include unicode characters. Currently
  identifiers may only contain ASCII letters and underscores. 
- Implement a syntax extension that operates on the token tree and that
  replaces all occurrences of certain emoji with their regular ASCII
  counterparts.


# Drawbacks
[drawbacks]: #drawbacks

None, although the proposal might seem less convincing on other days of the year.


# Alternatives
[alternatives]: #alternatives

Alternative Result emoji choices have been suggested:

- Proposal (:ring:) with success (:heart:) or no success (:broken_heart:)
- Fear (:fearful:) with yay (:sparkles:) or nay (:skull:)



# Unresolved questions
[unresolved]: #unresolved-questions

- Not all commonly used fonts and text editors offer full support of all
  required Unicode characters. But that problem could be resolved trivially
  by developing our own fully featured `rust.ttf` font and `rustty` terminal
  emulator.
- So far, a syntax extension to implement support for the proposed features has
  not yet been written and published.



[1]: https://github.com/rust-lang/rfcs/pull/243
[DB]: https://github.com/dbrgn
[SS]: https://github.com/dns2utf8
[RN]: https://github.com/rnestler
