- Feature Name: string_leak
- Start Date: (2022-09-18)
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Add `String::leak` analogous to [`Vec::leak`](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.leak).

# Motivation
[motivation]: #motivation

The existing alternative of `Box::leak(string.into_boxed_str())` may reallocate the string, which is presumably why
[`Vec::leak`](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.leak) exists.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

`String::leak` works exactly like [`Vec::<u8>::leak`](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.leak) except that
it preserves the guarantee that the bytes are valid `utf8`.

For example, the following code leaks a `String` without reallocation.
```rust
let string = String::with_capacity(1000);
string.push_str("Hello world!");
let leaked: &'static mut str = string.leak();
println!("{}", leaked);
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

```rust
impl String {
    pub fn leak(self) -> &'static mut str {
        let me = self.into_bytes().leak();
        // Safety: Bytes from a [`String`] are valid utf8.
        unsafe { std::str::from_utf8_unchecked_mut(me) }
    }	
}
```

# Drawbacks
[drawbacks]: #drawbacks

`String` API has added complexity.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- This design is the best because it is analogous to the existing [`Vec::leak`](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.leak).
- I couldn't think of any other designs to get the same effect.
- The impact of not doing this is that those who want the same effect will be forced to use `unsafe` to avoid reallocation or pay the performance and code size penalty for reallocation.

# Prior art
[prior-art]: #prior-art

As previously emphasized, this change is an extension of [`Vec::leak`](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.leak).

Right now, [StackOverflow answers](https://stackoverflow.com/a/30527289) suggest using
`Box::leak(string.into_boxed_str())`, which shrinks the string to fit, possibly causing unnecessary reallocation.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

N/A

# Future possibilities
[future-possibilities]: #future-possibilities

I couldn't think of any.
