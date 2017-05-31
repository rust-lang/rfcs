- Feature Name: bang_auto_impls
- Start Date: 2016-06-04
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

Make `!` automatically implement traits for which it has only a single possible
implementation.

# Motivation
[motivation]: #motivation

If a trait has no static methods or associated types then it has exactly one
possible implementation for the `!` type. To illustrate this, consider the
`Debug` trait and it's implementation for `!`.

```rust
trait Debug {
    fn fmt(&self, &mut Formatter) -> Result<(), fmt::Error>
}

impl Debug for ! {
    fn fmt(&self, _: &mut Formatter) -> Result<(), fmt::Error> {
        *self
    }
}
```

The important thing to note here is that it doesn't matter what we put in the
body of the `fmt` impl, because any method on `!` which takes a `self` can
never be called. For example, we could have also written the implementation as
this:

```rust
impl Debug for ! {
    fn fmt(&self, _: &mut Formatter) -> Result<(), fmt::Error> {
        println!("Formatting a `!`");
        Ok(())
    }
}
```

But this implementation is exactly equivalent to the first - the entire method
body gets eliminated as dead code.

Because of this, many traits - probably more than half in practice - have a
unique, trivial implementation for `!`. And this implementation can be
inferred. This RFC proposes that these implementations be automatically
inferred unless the user opts-out through an attribute on the trait.

To see why this would be a valuable feature, consider the alternative where we
simply write out the impl wherever we want it. In the standard library we
might write out implementations of `Debug`, `Display` and `Error` as obvious
cases. But what about, say, `Hasher`? `Hasher` only has non-static methods, if
someone wants to use `!` where a `Hasher` trait bound is in force there's no
reason why it shouldn't work. In fact, half of the traits in the standard
library are like this. [This comment](https://github.com/rust-lang/rfcs/pull/1216#issuecomment-212265320)
lists most of them.

Having to write out impls for `!` manually is a chore and will clutter code.
What's likely to happen in practice is that people won't think or won't bother
to impl their traits for `!` and then users of those traits won't be able to
use `!` even where it otherwise would make sense.

# Detailed design
[design]: #detailed-design

Any traits that have a unique, trivial implementation for `!` should have that
implementation automatically derived. This includes all traits *except*:

* Traits which have a static method:
  If a trait has a method which does not take a `self` then there may be many
  non-equivalent ways to implement that method.
* Traits which have an associated type:
  Because there are many possible choices for the type.

## Opting-out

Even where it's possible to infer the impl for `!` there may be cases where
people don't want this behaviour. For example, someone might define a marker
trait `trait Marker { }` whose purpose is to only include some small class of
types, not including `!`. For these cases this RFC proposes allowing the
following to opt-out of automatically implementing a trait.

```rust
impl !Marker for ! {}
```

# Drawbacks
[drawbacks]: #drawbacks

* Add's more complexity to the language and compiler.
* People who aren't aware of this feature might be surprised to learn that `!`
  implements their trait. In most cases this won't be a huge problem since `!`
  *should* implement their trait, however in the cases where it shouldn't
  they will need to know to opt-out. At any rate, `!` is already a rather
  surprising type in that it can magically transform into other types under the
  right conditions. This is possible essentially because there is exactly one
  possible implementation of `Into<T>` for `!` for all `T`, and the
  transformation only occurs in dead code anyway. The author sees this RFC as
  an extension in spirit of this behaviour.

# Alternatives
[alternatives]: #alternatives

* Not do this.
* Add a way to opt-in to derivining impls instead.  Using a `#[derive_impl(!)]`
  attribute on traits which have an inferable impl for `!` would be less
  cumbersome than writing these impls out by hand. However it still comes with
  the problems of clutter and that most traits could use this attribute but
  people won't think or won't bother to add it.

# Unresolved questions
[unresolved]: #unresolved-questions

* None known

