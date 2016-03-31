- Feature Name: cfg_assert
- Start Date: 2016-03-31
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Add a `#![cfg_assert]` attribute which triggers a compile-time error if the required configuration is not met.

Examples:
```rust
#![cfg_assert(target_feature = "avx", "This library requires AVX support")]

#![cfg_assert(target_os = "linux", "This library only works on Linux")]

#![cfg_assert(any(target_arch = "x86", target_arch = "x86_64"), "This library only works on x86")]

#![cfg_assert(all(feature = "foo", feature = "bar"), "The `foo` and `bar` features can't both be enabled at once")]
```

# Motivation
[motivation]: #motivation

The main motivation for this is to provide a nice error message when a library can't work with a given configuration. For example, the `cpuid` crate will only work on x86, and attempts to use it on other architectures (perhaps by accident through a dependency) will lead to some cryptic LLVM errors when it gets compiled.

This is roughly equivalent to the following C code:
```c
#ifndef _WIN32
#error "This only works on Windows"
#endif
```

# Detailed design
[design]: #detailed-design

Not much to add here. The only tricky part is that we need to handle nested attributes correctly:
```rust
#![cfg_assert(bar, "quux")]
#![cfg(foo)]
```
In this example the `foo` attribute will have precedence over the assert, since the whole module will be eliminated if `foo` is not set, including the assert.

# Drawbacks
[drawbacks]: #drawbacks

None

# Alternatives
[alternatives]: #alternatives

One alternative would be to instead create an `#[error("message")]` attribute which can be used like this:
```rust
#![cfg_attr(target_os = "linux", error("This library only works on Linux"))]
```

This reuses the existing `cfg_attr` attribute, however I don't see much use of `error` outside of it so it may not be as ergonomic as `cfg_assert`.

# Unresolved questions
[unresolved]: #unresolved-questions

None, except possibly finding a better name for the attribute.
