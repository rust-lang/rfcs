- Start Date: 2014-09-04
- RFC PR #:
- Rust Issue #:

# Summary

Require public trait items (currently all of them) to be marked `pub` so that we can later on introduce private trait items to the language without having to add a keyword, such as `priv`, for indicating *"private"*.

# Motivation

The term *trait item* here refers to any method, associated method, associated type or associated static declared/defined by a trait. The RFC [#52](https://github.com/rust-lang/rfcs/pull/52) talks about the motivation for **private** trait items. The main two motivations for this RFC are that:  
1) We don't want to re-introduce the `priv` keyword to the language.  
2) The language would be more coherent and logical if trait items were private by default, given that regular methods are private by default also.

# Detailed design

For now, make it a compile-time error to declare a trait item without the preceding `pub` keyword. At some later point in time we can allow trait items without the preceding `pub` keyword, which would make the trait item private (whatever the exact semantics of a private trait item happen to be).

# Drawbacks

This would add more typing even in the long run because public trait items are a lot more common than private ones.

# Alternatives

The alternative is to do nothing now and introduce the `priv` keyword to the language when (if) private trait items get implemented.

# Unresolved questions
