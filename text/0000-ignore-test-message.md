- Feature Name: `ignore_test_message`
- Start Date: 2022-01-07
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Providing ignore message if the message is provided.

# Motivation
[motivation]: #motivation

There may be meta value as a message for ignoring attributes on a test case.
[the-ignore-attribute](https://github.com/rust-lang/reference/blob/master/src/attributes/testing.md#the-ignore-attribute)


Consider following test
```rust
#[test]
#[ignore = "not yet implemented"]
fn test_ignored() {
    // …
}
#[test]
fn test_works() {
    // …
}
```

Currently, only ignored flag will print in the summary of `cargo test`.

```
running 2 tests
test tests::test_ignored ... ignored
test tests::test_works ... ok

test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

It is good to show the message if exist as following.

```
running 2 tests
test tests::test_ignored ... ignored, not yet implemented
test tests::test_works ... ok

test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

This RFC aims to easier use for large system.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation


Currently the `TestDesc` only store ignore flag, when the test case built, we can only know the case is ignore or not.

[https://github.com/rust-lang/rust/blob/master/library/test/src/types.rs#L121-L130](https://github.com/rust-lang/rust/blob/master/library/test/src/types.rs#L121-L130)

    
```rust
#[derive(Clone, Debug)]
pub struct TestDesc {
    pub name: TestName,
    pub ignore: bool,
    pub should_panic: options::ShouldPanic,
    pub allow_fail: bool,
    pub compile_fail: bool,
    pub no_run: bool,
    pub test_type: TestType,
}

```

Actually, there are more than two case for ignore test, with message and without message.

```rust
enum Ignoring {
  IgnoredWithMsg(String), // ignore with message
  Ignored,
  NotIgnored,
  // ...
}

```

The message meta value can be parsed here.

[https://github.com/rust-lang/rust/blob/master/src/tools/compiletest/src/header.rs#L879-L882](https://github.com/rust-lang/rust/blob/master/src/tools/compiletest/src/header.rs#L879-L882)

```rust
    ignore = match config.parse_cfg_name_directive(ln, "ignore") {
        ParsedNameDirective::Match => true,
        ParsedNameDirective::NoMatch => ignore,
    };
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

None known.

# Drawbacks
[drawbacks]: #drawbacks

None known.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

An alternative way is to show the message only user want, `cargo test --show-ignore-message`,
and the default behavior still show `ignored` only.

An alternative way to store the ignore message is to use additional field, for example `ignore_message`.

```rust
#[derive(Clone, Debug)]
pub struct TestDesc {
    pub name: TestName,
    pub ignore: bool,
    pub ignore_message: Option<String>,
    pub should_panic: options::ShouldPanic,
    pub allow_fail: bool,
    pub compile_fail: bool,
    pub no_run: bool,
    pub test_type: TestType,
}

```

If the ignore messages important for Rust user, and we want to print these out in summary,
we need to store this some way.

# Prior art
[prior-art]: #prior-art

None known.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None known.

# Future possibilities
[future-possibilities]: #future-possibilities

Other test framework can utilize the feature, can provide more description on integration test case.

Take [test-with](https://github.com/yanganto/test-with/) as example,

```text
running 2 tests
test tests::test_backup_to_S3 ... ignored, because following variables not found: ACCESS_KEY, SECRET_KEY
test tests::test_works ... ok

test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

People are requesting this kind of summary when running `cargo test`.
[https://github.com/rust-lang/rust/issues/68007](https://github.com/rust-lang/rust/issues/68007)
