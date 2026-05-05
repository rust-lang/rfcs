- Feature Name: `#[target_feature(..)]` In Trait Methods
- Start Date: 2020-12-25
- RFC PR: none yet
- Rust Issue: none yet

[RFC 2045 (`target_feature`)]: https://github.com/rust-lang/rfcs/pull/2045
[RFC 2396 (`target feature 1.1`)]: https://github.com/rust-lang/rfcs/pull/2396

# Summary
[summary]: #summary

This RFC builds on [RFC 2045 (`target_feature`)] and [RFC 2396 (`target feature 1.1`)] by:
* Allowing trait methods to have `#[target_feature(..)]` attributes
* Expanding target_feature 1.1's relaxed safety rules to include calls to and from the newly-allowed `#[target_feature(..)]` trait methods.

# Motivation
[motivation]: #motivation

Imagine we're writing an AVX library with the goal of being generic over f32 and f64.  We can start by creating a trait
called `AvxVector` with some methods common to both f32 and f64 vectors:

```rust
pub trait AvxVector {
    unsafe fn add(left: Self, right: Self) -> Self;
}
impl AvxVector for __m256 {
    unsafe fn add(left: Self, right: Self) -> Self {
        _mm256_add_ps(left, right)
    }
}
impl AvxVector for __m256d {
    unsafe fn add(left: Self, right: Self) -> Self {
        _mm256_add_pd(left, right)
    }
}
```
The `RustFFT` crate implements a [real-world example of this](https://github.com/ejmahler/RustFFT/blob/66bb2a9825ad0f6e5583a71f55930706daa26425/src/avx/avx_vector.rs#L18)

Now, users of our library can compute a sum of f32 and f64 vectors with the same code:
```rust
#[target_feature(enable = "avx")]
unsafe fn do_work() {
    let left32 : __m256 = ...;
    let right32 : __m256 = ...;
    let sum32 = unsafe { AvxVector::add(left32, right32) }

    let left64 : __m256d = ...;
    let right64 : __m256d = ...;
    let sum64 = unsafe { AvxVector::add(left64, right64) }
}
```

Everything so far compiles on stable Rust, but there's a major flaw: Despite every single function being marked unsafe, absolutely nothing unsafe is happening here! [RFC 2396 (`target feature 1.1`)] improves the situation for standalone functions by allowing calls to a `#[target_feature(..)]` function to be safe as long as the caller also has `#[target_feature(..)]` with the same features.

However, that change doesn't apply here, because `#[target_feature(..)]` isn't allowed on trait methods. If it was allowed, this example could be written with no unsafe whatsoever.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Currently, we don't allow `#[target_feature]` to appear on trait methods:

```rust
pub trait AvxVector {
    #[target_feature(enable = "avx")]
    fn add(left: Self, right: Self) -> Self;
}
```

```rust
error: attribute should be applied to a function
 --> src\lib.rs:6:5
  |
6 |     #[target_feature(enable = "avx")]
  |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
7 |     fn add(left: Self, right: Self) -> Self;
  |     ---------------------------------------- not a function
```

With this RFC, the error above is no longer an error, and the trait definition comples successfully. To ensure consistency between trait definitions and trait impls, we require that the `#[target_feature]` declaration is identical between the trait definition and the trait impl. Some examples:

```rust
pub trait AvxVector {
    #[target_feature(enable = "avx")]
    fn add(left: Self, right: Self) -> Self;

    fn sub(left: Self, right: Self) -> Self;

    #[target_feature(enable = "avx")]
    fn mul(left: Self, right: Self) -> Self;

    #[target_feature(enable = "avx")]
    fn div(left: Self, right: Self) -> Self;
}

impl AvxVector for __m256 {
    // ERROR: AvxVector::add has a #[target_feature], but this impl doesn't
    fn add(left: Self, right: Self) -> Self {
        _mm256_add_ps(left, right)
    }

    // ERROR: AvxVector::sub doesn't have any target features, but this impl does
    #[target_feature(enable = "avx")]
    fn sub(left: Self, right: Self) -> Self {
        _mm256_sub_ps(left, right)
    }

    // ERROR: This impl's target features must exactly match AvxVector::mul's target features
    #[target_feature(enable = "avx", enable = "avx2")]
    fn mul(left: Self, right: Self) -> Self {
        _mm256_mul_ps(left, right)
    }

    // OK
    #[target_feature(enable = "avx")]
    fn div(left: Self, right: Self) -> Self {
        _mm256_div_ps(left, right)
    }
}
```

Calling trait methods with target features would work exactly like the relaxed rules in [RFC 2396 (`target feature 1.1`)]:
Safe `#[target_feature]` trait methods can be called _without_ an `unsafe {}`
block _only_ from functions and trait methods that have at least the exact same set of
`#[target_feature]`s. Some examples:


```rust
pub trait AvxVector {
    #[target_feature(enable = "avx")]
    fn add(left: Self, right: Self) -> Self;

    #[target_feature(enable = "avx", enable = "fma")]
    fn mul_add(left: Self, right: Self, add: Self) -> Self;
}

// This function does not have any target feature:
fn meow() {
    let abc1 : __m256 = ...;
    let abc2 : __m256 = ...;
    let abc3 : __m256 = ...;

    AvxVector::add(abc1, abc2); // ERROR (unsafe block required, because target features don't match)
    unsafe { AvxVector::add(abc1, abc2) }; // OK

    AvxVector::mul_add(abc1, abc2, abc3); // ERROR (unsafe block required, because target features don't match)
    unsafe { AvxVector::mul_add(abc1, abc2, abc3) }; // OK
}

// This function has the AVX target feature, but not the FMA target feature
#[target_feature(enable = "avx")]
fn bark() {
    let abc1 : __m256 = ...;
    let abc2 : __m256 = ...;
    let abc3 : __m256 = ...;

    AvxVector::add(abc1, abc2); // OK, because target features match

    AvxVector::mul_add(abc1, abc2, abc3); // ERROR (unsafe block required, because target features don't match)
    unsafe { AvxVector::mul_add(abc1, abc2, abc3) }; // OK
}

// This function has both AVX and FMA target features
#[target_feature(enable = "avx", enable = "fma")]
fn moo() {
    let abc1 : __m256 = ...;
    let abc2 : __m256 = ...;
    let abc3 : __m256 = ...;

    AvxVector::add(abc1, abc2); // OK, because target features match

    AvxVector::mul_add(abc1, abc2, abc3); // OK, because target features match
}
```

This change resolves the problem raised in the motivation, because moo() can now be implemented entirely in safe code.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This RFC proposes changes to the language with respect to [RFC 2045 (`target_feature`)] and [RFC 2396 (`target feature 1.1`)]:

* Allow trait methods to have `#[target_feature(..)]` attributes.
    * If a method in a trait definition has a `#[target_feature(..)]` attribute,
    all impls of that trait method must have exactly matching `#[target_feature(..)]` attributes.
    * If a method in a trait definition has no `#[target_feature(..)]` attribute, its impls may not have it either.
* Expand [RFC 2396 (`target feature 1.1`)]'s' relaxed safety rules to include calls to the newly-allowed `#[target_feature(..)]` trait methods: 
    * A safe function may only call a safe `#[target_feature(..)]` trait method without unsafe blocks if the calling function has a 
        `#[target_feature]` attribute with a superset of the trait method's features.

# Drawbacks
[drawbacks]: #drawbacks

Developers would be required to duplicate entries in target_feature attributes between trait methods and trait impls,
making refactoring more difficult.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

One obvious alternative is to remove the requirement for the trait impl to duplicate the `#[target_feature(..)]` attribute. Developers would have to repeat themselves less, leading to less friction when writing and editing code.

A drawback to this alternative is increased cognitive load: If you're editing one of these trait impls, you'll have to scroll to somewhere else in the file, or to another file, or to another crate entirely, in order to see what features the trait method enables. We're effectively expecting the developer to keep more information in their head while editing, which will result in more mistakes.

# Prior art
[prior-art]: #prior-art
I'm not aware of any prior art.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None yet.

# Future possibilities
[future-possibilities]: #future-possibilities

The overall goal of this RFC is to reduce the amount of unsafe code required when writing generic SIMD libraries. In RustFFT, once this RFC is merged and implemented, the main source of unsafe for SIMD code will be in loads/stores.

For example, we have a slice of `Complex<f32>`, and we want to load a __m256 from a particular index in that slice, which will result in an AVX vector containing 4 complex numbers. We could accomplish that with the following code:

```rust
#[target_feature(enable = "avx")]
fn load_complex(data: &[Complex<f32>], index: usize) -> __m256 {
    assert!(data.len() >= index + 4);
    unsafe { _mm256_loadu_ps(data.as_ptr().add(index) as *const f32) }
}
```

Thankfully, this is already pretty safe: num_complex::Complex is repr(C), so the pointer cast is valid, we assert that the slice is long enough, and the unsafety can be completely isolated within this function.

However, there's another kind of load that's extremely unsafe: AVX2 gather. `_mm256_i64gather_pd` takes a pointer and an __m256i containing arbitrary 64-bit indexes, offsets the pointer by those indexes, and loads a f64 from each pointer offset. How in the world can we wrap this in a safe API, in such a way that the unsafety is completely isolated?

One possibility is the following:
1: Broadcast the slice length into a __m256i
2: Use cmpgt + movemask to verify that all of the indexes are less than len
3: The result of cmpgt + movemask will be -1 if all indexes are less than len, so assert that the result is -1
4: We now know that the gather is safe
```rust
#[target_feature(enable = "avx", enable = "avx2")]
fn safe_gather(data: &[f64], indexes: __m256i) -> __m256d {
    // create an AVX vector containing the length of the slice repeated over and over
    let len = _mm256_set1_epi64x(data.len() as i64);

    // Compare `len` with `indexes`. If every entry in `len` is greater than its corresponding element in `indexes`, comparison_mask will be -1
    let comparison_mask = _mm256_movemask_epi8(_mm256_cmpgt_epi64(len, indexes));
    assert_eq!(comparison_mask, -1);

    // We now know that every index is in range
    _mm256_i64gather_pd(data.as_ptr(), indexes, 8)
}
```

This is safe, but unlike normal bounds checking, I have a hard time believing that the compiler would be able to elide it. At the very least, I suppose the compiler would be able to lift the `_mm256_set1_epi64x` out of the inner loop. Is there a better way to do safe gathers? One that allows for elision, or lifting more of the instructions out of the inner loop?
