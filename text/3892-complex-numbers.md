- Feature Name: `complex_numbers`
- Start Date: 2025-12-02
- RFC PR: [rust-lang/rfcs#3892](https://github.com/rust-lang/rfcs/pull/3892)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

## Summary
[summary]: #summary

FFI-compatible and calling-convention-compatible complex types are to be introduced into `core` to ensure synchronity with C primitives.

## Motivation
[motivation]: #motivation

The C standard defines the _memory layout_ of a complex number, but not their _calling convention_. 
This means crates like `num-complex` require workarounds to interface with FFI using `_Complex`, and cannot pass values directly.
The addition of complex numbers to Rust as a lang-item ensures a correct calling convention consistent with C on all platforms, thus better allowing C interop.

In essence, this RFC makes code like this:
```C
extern double _Complex computes_function(double _Complex x);
```
callable in Rust without indirection:
```rust
extern "C" {
  fn computes_function(x: Complex<f64>) -> Complex<f64>;
}
fn main() {
  let returned_value = computes_function(Complex::<f64>::new(3.0, 4.0))
}
```
using the standard library's FFI-compatible complex numbers.

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation
`Complex<T>` numbers are in core::num and reexported in std::num, like `use core::num::Complex` or `use std::num::Complex`
`Complex<T>` numbers can be instantiated with any component type using `Complex::new(re, im)` where `re` and `im` are of the same type ( includes all numbers).
```rust
let x = Complex::new(3.0, 4.0);
```

Simple arithmetic is supported:

```rust
let first = Complex::new(1.0, 2.0);
let second = Complex::new(3.0, 4.0);
let a = first + second; // 4 + 6i
let b = first - second; // -2 - 2i
let c = first * second; // -5 + 10i
let d = float_second / float_first; // 0.44 - 0.8i
```

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The `core` crate will provide implementations for operator traits for possible component types.
They will have an internal representation similar to this (with public fields for real and imaginary parts):
```rust
// in core::num::complex, which would be a private module holding complex types
#[lang = "complex"] // for calling convention.
#[repr(C)]
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Complex<T> {pub re: T, pub im: T};
```
have a constructor
```rust
impl Complex<T> {
  fn new(re: T, im: T) -> Self;
}
```
and have simple arithmetic implementations supported:
```rust
impl<T: Add> Add for Complex<T> { type Output = Self; /* ... */ }
impl<T: Sub> Sub for Complex<T> { type Output = Self; /* ... */ }

impl Mul for Complex<T> where T: Add + Sub + Mul{ type Output = Self; /* ... */ }
impl Div for Complex<T> where Complex<T>: Div<T> { type Output = Self; /* ... */ }
```
## Drawbacks
[drawbacks]: #drawbacks

The multiple emitted calls to `libgcc.so` (`__mulsc3` and the like) via compiler-builtins may cause a bit of overhead and may not be what the Rust lang team and compiler team want.

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The rationale for this type is mostly FFI: C libraries that may be linked from Rust code currently cannot provide functions with direct struct implementations of Complex - they must be hidden under at least a layer of indirection. This is because of the undefined calling convention of complex numbers in C. For example: on powerpc64-linux-gnu, [returning double _Complex doesn't do the same thing as returning a struct with a field of type double[2].](https://gcc.godbolt.org/z/hh7zYcnK6) However, it is not always possible to write a C complex-valued function that wraps the first function in a pointer. Thus, FFI becomes a problem if such complex-valued functions are passed by value and not by reference.  

Additionally, this provides a unified API for complex numbers. Right now, many crates define their own complex types, making interoperability complicated, even though `num-complex` already exports its own type. (`rug::Complex` being an example)
You could theoretically do something like this:
```c
double _Complex function(double _Complex value);
void wrapper_function(double _Complex* value, double _Complex* out) {
    *out = function(*value);
}
```
for all functions you wish for. But this still needs to happen in C.

### Alternatives:
- Don't do this: There are, obviously, millions of alternatives on crates.io, the foremost being `num-complex`. However, I believe that if we wish to support proper FFI with C, then a standard type that matches calling conventions with C complex numbers is an important feature of the language. Hence, I do not recommend this idea.
- Use a polar layout: Polar complex numbers are undoubtedly a more optimal solution for multiplying complexes. However, I believe that if we wish to have proper FFI with C, then complex number layout should be chosen in accordance with the layout that is used in the C standard, and that is the orthogonal layout. This is also the layout used by most other languages and other crates on crates.io. Additionally, the polar form suffers from many structual issues: it is not a "natural" form for expressing complex numbers in computers - you cannot express pi exactly, so you cannot use radians for angle units. Moreover, polar complex numbers do not have a unique representation for each number - it has an infinity of zeros with all possible angles. The final problem, and in my opinion the most fatal, is the complexity of addition:

$\left(r_1\angle\theta_1\right) + \left(r_2\angle\theta_2\right) = \left(\sqrt{r_1^2+r_2^2+2r_1r_2\cos(\theta_1-\theta_2)}\right)\angle(atan2({r_1\sin\theta_1+r_2\sin\theta_2},{r_1\cos\theta_1+r_2\cos\theta_2}))$

which offsets any benefits that multiplication may bring.
- Non-generic primitive types: These are, obviously, the most obvious and practical solution. However, if we implemented lots of such types, then we would not be able to expand for `f16` and `f128` support without repeating the code already implemented. It would be extremely repetitive and tedious to document new types and their behavior, even if we used macros to generate implementations
- Only in `std::ffi`: Many suggestions have been given that `Complex` remains a type in only `std::ffi`. However, these miss a key point of the RFC: this addition is also about creating a unified interface for complex number support in std itself, and making it an FFI type would go against that.
## Prior art
[prior-art]: #prior-art

FORTRAN, C, C++, Go, Perl and Python all have complex types implemented in the standard library or as a primitive. This clearly appears to be an important feature many languages have.
For example, in Python:
```py
complex_num = 1 + 2j
complex_second = 3 + 4j
print(complex_num * complex_second)
```
or in C:
```c
float _Complex cmplx = 1 + 2*I;
float _Complex two_cmplx = 3 + 4*I;
printf("%.1f%+.1fi\n", creal(cmplx * two_cmplx), cimag(cmplx * two_cmplx));
```
Even in Rust, it has been discussed two times in IRLO:
- [First discussion](https://internals.rust-lang.org/t/c-compatible-complex-types-using-traits/13757)
- [Second discussion](https://internals.rust-lang.org/t/standard-complex-number-in-std-library/23748)

Many crates, like `num-complex` also provide this feature, though it is not FFI-safe.
## Unresolved questions
[unresolved-questions]: #unresolved-questions


## Future possibilities
[future-possibilities]: #future-possibilities

- Maybe later on, we can think of adding a special custom suffix for complex numbers (`1+2j` for example), and using that as a simpler way of writing complex numbers if this RFC is accepted? This is very similar to how most languages implement complex numbers? Or perhaps we could consider a constant:
```rust
impl<T: Float> Complex<T: Float> {
  const I: T = Complex::new(T::zero(), T::one());
}
```
where `zero` and `one` is implemented on the `Float` trait similar to `num_traits`?
Or maybe we could have a method on normal numbers:
```rust
// for example
impl f32 {
  fn i(self) -> Complex<f32> {
    Complex::new(0, self)
  }
}
```
that could help simplify the life of people who otherwise would have to keep writing `Complex::new()`?
- Arithmetic operations for primitives with complexes? (E.g. `1+Complex::new(0, 2)`). This goes hand in hand with the previous suggestion, so if we choose to implement this we should implement it with the previous suggestion.
- Should we support Imaginary eventually? This RFC doesn't cover it, but I think we can do this later in another RFC.
- Eventually we may support Gaussian integers (an extension of the real integers) which have a Euclidean division procedure with remainder. GCC has these, and we could theoretically eventually support these integers alongside GCC FFI.
- We can also support f16 and f128 once methods for them are stabilised. 
- We should also think about a `Display` implementation. Should we support something like `1 + 2i` or something else? Should we not make a `Display` impl at all, and just use re() and im() for the implementation?
- We should also consider adding aliases (like c32 and c64) for floating points once they are established, to allow for a shorthand syntax.
- Eventually, we should also consider adding polar conversions (e.g, `modulus` and `angle`)
- And also, we should consider adding complex trig functions (`csin`, `ccos`, etc.) that were deliberately left out of the MVP.
- Compatibility with other `core::num` types (`NonZero`, `Saturating`, `Wrapping`)
