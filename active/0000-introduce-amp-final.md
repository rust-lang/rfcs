- Start Date: 2014-05-05
- RFC PR #:
- Rust Issue #:

# Summary

Introduce a deeply immutable reference type, `&final`.

# Motivation

Rust currenltly has two references types: `&` and `&mut`. A `&` reference is a
potentially aliased reference to an object. A `&mut`, meanwhile, is a reference
that is guaranteed to be non-alised. Despite the name `&mut` implying that `&`
is immutable, in practice, this isn't necessarily so. Consider:

```rust
use std::cell::Cell;

fn main() {
    let c1 = Cell::new(0);
    {
        let c2: &Cell = &c1;
        c2.set(1);
    }
    println!("Value: {}", c1.get());
    // Output: 1
}
```

There are no mutable references or slots, but the value of `c1` is still clearly
changed. Conclusion: there is no current reference type that can be safely passed to
another function and maintain the guarantee that the object that is being
passwed will not be mutated. Additionally, even if we know that a type does not
implement interior mutability, the compiler has no mechanism to warn us if the
type's implementation changes to include interior mutability in the future.

# Detailed design

Re-introduce the `Freeze` bound. In order to satisfy `Freeze`, a type must not
contain an `Unsafe` either directly or transitively. Additionally, all type
parameters or contained Trait objects must also be `Freeze`. (Question: is it
possible to guarantee that a Trait object must be `Freeze` at compile time?)

Introduce a new reference type, `&final`. A `&final` reference is a potentially
aliased reference like `&`. However, it is guaranteed that an object is deeply
immutable through a `&final` reference unlike with a `&`. It is only possible
to borrow a `&final` reference from an object that is known at compile time to
be `Freeze`.

`&mut` and `&` references may be re-borrowed to a `&final` reference as
long as the referenced type is `Freeze`. Just like when borrowing a `&mut` to a
`&`, when borrowing a `&mut` to a `&final`, the `&mut` becomes inaccessible
until the `&final` goes out of scope. This re-borrowing must be done manually
so that it is clear at the call site that a `&final` reference is being passed
and that this cannot change based on updates to the function being called.

A `&final` can be automatically re-borrowed to a `&` as long as the referenced
typed is `Freeze`. This is safe because as long as a type is `Freeze`, 
a `&` is deeply immutable just like a `&final`. 

A `&final` may not be re-borrowed to a `&mut` except with `Unsafe` code.

The goal of this RFC is to solve problems like the following:

```rust
use some_crate::SomeType;

fn some_func(x: &SomeType) {
    // ...
}

fn do_something(x: &mut SomeType) {
    some_func(x);
    // ... some code that assumes some_func didn't modify x
}
```

Does `some_func` modify `x`? Its not really easy to tell. If `SomeType` is 
`Freeze`, we know that it can't be modified. However, there is no good way
to say that as a pre-condition to `do_something`, that `SomeType` must be
`Freeze`. As another scenario, lets say that we know for a fact that `SomeType`
is `Freeze`. However, at some point in the future, `some_func` is updated
so that it starts taking its parameter as a `&mut`. It would be nice if the
compiler would warn us of this change, however, the compiler will currently
just silently pass it through.

With this RFC, this code would become:

```rust
use some_crate::SomeType;

fn some_func(x: &final SomeType) {
    // ...
}

fn do_something(x: &mut SomeType) {
    some_func(&final *x);
    // Does some_func modify x? It can't since we passed x as &final
}
```

Since we're using a `&final` reference, we know that `SomeType` does not
implement interior mutability. If `SomeType` is changed to implement 
interior mutability in the future, this will create a compiler error
since `SomeType` will no longer be `Freeze` and thus it will not be possible
to borrow a `&final`.

Further, if `some_func` is updated in the future to take a `&mut` instead of
a `&final`, that will also cause a compiler error since a `&final` cannot be
reborrowed to a `&mut`.

# Drawbacks

This would created a 3rd pointer type which adds significant complexity to the
language, especially since the trend recently has been one of simplification.

# Alternatives

* The status quo - no deeply immutable reference types.

* Other proposals?

# Unresolved questions

* Is the syntax for borrowing a `&mut` to a `&final` reasonable? Its a bit
ugly, so, it might need some sugar.

* Is `&final` the right name? `&const` seems appealing, but my understanding
is that its previous meaning was very different than what is proposed here
which might lead to confusion.

