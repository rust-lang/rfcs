- Feature Name: Implementation of more user-friendly comparator macros
- Start Date: 2019-03-15
- RFC PR: (to be updated)
- Rust Issue: (to be updated)
<!-- TODO -> Update the issue and PR links -->
# Summary
[summary]: #summary

This RFC implements two macros, for asserting whether the left hand side expression is greater or less than the right hand side expression, which will make writing tests easier, by providing a streamlined and fluent API, that users are already used to and providing more understandable output than the existing macros.

# Motivation
[motivation]: #motivation

The test output provided by assert while writing tests is becoming inconsistent, with the test output from the other macros that we use for testing such as `assert_eq!` and `assert_ne!`. If we currently have to assert that a > b, then the option would either be:
`assert!(a > b)` which on panic would output this: 
```
thread 'main' panicked at 'assertion failed: a > b', src/main.rs:79:5
```
Or with something like `assert_ne!(!(a > b), true)` which on panic would output this, which is even more vague:
```
thread 'main' panicked at 'assertion failed: `(left != right)`
  left: `true`,
 right: `true`', src/main.rs:79:5
```
Hence this RFC will implement the `assert_lt!` and `assert_gt!` macros:
So the example above could be rewritten like this:
```rust
assert_gt!(a, b);
```
And on panic, this would be:
```
thread 'main' panicked at 'assertion failed: `(left <= right)`
  left: `100`,
 right: `200`', src/main.rs:79:5
```
This provides a more graceful output and is more helpful while debugging. Also using an external crate or writing new macros for every project may pose an extra effort for developers.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When you want to test right hand side and left hand side expressions, you have several options available to you. To ensure that the left hand side expression is greater than the right hand side expression, we make use of the `assert_gt!` macro. A trivial example would be:
```rust
let a = 100; let b = 50;
assert_gt!(a, b);
// The code will panic if a is not greater than b
// Even if b is 100, then that means a is not
// greater than b and the code will panic
``` 
To ensure that the left hand side expression is always smaller than the right hand side expression, we make use of the `assert_lt!` macro. This is used in a similar way to `assert_gt!`, except that the code will panic if the LHS is greater than or equal to the RHS.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This is coded in a similar way to the `assert_ne!` and `assert_eq!` macros, making the output look familiar with existing users. The macro is implemented in the following way, leveraging `PartialEq` :
```rust
macro_rules! assert_gt {
    ($left:expr, $right:expr) => ({
        match (&$left, &$right) {
            (left_val, right_val) => {
                if !(*left_val > *right_val) {
                    panic!(r#"assertion failed: `(left <= right)`
  left: `{:?}`,
 right: `{:?}`"#, &*left_val, &*right_val)
                }
            }
        }
    });
    ($left:expr, $right:expr,) => {
        assert_gt!($left, $right)
    };
    ($left:expr, $right:expr, $($arg:tt)+) => ({
        match (&($left), &($right)) {
            (left_val, right_val) => {
                if !(*left_val > *right_val) {
                    panic!(r#"assertion failed: `(left <= right)`
  left: `{:?}`,
 right: `{:?}`: {}"#, &*left_val, &*right_val,
                           format_args!($($arg)+))
                }
            }
        }
    });
}
```

# Drawbacks
[drawbacks]: #drawbacks

Pushing something into the standard library means that we will have to continue maintaing this for years to come. As this RFC will not put in a major change and will *not* break backward-compatibility, I believe it has little drawbacks.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This change will eliminate the need to use external crates just to ensure asserts. Also the implementation is almost analogous to the other assert macros hence making it easy for future maintenance by the community. Also the elimination of the need for external crates reduces binary size in many cases and also reduces the build time in several situations. Further more, output will be more sensible, showing what caused the panic instead of outputting the variable names, and these variable names may not be helpful at all during debugging. There is also no existing way to do this using `assert_eq!` and the only way available now is to define a custom macro, or continue using `assert!` which does not provide very sensible output.
<br>An alternative would be to use any of the existing crates available on [crates.io](https://crates.io/).
Other options would be to use a cubersome API:
`assert_eq!(!(a < b), true)`, which does do the job but still provides insufficient debugging output. Another option would be to continue using `assert!(a > b)`, which would also provide _not very helpful_ output.

# Prior art
[prior-art]: #prior-art
Nothing significant as of now

# Unresolved questions
[unresolved-questions]: #unresolved-questions

Nothing significant as of now

# Future possibilities
[future-possibilities]: #future-possibilities

None as of now
