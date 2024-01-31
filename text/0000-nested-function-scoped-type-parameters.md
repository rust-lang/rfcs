- Feature Name: Nested function scoped type parameters
- Start Date: 31/1/2024
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Allow the use of type parameters bound in an outer function in an inner function. Removes E0401.

# Motivation
[motivation]: #motivation

Currently, nested functions appear slightly "second class" compared to closures in this regard.
By allowing the use of type parameters bound in the outer function, it allows these functions to be simpler to use, and easier on the programmer mentally.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

In short, permit:
```rust
fn foo<T>(x: T) -> T {
    fn bar(y: T) -> T {
        y
    }
    bar(x)
}
```
as the type parameter now fully spans the outer function instead of only for variables/closures/etc

The main usecase is to allow cases like the following:
```rust
fn foo<T>(x: T) -> T
    where
        T: Some + Very + Long + List + Of + Constraints
 {
    fn bar(y: T) -> T {
        y
    }
    bar(x)
}
```
In this case, to utilize an inner function, one would have to duplicate all of these constraints, obviously discouraging said usage.  

I am personally unable to deduce why this behavior exists, as there is no rational past "inner functions are treated as toplevel functions", which seems odd to me - they are not, so why treat them as such? They can still be hoisted to the toplevel with this change and the elaboration detailed below. 


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This proposal is, in essence, "syntax sugar". All occurences of a type parameter in a nested function could be replaced with their outer level counterparts, and annotations are added on call sites to aid inference, as the information is avalible. From the above, we generate

```rust
fn foo<T>(x: T) -> T
    where
        T: Some + Very + Long + List + Of + Constraints
 {
    fn bar<G>(y: G) -> G
        where
            G: ...
    {
        y
    }
    bar::<T>(x)
}
```

As there is already an error to detect this exact situation (E0401), no existing code should be broken by this implementation. There may however be edge cases in more complex examples I am not aware of.
The prerequesits required to implement this are already established, as closures can already do the above. 

# Drawbacks
[drawbacks]: #drawbacks

This proposal does slightly complicate the typechecking of functions, but hopefully not to a large extent depending on how is implemented.
It also could add complexity to the understanding of very complex nested functions utilising similar type parameter names.
Checking would also need to include scope due to shadowing, potentially making the design and usage more complex.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

As this change is essentially "syntax sugar" to make nested functions more consistent with other constructions, there is no major replacement or alternative past simply writing the more complex functions.

# Prior art
[prior-art]: #prior-art

This proposal is inspired by `-XScopedTypeVariables` in GHC Haskell and `(type a)` statements in OCaml, which allows essentially the same feature. This flag/feature is generally regarded as useful and time saving when writing Haskell/OCaml code.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

I would like to ensure that there are no edge cases where this behavior could case regressions.

# Future possibilities
[future-possibilities]: #future-possibilities

I do not see any way (or reason) to extend this proposal in the future.
