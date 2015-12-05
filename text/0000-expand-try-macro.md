- Feature Name: expand-try-macro
- Start Date: 2015-12-05
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Add additional `try!(expr => return)` that will return without value.

# Motivation
[motivation]: #motivation

After I have created [Soma][soma] crate there was [suggestion in Reddit
post][reddit] to do so.

[reddit]: https://www.reddit.com/r/rust/comments/3vblc5/soma_simple_solution_to_rfc_1303/cxnhbfs
[soma]: https://github.com/hauleth/soma

# Detailed design
[design]: #detailed-design

It would be simple as:

```rust
macro_rules! try {
    // existing definition

    ($expr:expr => return) => (match $expr {
        $crate::result::Result::Ok(val) => val,
        $crate::result::Result::Err(..) => return,
    });
}
```

# Drawbacks
[drawbacks]: #drawbacks

I am not so sure if it has raison d'etre in Rust `libcore` but I think that we
should at least discuss it's usability.

# Alternatives
[alternatives]: #alternatives

Left as is. We can use `if let …` syntax in the same manner.

# Unresolved questions
[unresolved]: #unresolved-questions

Would it be usable and would be seen as a good practise to implicitly reject
`Err(…)` value?
