- Feature Name: `transparent_enunions`
- Start Date: 2019-02-13
- RFC PR: [rust-lang/rfcs#2645](https://github.com/rust-lang/rfcs/pull/2645)
- Rust Issue: [rust-lang/rust#60405](https://github.com/rust-lang/rust/issues/60405)

# Summary
[summary]: #summary

Allow `#[repr(transparent)]` on `union`s and univariant `enum`s that have exactly one non-zero-sized field (just like `struct`s).

# Motivation
[motivation]: #motivation

Some `union` types are thin newtype-style wrappers around another type, like `MaybeUninit<T>` (and [once upon a time](https://doc.rust-lang.org/1.28.0/src/core/mem.rs.html#955), `ManuallyDrop<T>`). This type is intended to be used in the same places as `T`, but without being `#[repr(transparent)]` the actual compatibility between it and `T` is left unspecified.

Likewise, some `enum` types only have a single variant, and are similarly thin wrappers around another type.

Making types like these `#[repr(transparent)]` would be useful in certain cases. For example, making the type `Wrapper<T>` (which is a `union` or univariant `enum` with a single field of type `T`) transparent:

- Clearly expresses the intent of the developer.
- Protects against accidental violations of that intent (e.g., adding a new variant or non-ZST field will result in a compiler error).
- Makes a clear API guarantee that a `Wrapper<T>` can be transmuted to a `T` or substituted for a `T` in an FFI function's signature (though users must be careful to not pass uninitialized values through FFI to code where uninitialized values are undefined behavior (like C and C++)).

Transparent `union`s and univariant `enum`s are a nice complement to transparent `struct`s, and this RFC rounds out the `#[repr(transparent)]` feature.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

A `union` may be `#[repr(transparent)]` in exactly the same conditions in which a `struct` may be `#[repr(transparent)]`. An `enum` may be `#[repr(transparent)]` if it has exactly one variant, and that variant matches the same conditions which `struct` requires for transparency. Some concrete illustrations follow.

A union may be `#[repr(transparent)]` if it has exactly one non-zero-sized field:

```rust
// This union has the same representation as `f32`.
#[repr(transparent)]
union SingleFieldUnion {
    field: f32,
}

// This union has the same representation as `usize`.
#[repr(transparent)]
union MultiFieldUnion {
    field: usize,
    nothing: (),
}

// This enum has the same representation as `f32`.
#[repr(transparent)]
enum SingleFieldEnum {
    Variant(f32)
}

// This enum has the same representation as `usize`.
#[repr(transparent)]
enum MultiFieldEnum {
    Variant { field: usize, nothing: () },
}
```

For consistency with transparent `struct`s, `union`s and `enum`s must have exactly one non-zero-sized field. If all fields are zero-sized, the `union` or `enum` must not be `#[repr(transparent)]`:

```rust
// This (non-transparent) union is already valid in stable Rust:
pub union GoodUnion {
    pub nothing: (),
}

// This (non-transparent) enum is already valid in stable Rust:
pub enum GoodEnum {
    Nothing,
}

// Error: transparent union needs exactly one non-zero-sized field, but has 0
#[repr(transparent)]
pub union BadUnion {
    pub nothing: (),
}

// Error: transparent enum needs exactly one non-zero-sized field, but has 0
#[repr(transparent)]
pub enum BadEnum {
    Nothing(()),
}

// Error: transparent enum needs exactly one non-zero-sized field, but has 0
#[repr(transparent)]
pub enum BadEmptyEnum {
    Nothing,
}
```

The one exception is if the `union` or `enum` is generic over `T` and has a field of type `T`, it may be `#[repr(transparent)]` even if `T` is a zero-sized type:

```rust
// This union has the same representation as `T`.
#[repr(transparent)]
pub union GenericUnion<T: Copy> { // Unions with non-`Copy` fields are unstable.
    pub field: T,
    pub nothing: (),
}

// This enum has the same representation as `T`.
#[repr(transparent)]
pub enum GenericEnum<T> {
    Variant(T, ()),
}

// This is okay even though `()` is a zero-sized type.
pub const THIS_IS_OKAY: GenericUnion<()> = GenericUnion { field: () };
pub const THIS_IS_OKAY_TOO: GenericEnum<()> = GenericEnum::Variant((), ());
```

Transparent `enum`s have the additional restriction that they require exactly one variant:

```rust
// Error: transparent enum needs exactly one variant, but has 0
#[repr(transparent)]
pub enum TooFewVariants {
}

// Error: transparent enum needs exactly one variant, but has 2
#[repr(transparent)]
pub enum TooManyVariants {
    First(usize),
    Second(usize),
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The logic controlling whether a `union` of type `U` may be `#[repr(transparent)]` should match the logic controlling whether a `struct` of type `S` may be `#[repr(transparent)]` (assuming `U` and `S` have the same generic parameters and fields). An `enum` of type `E` may be `#[repr(transparent)]` if it has exactly one variant, and that variant follows all the rules and logic controlling whether a `struct` of type `S` may be `#[repr(transparent)]` (assuming `E` and `S` have the same generic parameters, and `E`'s variant and `S` have the same and fields).

Like transarent `struct`s, a transparent `union` of type `U` and transparent `enum` of type `E` have the same layout, size, and ABI as their single non-ZST field. If they are generic over a type `T`, and all their fields are ZSTs except for exactly one field of type `T`, then they have the same layout and ABI as `T` (even if `T` is a ZST when monomorphized).

Like transparent `struct`s, transparent `union`s and `enum`s are FFI-safe if and only if their underlying representation type is also FFI-safe.

A `union` may not be eligible for the same nonnull-style optimizations that a `struct` or `enum` (with the same fields) are eligible for. Adding `#[repr(transparent)]` to  `union` does not change this. To give a more concrete example, it is unspecified whether `size_of::<T>()` is equal to `size_of::<Option<T>>()`, where `T` is a `union` (regardless of whether it is transparent). The Rust compiler is free to perform this optimization if possible, but is not required to, and different compiler versions may differ in their application of these optimizations.  

# Drawbacks
[drawbacks]: #drawbacks

`#[repr(transparent)]` on a `union` or `enum` is of limited use. There are cases where it is useful, but they're not common and some users might unnecessarily apply `#[repr(transparent)]` to a type in a cargo-cult fashion.

# Rationale and alternatives
[alternatives]: #alternatives

It would be nice to make `MaybeUninit<T>` `#[repr(transparent)]`. This type is a `union`, and thus this RFC is required to allow making it transparent. One example in which a transparent representation would be useful is for unused parameters in an FFI-function:

```rust
#[repr(C)]
struct Context {
    // Imagine there a few fields here, defined by an external C library.
}

extern "C" fn log_event(message: core::ptr::NonNull<libc::c_char>,
                        context: core::mem::MaybeUninit<Context>) {
    // Log the message here, but ignore the context since we don't need it.
}

fn main() {
    extern "C" {
        fn set_log_handler(handler: extern "C" fn(core::ptr::NonNull<libc::c_char>,
                                                  Context));
    }

    // Set the log handler so the external C library can call log_event.
    unsafe {
        // Transmuting is safe since MaybeUninit<Context> and Context
        // have the same ABI.
        set_log_handler(core::mem::transmute(log_event as *const ()));
    }

    // We can call it too. And since we don't care about the context and
    // we're using MaybeUninit, we don't have to pay any extra cost for
    // initializing something that's unused.
    log_event(core::ptr::NonNull::new(b"Hello, world!\x00".as_ptr() as *mut _).unwrap(),
              core::mem::MaybeUninit::uninitialized());
}
```

It is also useful for consuming pointers to uninitialized memory:

```rust
#[repr(C)]
struct Cryptor {
    // Imagine there a few fields here, defined by an external C library.
}

// This function may be called from C (or Rust!), and matches the C
// function signature: bool(Cryptor *cryptor)
pub extern "C" fn init_cryptor(cryptor: &mut core::mem::MaybeUninit<Cryptor>) -> bool {
    // Initialize the cryptor and return whether we succeeded
}
```

# Prior art
[prior-art]: #prior-art

See [the discussion on RFC #1758](https://github.com/rust-lang/rfcs/pull/1758) (which introduced `#[repr(transparent)]`) for some discussion on applying the attribute to a `union` or `enum`. A summary of the discussion:

[nagisa_1]: https://github.com/rust-lang/rfcs/pull/1758#discussion_r80436621
> + **[nagisa][nagisa_1]:** "Why not univariant unions and enums?"
> + **nox:** "I tried to be conservative for now given I don't have a use case for univariant unions and enums in FFI context."

[eddyb_1]: https://github.com/rust-lang/rfcs/pull/1758#issuecomment-254872520
> + **[eddyb][eddyb_1]:** "I found another important usecase: for `ManuallyDrop<T>`, to be useful in arrays (i.e. small vector optimizations), it needs to have the same layout as `T` and AFAICT `#[repr(C)]` is not guaranteed to do the right thing"
> + **retep998:** "So we'd need to be able to specify `#[repr(transparent)]` on unions?"
> + **eddyb:** "That's the only way to be sure AFAICT, yes."

[joshtriplett_1]: https://github.com/rust-lang/rfcs/pull/1758#issuecomment-274670231
> + **[joshtriplett][joshtriplett_1]:** "In terms of interactions with other features, I think this needs to specify what happens if you apply it to a union with one field, a union with multiple fields, a struct (tuple or otherwise) with multiple fields, a single-variant enum with one field, an enum struct variant where the enum uses `repr(u32)` or similar. The answer to some of those might be "compile error", but some of them (e.g. the union case) may potentially make sense in some contexts."

[pnkfelix_1]: https://github.com/rust-lang/rfcs/pull/1758#issuecomment-290757356
> + **[pnkfelix][pnkfelix_1]:** "However, I personally do not think we need to expand the scope of the feature. So I am okay with leaving it solely defined on `struct`, and leave `union`/`enum` to a follow-on RFC later. (Much the same with a hypothetical `newtype` feature.)"

In summary, many of the questions regarding `#[repr(transparent)]` on a `union` or `enum` were the same as applying it to a multi-field `struct`. These questions have since been answered, so there should be no problems with applying those same answers to `union` univariant `enum`.

# Unresolved questions
[unresolved]: #unresolved-questions

The role of `#[repr(transparent)]` in nonnull-style optimizations is not entirely clear. Specifically, it is unclear whether the user can rely on these optimizations to be performed when they make a type transparent. [Transparent `union`s somewhat complicate the matter](https://github.com/rust-lang/rfcs/pull/2645#issuecomment-470699497). General consensus seems to be that the compiler is free to decide where and when to perform nonnull-style optimizations on `union`s (regardless of whether or not the `union` is transparent), and no guarantees are made to the user about when and if those optimizations will be applied. It is still an open question exactly what guarantees (if any) Rust makes about transparent `struct`s (and `enum`s) and nonnull-style optimizations.

This RFC doesn't propose any changes to transparent `struct`s, and so does not strictly depend on this question being resolved. But since this RFC is attempting to round out the `#[repr(transparent)]` feature, it seems reasonable to dedicate some time to attempting to round out the guarantees about `#[repr(transparent)]` on `struct`s.

# Future possibilities
[future-possibilities]: #future-possibilities

If a `union` has multiple non-ZST fields, a future RFC could propose a way to choose the representation of that `union` ([example](https://internals.rust-lang.org/t/pre-rfc-transparent-unions/9441/6)).
