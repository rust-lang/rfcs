- Feature Name: `as_keyword_consider_into_trait`
- Start Date: 2018-01-21
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

Permit to use the common `as` keyword with any type that implement the `Into` Trait,
allowing explicit conversions whose primitives already benefit, more visible than simple function calls.

# Motivation
[motivation]: #motivation

Many operators are allowed to be implemented permitting a much better programming flow:
- the `+`, `-`, `*`, `\` operators are the result of the `Add`, `Sub`, `Mul`, `Div` Traits implementations.
- the `+=`, `-=`, `*=`, `\=` operators are the result of the `AddAssign`, `SubAssign`, `MulAssign`, `DivAssign` Traits implementations.
- the `container[index]` operator is the result of the `Index` and `IndexMut` Traits implementations.
- and many others can be found on the [`std::ops`](https://doc.rust-lang.org/std/ops/) documentation page.

The `as` operator is actually reserved to primitives only but these given primitives already implement `Into` Traits for all possible conversions:
- the `u32` implement `Into<u64>` in the form of an [`impl From<u32> for u64`](https://doc.rust-lang.org/std/primitive.u64.html#impl-From%3Cu32%3E).
- the `u8` implement `Into<u16>` in the form of an [`impl From<u8> for u16`](https://doc.rust-lang.org/std/primitive.u16.html#impl-From%3Cu8%3E).

All of these conversions can be found in the [`libcore/num/mod.rs` file](https://github.com/rust-lang/rust/blob/d9d5c667d819ce400fc7adb09dcd6482b0aa519e/src/libcore/num/mod.rs#L3343-L3400).

Some special primitives like `usize` and `isize` doesn't implement many `From` Traits because these are [the representation of the address register](https://users.rust-lang.org/t/cant-convert-usize-to-u64/6243) and it can break compilation on some architectures.

Adding this feature to the compiler will probably unify the [actual conversion error detections](https://users.rust-lang.org/t/cant-convert-usize-to-u64/6243/8) of the `as` keyword and the `into` method.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The actual design is not far from what we already know of the `as` keyword.
It doesn't add new grammar to the language, it add more freedom to the actual syntax:

```rust
struct Foo(i32);

struct Bar(i32);

impl Into<Bar> for Foo {
    fn into(self) -> Bar {
        Bar(self.0)
    }
}

// this old syntax
let x = Foo(42);
let y: Bar = x.into();

// is equivalent to the new one
let x = Foo(42);
let y = x as Bar;
```

The actual syntax **is not deprecated in any way**, it has to mimic the `Add::add` or the `Deref::deref` methods and permit the user to directly call these methods or use the operators as he likes.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The `as` keyword is use to reflect the actual `Into::into` method call, if the `Into` Trait is implemented for the type to be converted, the compiler needs to emit an error informing the user that the conversion is not possible.

The actual `as` keyword is reserved for primitives to convert into other primitives but it shouldn't be a breaking change to change the behavior and to work with any type implementing `Into` because the primitives already implement these Traits.

The internal desing could be a simple syntax sugar to something like that:

```rust
// this actual syntax
let x = Foo(42);
let y = x as Bar;

// is a syntax sugar for
let x = Foo(42);
let y = Into::<Bar>::into(x);
```

# Drawbacks
[drawbacks]: #drawbacks

The `Into` Trait is not in the `std::ops` page and there is a reason why,
it's probably not considered overridable to be used on an operator/keyword like the `as` one.

The `as` keyword differ from the `Into::into` named method, it can be confusing.

# Rationale and alternatives
[alternatives]: #alternatives

With this design we doesn't introduce a new syntax, we just enlarge its possibilities.

The `as` keyword can be confusing with the `Into::into` named method,
we can add a new `into` keyword and/or deprecate the `as` one.

# Unresolved questions
[unresolved]: #unresolved-questions

What about `Copy` on the actual types ? Can it be a problem ?

Is the actual restriction on primitives due to the fact that they implement `Copy` and the conversion is low performance impact ?

How can we handle generic type conversions ?
Do we need to disallow them and only accpet non-generic ones ?

```rust
struct Foo<T>(T);

struct Bar<T>(T);

impl Into<Bar<i64>> for Foo<i32> {
    fn into(self) -> Bar<i64> {
        Bar(self.0 as i64)
    }
}

// this old syntax
let x = Foo(42_i32);
let y: Bar<i64> = x.into();

// is equivalent to the new one
let x = Foo(42);
let y = x as Bar<i64>;

// we can let the type inference guess the generic type
let x = Foo(42_i32);
let y: Bar<_> = x.into();

// equivalent to
let x = Foo(42);
let y = x as Bar<_>;
```
