- Feature Name: debug-noop-in-release
- Start Date: 2014-03-04
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

The `{:?}` format specifier should not print values in release builds.

# Motivation

Many commonly used methods, such as `unwrap`, use `{:?}` to print out representations of values if their conditions are not met. This is useful for debugging but is not so useful in release builds, and it can bloat up the generated binaries. Worse, it adds opaque calls (as `Debug` is frequently a lot of code and rarely gets inlined), which can mess up the inlining heuristics and cause cascades of missed optimization opportunities.

# Detailed design

In release mode, `{:?}` should simply print `(debugging representation omitted in release builds)`. This makes it clear to the programmer what should be done to reenable the feature. If debugging information is *really* desired in release builds, the method from the `Debug` trait can still be called explicitly.

# Drawbacks

* This will reduce error message quality in production.

* This may lead to confusion as to why debug messages are not being printed out.

# Alternatives

An alternative would be to use `#[cfg(debug)]` inside every function that calls `{:?}`. This, however, shifts the burden to every function that wants to log and is likely to result in missed cases as a result.

# Unresolved questions

Should `#[derive(Debug)]` also turn into a no-op in release builds? This would improve compiler times but would make it impossible to call `Debug` explicitly in most cases.
