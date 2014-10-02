- Start Date: 10-02-2014
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Add a `#[test_main]` attribute that overrides the behavior of the `--test`
compiler flag so that instead of building and running all `#[test]`s, it
instead just calls the function annotated `#[test_main]`.

# Motivation

This allows full overloading of the normal testing behavior by downstream
libraries which wish to provide more complex features than those offered by the
builtin testing suite - for instance, the ability to run something before each
test or before and after all tests.

Libraries can currently do this by forcing users to build with just `--cfg test`,
but this is a poor solution as this doesn't support `cargo test` and other
tools which assume the normal `--test` behavior.

# Detailed design

A new builtin attribute, `#[test_main]` would be added, which can annotate any
`fn` with the signature `fn() -> ()` and causes it to be run as `main` when
`--test` is passed. Annotating any `fn` with `#[test_main]` disables the normal
`--test` behavior.

Only one `fn` can be annotated with this signature.

# Drawbacks

Small increase in complexity.

Possibly makes the test output inconsistent if libraries implement their own
formatting or test runners.

# Alternatives

Do not expose this and force users to use the builtin testing suite in all
cases.

Add more complex features, like before-each and before/after, to the builtin
testing suite to appease users who want to create downstream libraries.

# Unresolved questions

Should the name of the `fn` annotated with `#[test_main]` be restricted?

