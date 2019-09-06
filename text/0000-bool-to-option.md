- Feature Name: `bool_to_option`
- Start Date: 2019-09-05
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Add methods to `bool` for converting to an `Option<T>`, given a `t: T`, where `true` maps
to `Some(t)` and `false` maps to `None`, abstracting the following common pattern in Rust.

```rust
if some_condition {
    Some(t)
} else {
    None
}
```

# Motivation
[motivation]: #motivation

This is an
[extremely common pattern](https://sourcegraph.com/search?q=repogroup%3Acrates+%2Felse%5Cs*%7B%5Cs*None%5Cs*%7D%2F+count%3A1000)
in Rust code, but is quite verbose, taking several lines to achieve something with only two
important data: the `bool` and the `T`. If we instead collapse the expression on to a single line by
simply removing newlines (i.e. `if some_condition { Some(t) } else { None }`), the legibility of the
code is reduced. In addition, chaining a conversion from a `bool` to an `Option<T>` is inconvenient
in this form and usually requires binding an extra variable to remain readable. Abstracting this
common pattern into a method will make code more readable and require less repetitive typing on the
user's part.

A method for converting from `bool` to `Option<T>` has been requested several times in the past
[[1]](https://github.com/rust-lang/rfcs/pull/2180)
[[2]](https://github.com/rust-lang/rust/issues/50523)
[[3]](https://github.com/rust-lang/rfcs/issues/2606) and shows a significant desire from users.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Following this proposal, we will add two methods to `bool`:

```rust
impl bool {
    fn then<T>(self, t: T) -> Option<T> {
        if self {
            Some(t)
        } else {
            None
        }
    }

    fn then_with<T, F: FnOnce() -> T>(self, f: F) -> Option<T> {
        if self {
            Some(f())
        } else {
            None
        }
    }
}
```

The primitive type `bool` currently has no methods, so it will be necessary to add support similarly
to other primitive types that do have methods, like
[`char`](https://doc.rust-lang.org/src/core/char/methods.rs.html#11-1393). This will require the
addition of a new lang item: `#[lang = "bool"]`.

# Drawbacks
[drawbacks]: #drawbacks

Save the usual drawbacks of adding a new method to the standard library, there are no
drawbacks.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The implementations here are the only reasonable ones.

The following names have been suggested in the past. The choice in this proposal has been made to
avoid possible confusion (e.g. avoiding a name that suggests a conversion from `bool` to
`Option<()>` rather than `Option<T>`), and to be consist with existing naming conventions (e.g.
`then_with` rather than `then_do` to be consist with methods such as `get_or_insert_with`),
but ultimately comes down to personal preference.

- [`then` and `then_do`](https://github.com/rust-lang/rfcs/pull/2180#issuecomment-350498489)
- [`some`](https://github.com/rust-lang/rfcs/issues/2606#issue-387773675)
- [`as_some` and `as_some_from`](https://docs.rs/boolinator/2.4.0/boolinator/trait.Boolinator.html)
- [`to_opt` or `to_option`](https://github.com/rust-lang/rfcs/issues/2606#issuecomment-476019577)
- [`some_if` and `lazy_if`](https://github.com/rust-lang/rfcs/pull/2180)

This functionality could instead be provided by a crate (e.g.
[boolinator](https://docs.rs/boolinator/2.4.0/boolinator/)), but this functionality is commonly
desired and an obvious candidate for the standard library, where an external crate for such simple
functionality is not convenient.

# Prior art
[prior-art]: #prior-art

- Jane Street's OCaml Core library contains a
[`some_if`](https://ocaml.janestreet.com/ocaml-core/109.55.00/tmp/core_kernel/Option.html#VALsome_if)
method on `Option`.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None.

# Future possibilities
[future-possibilities]: #future-possibilities

Methods for converting from a `bool` to a `Result<T, E>`, e.g. `then_else` and `then_else_with`,
would be obvious candidates for inclusion into the standard library, which could also be included
as an addendum to this proposal if desire is expressed.
