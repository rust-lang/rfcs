- Feature Name: gate-extern-fn-methods
- Start Date: 2015-12-27
- RFC PR: (leave this empty)
- Rust Issue: #30235

# Summary
[summary]: #summary

Restrict the use of non-Rust ABI methods to a feature gate.

# Motivation
[motivation]: #motivation

Currently we allow this code:

```rust
trait Foo {
    extern fn foo(&self);
}
```

In the absense of an ABI string, `extern fn` defaults to using "C". This functionality does not
work well and appears to frequently be a mistake on the part of the user. Since the decision to
support this feature either way should be subject to discussion, this RFC is only proposing the
addition of a feature gate that controls access to this feature.

# Detailed design
[design]: #detailed-design

Detect use of `extern fn` in trait definitions and `impl` blocks that don't have a Rust ABI (either
"Rust" or "rust-call") and disallow it unless a feature gate is enabled.

The name of the feature gate is not important, but `extern-fn-methods` seems like a reasonable name.

# Drawbacks
[drawbacks]: #drawbacks

It may break some existing code, however evidence suggests that code attempting to use this feature
runs into one of the bugs surrounding the feature and therefore already doesn't work.

# Alternatives
[alternatives]: #alternatives

Don't feature gate and instead skip straight to discussion about the feature's support. This isn't
appealing as the feature will continue to be a source of confusion until the discussion is
concluded.

# Unresolved questions
[unresolved]: #unresolved-questions

None
