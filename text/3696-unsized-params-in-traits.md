- Feature Name: `unsized_params_in_traits`
- Start Date: 2024-12-19
- RFC PR: [rust-lang/rfcs#3745](https://github.com/rust-lang/rfcs/pull/3745)
- Rust Issue: [rust-lang/rust#134475](https://github.com/rust-lang/rust/issues/134475)

# Summary
[summary]: #summary

A lint which will detect unsized parameter types in required trait methods

it will be removed once feature `unsized_fn_params` is stabilized

# Motivation
[motivation]: #motivation

To prevent confusion, soft-locking (making a user's trait unimplementable), and to 
(possibly) prevent an `ICE` from happening

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## Without the lint
Rust does not check for `T: Sized` in required trait methods, which makes this code valid:
```rust
trait Foo {
    fn bar(self, x: str); // compiles
}
```

in the above code, `self` has type `Self`, and `Self: ?Sized`; Function parameters must always be sized

And x has type `str` which is also not sized

On top of that, this also prevents a user from implementing this trait for a type
```rust
impl Foo for str {
    fn bar(self, x: str) {} // err: `str` and `str` are not sized
}
```
Basically making the trait useless
## With the lint
the lint prevents the user from making this error
```rust
#![deny(unsized_params_in_traits)]

trait Foo {
    fn bar(self, x: str); // err: `Self` and `str` are not sized
}
```
Now the user is guided to change their types to sized ones
```rust
trait Foo: Sized // remember this `Self: Sized` bound
{
    fn bar(self, x: String);
}
```

## Lint
this section contains data about the lint itself

`Default Level`: `Deny`

`Feature`: `unsized_params_in_traits` (if any, see the unanswered questions)

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This lint may clash with the `unsized_fn_params` feature (though it is internal), as the entire point of 
this feature is to allow what this lint detects.

look at the first example:
```rust
#![feature(unsized_fn_params)]
#![allow(internal_features)]
// implicit #![deny(unsized_params_in_traits)]
trait Foo {
    fn bar(self, x: str);
}
```
The code fails even though it shouldn't, which may cause confusion to the user.

if there is someway to disable this lint by default when `unsized_fn_params` is enabled, it should be implemented.

The above is pretty much only a minor inconvenience, but if the user has many nested crates (like rust itself for example, having std, proc-macro, etc. all as different crates) it may be harder than just one lint.

## `dyn` compatibility
When dealing with a `receiver` the user **may** have meant to have an unsized receiver for `dyn` compatibility[^1]

While it is confusing, it is still something that is intentional; look at the last example:
```rust
trait Foo: Sized {
    fn bar(self, x: String);
}
```

in the above code, `Foo` has the bound `Self: Sized` which makes it `dyn` incompatible[^1]. So this feature will likely require a different feature to be added along-side it for receivers in specific.


[^1]: [...] A trait is object safe if it has the following qualities [...] All associated functions must either be dispatchable from a trait object or be explicitly non-dispatchable: [...] Dispatchable functions must: [...] Not have a where Self: Sized bound (receiver type of Self (i.e. self) implies this). [...] Explicitly non-dispatchable functions require: Have a where Self: Sized bound (receiver type of Self (i.e. self) implies this).

# Drawbacks
[drawbacks]: #drawbacks

- It may be breaking but it is a a lint that will be removed
- `dyn` compatibility of some traits (mentioned above)

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This design is best as it is a lint, which will be removed when what it detects is stabilized

Not doing this would cause confusion, possible soft locking and (possibly) an `ICE`

Here are some other considered designs:
- For the dyn compatibility issues, creating a `separate lint` for receivers in specific

There aren't really that much relevant designs as all of them would either not fix the issue or not give enough time for the user to migrate their code

# Prior art
[prior-art]: #prior-art

I haven't really seen anything like this before, rust is pretty-well designed, there are some similar things (e.g. trivial bounds, thanks @oli-obk for that)

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Should this be feature gated?
  - I personally answer this with `no` as it is pretty important
- Should this be in clippy instead
  - Again, I personally answer this with `no` as it is pretty important

# Future possibilities
[future-possibilities]: #future-possibilities

None, this will not exist that long anyways, maybe like 3-9 months maximum
