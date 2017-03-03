- Start Date: 2015-01-14
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Adds the possibility to mark an member as unsafe to access.

# Motivation

Struct are commonly used to encapsulate possibly unsafe code by providing a safe interface. A private member variable may be used to store a state on which the `unsafe` code relies in order to provide a safe interface. However, it is possible for any other function in the same module to change this state without requiring an `unsafe` block or being marked as being unsafe. Thus the whole module and not only the unsafe parts require extensive auditing to ensure safety. An example is `Vec` which requires its `length` to be set correctly.

# Detailed design

This RFC proposes to introduce the member attribute `unsafe` (like the previous `priv` attribute) which can be used to mark a member as unsafe to change, indicating that special care is required, when changing them. The presence of this attribute requires an `unsafe`-block or function for every `mut` access to this member. `unsafe` seems to be the most sensible choice for the attribute name since it is already present in the language. Thus it can intuitively related to the other uses.

The implementation should be straight-forward since member attributes once were part of the language.

## Example

    struct Vec {
        unsafe length: usize
    }
    
    fn change_length(v: &mut Vec) {
        unsafe {  v.length = 0 }
    }

# Drawbacks

Introduces complexity to the language.

# Alternatives

The alternative would be to require an unsafe block for every `mut` access to any member `unsafe` blocks or functions depend on. This could be implementing by automatically tracing such dependencies in the AST.

If this RFC is not accepted we would stay at the status quo: Modules which contain structures that encapsulate possibly unsafe behavior require overall special attention.

# Unresolved questions

None.