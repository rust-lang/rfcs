- Feature Name: `ref_wrapping`
- Start Date: 2023-02-01
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

For `#[repr(transparent)]` structs and variants, allow `&Wrapper(*reference)` as a safe alternative to transmutation.

# Motivation
[motivation]: #motivation

Rust as a language prides itself on zero-cost abstraction. One extremely common such abstraction is the newtype pattern, where values are wrapped in types entirely for the sake of type-safety:

```rust

pub struct Metres(f64);

pub struct Seconds(f64);

pub fn wait(time: Seconds) {
    // ...
}

pub fn walk_north(distance: Metres) {
    // ...
}
```

However, this kind of abstraction falls apart in the presence of dynamically sized types like `str`:

```rust
pub struct Identifier(str);
impl Identifier {
    pub fn from_str(s: &str) -> Option<&Identifier> {
        if is_valid_identifier(s) {
            // what do we do here?
        } else {
            None
        }
    }
}
```

If we want to avoid using unsafe code, we're often forced to add a lifetime parameter:

```rust
pub struct Identifier<'a>(&'a str);
impl<'a> Identifier<'a> {
    pub fn from_str(s: &str) -> Option<Identifier<'_>> {
        if is_valid_identifier(s) {
            Some(Identifier(s))
        } else {
            None
        }
    }
}
```

Or store the result on the heap:

```rust
pub struct Identifier(String);
impl Identifier {
    pub fn from_str(s: &str) -> Option<Identifier> {
        if is_valid_identifier(s) {
            Some(Identifier(s.to_owned()))
        } else {
            None
        }
    }
}
```

For now, we'll focus more on the first result, since it has much more interesting implications. The second result is definitely *not* a zero-cost abstraction, but a lot of people will consider it acceptable if you're already boxing the result anyway.

One of the biggest issues with adding an extra layer of references is that mutability is fixed, which requires either additional types:
```rust
pub struct Identifier<'a>(&'a str);
pub struct IdentifierMut<'a>(&'a mut str);
```

*Or* an exclusive borrow over the original value:

```rust
pub struct Identifer<'a>(&'a mut str);
// there is no immutable version
```

Another issue is that adding a lifetime parameter makes it much more difficult to implement pre-GAT traits which expect a cohesive type:

```rust
pub struct Identifier<'a>(&'a str);
pub struct OwnedIdentifier(String);

impl Deref for OwnedIdentifier {
    type Target = Identifier<'???>; // even if we could put a lifetime here
    fn deref(&self) -> &Identifier<'???> { // it won't work here
        // ???
    }
}
```

Ultimately, there are more issues with dynamically sized types than the ones presented here. Structs and enum variants with more than one dynamically sized field will not be able to use the solution this RFC proposes. However, the presence of and increased usage of `#[repr(transparent)]` suggests a rather elegant solution that would fit well in the language.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Right now, Rust lets you define `#[repr(transparent)]` structs with dynamically sized fields. Although this *isn't* the current implementation, imagine this version of `std::path::Path`:

```rust
#[repr(transparent)]
pub struct Path([u8]);
```

In order to create a `Path` from a slice, we use the below peculiar syntax:

```rust
impl Path {
    pub fn from_bytes(s: &[u8]) -> &Path {
        &Path(*s)
    }
}
```

This kind of thing *won't* work for ordinary functions, for two reasons:

1. Since `*s` is dynamically sized, we can't pass it into a function as an argument.
2. `Path(*s)` would be dropped at the end of the function, and we can't return a reference that would outlive the function.

However, since `Path` is specifically a `#[repr(transparent)]` struct, the compiler is able to see through this syntax and make sense of it. Since `Path` *by definition* cannot be structurally different from `[u8]`, it isn't actually creating anything new on top of the reference. So, unlike a normal function, `Path` cannot shorten the lifetime of `*s`, and the resulting reference will have the same lifetime as `s`.

This is called "ref-wrapping," since we're wrapping a referenced value with another type. This is allowed for `#[repr(transparent)]` structs and enums, and it even works if the struct or enum involves other, zero-sized values:

```rust
use std::marker::PhantomData;

pub struct WeirdType<T> {
    marker: PhantomData<T>,
    id: str,
}

impl<T> WeirdType<T> {
    pub fn from_str(s: &str) -> &WeirdType<T> {
        &WeirdType {
            marker: PhantomData,
            id: *s,
        }
    }
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

For simplicity, this section will use the term "`#[repr(transparent)]` literals" to indicate struct or enum literals of a `#[repr(transparent)]` type. Since the behaviour of `#[repr(transparent)]` unions is still unstable, this RFC will only describe the semantics of structs and enums, although discussion of unions will be left for the future possibilities section.

Additionally, the term "primary field" will be used when talking about `#[repr(transparent)]` types, and this will refer to the non-zero-sized field. Technically, `#[repr(transparent)]` structs and enums can also be zero-sized, but since these types can effectively be referenced and dereferenced at will, we'll leave them out of this description since it does nothing besides induce extra headaches and pedantry.

For the actual explanation: ref-wrapping allows `#[repr(transparent)]` literals to opportunistically inherit the lifetimes of the values in their primary field.

When a `#[repr(transparent)]` literal is constructed, the type checker will work as usual, ensuring that the actual value of the primary field matches what's expected. However, the borrow checker will instead look at the literal as if it were the value of the primary field directly, rather than being moved into the literal. And since the type is transparent, the actual construction of the literal will be removed when computing the result, as it is today.

When borrowing the literal, the same rules will apply for borrowing the field itself, e.g. you can't perform `&mut *value` if `value` is an immutable reference, etc. However, if `&*value` is allowed, then `&Transparent { primary_field: *value, .. }` is allowed, among similar constructs.

# Drawbacks
[drawbacks]: #drawbacks

* This is possible today with transmutes.
* This may complicate the compiler for something that may be a relatively niche use case.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The most obvious alternative is to do nothing, and to force people to use unsafe code to achieve this. I personally disagree that this is acceptable, but it is a genuine alternative.

The other alternative is to allow ref-wrapping via `as`-casting. For example, the `Path` example under this model could be done like the below:

```rust
fn from_bytes(s: &[u8]) -> &Path {
    value as &Path
}
```

This has a few problems:

1. The language, as it stands, is moving away from `as` casts in favour of generic traits like `From`, and it's unclear how this could be replaced under this model. Deriving `From` seems prone to a bunch of errors, and potentially an alternative syntax would have to be introduced.
2. This removes the semantics of constructing the type, including the values added to zero-sized fields. In effect, this is "revealing the hand" of the abstraction being a farce by not needing to construct the type. It could be argued that this is not a downside, but it feels worth including.
3. It's unclear whether this should be allowed for values in addition to references, e.g. could `struct Seconds(u32)` allow casting `0 as Seconds`?
4. It's less clear how this translates with privacy rules. Presumably, the privacy rules would be the same as this RFC's proposal, where it would only be allowed if the constructor is accessible, but that's still less clear than using the constructor directly.

# Prior art
[prior-art]: #prior-art

As far as this RFC is concerned, there really isn't much prior art for this. Discussions have been had about using `as`-casting for this purpose, although as mentioned in the alternatives section, that has downsides.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None currently, besides union semantics, which is separate from this RFC. (See future possibilities.)

# Future possibilities
[future-possibilities]: #future-possibilities

If `#[repr(transparent)]` unions retain their current semantics, then this RFC would transparently (pun intended) work for them for immutable references only. Specifically, if mutable references are allowed, this could permit a known soundness issue.

As an example, imagine that `&mut bool` is wrapped to `&mut MaybeUninit<bool>`. Now, the `MaybeUninit` reference can be written to contain any bit pattern, which could be observed under the old `&mut bool` reference, inducing undefined behaviour.

Ref-wrapping for immutable references could potentially also be useful for transparent union *variants*, if that ever were an option. For example, imagine the below union:

```rust
union Mixed {
    Float(f64),
    Int(u64),
}
```

Now, it would make complete sense to ref-wrap `&f64` via `&Mixed::Float(*value)`. However, this could potentially complicate things based upon how unions are padded and what aspects of that padding are meaningful, and such issues would have to be explored more thoroughly if this were to be accepted.
