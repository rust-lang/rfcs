- Start Date: 2014-07-19
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Add a new binary operator for exponentiation. This involves two additions to the
language:

* Addition of new syntax for exponentiation (`expr ** expr` or `expr ^^ expr`)
* Addition of a new lang item for the corresponding operator trait `Pow`

# Motivation

Exponentiation (raising to a power) is a common operation in numeric code that
can benefit from sugaring, and thus would make Rust more attactive for numeric
programmers. Additionally, exponentiation is a common feature of high-level
scripting languages (such as Ruby and Python) which would enamour Rust to those
communities as well.

# Detailed design

## Syntax

### Operator token

There are at least three reasonable tokens to use for exponentiation:

* `^` - This one is the most natural, and is used in numeric-focused software
    such as MATLAB/Octave, R, Mathematica, Maxima etc. This is not a good fit
    for Rust since `^` is currently used for the XOR operator. While changing
    XOR to be something else is in principle possible, this would be surprising
    to C/C++ programmers that switch to Rust.

* `**` - This is a common alternative used in languages where `^` is used for
    XOR like in Rust. This includes popular languages such as Ruby and Python.
    This choice is possible for Rust, but it currently conflicts with
    multiplication followed by dereferencing and multiple-dereferencing. The
    first case, `a**b`, can become a type error, requiring the user to
    disambiguate via parentheses: `a*(*b)`. The second case could be handled
    specially by the parser (i.e. it could treat unary `**` as a
    double-dereference). Overall, though, this token choice would not be
    backwards compatible.

* `^^` - This is rare, but not unprecedented. It is used in Haskell (in
    addition to `^` and `**`) and D. The D case is in particular relevant to
    Rust, as it has the same issue of dereferencing introducing ambiguity.
    Notably, since unary `^` is not valid in current Rust syntax, this token is
    fully backwards compatible.

### Precedence

The precedence should be above multiplication, but below casting. That is, the
following pair of assignments are equivalent:

~~~rust
let a = 1.0 * 2.0 ^^ 3.0 as f32;
let a = 1.0 * (2.0 ^^ (3.0 as f32));
~~~

## Trait and lang item

### Declaration

The trait and lang item will follow the pattern used by other operators. Here
is a possible declaration:

~~~rust
#[lang="pow"]
pub trait Pow<RHS,Result> {
    /// The method for the `^^` operator
    fn pow(&self, rhs: &RHS) -> Result;
}
~~~

The mechanism for the sugar will be identical to that of other binary operators.

### Implementation

The trait can be implemented alongside the other traits in `libcore` because
that's where the numerical traits live currently. In principle, this trait
merely needs access to the `pow` intrisics, so if those get moved out of
`libcore` for some reason, the implementation for this trait would follow.

For integral types, integration by squaring will be used for positive powers,
and underflow to 0 for negative powers:

~~~rust
use num::pow;

impl Pow<int,int> for int {
    fn pow(&self, rhs: &int) -> int {
        if rhs < 0 {
            0
        }
        else {
            pow(self, *rhs as uint)
        }
    }
}
~~~

For floating types, the implementation will forward to the `Float::powf` method:

~~~rust
impl Pow<f32,f32> for f32 {
    fn pow(&self, rhs: &f32) -> f32 {
        self.powf(*rhs)
    }
}
~~~

With trait reform, an additional implementation for floating types should be
implemented, to serve the common case of integer power (using `Float::powi`
method):

~~~rust
impl Pow<i32,f32> for f32 {
    fn pow(&self, rhs: &i32) -> f32 {
        self.powi(*rhs)
    }
}
~~~

# Drawbacks

* Increases the complexity of the language (in particular the number of lang
    items and operators).

* Without trait reform, this would encourage inefficient code (e.g. calling
    `2.0 ^^ 2.0` is less efficient than `2.0 * 2.0`). With trait reform,
    `2.0 ^^ 2` will be valid and should optimize well.

# Alternatives

One alternative is just to not do this. Arguably, C/C++ managed fine without
it. Even in languages with this operator, it is rarely used (although this
applies to a few other operators, e.g. binary NOT and XOR that are also rare in
high level code, yet get syntax sugar for historical reasons).

The integer case is problematic due to negative powers, which might suggest
that this sugar should not be implemented for integers. It doesn't seem that
terrible, however, as in principle it is analogous to code such as `1 / 2`
which Rust happily accepts.

One alternative is to implement this trait for builtin types only with a `uint`
RHS, as integer powers are by far the most common in mathematical formulas.

# Unresolved questions

* Which token to actually use.

* The name of the trait/method.

* How to handle the negative powers for integers.
