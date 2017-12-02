- Feature Name: imply-option
- Start Date: 2017-10-18
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

This is an RFC to reduce common boiler plate code when making use of the `Option` type, providing two functions
```
    impl OptionWrap for bool {
        /// Construct `Some(T)`, conditionally on a boolean.
        fn some_if<T>(self, T) -> Option<T>;
        /// A lazy equivalent of `some_if`.
        fn lazy_if<T, F: FnOnce() -> T>(self, F) -> Option<T>;
    }
```

# Motivation
[motivation]: #motivation

This addition will increase the legibility of code segments and assist in defining the thought processes and motivations of programmers through their code. The use cases of this addition are problems which are expressable in the following Rust pseudocode:
```
    if boolean {
        Some(value)
    } else {
        None
    }
```
The outcome of this addition will reduce repeated unexpressive code which introduces bugs during refactoring and present the thought process of the programmer in a clearer fashion through their code.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The `Option` type is useful when your code may or may not yield a return value.
Such code may looks similar to this:
```
    let x = 0;
    
    if x == 0 {
        Some(x)
    } else {
        None
    }
```
However only the `if` branch of this code segment is the important part we're concerned about in our code:
```
    if x == 0 {
        Some(x)
    }
```
But the `else` branch is required for returning the `None` value if `x == 0` evaluates to false.
Fortunately Rusts `bool` type has functionality to get rid of the unecessary code:
```
    let x = 0;
    
    (x == 0).some_if(x)
```
This code has the exact same behaviour as our original `if` statement. Our code is however compressed to a single line and our intentions are just as clear.
Have you spotted the possible issue with this solution introduces however? What about this code:
```
    false.some_if(foo())
```
The above line of code will always return `None` and always throw away the result of `foo()` wasting our precious computing power every time our code needs to return `None`.
Rust has thought ahead of this problem though:
```
    false.lazy_if(foo)
```
`bool`s `lazy_if` function leverages lazy evaluation by taking a function pointer as its second argument. If its first argument evaluates to `true` it will return `Some(foo())` but if its first argument is `false` it returns `None` without having to run `foo`. This solves the problem presented in our earlier example without sacrificing the advantages it gave us.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The Rust addition this RFC suggests can be encapsulated as "If `x` is `true`, I care about the value of `y`; else I do not care about the value of `y`." or:
```
    | x | x.some_if(y) |
    | F | None         |
    | T | Some(y)      |
```
This RFCs initial proposal for how this addition could be implemented is:
```
    trait OptionWrap {
        /// Wrapper around common if/else boiler plate.
        fn some_if<T>(self, value: T) -> Option<T>;
        /// A lazy implementation of `some_if`.
        fn lazy_if<F>(self, func: F) -> Option<T>;
    }
    
    impl OptionWrap for bool {
        fn some_if<T>(self, value: T) -> Option<T> {
            if self {
                Some(value)
            } else {
                None
            }
        }
        fn lazy_if<F>(self, func: F) -> Option<T>
            where F: FnOnce() -> T {
            if self {
                Some(func())
            } else {
                None
            }
        }
    }
```
This implementation covers the use cases proposed in the earlier examples and any others of similar form without any external dependencies; this should make the implementation stable as Rust continues to develop.

# Drawbacks
[drawbacks]: #drawbacks

This is functionality which has functional programming and monads in mind with its design and this may make it another stepping stone to be learned for programmers which are new to Rust or functional programming concepts.

# Rationale and alternatives
[alternatives]: #alternatives

The implementation proposed is clear, easily documented and is the minimal ammount of code necessary to add this into the Rust language without sacrificing any of the advantages of the `if/else` blocks.
Other designs which have been considered are:
- Not including the `some_if` function. However this adds additional boiler plate code to make use of the functionality when passing in a value directly:
```
    let x = 0;
    
    //true.some_if(x)
    true.lazy_if(|| x)
```
It is very little boiler plate code compared to the `if/else` alternative but it is suboptimal from an execution standpoint and a more obtuse implementation for new Rust programmers to learn.
- Not including the `lazy_if` function. However, as discussed, this leaves the `some_if` function at a disadvantage when the equivalent `if` block is computationally intesive as it wastes computation on a value which may simply be discarded.
- Providing syntax support for this implementation in Rust (similar to the `?` operator for the `Result` type). However, pushing the abstraction of the logic this far reduces the clarity of the code and the expression of the programmers intention. Additionally discussion has yet to adequately cover syntax support for both the `some_if` and `lazy_if` functions in a meaningful manner and removing either one is disadvantageous as discussed above.

# Unresolved questions
[unresolved]: #unresolved-questions

Through the RFC process I hope to qualify:
- That this is first a problem which does affect other Rust programmers.
- That my proposed solution would meaningfully improve the experience of other programmers in Rust.
- That my proposed implementation cannot be further optimised or stabilised.
As mentioned under the [alternatives] section syntax support of this feature is a possibility in future but I feel is outside the scope of this RFC before the implementation is stabilised in Rust and a meaningful syntax for this feature is yet to be determined.
