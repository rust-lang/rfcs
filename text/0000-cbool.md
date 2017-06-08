- Feature Name: cbool
- Start Date: Mon Mar  9 00:16:53 CET 2015
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Specify that `bool` is compatible with the `_Bool` C type.

# Motivation

You cannot safely call ffi functions with boolean arguments without a compatible
type.

# Detailed design

Specify that `bool` is compatible with the `_Bool` C type.

# Drawbacks

None.

# Alternatives

Define `_Bool` as a platform dependent integer type. This is unsafe because the behavior is supposedly undefined if you pass a value other than 0 or 1 to such a function.

# Unresolved questions

None.
