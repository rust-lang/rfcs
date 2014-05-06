- Start Date: 2014-04-29
- RFC PR #:
- Rust Issue #:

# Summary

Allow generics to have static values as parameters in addition to lifetime and
type parameters.

# Motivation

Generic types are very useful to ensure type-safety while writing generic code
for a variety of types. Parametrisation over static values extends this feature
to certain use cases, where it is necessary to maintain type-safety in terms of
these values.

To illustrate this further, consider the following two use cases as examples:

* *Algebraic types*: algebraic vectors and matrices generally have a certain
  dimensionality, which changes their behaviour. For example, it does not make
  sense to add a 2-vector with a 3-vector or multiply a 3x4 matrix by a
  5-vector. But the algorithms can be written generically in terms of the
  dimensionality of these types.
* *Physical units*: In science one often deals with numerical quantities
  equipped with units (meters, hours, kilometers per hour, etc.). To avoid
  errors dealing with these units, it makes sense to include in the data type.
  For performance reasons, however, having the computational overhead in every
  single calculation at run-time might be prohibitively slow. Here static
  values as generic parameters could allow to convert between units and check
  for consistency at compile-time.

# Drawbacks

First of all, allowing another kind of parameter for generics adds a certain
degree of complexity to Rust. Thus the potential merits of this feature have to
justify this.

Furthermore, it is not entirely clear, how well this feature fits with Rust's
current approach to meta-programming. When implemented completely, this feature
requires compile-time function execution (CTFE), which has been
[discussed][issue_11621] [in the past][ctfe_mail] without a clear outcome. This
feature would also introduce the possibility for
[template metaprogramming][template_meta].

# Detailed design

Currently generics can be parametrized over type and lifetime parameters. This
RFC proposes to additionally allow for static values as parameters. These
values must be static, since they encode type-information that must be known at
compile-time.

To propose a concrete syntax, consider this simple generic function:

```rust
fn add_n<n: int>(x: int) -> int {
    x + n
}

fn main() {
    add_n<3>(4); // => 7
}

```

The syntax `<n: int>` closely resembles the syntax for type parameters with
trait bounds. Traits would probably not be allowed as the type of a static
parameter, since they could not be statically resolved at compile-time.
Therefore the parser should be able to distinguish type parameters from static
value parameters despite the similarity. However, one could also annotate the
parameter in some way to differentiate it more clearly from type parameters.

Structs could be parametrized similarly, as this (incomplete) implementation of
an arbitrarily-sized algebraic vector illustrates:

```rust
struct Vector<T, n: uint> {
    pub data: [T, ..n]
}

impl<T, n: uint> Vector<T, n> {
    fn new(data: [T, ..n]]) -> Vector<T, n> {
        Vector{data: data}
    }
}

impl<T: Add, n: uint> Add<Vector<T, n>, Vector<T, n>> for Vector<T, n> {
    fn add(&self, rhs: &Vector<T, n>) -> Vector<T, n> {
        let mut new_data: [T, ..n] = [0, ..n];
        for (i, (&x, &y)) in self.data.iter().zip(rhs.data.iter()).enumerate() {
            new_data[i] = x + y;
        }
        Vector::new(new_data)
    }
}

fn main() {
    assert_eq!(
        Vector::new([1, 2]) + Vector([2, 3]),
        Vector::new([3, 4])
    );
    assert_eq!(
        Vector::new([1, 2, 3]) + Vector([2, 3, 7]),
        Vector::new([3, 4, 5])
    );
}

```

It should also be possible to do some algebra with the parameters, like this:

```rust
fn concatenate<T, n: uint, m: uint>
    (x: Vector<T, n>, y: Vector<T, m>) -> Vector<T, n + m>
{
    let mut new_data: [T, ..n + m] = [0, ..n + m];
    for (i, &xx) in x.data.iter().enumerate() { new_data[i] = xx; }
    for (i, &yy) in y.data.iter().enumerate() { new_data[i + n] = yy; }
    Vector::new(new_data)
}

```

# Alternatives

Parts of the functionality provided by this change could be achieved using
macros, which is currently done in some libraries (see for example in
[Servo][servo_macros] or @sebcrozet's libraries [nalgebra][nalgebra] and
[nphysics][nphysics]. However, macros are fairly limited in this regard.

# Unresolved questions

* How does type inference work in this context?
* In how far is compile-time function execution acceptable to support this?
* How exactly does this work with traits and enums?


[nalgebra]: https://github.com/sebcrozet/nalgebra
[nphysics]: https://github.com/sebcrozet/nphysics
[issue_11621]: https://github.com/mozilla/rust/issues/11621
[ctfe_mail]: https://mail.mozilla.org/pipermail/rust-dev/2014-January/008252.html
[template_meta]: http://en.wikipedia.org/wiki/Template_metaprogramming
