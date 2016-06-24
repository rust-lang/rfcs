- Feature Name: generic_closure
- Start Date: 2015-06-15
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

This RFC adds the ability to define closures that are generic over types.

# Motivation
[motivation]: #motivation

Generic closures can be used to support compound operations on tuple types:

```rust
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
struct Tuple<A, B, C>(pub A, pub B, pub C);

impl<A, B, C> Tuple<A, B, C> {
    fn map<A2, B2, C2, F>(self, mut f: F) -> Tuple<A2, B2, C2>
        where F: FnMut(A) -> A2 + FnMut(B) -> B2 + FnMut(C) -> C2
    {
        Tuple(f(self.0), f(self.1), f(self.2))
    }
    
    fn fold<T, F>(self, val: T, mut f: F) -> T
        where F: FnMut(T, A) -> T + FnMut(T, B) -> T + FnMut(T, C) -> T
    {
        let val = f(val, self.0);
        let val = f(val, self.1);
        let val = f(val, self.2);
        val
    }
}

let a = Tuple(1u8, 2u32, 3.5f32).map(<T: Into<f64>>|x: T| x.into() + 1.0);
assert_eq!(a, (2f64, 3f64, 4.5f64));

let b = Tuple(1u8, 2u32, 3.5f32).fold(10.0, <T: Into<f64>>|x, y: T| x + y.into());
assert_eq!(b, 16.5f64);
```

A fully working example of this code (with manually implemented closures) can be found [here](https://play.rust-lang.org/?gist=ea867336945253752e31873fc752ec06&version=nightly&backtrace=0).

# Detailed design
[design]: #detailed-design

## Syntax

There are two ways to specify generic bounds on closures:

```rust
<T: Debug>|x: T| println!("{:?}", x);

<T>|x: T| where T: Debug {
    println!("{:?}", x);
}
```

When using the `where` syntax, the braces around the closure body are mandatory.

If the `move` keyword is used then it must appear before the generic parameter list:

```rust
move <T: Debug>|x: T| println!("{:?}", x);

move <T>|x: T| where T: Debug {
    println!("{:?}", x);
}
```

All generic parameters must be used in the closure argument list. This is necessary to ensure that the closure can implement all the required `Fn` traits.

## Implementation

The generated closure type will have generic implementations of `Fn`, `FnMut` and `FnOnce` with the provided type bounds. This is similar to the way closures currently have generic implementations over lifetimes.

# Drawbacks
[drawbacks]: #drawbacks

Increased language complexity.

# Alternatives
[alternatives]: #alternatives

If the given syntax is determined to be ambiguous, this one can be used instead:

```rust
for<T: Debug>|x: T| println!("{:?}", x);

for<T>|x: T| where T: Debug {
    println!("{:?}", x);
}
```

We could just not add this, however it would make generic operations on tuples less ergonomic. This feature is going to be even more useful when variadic generics are added in the future.

# Unresolved questions
[unresolved]: #unresolved-questions

What are the syntax interactions of `move` generic closures with the proposed `&move` reference type?

Is the syntax in this RFC ambiguous for the parser?
