- Feature Name: `result_missing_methods_for_err`
- Start Date: 2020-04-04
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Add these methods:
 - `into_err`: does the same as `into_ok` but for `Err`,
 - `map_err_or`: does the same as `map_or` but for `Err`,
 - `map_err_or_else`: does the same as `map_or_else` but for `Err`,
 - `unwrap_err_or`: does the same as `unwrap_or` but for `Err`,
 - `unwrap_err_or_default`: does the same as `unwrap_or_default` but for `Err`,
 - `unwrap_err_or_else`: does the same as `unwrap_or_else` but for `Err`.

# Motivation
[motivation]: #motivation

These methods should be present because of logical reasons.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

# Drawbacks
[drawbacks]: #drawbacks

No drawbacks.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

# Prior art
[prior-art]: #prior-art

# Unresolved questions
[unresolved-questions]: #unresolved-questions

# Future possibilities
[future-possibilities]: #future-possibilities