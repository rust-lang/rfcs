- Feature Name: overload-logic-operators
- Start Date: 2019-07-04
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This feature would allow for the two short circuit operators `||` and `&&` to be overloadable by
users-of-rust and for the standard library (probably in core/alloc) to implement such an overload for
the `Option<T>` and `Result<T, E>` types for `||` and for the `Option<T>` type `&&` but not for
`Result<T, E>`.

# Motivation
[motivation]: #motivation

This idea was original floated as a way to clear up the differences between `.or(...)`, `.or_with(|| ...)`, `.and(...)`, `.and_with(|| ...)`, `.unwrap_or(...)`, and `.unwrap_or_with(|| ...)`. Not only was the requirement to remember that there were additional methods that are supposed to be used when you don't want to compute the value before the check (short circuiting). There was also a concern about the overhead of the additional closure.

This proposal is mostly about reducing the mental strain when chaining `Option<T>`'s and `Result<T, E>`'s. But has a very nice side effect of allowing users-of-rust the ability to overload these operators.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This proposal starts with an enum definition and trait definitions for each of the operators:

```rust
enum ShortCircuit<S, L> {
    Short(S),
    Long(L),
}

trait LogicalOr<Rhs = Self>: Sized {
    type Output;

    /// Decide whether the *logical or* should short-circuit
    /// or not based on its left-hand side argument. If so,
    /// return its final result, otherwise return the value
    /// that will get passed to `logical_or()` (normally this
    /// means returning self back, but you can change the value).
    fn short_circuit_or(self) -> ShortCircuit<Self::Output, Self>;

    /// Complete the *logical or* in case it did not short-circuit.
    /// Normally this would just return `rhs`.
    fn logical_or(self, rhs: Rhs) -> Self::Output;
}

trait LogicalAnd<Rhs = Self>: Sized {
    type Output;

    /// Decide whether the *logical and* should short-circuit
    /// or not based on its left-hand side argument. If so,
    /// return its final result, otherwise return the value
    /// that will get passed to `logical_and()` (normally this
    /// means returning self back, but you can change the value).
    fn short_circuit_and(self) -> ShortCircuit<Self::Output, Self>;

    /// Complete the *logical and* in case it did not short-circuit.
    fn logical_and(self, rhs: Rhs) -> Self::Output;
}
```

With a matching desugaring:

```rust
<expr_a::<T>> || <expr_b::<T>>

==>

match expr_a.short_circuit_or() {
    ShortCircuit::Short(res) => res,
    ShortCircuit::Long(lhs) => lhs.logical_or(expr_b)
}
```

and

```rust
<expr_a::<T>> && <expr_b::<T>>

==>

match expr_a.short_circuit_and() {
    ShortCircuit::Short(res) => res,
    ShortCircuit::Long(lhs) => lhs.logical_and(expr_b)
}
```

From taking into consideration the current functions, and previous discussion on [internals](https://internals.rust-lang.org/t/pre-rfc-overload-short-curcuits/10460) it seems that the following makes the most sense in terms of outcomes.

#### For `Option<T>`:

```rust
fn foo() -> Option<i32> {
    Some(3)
}

fn main() {
    Some(4) || Some(5); // == Some(4)
    None || Some(5);    // == Some(5)
    Some(4) || foo();   // == Some(4) (foo is *not* called)
    None || foo();      // == Some(3) (foo is called)
    None || 3;          // == 3
    Some(2) || 1;       // == 2
    Some(1) || panic!() // == Some(1)               + These two are side effects from !
    None || return      // returns from function    + and are the same to how boolean || works
    Some(2) && Some(3)  // Some(3)
    None && Some(1)     // None
    Some(3) && None     // None

    Some(2) || Some("hello") // Error: LogicalOr<Option<&str>> not implemented for Option<i32>
    Some(2) || 2 || 3        // Error: LogicalOr<i32> is not implemented for i32
}
```

#### For `Result<T, E>`
```rust
struct MyError;

fn foo() -> Result<i32, MyError> {
    Ok(3)
}

fn main() {
    Ok(4) || Ok(5);           // == Ok(4)
    Err<MyError{}> || Ok(5);  // == Ok(5)
    Ok(4) || foo();           // == Ok(4) (foo is *not* called)
    Err<MyError{}> || foo();  // == Ok(3) (foo is called)
    Err<MyError{}> || 3;      // == 3
    Ok(2) || 1;               // == 2

    Ok(2) || Ok("hello");  // Error: LogicalOr<Result<&str, _>> not implemented for Result<i32, _>
    Ok(2) || 2 || 3;       // Error: LogicalOr<i32> is not implemented for i32
}
```

The feature should be thought about as moving the logic from methods and into the current function.
It maps very seamlessly from using the methods and is equivalent in use without having to worry about
the naming convention of the short circuit methods. The same mental state of short circuit from
bools applies directly, without having any recourse to "truthiness" which is not a desirable trait.

This RFC also proposes to deprecate the `.or(...)`, `.or_with(|| ...)`, `.and(...)`, `.and_with(||
...)`, `.unwrap_or(...)`, and `.unwrap_or_with(|| ...)` methods on `Option<T>` and `.or(...)`,
`.or_with(|| ...)`, `.unwrap_or(...)`, and `.unwrap_or_with(|| ...)` methods on `Result<T, E>` since
using this feature renders them unneeded.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The basis of this proposal are the two new traits. These should be implemented along the same lines
as other operator traits. However, the desugaring should follow the `match` output in the previous
section so as to obtain the desired short circuit operation.

Once the traits have been implemented, several trait implementations should be added to the std library and the methods marked as deprecated.

```rust
impl LogicalOr<Option<T>> for Option<T> {
    type Output = Self;
    ...
}

impl trait LogicalOr<T> for Option<T> {
    type Output = T;
}

impl trait LogicalAnd<Option<T>> for Option<T> {
    type Output = Self;
    ...
}

impl trait LogicalOr<Result<T, E>> for Result<T, E> {
    type Output = Self;
    ...
}

impl trait LogicalOr<T> for Result<T, E> {
    type Output = T;
}

```

# Drawbacks
[drawbacks]: #drawbacks

1. Leaves the `||` and the `&&` as not strictly boolean operators, which might hurt readability
2. Could lead to similarities to C++'s Operator bool() which => truthiness and is undesirable.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- This design is the best because it does not rely on other traits, which are how the other operator traits work in Rust. It has the minimum overhead since it does not rely on closures.
- Two alternatives were discussed and shown to be inferior in either performance, explicitness, or design.
- The first being "truthiness" conversion trait and then automatically allowing `||` and `&&` if both that trait and the `BitOr` or `BitAnd` traits were also implemented. This was discarded because we did not want to go down the javascript route of auto converting things to boolean (Rust already does not allow non-bools in check expressions) and auto traits are not something that Rust has so that was another reason not to go down this route.
- The second being a trait that accepts an `FnOnce` argument and then the second argument of the operator would be then implicitly hoisted into a closure. This was rejected both because hiding closures is not a zero-cost abstraction, it would break the similarity with the boolean operators because `None || return` would not return from the function unlike `false || return`. This also does not have any benifit over just using `or_with` directly except for a few characters.
- If this is not done it then the usability of Rust without having to go to the docs would stay the same.

# Prior art
[prior-art]: #prior-art

The only other language that seems to have implemented short circuit `||` and `&&` is C#.

C# did it with the combined auto trait and truthiness (bool() operator) functions. While this is similar to this proposal, it is thought that since auto converting to bool is not happening (just checking if it should short circuit) thinking about the functions as truthiness. C# already doesn't use lambdas (similar to closures) for its solution.

C++ also allows overloading these operators but without short circuiting which is undesirable.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

No unresolved questions.

# Future possibilities
[future-possibilities]: #future-possibilities
