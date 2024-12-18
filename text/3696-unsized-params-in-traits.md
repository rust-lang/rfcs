- Feature Name: unsized_params_in_traits
- Start Date: 2024-12-18
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#134475](https://github.com/rust-lang/rust/issues/134475)

# Summary
[summary]: #summary

A (temporary) lint which detects unsized parameter in required trait methods, which will become a hard 
error in the future


# Motivation
[motivation]: #motivation

This rfc is to prevent the use from making their trait unimplementable (in some cases, for unsized types)

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Think of this lint as the `missing_fragment_specifier` feature (if thats what its called), it is only meant to be temporary and will be a hard error in the future
```rust
#![deny(unsized_params_in_traits)]

trait Foo {
    fn bar(self);
}
```
the above code fails, because of the lint; this happens because here `Self: ?Sized`

Also look at this code:
```rust
#![deny(unsized_params_in_traits)] // this is default, but here for clearness

trait Foo {
    fn bar(bytes: [u8]);
}
```
the above code would work without the lint (how did no one notice this?)

While both of the above _would_ work without the lint, you cant actually implement it
```rust
impl Foo for i32 {
    fn bar(bytes: [u8]) {}
}
```
Produces:
```
error: The Size value for `[u8]` is not known at compile time
```

So in all: this rfc is to prevent confusion

Now, if you do notice, in the [summary], i did say `required methods`; provided methods are `Sized`-checked
```rust
trait Foo {
    fn bar(self) {

    }
}
```
Produces:
```
error: the size value for `Self` is not know at compile time
```
# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

There is one feature this may clash with, and it is the feature for `unsized fn params`, the lint should be disabled when this feature is enabled; if that is possible, however if it may cause confusion to the user.

Here is the first example, but it clashes with the `unsized fn params` feature
```rust
#![feature(unsized_fn_params)]
// implicit #![deny(unsized_params_in_trait)]
#![allow(internal_features)]

trait Foo {
    fn bar(bytes: [u8]);
}
```

The above code fails, while it shouldn't due to the feature

However, it is internal so it is semi-ok

this feature will be very simple to implement (i hope), just port the `Sized`-checking logic from provided methods to required methods, if that is possible (also maybe from regular functions) and throw an error/warning/nothing depending on the lint level.

here is some rust-pseudo code:
```rust
if !req_function.
    params
    .filter(|param| !param.is_sized())
    .is_empty() {
        match lint_level {
            Allow => (),
            Warn => warn("..."),
            Deny | Forbid => error("...")
        }
    }
```
replace the `...` with: 
```
The size value for {param_type} is not know at compile time
# ...
This was previously accepted by the compiler, but it will become a hard error in the future!
# if it was the default value of 'deny' the next line would be added
`#[deny(unsized_param_in_trait)]` on by default
```
Obviously the above code isnt actually correct as it doesnt check _which_ param is unsized, it just checks if there is, (you can probably loop over the 'filter' object, and make an individual error for each one)

# Drawbacks
[drawbacks]: #drawbacks

- This could cause breaking changes, however the lint gives time for migration
- This could be intended for `dyn` compatibility (see [rationale-and-alternatives] for a way to fix this)
This drawback is about receivers, take this example
```rust
trait Bar {
    fn foo(self);
}
```
if `self` was sized checked here, and the value _should_ be consumed, then this code would be impossible without a `Self: Sized` bound, but as you know, adding that bound removes `dyn-Compatibility`
```rust
let x: &dyn Bar = &10;
```
Produces:
```
Bar is not dyn compatible
# ...
```

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This design is best as it helps users to migrate their code before it breaks

Other Considered Designs:
- as mentioned in [drawbacks], this may be intentional with `receivers` for `dyn` Compatibility
so another design is dividing this lint into two, one for receivers, and one for regular parameters
- Not making it a hard error, which could work but it may cause users to make their traits unimplementable.
- A direct hard error, though not recommended for migration purposes (mentioned above)
- Leaving it be, though again not recommended as mentioned in [motivation]

The impact of not doing this:
May cause confusion because a parameter is unsized, and the trait cannot be implemented, which is not good.

This may also cause an `ICE` in some way because the parameters are unsized

# Prior art
[prior-art]: #prior-art

This feature is not a problem in other languages, weather a type is `Sized` or not, it is abstracted away and you can never know

For example: C++ does not really have unsized types (that i know of)
Another: C# abstracts the idea of a `Sized` value

Higher level languages do not need to run on the machine directly so there is no need to know the size of a value

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Should this be a lint, or something else
- Does this need to become a hard error

# Future possibilities
[future-possibilities]: #future-possibilities

None, this isnt really something that will stay that long