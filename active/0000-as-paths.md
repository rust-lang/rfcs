- Start Date: 2014-05-28
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

The `as` production in the grammar should be `as PATH` or `as (TYPE)` (with the parentheses).

# Motivation

When adding `+` to separate trait object paths from bounds in the type grammar, we broke a bunch of code that looked like `1 as uint + 3`. This is because the type parser is greedy and started parsing after `+`. This will allow us 

# Detailed design

The `as` production in the grammar should be `as PATH` or `as (TYPE)` (with the parentheses). `PATH` productions in this context should be parsed as types.

# Drawbacks

Complex types with `as` might become slightly more verbose, and the grammar becomes slightly more complicated.

# Alternatives

The impact of not doing this is that if we extend the type grammar (e.g. with `+`) then we will break existing code.

# Unresolved questions

N/A.