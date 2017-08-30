- Feature Name: must_use_fn
- Start Date: 2017-08-31
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

The feature `#[must_use = reason]` can be used as annotation of a type to enforce that any function
returning a value of such a type should be used – `Iterator`s are a good example. This works great
with type which inhabitants must always be used. However, it would be preferable to have a finer
control on when a value must be used. This RFC enables the use of the `must_use` annotation on
functions to state a per-value (return value) semantic instead of a per-type semantic.

# Motivation
[motivation]: #motivation

The concept of *must use* is lead by a context and a domain of application. For instance, iterators
must be used because they’re always lazy. However, sometimes, the concept is not directly expressed
via a type. One could imagine a network primitive that would send bytes over as socket. In order to
be sure we’re doing it right, we would like to ensure the programmer uses the returned value of such
a function. We could – as [`Write` trait](https://doc.rust-lang.org/std/io/trait.Write.html) does
it – have a function returning `Result<usize>`. But we couldn’t enforce that such a returned value
must be used, because it’s currently forbidden to annotate such values.

However, it’s not that dark: since you can annotate a type, you could do the following:

```rust
#[must_use = "you must read how many bytes were sent over the network"]
pub struct ReadBytes(Result<usize>);

impl WriteSocket {
  pub fn send(&mut self, bytes: Bytes) -> ReadBytes {
    // elided
  }
}
```

That would work. However, it’s just a trick and a way to hack around a missing feature. This RFC
fills in the gap.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The `#[must_use = reason]` annotation can be used to express the *must use* concept.

## On a per-value context

It’s possible to annotate a function with `#[must_use = reason]`. This will cause any unused value
from a call to that function to trigger a warning at compilation with the reason of why it must be
used. This is slightly different than an unused variable.

```rust
#[must_use = "you must use the name"]
fn get_name() -> String {
  // …
}

fn main() {
  let x = get_name(); // unused x variable
  get_name(); // unused return value that must be used
  let _ = get_name(); // no warning
}
```

## On a per-type context

It’s also possible to derive such a behavior for a given type by putting the annotation directly
on the type definition, as it’s already the case with `Iterator` adapters, for instance. If a
function returns a type annotated with the `#[must_use = reason]`, it behaves as if it would
automatically forward the annotation to the function as well.

## Overlapping annotations

If a function returns a type `Foo` that is annotated with a `#[must_use = reason]`, annotating that
function with another `#[must_use = reason]` should trigger both warnings at compilation, so that
no overriding is possible – hence no less of information.

## Point of interest of such a feature

Adding more power at compilation brings more interesting crates. By being able to tune the behavior
of the compiler and providing people with customizable warnings both per-value and per-type
is a great opportunity to enhance our community’s crates. This feature is not intended to target
*end-user programmers*, only *crate developers*.

## Teaching to existing Rust programmers

Simply put, this feature brings the `#[must_use = reason]` annotation to the per-value level by
annotating a function definition. If the return type is also annotated (at its definition level),
both warnings are triggered when an unused result is found.

## Teaching to the newcomers

The whole section [guide-level-explanation] – without the subsection about experienced Rustaceans –
should be enough.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Interaction with other features

No interaction with other features is to foresee.

## How would it be implemented

- It must be possible to put `#[must_use = reason]` annotations on functions
- When a function call’s result is not used (no `let`):
  1. if the corresponding type has the annotation but not the function, trigger the warning with the
     reason defined in the type’s annotation;
  2. if the function has the annotation but not the type, trigger the warning with the reason
     defined in the function’s annotation;
  3. if they both have the annotation, trigger both the warnings.

# Drawbacks
[drawbacks]: #drawbacks

We can already do it with a few work around – type wrappers. It forces us to reason with types,
which is far from a bad thing. However, reasoning in terms of values is also a good thing.

# Rationale and Alternatives
[alternatives]: #alternatives

Adding that feature to the language will help us bring more interesting crates with more interesting
semantics at the compilation level. There are no direct alternatives to such a design. The only way
to mimick such a behavior is the type wrapper and using the already existing `#[must_use = reason]`
annotation on such a type.

# Unresolved questions
[unresolved]: #unresolved-questions

This RFC is missing some points about why it’s not already implemented and why the current design
only applies to types. Please someone provide such a useful information. Maybe there’s a very good
reason why it wasn’t implemented upfront (C++ has it, for instance).
