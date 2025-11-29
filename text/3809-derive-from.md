- Feature Name: `derive_from`
- Start Date: 2025-05-06
- RFC PR: [rust-lang/rfcs#3809](https://github.com/rust-lang/rfcs/pull/3809)
- Tracking Issue: [rust-lang/rust#144889](https://github.com/rust-lang/rust/issues/144889)

## Summary
[summary]: #summary

Allow deriving an implementation of the `From` trait using `#[derive(From)]` on structs with a single field.

```rust
#[derive(From)]
struct TcpPort(u16);

// Generates:
impl From<u16> for TcpPort {
    fn from(value: u16) -> Self {
        Self(value)
    }
}
```

This would only be allowed for single-field structs for now, where we can unambiguously determine the source type from which should the struct be convertible.

## Motivation
[motivation]: #motivation

The primary motivation is to remove one of the gaps in the Rust language which prohibit combining language features in intuitive ways. Both the `#[derive(Trait)]` macro and the `From` trait are pervasively used across the Rust ecosystem, but it is currently not possible to combine them, even in situations where the resulting behavior seems *completely obvious*.

Concretely, when you have a struct with a single field and want to implement the `From` trait to allow creating a value of the struct from a value of the field, `#[derive(From)]` seems like the most intuitive way of achieving that. `From` is a standard library trait, `#[derive(Trait)]` works with many other such traits (such as `Hash`, `Eq`, `Clone`, etc.), and there is essentially only one possible implementation that makes sense. However, when users currently try to do that, they are met with a compiler error.

Enabling this would make one more intuitive use-case in the language "just work", and would reduce boilerplate that Rust users either write over and over again or for which they have to use macros or external crates.

### Newtype pattern
As a concrete use-case, `#[derive(From)]` is particularly useful in combination with the very popular [newtype pattern](https://doc.rust-lang.org/rust-by-example/generics/new_types.html). In this pattern, an inner type is wrapped in a new type (hence the name), typically a tuple struct, to semantically make it a separate concept in the type system and thus make it harder to mix unrelated types by accident. For example, we can wrap a number to represent things like `Priority(i32)`, `PullRequestNumber(u32)` or `TcpPort(u16)`.

When using the newtype pattern, it is common to implement standard library traits for it by delegating to the inner type. This is easily achievable with `#[derive]`:

```rust
#[derive(Hash, PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug)]
struct UserId(u32);
```

However, not all standard library traits can be derived in this way, including the `From` trait. Currently, users have to write the boilerplate `From` implementation by hand. If there are many newtypes in a crate, this might lead users to implement a macro, which unnecessarily obfuscates the code, or use an external crate to derive the implementation, which increases code size and compile times.

It should be noted that there are cases where the newtype should not be able to store all possible values of the inner field, e.g. `struct Email(String)`. In that case an implementation of `From` might not be desirable, and the newtype will likely implement its own constructor function that performs validation. For cases where the newtype can represent all values of the inner field, implementing `From` for it is quite natural, as it is the designated Rust trait for performing lossless conversions.

Is `From` really so useful for newtypes? There are two other common alternatives for constructing a value of a newtype apart from using `From`:
- Using the struct literal syntax directly, such as `UserId(5)` or `UserId { id: 5 }`. This is explicit, but it does not work in generic code (unlike `From`) and it can either only be used in the module of the struct, or the struct field has to become publicly visible, which is usually not desirable.
- Using a constructor function, often called `new`. This function cannot be derived (without using custom proc macros) and has to be implemented using a manual `impl` block. It is essentially boilerplate code if the newtype does not need to perform any validation of the field value. If it was possible to easily derive `From`, then it could be used instead of an explicit `new` function, which could reduce the need to create any `impl` blocks for simple newtypes.

To summarize, if `From` was `derive`-able, it could reduce the need for using macros or external crates and increase the number of cases where `#[derive]` takes care of all required `impl`s for a given newtype.

### Why does it make sense to derive `From`?
There are various "standard" traits defined in the Rust standard library that are pervasively used across the ecosystem. Currently, some of these traits can already be automatically derived, for example `Hash` or `Debug`. These traits can be derived automatically because they are composable; an implementation of the trait for a struct can be composed of the trait implementations of its fields.

One reason why we might not want to enable automatic derive for a specific trait is when the implementation would not be *obvious*. For example, if we allowed deriving `Display`, it is unclear how should the individual field implementations be composed. Should they be separated with a newline? Or a comma? That depends on the given type.

However, when deriving a `From` implementation for a struct with a single field, the implementation seems straightforward and *obvious* (simply wrap the inner type in the struct). It should thus be possible to automatically derive it.

That being said, the fact that the `From` trait is generic does present more opportunities for alternative designs. These are discussed in [Rationale and alternatives](#rationale-and-alternatives).

### How common is implementing and deriving `From`?
[This](https://github.com/search?type=code&q=lang%3ARust+%2F%5C%5Bderive%5C%28.*%5CbFrom%5B%2C+%5C%29%5D%2F) GitHub Code Search query shows tens of thousands of occurrences of the `From` trait being derived, typically using the `derive_more` crate.

I have also scanned the top 100 crates from crates.io together with their dependencies using a simple [script](https://github.com/Kobzol/scan-from-impls), to find all instances of tuple structs with a single field where the struct implements `From<FieldType>`.

In the analyzed 168 crates, 559 single-field tuple structs were found, and 49 out of them contained the `From` implementation from their field type.

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

You can use `#[derive(From)]` to automatically generate an implementation of the `From` trait for the given type, which will create a value of the struct from a value of its field:

```rust
#[derive(From)]
struct UserId(u32);

// Will generate:
impl From<u32> for UserId {
    fn from(value: u32) -> Self {
        Self(value)
    }
}
```

You can only use `#[derive(From)]` on structs that contain exactly one field, otherwise the compiler would not know from which type should the `From` implementation be generated. For example, the following code snippet does not compile:

```rust
#[derive(From)] // <-- This DOES NOT compile
struct User {
    id: u32,
    name: String
}
```

In this case, the compiler wouldn't know if it should generate `From<u32> for User` or `From<String> for User`, nor how it should figure out which value to use for the other field when constructing `User`.

Note that the generated `From` implementation only allows converting the value of the field into a value of the struct. It does not allow conversion in the opposite direction:

```rust
#[derive(From)]
struct UserId(u32);

fn foo() {
    let user_id: UserId = 0.into(); // works
    let user_id: u32 = user_id.into(); // does NOT work
}
```

If you need to support conversion in the opposite direction, you will need to implement `impl From<FieldType> for StructType` manually.

If the struct is generic over the type of the inner field, the `From` implementation will be also generic:

```rust
#[derive(From)]
struct Id<T: Debug>(T);

// Will generate:
impl<T: Debug> From<T> for Id {
    fn from(value: T) -> Self {
        Self(value)
    }
}
```

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Placing `#[derive(From)]` on a tuple struct or a struct with named fields named `$s` is permissible if and only if the struct has exactly one field that we will label as `$f`. We will use the name `$t` for the type of the field `$f`. In that event, the compiler shall generate the following:

- If `$s` is a tuple struct:
    ```rust
    impl ::core::convert::From<$t> for $s {
        fn from(value: $t) -> Self {
            Self(value)
        }
    }
    ```
- If `$s` is a struct with a named field `$f`:
    ```rust
    impl ::core::convert::From<$t> for $s {
        fn from($f: $t) -> Self {
            Self {
                $f
            }
        }
    }
    ```

Using `#[derive(From)]` on unit structs, enums or tuple/named field structs that do not have exactly one field produces a compiler error.

## Drawbacks
[drawbacks]: #drawbacks

While this does enable more Rust code to "just work", it also introduces a special case that will have to be explained to the users. In this case it seems quite easily understandable though ("it only works for structs with a single field"), and we should be able to produce high-quality error messages in the compiler, as it is trivial to detect how many fields a struct has.

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Based on the popularity of the `derive_more` crate (discussed in [Prior art](#prior-art)), which had more than 125 million downloads when this RFC was proposed, it seems that there is a lot of appetite for extending the set of use-cases where deriving standard traits is allowed. This feature was discussed in the past [here](https://github.com/rust-lang/rfcs/issues/2026).

The proposed change enables the usage of an existing feature (`#[derive]`) in more situations. It makes code easier to read by using an intuitive built-in feature instead of forcing users to write boilerplate code or use macros.

As always, an alternative is to just not do this, in that case users would continue implementing `From` using boilerplate code, macros or external crates.

Because the scope of the proposed change is quite minimal, it should be forward-compatible with designs that would make it work in more situations in the future (some ideas are discussed in [Future possibilities](#future-possibilities)). There is one potential (although unlikely) incompatibility discussed below.

### Alternative design using tuples
There is one possible alternative design that comes to mind which could be in theory incompatible with this proposal. We could enable `#[derive(From)]` for tuple structs with an arbitrary number of fields by generating a `From` implementation from a tuple containing the types of the struct fields:

```rust
#[derive(From)]
struct Foo(u32, u16, bool);

impl From<(u32, u16, bool)> for Foo { ... }
```

The question then becomes what would be generated under this design when the struct has exactly one field.

- We could either generate `From<T> for Type`, which would be compatible with this RFC. It would also be slightly inconsistent though, as it would generate something different only for the case with a single field.
    - This is how the `derive_more::From` macro behaves.
- Or, we could generate `From<(T, )> for Type`, which would be consistent with the logic of generating `From<tuple>`. However, single-field tuples are not idiomatic and it would be awkward having to write e.g. `(value, ).into()` to make use of the impl.

I think that the second approach is not a good idea, and I find it unlikely that we would want to use it.

### Generating `From` in the other direction
This proposed change is useful to generate a `From` impl that turns the inner field into the wrapper struct (`impl From<Inner>` for `Newtype`). However, sometimes it is also useful to generate the other direction, i.e. turning the newtype back into the inner type. This can be implemented using `impl From<Newtype> for Innertype`.

We could make `#[derive(From)]` generate both directions, but that would make it impossible to only ask for the "basic" `From` direction without some additional syntax.

A better alternative might be to support generating the other direction in the future through something like `#[derive(Into)]`.

### More general blanket implementation
As an alternative to generating `From<Inner> for Newtype`, the compiler could generate a more generic blanket implementation, such as `impl<T> From<T> for Newtype where Inner: From<T>`[^blanket].

This would allow "recursive conversions", for example:
```rust
#[derive(From)]
struct UserId(u64);

// Generated code:
impl<T> From<T> for UserId where u64: From<T> {
    fn from(value: T) -> Self {
        let value: u64 = value.into();
        Self(value)
    }
}

fn create_user_id(value: u32) -> UserId {
    value.into()
}
```

While this can be certainly useful in some scenarios, it feels too "magical" to be the default; it does not seem like it is the most straightforward implementation that users would expect to be generated. The existing standard library traits are not derived in this way, as they are not generic (unlike `From`).

This generated implementation would also conflict with a `From` implementation in the "other direction", from the newtype to the inner field (`impl From<UserId> for u32`), which seems problematic.

The `derive_more` crate allows opting into the blanket implementation using a custom attribute (`#[from(forward)]`).

[^blanket]: Noted [here](https://internals.rust-lang.org/t/pre-rfc-derive-from-for-newtypes/22567/6).

### Direction of the `From` impl
In theory, someone could be confused if this code:
```rust
#[derive(From)]
struct Newtype(Inner);
```
generates this impl:
```rust
impl From<Inner> for Newtype { ... }
```
or this impl:
```rust
impl From<Newtype> for Inner { ... }
```
However, `impl From<Inner> for Newtype` is consistent with all other standard traits that can currently be derived, as they all generate code in the form of `impl Trait for Type`. It should thus not be very surprising that `#[derive(From)]` provides the impl for the outer type, not the inner type. This will also be clearly documented.

Generating the other direction of the impl is best left as a separate feature, which is briefly discussed in [Future possibilities][future-possibilities].

## Prior art
[prior-art]: #prior-art

### Ecosystem crates
There are several crates that offer deriving the `From` trait. The most popular one is [derive_more](https://crates.io/crates/derive_more), which allows deriving several standard traits that are normally not derivable, including `From`, `Display` or `Add`.

[`#[derive(derive_more::From)`](https://docs.rs/derive_more/latest/derive_more/derive.From.html) works in the same way as proposed in this RFC for structs with a single field. However, it can also be used for other kinds of structs and even enums and supports more complex use-cases. For example:
- For structs with multiple fields, it generates an impl from a tuple containing these fields:
    ```rust
    #[derive(derive_more::From)]
    struct Point(i32, i32);

    assert_eq!(Point(1, 2), (1, 2).into());
    ```
- You can opt into additional types for which a `From` impl will be generated:
    ```rust
    #[derive(derive_more::From)]
    #[from(Cow<'static, str>, String, &'static str)]
    struct Str(Cow<'static, str>);
    ```
- For enums, it generates a separate `From` impl for each enum variant:
    ```rust
    #[derive(derive_more::From)]
    enum Foo {
        A(u32),
        B(bool)
    }
    // Generates
    impl From<u32> for Foo {
        fn from(value: u32) -> Self {
            Self::A(value)
        }
    }

    impl From<bool> for Foo {
        fn from(value: bool) -> Self {
            Self::B(value)
        }
    }
    ```

The design proposed by this RFC should be forward compatible with all features of `derive_more`[^enums], if we decided to adopt any of them in the future.

[^enums]: If we only allow the `#[derive(From)]` on structs, and not enums, see [Unresolved questions](#unresolved-questions).

### Default trait
There is a precedent for a trait that can only be automatically derived in certain situations. The `Default` trait was originally only derivable on structs, not on enums, because it was not clear which enum variant should be selected as the default. This was later rectified by adding custom syntax (`#[default]`) to select the default variant.

A similar solution could be used in the future to also extend `#[derive(From)]` to more use-cases; this will be discussed in [Future possibilities](#future-possibilities).

The `Default` trait actually shares a similarity with `From`, in that they are both "constructor" traits that create a new value of a given type, so it feels natural that both should be automatically implementable, at least in some cases.

## Unresolved questions
[unresolved-questions]: #unresolved-questions

### Enum support
Should we also support enums? The design space there is more complex than for structs. For example, `derive_more` generates a separate `From` impl for each enum variant by default, which means that the individual variants must not contain the same inner type, otherwise an impl conflict happens:

```rust
#[derive(derive_more::From)]
enum Foo {
    A(u32),
    B(u32)
}
// Generates the following two impls:
impl From<u32> for Foo {
    fn from(value: u32) -> Self {
        Foo::A(value)
    }
}

// [ERROR] Conflicting impl
impl From<u32> for Foo {
    fn from(value: u32) -> Self {
        Foo::B(value)
    }
}
```

This could be difficult to explain.

As an alternative, we could use a simpler approach and only allow `#[derive(From)]` for single-variant enums containing a single field. However, these are likely not very common.

A better solution might be to use a custom attribute (such as `#[from]`) to allow users to customize which variant of the enum should be created (see [Future possibilities](#future-possibilities)), but this complicates the design space.

For these reasons, this RFC only proposes to support structs as the first step, similarly to the `Default` trait, which was originally also only derivable on structs. We could support more use-cases with future extensions.

## Future possibilities
[future-possibilities]: #future-possibilities

### `#[from]` attribute
In the future, we could extend the set of supported use-cases even to structs with multiple fields. For example, we could allow it in cases where the user marks a specific field with a `#[from]` attribute, and all other fields implement `Default`:

```rust
#[derive(From)]
struct Port {
    #[from]
    port: u16,
    protocol: Protocol
}

#[derive(Default)]
enum Protocol {
    #[default]
    Tcp,
    Udp
}
```
which would generate this impl:
```rust
impl From<u16> for Port {
    fn from(port: u16) -> Self {
        Self {
            port,
            protocol: Default::default()
        }
    }
}
```

This is similar to how [RFC#3107](https://rust-lang.github.io/rfcs/3107-derive-default-enum.html) extended the deriving of the `Default` trait using the `#[default]` attribute.

### Deriving From in the other direction

It is also quite useful to generate `From<InnerType> for Struct`, i.e. generating `From` in the other direction. This could be done in the future using e.g. `#[derive(Into)]`.

### Enum support
We could add support for enums in a similar way, where users could mark the variant that should be constructed using `#[from]`.

### Supporting other traits
We could extend the same logic (only allowing deriving a standard trait for structs with a single field) to more traits. For example, `AsRef`, `Deref` or even things like `FromStr` or `Iterator` could be potentially derivable in the same way, when used on a struct with a single field.
