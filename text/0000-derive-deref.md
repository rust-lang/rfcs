- Feature Name: `derive_deref`
- Start Date: 2026-01-22
- RFC PR: [rust-lang/rfcs#3911](https://github.com/rust-lang/rfcs/pull/3911)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Allow deriving an implementation of the `Deref` trait using `#[derive(Deref)]` on structs and enums.

```rust
#[derive(Deref)]
struct TcpPort(u16);

// Generates:
impl Deref for TcpPort {
    type Target = u16;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Deref)]
struct Info {
    inner: u16,
}

// Generates:
impl Deref for Info {
    type Target = u16;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Deref)]
enum Enum {
    V1(u16),
    V2 { field: u16 },
}

// Generates:
impl Deref for Enum {
    type Target = u16;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::V1(v) => v,
            Self::V2 { field } => field,
        }
    }
}
```

If there is more than one field, a `#[deref]` attribute will be required on the field we want to "deref":

```rust
#[derive(Deref)]
struct TcpPort(u16, #[deref] u16);

// Generates:
impl Deref for TcpPort {
    type Target = u16;

    fn deref(&self) -> &Self::Target {
        &self.1
    }
}

#[derive(Deref)]
struct Info {
    a: u16,
    #[deref]
    b: u16,
}

// Generates:
impl Deref for Info {
    type Target = u16;

    fn deref(&self) -> &Self::Target {
        &self.b
    }
}

#[derive(Deref)]
enum Enum {
    V1(u16),
    V2 { #[deref] a: u16, b: u32 },
}

// Generates:
impl Deref for Enum {
    type Target = u16;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::V1(v) => v,
            Self::V2 { a, b } => a,
        }
    }
}
```

# Motivation
[motivation]: #motivation

The primary motivation is to remove one of the gaps in the Rust language which prohibit combining language features in intuitive ways. Both the `#[derive(Trait)]` macro and the `Deref` trait are pervasively used across the Rust ecosystem, but it is currently not possible to combine them, even in situations where the resulting behavior seems *completely obvious*.

Concretely, when you have a struct with a single field and want to implement the `Deref` trait to allow getting access to a field or field's method of the type without needing to do explicitly access the field first, `#[derive(Deref)]` seems like the most intuitive way of achieving that. `Deref` is a standard library trait, `#[derive(Trait)]` works with many other such traits (such as `Hash`, `Eq`, `Clone`, etc.), and there is essentially only one possible implementation that makes sense. However, when users currently try to do that, they are met with a compiler error.

Enabling this would make one more intuitive use-case in the language "just work", and would reduce boilerplate that Rust users either write over and over again or for which they have to use macros or external crates.

## Newtype pattern
As a concrete use-case, `#[derive(Deref)]` is particularly useful in combination with the very popular [newtype pattern](https://doc.rust-lang.org/rust-by-example/generics/new_types.html). In this pattern, an inner type is wrapped in a new type (hence the name), typically a tuple struct, to semantically make it a separate concept in the type system and thus make it harder to mix unrelated types by accident. For example, we can wrap a number to represent things like `Priority(i32)`, `PullRequestNumber(u32)` or `TcpPort(u16)`.

When using the newtype pattern, it is common to implement standard library traits for it by delegating to the inner type. This is easily achievable with `#[derive]`:

```rust
#[derive(Hash, PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug)]
struct UserId(u32);
```

However, not all standard library traits can be derived in this way, including the `Deref` trait. Currently, users have to write the boilerplate `Deref` implementation by hand. If there are many newtypes in a crate, this might lead users to implement a macro, which unnecessarily obfuscates the code, or use an external crate to derive the implementation, which increases code size and compile times.

Is `Deref` really so useful for newtypes? Newtypes are used to make it more explicit what a type is about, like `PullRequestNumber(u32)`. Or to allow to implement traits on external types, like implementing `Drop` on an integer representing a system ID. However, a lot of time, the wrapping "gets in the way" so you have to implement `Deref` to have access to the field without doing it explicitly.

To summarize, if `Deref` was `derive`-able, it could reduce the need for using macros or external crates and increase the number of cases where `#[derive]` takes care of all required `impl`s for a given newtype.

## Other cases
For other cases, like struct with multiple fields or enums, the `Deref` trait becomes useful when one field is the "main" information and the rest is "additional" information. For example:

```rust
struct Id {
    id: u32,
    needs_to_be_released: bool,
}
```

In this case, the `id` field is the actual information we want to manipulate whereas the `needs_to_be_released` is only useful in specific contexts (like a `Drop` trait implementation).

For enums, it's similar:

```rust
enum Id {
    Owned(u32),
    Borrowed(u32),
}
```

In case it's `Owned`, then the `Drop` implementation will release the resource, otherwise it won't. However, the `u32` always represents the same data, whatever the variant.

## Why does it make sense to derive `Deref`?
There are various "standard" traits defined in the Rust standard library that are pervasively used across the ecosystem. Currently, some of these traits can already be automatically derived, for example `Hash` or `Debug`. These traits can be derived automatically because they are composable; an implementation of the trait for a struct can be composed of the trait implementations of its fields.

One reason why we might not want to enable automatic derive for a specific trait is when the implementation would not be *obvious*. For example, if we allowed deriving `Display`, it is unclear how should the individual field implementations be composed. Should they be separated with a newline? Or a comma? That depends on the given type.

However, when deriving a `Deref` implementation for a struct with a single field, the implementation seems straightforward and *obvious* (simply wrap the inner type in the struct). It should thus be possible to automatically derive it. For other cases, the `#[deref]` attribute will remove any ambiguity.

## How common is implementing and deriving `Deref`?

[This](https://github.com/search?type=code&q=lang%3ARust+%2F%5C%5Bderive%5C%28.*%5CbDeref%5B%2C+%5C%29%5D%2F) GitHub Code Search query shows tens of thousands of occurrences of the `Deref` trait being derived, typically using the `derive_more` crate.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

You can use `#[derive(Deref)]` to automatically generate an implementation of the `Deref` trait for the given type, which will allow to implicitly have access to the derefed field:

```rust
#[derive(Deref)]
struct UserId(u32);

// Will generate:
impl Deref for UserId {
    type Target = u32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
```

If the type is generic over the type of the inner field, the `Deref` implementation will be also generic:

```rust
#[derive(Deref)]
struct Id<T: Debug>(T);

// Will generate:
impl<T: Debug> Deref for Id<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
```

For enums where all variants contain one field of the same type, it will give:

```rust
#[derive(Deref)]
enum Id {
    Owned(u32),
    Borrowed { value: u32 },
}

// Will generate:
impl Deref for Id {
    type Target = u32;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Owned(v) => v,
            Self::Borrowed { value } => value,
        }
    }
}
```

If there any ambiguities because there are more than one field, then the user will need to add the `#[deref]` attribute on the field they want to be derefed:

```rust
#[derive(Deref)]
struct UserId(#[deref] u32, u8);

// Will generate:
impl Deref for UserId {
    type Target = u32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Deref)]
enum Id {
    Owned(#[deref] u32, bool),
    Borrowed { #[deref] value: u32, something_else: bool },
}

// Will generate:
impl Deref for Id {
    type Target = u32;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Owned(v, _) => v,
            Self::Borrowed { value, .. } => value,
        }
    }
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Placing `#[derive(Deref)]` on a struct is always permissible as long as there is at least one field, otherwise it will produce a compilation error. If there are multiple fields and the `#[deref]` attribute is not used, it will produce a compilation error. If the `#[deref]` attribute is present more than once, it will produce a compilation error.

Type we deref is `$t`;

- If `$s` is a tuple struct with one field:
    ```rust
    impl ::std::ops::Deref for $s {
        type Target = $t;

        fn deref(&self) -> &Self::Target {
            self.0
        }
    }
    ```
- If `$s` is a tuple struct with x fields when `$n` is the field with the `#[deref]` attribute:
    ```rust
    impl ::std::ops::Deref for $s {
        type Target = $t;

        fn deref(&self) -> &Self::Target {
            self.$n
        }
    }
    ```
- If `$s` is a struct with a named field `$f`:
    ```rust
    impl ::std::ops::Deref for $s {
        type Target = $t;

        fn deref(&self) -> &Self::Target {
            self.$f
        }
    }
    ```

    This is the same implementation if there are more than one field thanks to the `#[deref]` attribute.

Placing `#[derive(Deref)]` on enums, requires for all its variants to share one common type, otherwise it'll produce a compiler error. If a variant has no field, it will produce a compilation error. If a variant has more than one field and the `#[deref]` attribute isn't used, it will produce a compilation error. If the `#[deref]` attribute is present more than once on a variant, it will produce a compilation error.

An enum `$e`, with a first variant named `$v1` which is a tuple variant ; and a second variant named `$v2` which is struct-like variant with a field named `$f`:

- Both variants only have one field:
    ```rust
    impl ::std::ops::Deref for $e {
        type Target = u32;

        fn deref(&self) -> &Self::Target {
            match self {
                Self::$v1(v) => v,
                Self::$v2 { $f } => $f,
            }
        }
    }
    ```
- Both variants have two fields and use the `#[deref]` attribute on the first field of each variant:
    ```rust
    impl ::std::ops::Deref for $e {
        type Target = u32;

        fn deref(&self) -> &Self::Target {
            match self {
                Self::$v1(v, _) => v,
                Self::$v2 { $f, .. } => $f,
            }
        }
    }
    ```


Using `#[derive(Deref)]` on unions produces a compilation error.

# Drawbacks
[drawbacks]: #drawbacks

While this does enable more Rust code to "just work", it also means that we should be able to produce high-quality error messages in the compiler, as it is trivial to detect how many fields a struct or an enum variant has.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Based on the popularity of the `derive_more` crate (discussed in [Prior art](#prior-art)), which had more than 125 million downloads when this RFC was proposed, it seems that there is a lot of appetite for extending the set of use-cases where deriving standard traits is allowed. This feature was discussed in the past [here](https://github.com/rust-lang/rfcs/issues/2026).

The proposed change enables the usage of an existing feature (`#[derive]`) in more situations. It makes code easier to read by using an intuitive built-in feature instead of forcing users to write boilerplate code or use macros.

As always, an alternative is to just not do this, in that case users would continue implementing `Deref` using boilerplate code, macros or external crates.

Because the scope of the proposed change is quite minimal, it should be forward-compatible with designs that would make it work in more situations in the future (some ideas are discussed in [Future possibilities](#future-possibilities)). There is one potential (although unlikely) incompatibility discussed below.

# Prior art
[prior-art]: #prior-art

## Ecosystem crates
There are several crates that offer deriving the `Deref` trait. The most popular one is [derive_more](https://crates.io/crates/derive_more), which allows deriving several standard traits that are normally not derivable, including `Deref`, `Display` or `Add`.

[`#[derive(derive_more::Deref)`](https://docs.rs/derive_more/latest/derive_more/derive.Deref.html) works in the same way as proposed in this RFC for structs with a single field. However, it can also be used for other kinds of structs and even enums and supports more complex use-cases. For example:
- For structs with multiple fields, you need to add the `#[deref]` attribute on the field to be the target of the `Deref` trait:
    ```rust
    #[derive(derive_more::Deref)]
    struct Point(#[deref] i32, i32);
    ```
- You can use the `#[forward]` attribute to use the `Deref` target of the current derefed item.
    ```rust
    #[derive(Deref)]
    #[deref(forward)]
    struct MyBoxedInt(Box<i32>);

    // generates:
    impl derive_more::core::ops::Deref for MyBoxedInt {
        type Target = <Box<i32> as derive_more::core::ops::Deref>::Target;
        #[inline]
        fn deref(&self) -> &Self::Target {
            <Box<i32> as derive_more::core::ops::Deref>::deref(&self.0)
        }
    }
    ```

The design proposed by this RFC should be forward compatible with all features of `derive_more`[^enums], if we decided to adopt any of them in the future.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

Should we also add a new `DerefMut` derive, with the same conditions as described here for the `Deref` trait? It comes with new questions like, "what happens when you implement `Deref` manually but use `derive(DerefMut)` ; should it error when `Deref::Target` isn't the expected type?" Example:

```rust
#[derive(DerefMut)] // should this error or rely on silent coercion of String to str?
pub struct S(String);

impl Deref for S {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}
```

# Future possibilities
[future-possibilities]: #future-possibilities

## `#[forward]` attribute
In the future, we could extend the set of supported use-cases. For example, we could allow to have a "traversal" `Deref` when the target already implements `Deref`:

```rust
#[derive(Deref)]
#[deref(forward)]
struct MyBoxedInt(Box<i32>);

```

which would generate this impl:

```rust
impl derive_more::core::ops::Deref for MyBoxedInt {
    type Target = <Box<i32> as derive_more::core::ops::Deref>::Target;
    #[inline]
    fn deref(&self) -> &Self::Target {
        <Box<i32> as derive_more::core::ops::Deref>::deref(&self.0)
    }
}
```

This is similar to how [RFC#3107](https://rust-lang.github.io/rfcs/3107-derive-default-enum.html) extended the deriving of the `Default` trait using the `#[default]` attribute.
