- Feature Name: transparent_unions
- Start Date: 2019-02-13
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

Allow `#[repr(transparent)]` on `union`s that have exactly one non-zero-sized field (just like `struct`s).

# Motivation
[motivation]: #motivation

Some `union` types are thin newtype-style wrappers around another type, like `MaybeUninit<T>` (and [once upon a time](https://doc.rust-lang.org/1.26.1/src/core/mem.rs.html#950), `ManuallyDrop<T>`). This type is intended to be used in the same places as `T`, but without being `#[repr(transparent)]` the actual compatibility between it and `T` is left unspecified.

Making types like these `#[repr(transparent)]` would be useful in certain cases. For example, making a `union Wrapper<T>` transparent:

- Clearly expresses the intent of the developer.
- Protects against accidental violations of that intent (e.g., adding a new non-ZST field to a transparent union will result in a compiler error).
- Makes a clear API guarantee that a `Wrapper<T>` can be transmuted to a `T`.

Transparent `union`s are a nice complement to transparent `struct`s, and this RFC rounds out the `#[repr(transparent)]` feature.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

A `union` may be `#[repr(transparent)]` in exactly the same conditions in which a struct may be `#[repr(transparent)]`. Some concrete illustrations follow.

A union may be `#[repr(transparent)]` if it has exactly one non-zero-sized field:

```rust
// This union has the same representation as `usize`.
#[repr(transparent)]
union CustomUnion {
    field: usize,
    nothing: (),
}
```

If the `union` is generic over `T` and has a field of type `T`, it may also be `#[repr(transparent)]` (even if `T` is a zero-sized type):

```rust
// This union has the same representation as `T`.
#[repr(transparent)]
pub union GenericUnion<T: Copy> { // Unions with non-`Copy` fields are unstable.
    pub field: T,
    pub nothing: (),
}

// This is okay even though `()` is a zero-sized type.
pub const THIS_IS_OKAY: GenericUnion<()> = GenericUnion { field: () };
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The logic controlling whether a `union` of type `U` may be `#[repr(transparent)]` should match the logic controlling whether a `struct` of type `S` may be `#[repr(transparent)]` (assuming `U` and `S` have the same generic parameters and fields).

# Drawbacks
[drawbacks]: #drawbacks

- `#[repr(transparent)]` on a `union` is of limited use. There are cases where it is useful, but they're not common and some users might unnecessarily apply `#[repr(transparent)]` to a `union`.

# Rationale and alternatives
[alternatives]: #alternatives

It would be nice to make `MaybeUninit<T>` `#[repr(transparent)]`. This type is a `union`, and thus this RFC is required to allow making it transparent.

Of course, the standard "do nothing" alternative exists. Rust doesn't strictly *require* this feature. But it would benefit from this, so the "do nothing" alternative is undesirable.

# Prior art
[prior-art]: #prior-art

See [the discussion on RFC #1758](https://github.com/rust-lang/rfcs/pull/1758) (which introduced `#[repr(transparent)]`) for some discussion on applying the attribute to a `union`. A summary of the discussion:

> https://github.com/rust-lang/rfcs/pull/1758#discussion_r80436621
> **nagisa:** "Why not univariant unions and enums?"
> **nox:** "I tried to be conservative for now given I don't have a use case for univariant unions and enums in FFI context."

> https://github.com/rust-lang/rfcs/pull/1758#issuecomment-254872520
> **eddyb:** "I found another important usecase: for `ManuallyDrop<T>`, to be useful in arrays (i.e. small vector optimizations), it needs to have the same layout as `T` and AFAICT `#[repr(C)]` is not guaranteed to do the right thing"
> **retep998:** "So we'd need to be able to specify `#[repr(transparent)]` on unions?"
> **eddyb:** "That's the only way to be sure AFAICT, yes."

[joshtriplett_1]: https://github.com/rust-lang/rfcs/pull/1758#issuecomment-274670231
> + **[joshtriplett][joshtriplett_1]:** "In terms of interactions with other features, I think this needs to specify what happens if you apply it to a union with one field, a union with multiple fields, a struct (tuple or otherwise) with multiple fields, a single-variant enum with one field, an enum struct variant where the enum uses `repr(u32)` or similar. The answer to some of those might be "compile error", but some of them (e.g. the union case) may potentially make sense in some contexts."

[pnkfelix_1]: https://github.com/rust-lang/rfcs/pull/1758#issuecomment-290757356
> + **[pnkfelix][pnkfelix_1]:** "However, I personally do not think we need to expand the scope of the feature. So I am okay with leaving it solely defined on `struct`, and leave `union`/`enum` to a follow-on RFC later. (Much the same with a hypothetical `newtype` feature.)"

In summary, many of the questions regarding `#[repr(transparent)]` on a `union` were the same as applying it to a multi-field `struct`. These questions have since been answered, so there should be no problems with applying those same answers to `union`.

# Unresolved questions
[unresolved]: #unresolved-questions

None (yet).

# Future possibilities
[future-possibilities]: #future-possibilities

Univariant `enum`s are ommitted from this RFC in an effort to keep the scope small and avoid unnecessary bikeshedding. A future RFC could explore `#[repr(transparent)]` on a univariant `enum`.

If a `union` has multiple non-ZST fields, a future RFC could propose a way to choose the representation of that `union` ([example](https://internals.rust-lang.org/t/pre-rfc-transparent-unions/9441/6)).
