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
extern double _Complex computes_function(x: double _Complex);
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
let x = Complex::new(3, 4); // this instantiates them as integers, not floats!
```
They can even be passed as an array:
```rust
let y = Complex::from_array([3, 4]);
```
or as a tuple:
```rust
let z = Complex::from_tuple((3, 4));
```
They can even be passed in polar form (but only as a float):
```rust
let polar = Complex::from_polar(3.0, f32::PI/2.0);
```
But the easiest way to usually instantiate them is by using them like this:
```rust
let easy = 1 + 2.i()
```
where .i() turns a real number into a complex one transposing the real value to a complex value.

They are added and multiplied as complexes are:
```rust
let first = 1 + 2.i()
let second = 3 + 4.i();
let added = first + second; // 4 + 6.i()
let multiplied = first * second; // -4 + 10.i()
```

They can be divided using Gaussian division (if integers) and normal division (if floats):
```rust
let first = 1 + 2.i();
let second = 3 + 4.i();
let divided = second / first; // 2 + 0.i()
let float_first = 1.0 + 2.0.i();
let float_second = 3.0 + 4.0.i();
let divided = float_second / float_first; // 2.4 - 0.2.i()
```

If the values are floating point, you can even calculate the complex sine, cosine and more:
```rust
let val = 3 + 4.i();
let sine_cmplx = csin(3+4.i()); // 3.8537380379 - 27.016813258i
```
It's not too much of a problem to print them:
```
println!("{}, 1 + 2.i()); // prints 1 + 2i
```
If you want to call certain C libraries with complex numbers, you use this type:
```C
// in the C library
extern double _Complex computes_function(x: double _Complex);
```
```rust
// in YOUR Rust code
extern "C" {
  fn computes_function(x: Complex<f64>) -> Complex<f64>;
}
fn main() {
  let returned_value = computes_function(Complex<f64>::new(3, 4))
}
```

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Complex numbers will be implemented by using public traits in the `core` crate that implement as an abstraction numeric features such as:
```
trait PrimNum: Add + Sub + Mul + Div + Neg + SimdElement + Copy + Clone {
  fn zero();
  fn one();
}
trait PrimFloat: PrimNum {
  fn sin();
  fn cos();
  fn tan();
  // etc!
}
```
to properly classify all types complex numbers can be implemented on.
They will have an internal representation of a Simd array to increase speed:
```rust
// in core::complex
#[lang = "complex"] // For matching the calling convention
#[repr(transparent)]
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Complex<T: PrimNum>(Simd<T, 2>);
```
have methods to calculate their real and imaginary part (`.re()` and `.im()`)
and have arithmetic implementations similar to this:
```rust
impl<T: PrimNum> Add for Complex<T> {
  fn add(self, other: Self) {
    Complex(self.0 + other.0)
  }
}
impl<T: PrimNum> Add<T> for Complex<T> {
  fn add(self, other: T) {
    Complex(self.0 + Simd::from_array(other)
  }
}
impl<T: PrimNum> Add<Complex<T>> for T {
  fn add(self, other: Complex<Self>) {
    Complex(Simd::from_array([self, 0]) + other.0)
  }
}
impl<T: PrimNum> Sub for Complex<T> {
  fn sub(self, other: Self) {
    Complex(self.0 + other.0)
  }
}
impl<T: PrimNum> Sub<T> for Complex<T> {
  fn add(self, other: T) {
    Complex(self.0 + Simd::from_array([other, 0])
  }
}
impl<T: PrimNum> Sub<Complex<T>> for T {
  fn add(self, other: Complex<Self>) {
    Complex(Simd::from_array([self, 0]) - other.0)
  }
}
impl<T: PrimNum> Mul for Complex<T> {
  fn mul(self, other: Self) {
    Complex(Simd::from_array([self.0.re() * other.0.re() - self.0.im() * other.0.im(), self.0.im() * other.0.re() + self.0.re() * other.0.im()]))
  }
}
impl<T: PrimNum> Mul<T> for Complex<T> {
  fn mul(self, other: T) {
    Complex(self.0 * Simd::from_array(other, other))
  }
}
impl<T: PrimNum> Mul<Complex<T>> for T {
  fn mul(self, other: Complex<Self>) {
    Complex(Simd::from_array(self, self) * other.0)
  }
}
impl<T: PrimNum> Div<T> for Complex<T> {
  fn div(self, other: T) {
    Complex(self.0 / Simd::from_array(other, other))
  }
}
impl<T: PrimNum> Div for Complex<T> {
  fn Div(self, other: Self) {
    (self * other.conj()) / (other.modulus() * other.modulus())
  }
}
```
The floating point numbers shall have sine and cosine and tangent functions, their inverses, their hyperbolic variants, and their inverses defined as per the mathematical definitions.
## Drawbacks
[drawbacks]: #drawbacks

There is only one drawback I can think of, and that is it might cause churn:
if there is suddenly a standard-library Complex type, people may rush to include it in their current implementations, which would leave people behind if they didn't know about it. I really don't think this is a drawback though, since similar things have happened in Rust before: the inclusion of `OnceCell` in Rust, for example.

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Some people on IRLO believe that complex numbers should be part of the std due to them being a language feature in many other languages including C, C++ and Go. They also mention that if Rust aims for good FFI with C, then complex numbers are an important feature.
### Alternatives:
Don't do this: There are, obviously, millions of alternatives on crates.io, the foremost being `num-complex`. However, I believe that if we wish to support proper FFI with C, then a standard type that matches calling conventions with C complex numbers is an important feature of the language. Hence, I do not recommend this idea.

## Prior art
[prior-art]: #prior-art

FORTRAN, C, C++, Go, Perl and Python all have complex types implemented in the standard library or as a primitive. This clearly appears to be an important feature many languages have.
Even in Rust, it has been discussed two times in IRLO:
- [First discussion](https://internals.rust-lang.org/t/c-compatible-complex-types-using-traits/13757)
- [Second discussion](https://internals.rust-lang.org/t/standard-complex-number-in-std-library/23748)

Many crates, like `num-complex` also provide this feature, though it is not FFI-safe.
## Unresolved questions
[unresolved-questions]: #unresolved-questions

Is this layout for the complex numbers fine? I just intended it to increase speed for `Add` and `Sub`, but I'm not entirely sure whether Simd arrays are needed for this.
## Future possibilities
[future-possibilities]: #future-possibilities

- Maybe later on, we can think of adding a special custom suffix for complex numbers (`1+2j` for example), and using that as a simpler way of writing complex numbers if this RFC is accepted? This is very similar to how most languages implement complex numbers.
- Should we support Imaginary eventually? This RFC doesn't cover it, but I think we can do this later in another RFC.
