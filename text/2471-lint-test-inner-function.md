- Feature Name: `lint_test_inner_function`
- Start Date: 2018-06-10
- RFC PR: [rust-lang/rfcs#2471](https://github.com/rust-lang/rfcs/pull/2471)
- Rust Issue: [rust-lang/rust#53911](https://github.com/rust-lang/rust/issues/53911)

# Summary
[summary]: #summary

Add a lint that warns when marking an inner function as `#[test]`.

# Motivation
[motivation]: #motivation

`#[test]` is used to mark functions to be run as part of a test suite. The
functions being marked need to be addressable for this to work. Currently,
marking an inner function as `#[test]` will not raise any errors or warnings,
but the test will silently not be run. By adding a lint that identifies these
cases, users are less likely to fail to notice.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This is a lint that triggers when a `#[test]` annotation is found in a non
addressable function, warning that that function cannot be tested.

For example, in the following code, `bar` will never be called as part of a
test run:

```rust
fn foo() {
    #[test]
    fn bar() {
        assert!(true);
    }
}
```

The output should resemble the following:

```
error: cannot test inner function
  --> $DIR/test-inner-fn.rs:15:5
   |
LL |     #[test] //~ ERROR cannot test inner function [untestable_method]
   |     ^^^^^^^
   |
   = note: requested on the command line with `-D untestable-method`
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This is a new lint that shouldn't interact with others. Due to the interaction
with `cfg` attributes, the lint might only warn when run as part of a `--test`
compilation. This would be acceptable.

# Drawbacks
[drawbacks]: #drawbacks

Can't think of any reason not to do this.

# Rationale and alternatives
[alternatives]: #alternatives

Adding as a lint allows users to silence the error if they so wish.

Not addressing this issue will let this problem continue happening without
warning to end users.

# Prior art
[prior-art]: #prior-art

This would act in the same way as other lints warning for potentially
problematic valid code.

# Unresolved questions
[unresolved]: #unresolved-questions

None.
