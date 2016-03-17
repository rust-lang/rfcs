- Feature Name: as_millis
- Start Date: 2016-03-17
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Add `as_millis` function to `std::time::Duration`.

# Motivation
[motivation]: #motivation

Working with milliseconds is very common and since `std::time::Duration` has the `from_millis` and `as_secs` functions it 
makes sense to create a `as_millis`.

# Detailed design
[design]: #detailed-design

Add a `as_millis` function on `std::time::Duration` that divides `self.nanos` by `NANOS_PER_MILLI` and returns it.

# Drawbacks
[drawbacks]: #drawbacks

No drawbacks.

# Alternatives
[alternatives]: #alternatives

The impact is that everytime someone wants to work with durations in milliseconds they have to convert it.

# Unresolved questions
[unresolved]: #unresolved-questions

None.
