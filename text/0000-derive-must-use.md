- Feature Name: Let `Option` derive `#[must_use]`
- Start Date: 2026-01-07
- RFC PR: [rust-lang/rfcs#3906](https://github.com/rust-lang/rfcs/pull/3906)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Let `Option` and `Box` derive `#[must_use]` from their generic parameter `T`.

# Motivation
[motivation]: #motivation

If we write:
```rust
#[must_use]
struct Redraw;

fn do_thing() -> Option<Redraw> {
    // Do some thing which requires a redraw...
    Some(Redraw)
}
```
then `do_thing` should be `#[must_use]`, and while we can apply the `#[must_use]` attribute to the function `do_thing`, we shouldn't have to (remember to do so).

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The `Option` and `Box` types will "magically" have the `#[must_use]` attribute if and only if their generic parameter `T` does.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This will be an internal detail of the standard library. It may use another special attribute like `#[derive_must_use_from(T)]`, but for the purposes of this RFC, the `derive_must_use_from` attribute may remain unstable forever.

# Drawbacks
[drawbacks]: #drawbacks

I see no drawbacks besides the small amount of complexity involved.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The only obvious (non-empty) alternative is to add (and stabilise) a new `#[derive_must_use_from(T)]` attribute and apply this to `Option<T>` (and [other types](#future-possibilities)).

This would not be a strict alternative in that nothing prevents this from being done later.

# Prior art
[prior-art]: #prior-art

[RFC #3737](https://github.com/rust-lang/rfcs/pull/3737) is vaguely related (only in that it also pertains to `#[must_use]`).

`#[must_use`] is already tracked through tuples ([example](https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&gist=488fc81eba51a4aded6faeab7ee9bf44)), though strictly speaking this does not apply `#[must_use]` to the tuple type.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

# Future possibilities
[future-possibilities]: #future-possibilities

Possibly a few other standard library types would benefit from this derivation of `#[must_use]`:

- `Box<T>` can do so (included in this RFC, though motivation is weaker)
- `RefCell<T>` and `Mutex<T>` *could* do so but it is unlikely of any use
- `Rc<T>`, `Arc<T>` and various reference types *should not* since they do/may not have exclusive ownership of the value
- `Vec<T>` and other containers *could* do so
