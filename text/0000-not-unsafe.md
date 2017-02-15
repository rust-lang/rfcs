- Feature Name: not_unsafe
- Start Date: 2017-02-15
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Provide ability to mark unsafe-by-default entities, like foreign items, as safe.

# Motivation
[motivation]: #motivation

Foreign functions, foreign statics and union fields are defined as unsafe by
default.

Often it's statically known that calling a certain foreign function or accessing
a static variable is safe, either because the other side of FFI is under control
of the same person, or the behavior of that function/static is well documented
as being safe.

A foreign function may, for example, be a pure math function from a C library.
Or it may be a safe Rust intrinsic, which are defined as foreign functions as
well.  
A
[comment](https://github.com/rust-lang/rust/issues/36247#issuecomment-247903943)
on tracking issue for `safe_extern_statics` compatibility lint provides a use
case for safe foreign statics - modeling MMIO registers.

Currently there are two ways to communicate this statically known safety to the
compiler.

The first way is just to surround calls/accesses to unsafe items with
`unsafe` blocks on each use, asserting that those calls/accesses are indeed
safe. This is verbose and creates too many superfluous `unsafe` blocks where
only one `unsafe` would suffice, reducing the value of others "truly unsafe"
blocks.

Alternatively, a safe wrapper function can be implemented and call/access
the unsafe item internally using `unsafe` block.
This way `unsafe` is shifted from each point of use to a single location,
similarly to how unsafe trait like `Send` is implemented for a type once and
then asserted in many locations. 

This RFC proposes to shift the `unsafe` further on the foreign item itself
removing the need in wrapper function boilerplate that often serves no other
purpose than removing unsafety.

The proposal fits into 2017 roadmap as an improvement to ergonomics and
integration with other languages.

# Detailed design
[design]: #detailed-design

Functions and statics in foreign modules as well as union fields can be
prepended with `!unsafe` and safety checker will treat them as safe. Example:
```rust
extern "C" {
    !unsafe fn f();
    !unsafe static S: u8;
}

#[repr(C)]
union LARGE_INTEGER {
    !unsafe qword: u64,
    !unsafe dwords: (u32, u32),
}
```

Mutable statics cannot be declard as `!unsafe`.  
Variadic foreign functions (e.g. `printf`) cannot be declared as `!unsafe`.

# How We Teach This
[how-we-teach-this]: #how-we-teach-this

With a paragraph of text and an example.
The syntax is intuitive and mirrors existing `unsafe` functions.

# Drawbacks
[drawbacks]: #drawbacks

None known.

# Alternatives
[alternatives]: #alternatives

Use contextual keyword `safe` instead of `!unsafe`.
This goes against the general rule "something bad happens -> search for
`unsafe`". Now you'll have to search for `safe` as well.

Postpone `!unsafe` on union fields and combine it in one proposal with `unsafe`
on struct fields.

# Unresolved questions
[unresolved]: #unresolved-questions

None.
