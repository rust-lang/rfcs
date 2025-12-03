- Feature Name: complex-numbers
- Start Date: 2025-12-02
- RFC PR: [rust-lang/rfcs#3892](https://github.com/rust-lang/rfcs/pull/3892)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

## Summary
[summary]: #summary

FFI-compatible and calling-convention-compatible complex types are to be introduced into `core` to ensure synchronity with C primitives.

## Motivation
[motivation]: #motivation

The definition of complex numbers in the C99 standard defines the _memory layout_ of a complex number but not its _calling convention_. 
This makes crates like `num-complex` untenable for calling C FFI functions containing complex numbers without at least a level of indirection (`*const Complex`) or the like.
Only in `std` is it possible to make an additional repr to match the calling convention that C uses across FFI boundaries. 
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
  let returned_value = computes_function(Complex<f64>::new(3, 4))
}
```
using the standard library's FFI-compatible complex numbers.

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

`Complex<T>` numbers can be instantiated as of any type using `Complex::new(re, im)` where `re` and `im` are of the same type (this includes all numbers).
```rust
let x = Complex::new(3.0, 4.0); // this instantiates them as integers, not floats!
```
They can even be passed as an array:
```rust
let y = Complex::from([3.0, 4.0]);
```
or as a tuple:
```rust
let z = Complex::from((3.0, 4.0));
```
They can even be passed in polar form (but only as a float):
```rust
let polar = Complex::from_polar(3.0, f32::PI/2.0);
```
where .i() turns a real number into a complex one transposing the real value to a complex value.

They are added and multiplied as complexes are:
```rust
let first = Complex::new(1.0, 2.0);
let second = Complex::new(3.0, 4.0);
let added = first + second; // 4 + 6.i()
let multiplied = first * second; // -4 + 10.i()
```

They can be divided using normal floating-point division
```rust
let float_first = Complex::new(1.0, 2.0);
let float_second = Complex::new(3.0, 4.0);
let divided = float_second / float_first; // 2.4 - 0.2.i()
```

If the values are floating point, you can even calculate the complex sine, cosine and more:
```rust
let val = Complex::new(3.0, 4.0);
let sine_cmplx = csin(val); // 3.8537380379 - 27.016813258i
```
It's not too much of a problem to print them:
```rust
println!("{}", Complex::new(1.0, 2.0)); // prints 1 + 2i
```
If you want to call certain C libraries with complex numbers, you use this type:
```C
// in the C library
extern double _Complex computes_function(double _Complex x);
```
```rust
// in YOUR Rust code
extern "C" {
  fn computes_function(x: Complex<f64>) -> Complex<f64>;
}
fn main() {
  let returned_value = computes_function(Complex::<f64>::new(3, 4))
}
```

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Complex numbers will be implemented by using traits in the `core` crate:
```
trait Float: Copy + Clone {}
impl Float for f32 {}
impl Float for f64 {}
```
Calls to some `libgcc` functions will also be needed:
```rust
#[link(name="libgcc")]
unsafe extern "C" {
  fn mulsc3(a: f32, b: f32, c: f32, d: f32);
  fn divsc3(a: f32, b: f32, c: f32, d: f32);
  fn muldc3(a: f64, b: f64, c: f64, d: f64);
  fn divdc3(a: f64, b: f64, c: f64, d: f64);
}
```
to properly classify all types complex numbers can be implemented on.
They will have an internal representation of a Tx2 array:
```rust
// in core::complex
#[lang = "complex"] // For matching the calling convention (special repr needed?)
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Complex<T: Float>([T; 2]);
```
have construction methods and `From` impls:
```rust
impl Complex<T> {
  fn new(re: T, im: T) {
    Complex([re, im])
  }
}

impl<T: Float> From<(T, T)> for Complex<T> {
  fn from(value: (T, T)) {
    Complex(value.0, value.1)
  }
}
impl<T: Float> From<(T, T)> for Complex<T> {
  fn from(value: [T; 2]) {
    Complex(value[0], value[1])
  }
}
```

have methods to calculate their real and imaginary part (`.re()` and `.im()`):
```rust
impl<T: Float> Complex<T> {
  fn re(self) {
    self.0[0]
  }
  fn im(self) {
    self.0[1]
  }
}
```
polar conversions:
```rust
impl<T: Float + Mul + Add> Complex<T> {
  fn modulus(self) {
    (self.0 * self.0) + (other.0 * other.0)
  }
}

impl Complex<f32> {
  fn angle(self) {
    f32::atan2(self.re(), self.im())
  }
}

impl Complex<f64> {
  fn angle(self) {
    f32::atan2(self.re(), self.im())
  }
}
```
and have arithmetic implementations similar to this:
```rust
impl<T: Add + Float> Add for Complex<T> {
  fn add(self, other: Self) {
    Complex::new(self.0.re() + other.0.re(), self.0.im() + other.0.im())
  }
}
impl<T: Add + Float> Add<T> for Complex<T> {
  fn add(self, other: T) {
    self + Complex::new(other, 0)
  }
}
impl<T: Add + Float> Add<Complex<T>> for T {
  fn add(self, other: Complex<Self>) {
    Complex::new(self, 0) + other
  }
}
impl<T: Sub + Float> Sub for Complex<T> {
  fn sub(self, other: Self) {
    Complex::new(self.0.re() - other.0.re(), self.0.im() - other.0.im())
  }
}
impl<T: Sub + Float> Sub<T> for Complex<T> {
  fn sub(self, other: T) {
    self - Complex::new(other, 0)
  }
}
impl<T: Sub + Float> Sub<Complex<T>> for T {
  fn sub(self, other: Complex<Self>) {
    Complex::new(self, 0) - other
  }
}
impl Mul for Complex<f32> {
  fn mul(self, other: Self) {
    __mulsc3(self.re(), self.im(), other.re(), other.im())
  }
}
impl Mul for Complex<f64> {
  fn mul(self, other: Self) {
    __muldc3(self.re(), self.im(), other.re(), other.im())
  }
}
impl Mul<f32> for Complex<f32> {
  fn mul(self, other: T) {
    self * Complex::new(other, 0);
  }
}
impl Mul<f64> for Complex<f64> {
  fn mul(self, other: T) {
    self * Complex::new(other, 0);
  }
}
impl Mul<Complex<f32>> for f32 {
  fn mul(self, other: Complex<Self>) {
    Complex::new(self, 0) * other
  }
}
impl Mul<Complex<f64>> for f64 {
  fn mul(self, other: Complex<Self>) {
    Complex::new(self, 0) * other
  }
}
impl Div for Complex<f32> {
  fn Div(self, other: Self) {
    __divsc3(self.re(), self.im(), other.re(), other.im())
  }
}
impl Div for Complex<f64> {
  fn Div(self, other: Self) {
    __divdc3(self.re(), self.im(), other.re(), other.im())
  }
}
impl Div<f32> for Complex<f32> {
  fn div(self, other: T) {
    self / Complex::new(other, 0);
  }
}
impl Div<f64> for Complex<f64> {
  fn div(self, other: T) {
    self / Complex::new(other, 0);
  }
}
impl Div<Complex<f32>> for f32 {
  fn div(self, other: Complex<Self>) {
    Complex::new(self, 0) / other
  }
}
impl Div<Complex<f64>> for f64 {
  fn div(self, other: Complex<Self>) {
    Complex::new(self, 0) / other
  }
}
```
The floating point numbers shall have sine and cosine and tangent functions, their inverses, their hyperbolic variants, and their inverses defined as per the C standard and with Infinity and Nan values defined as per the C standard.
## Drawbacks
[drawbacks]: #drawbacks

If there is suddenly a standard-library Complex type, people may rush to include it in their current implementations, which would leave people behind if they didn't know about it. I really don't think this is a drawback though, since similar things have happened in Rust before: the inclusion of `OnceCell` in Rust, for example.
Also, the multiple emitted calls to `libgcc.so` (`__mulsc3` and the like) may cause a bit of overhead and may not be what the Rust lang team and compiler team want.

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The rationale for this type is mostly FFI: C libraries that may be linked from Rust code currently cannot provide functions with direct struct implementations of Complex - they must be hidden under at least a layer of indirection. However, it is not always possible to write a C complex-valued function that wraps the first function in a pointer. Thus, FFI becomes a problem if such complex-valued functions are passed by value and not by reference.

### Alternatives:
- Don't do this: There are, obviously, millions of alternatives on crates.io, the foremost being `num-complex`. However, I believe that if we wish to support proper FFI with C, then a standard type that matches calling conventions with C complex numbers is an important feature of the language. Hence, I do not recommend this idea.
- Use a polar layout: Polar complex numbers, are undoubtedly a more optimal solution for multiplying complexes. However, I believe that if we wish to have proper FFI with C, then complex number layout should be chosen in accordance with the layout that is used in the C standard, and that is the orthogonal layout. This is also the layout used by most of other languages and crates on crates.io.
- Non-generic primitive types: These are, obviously, the most obvious and practical solution. However, if we implemented lots of such types, then we would not be able to expand for `f16` and `f128` support without repeating the code already implemented. It would be extremely repetitive and tedious to add new types, especially since Gaussian integers and other floating points could have added support.

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
printf("%.1f%+.1fi\n", creal(cmplx * two_cmplx), cimag(cmplx * two_cmplx)
Even in Rust, it has been discussed two times in IRLO:
- [First discussion](https://internals.rust-lang.org/t/c-compatible-complex-types-using-traits/13757)
- [Second discussion](https://internals.rust-lang.org/t/standard-complex-number-in-std-library/23748)

Many crates, like `num-complex` also provide this feature, though it is not FFI-safe.
## Unresolved questions
[unresolved-questions]: #unresolved-questions

Should this type be in `core::ffi`? This type's purpose is mostly FFI, but it might be useful in library contexts as well, so I am not sure if we should place it in `core::ffi`.

## Future possibilities
[future-possibilities]: #future-possibilities

- Maybe later on, we can think of adding a special custom suffix for complex numbers (`1+2j` for example), and using that as a simpler way of writing complex numbers if this RFC is accepted? This is very similar to how most languages implement complex numbers? Or perhaps we could consider a constant:
```rust
const I: T = Complex::new(T::zero(), T::one());
```
where `zero` and `one` is implemented on a trait similar to `num_traits`?
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
- Should we support Imaginary eventually? This RFC doesn't cover it, but I think we can do this later in another RFC.
- Eventually we may support Gaussian integers (an extension of the real integers) which have a Euclidean division procedure with remainder. We could theoretically eventually support these integers?
- We can also support f16 and f128 once methods for them are stabilised.
- We should also consider adding aliases (like c32 and c64) for floating points once they are established, to allow for a shorthand syntax.
