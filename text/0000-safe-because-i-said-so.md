- Feature Name: justify_safety_annotation
- Start Date: 2017-02-03
- RFC PR: 
- Rust Issue: 

# Summary
[summary]: #summary

Add a `#[safe("Reason")]` to annotate why unsafe blocks are actually safe.
Also add a lint to the compiler to forbid unsafe blocks without a safe
annotation. Add a `#[warn(unexplained_unsafe)]

# Motivation
[motivation]: #motivation

Simply put, [IntoIter.into_mut_slice](https://github.com/rust-lang/rust/pull/39466)
got into a stable release of Rust.

This is a technical fix to try to prevent this from happening again. It alone
is not sufficient as it still requires us to actually check the reason for
being safe is actually valid.

More generally, this annotation and warning helps establish a baseline process
for showing proof that your unsafe code is safe to fellow programmers.

# Detailed design
[design]: #detailed-design

## Safe Annotation

Add a new annotation `safe` that is only permitted on unsafe blocks, unsafe 
impls, and modules.
The annotation specifically is not allowed on unsafe functions or traits. It
takes as its only argument an arbitrary string. This annotation is discarded as
a comment in the same way that documentation comments are.

## Unexplained Unsafe Lint

Add a new lint `unexplained_unsafe`. This lint is triggered by an unsafe
block or impl that does not have a corresponding safe annotation, either on
itself or on the module containing it. A `safe` attribute on a module This lint
is not a warning by default.

## Example

```rust
fn silly_unsafe_noop(num: i32) -> i32 {
    #[safe("
        Transmuting from T to U and then from U to T is a no-op as long as both
        T and U are the same size and U has no invalid values. i32 and u32 are
        the same size, and u32 has no invalid values.
    ")]
    unsafe {
        let num = std::mem::transmute::<i32, u32>(num);
        std::mem::transmute::<u32, i32>(num)
    }
}
```

# How We Teach This
[how-we-teach-this]: #how-we-teach-this

The `#[safe("...")]` annotation should be taught alongside `unsafe`. Anywhere
we talk about proving the unsafe code is safe, we should be putting the proof
in this annotation. We should also be teaching that an unsafe block is
fundamentally different than an unsafe function in how safety should not leak
from a block.

For existing users, we would add a paragraph to the release notes showing
an example of forgetting the annotation, and then show the warning catching it.
We would also have posts in users.rust-lang.org and Reddit and people would
talk about it on IRC.

The new annotation and the new warning need to be documented in the reference.

_The Rust Programming Language_ would need to be updated as well.

The new _The Rust Programming Language_ does not seem to mention unsafe code
at all.

I've not check _Rust by Example_ for unsafe examples, but where there are any,
they would also need to be updated.

The _Rustonomicon_ would need to be updated. 

# Drawbacks
[drawbacks]: #drawbacks

It's just as easy to copy a `#[safe]` as it is to copy the unsafe block. To that
effect, it's also easy to have an IDE automatically put in a `#[safe("TODO")]`
before every unsafe block as you write them.

It's also just as easy to update the unsafe block without updating the reason
it is still safe.

The attribute might convince people to use terse explanations that are hard
to understand. This could be allieviated with doc comment like syntactic sugar.

It is annoying to justify every usage of unsafe.  For example, when writing
FFI code, there's typically a lot of unsafe when casting between raw pointers
and smart pointers.

# Alternatives
[alternatives]: #alternatives

Do nothing. There's already a pre-commit hook in the ecosystem for `// SAFE:`
comments. This RFC tries to reify the behaviour into the language itself, as
it puts a strong convention around actually giving the proof

Adding syntactical sugar in the same way `///` is sugar for a `#[doc()]`
attribute. This can still be done, but requires more design work.

Make the warning about missing safe annotations be an error. We cannot do this
because it would be backwards incompatible.

Don't use a `safe` annotation. Instead, allow a string after `unsafe` before the
block actually opens. Doing so would probably be unreadable because we're
forcing the comment to be in the middle of an expression, and would make
people really favor having terse descriptions. I've also seen other people want
something more executable there.

Put the `safe` annotation on every unsafe function call. This would make it hard
to declare safety because not every individual unsafe call is safe on its own.

Don't use a `safe` annotation. Instead, just lint for `// SAFE:` before unsafe
blocks. This would make the comment effectively be part of Rust's syntax, which
we may want to do, but we should also have an annotation that such comments
desugar into.

Make the lint a warning by default. This could be done in the future. Right now,
people do not want it to be a warning by default.

# Unresolved questions
[unresolved]: #unresolved-questions

The actual names for the annotation and warning are up for bikeshedding. I don't
want to get into a naming argument in the RFC.