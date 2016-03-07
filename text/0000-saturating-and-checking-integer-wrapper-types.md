- Feature Name: saturating-and-checking-integer-wrapper-types
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Implement two wrapper-types in `std::num` which provide two different types of behavior on overflow:
saturating arithmetic and always checked arithmetic with the latter signalling a thread panic on
overflow.

# Motivation
[motivation]: #motivation

* Currently the only wrapper type, `Wrapping<T>`, provides wrapping semantics on the basic operators
  but saturating or checked semantics are obtained through using methods directly on the primitive
  type.  `Saturating<T>` and `Checked<T>` types would improve the ergonomics of using saturating and
  checked arithmetic.
* Firefox media team wants to have a `Checked<T>` type which will panic on overflow in release mode
  which they can use in non-performance critical code. Currently writing checked code which causes
  a thread panic is far from ergonomic.
* `Saturating<T>` would provide defined saturating behavior for division, remainder and negation
  as well as left- and right-shift operations. Currently only addition, subtraction and
  multiplication are implemented for the primitive types in the form of the `saturating_*` methods.
* Improved performance can potentially be obtained for some of the operations by using intrinsics
  which would not be possible to do in a stable crate at the moment.

# Detailed design
[design]: #detailed-design

This proposal suggests two additional types alongside the intentionally wrapping wrapper-type
`Wrapping<T>`: `Saturating<T>` and `Checked<T>`.

The two types will implement the same traits as `Wrapping<T>`. Below `W<T>` is the wrapper-type
(`Wrapping<T>`, `Saturating<T>` or `Checked<T>`) and `T` is the wrapped integer primitive:

```rust
#[derive(Debug, Default, Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
pub struct W<T>(pub T);

impl Add<W<T>>    for W<T> { type Output = W<T>; ... }
impl Sub<W<T>>    for W<T> { type Output = W<T>; ... }
impl Mul<W<T>>    for W<T> { type Output = W<T>; ... }
impl Div<W<T>>    for W<T> { type Output = W<T>; ... }
impl Rem<W<T>>    for W<T> { type Output = W<T>; ... }

// Only for signed `T`
impl Neg          for W<T> { type Output = W<T>; ... }

impl Not          for W<T> { type Output = W<T>; ... }
impl BitXor<W<T>> for W<T> { type Output = W<T>; ... }
impl BitOr<W<T>>  for W<T> { type Output = W<T>; ... }
impl BitAnd<W<T>> for W<T> { type Output = W<T>; ... }

impl Shl<usize>   for W<T> { type Output = W<T>; ... }
impl Shr<usize>   for W<T> { type Output = W<T>; ... }

impl AddAssign<W<T>>    for W<T> { ... }
impl SubAssign<W<T>>    for W<T> { ... }
impl MulAssign<W<T>>    for W<T> { ... }
impl DivAssign<W<T>>    for W<T> { ... }
impl RemAssign<W<T>>    for W<T> { ... }
impl ShlAssign<usize>   for W<T> { ... }
impl ShrAssign<usize>   for W<T> { ... }
impl BitXorAssign<W<T>> for W<T> { ... }
impl BitOrAssign<W<T>>  for W<T> { ... }
impl BitAndAssign<W<T>> for W<T> { ... }
```

The `*Assign` trait implementations will perform the requested operation with the same semantics
as the base implementation (eg. `AddAssign` will perform an addition using `Add` and assign the
result to the left hand side).

## `Checked<T>`
[checked]: #checked

The `Checked<T>` wrapper type should provide checked operations for `+`, `-`, `*`, unary `-`, `/`,
`%`, `<<` and `>>` which panic in the case of overflow like the primitive types do in debug-mode.
Unlike the operations on the primitive types this checked arithmetic should be kept in release mode,
preserving the semantics of panic on overflow.

Bitwise operations will be forwarded to the wrapped type.

## `Saturating<T>`
[saturating]: #saturating

```rust
assert_eq!(Saturating(255u8) + Saturating(1), Saturating(255u8));
assert_eq!(Saturating(128u8) << 1, Saturating(255u8));
```

* The operators `+`, `-` and `*` saturate to `MAX` and `MIN` values for both signed and unsigned
  integers.

* `/` and `%` cannot overflow on unsigned integers. For signed integers `MIN / -1` and `MIN % -1`
  can overflow since signed types follow two's complement: `-1 * MIN = MAX + 1` and `MIN` is
  always even. The proposed results are `MIN / -1 = MAX` and `MIN % -1 = MAX`.

* Unary `-` (negation) of `MIN` in signed integers should saturate to `MAX` in the case of `-MIN`.

* Bitshift operators (`<<` and `>>`) saturate to `MAX` and `MIN` in the case of overflow of
  unsigned integers.  For signed non-zero positive integers `<<` saturate to `MAX` and `>>`
  saturate to `0`. For signed non-zero negative integers `<<` saturate to `MIN` and `>>` saturate
  to `-1`. Zero cannot saturate.

* Bitwise operators operate directly on the wrapped value, just as `Wrapping<T>`.

# Drawbacks
[drawbacks]: #drawbacks

* Two additional wrapper-types to maintain in the standard library
* The original numeric values still need to be lifted to the desired wrapping type (see
  [ergonomics-of-wrapping-operations](https://internals.rust-lang.org/t/ergonomics-of-wrapping-operations/1756)
  for a discussion about the ergonomics of `Wrapping<T>` and `wrapping_*` methods of primitive
  integer types). This is more of a question about the semantics of `T + W<T>` and `W<T> + T`
  which is not defined at the moment and not a part of this RFC.
* Interest for alternative behavior on overflow might not warrant additional wrapper-types in
  stdlib.

# Alternatives
[alternatives]: #alternatives

## Do nothing

Instead of `Checked<T>` and `Saturating<T>`, use the `checked_*` and `saturating_*` operations
which are provided as inherent methods on the primitive integer types.

This can result in excessively verbose code for calculations, especially in the case of
checked arithmetic:

```rust
let a = 5;
let b = 4;
a.checked_add(b).unwrap().checked_mul(3).unwrap()
a.checked_add(b.checked_mul(3).unwrap()).unwrap()
// vs
let a = Checked(5);
let b = Checked(4);
(a + b) * Checked(3)
a + b * Checked(3)

let a = 5;
let b = 4;
a.saturating_add(b).saturating_mul(3)
a.saturating_add(b.saturating_mul(3))
// vs
let a = Saturating(5);
let b = Saturating(4);
(a + b) * Saturating(3)
a + b * Saturating(3)
```

## Implement `Checked<T>` and `Saturating<T>` in an external library.

Additional dependency for a somewhat basic feature which already partially exists in stdlib
(through the `Wrapping<T>` type).

Performance can also be a concern since intrinsics cannot be used for the implementation.

## Implement checked and saturating arithmetic as separate operators

This has been proposed for wrapping operators and the conclusion was:

> Reasons this was not pursued: New, strange operators would pose an entrance barrier to the
> language. The use cases for wraparound semantics are not common enough to warrant having a
> separate set of symbolic operators.

See: [RFC #0560](https://github.com/rust-lang/rfcs/blob/master/text/0560-integer-overflow.md)

## Scoped attributes

This was also [proposed for wrapping operators](https://github.com/rust-lang/rfcs/pull/146),
but failed when the proposal for [checked arithmetic in debug mode](https://github.com/rust-lang/rfcs/pull/560)
was proposed. The latter also introduced `Wrapping<T>`.

# Unresolved questions
[unresolved]: #unresolved-questions

* The saturation behavior of `/` and `%` on signed integers with the parameters `MIN` and `-1`.

  Since they both evaluate to `MAX + 1` according to two's complement there is an argument to be
  made that they should saturate to `MAX`. The primitive signed integers keep their thread panic
  for the parameters `MIN` and `-1` for division and remainder, while multiplication wraps.
  Keeping in line with this behavior could be desired.

* The saturation behavior of `>>` on signed integers. Currently this is specified to saturate to
  `-1` since that is the right-shift behavior according to two's complement (ie. keep the sign bit).
