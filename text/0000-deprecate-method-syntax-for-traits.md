- Feature Name: dep_method_call_for_traits
- Start Date: 2018-12-11
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Deprecate method call syntax for traits and require trait usage which could
become ambiguous in the future to be fully qualified from the start.

This would be a warning in Rust 2015-2018, and could only be made an error in the
next edition of Rust.

# Motivation
[motivation]: #motivation

The rust community has generated several instances of mass breakage as a result
of use of method call syntax for traits. Here's a few.

- `Ord::min` and `Ord::max` caused breakage for many users when introduced.
- `Ord::clamp` was rejected completely because it caused similar breakage.
https://github.com/rust-lang/rust/pull/44438
- `failure` broke on 2018-12-11 due to newly introduced ambiguous trait calls:
https://github.com/rust-lang-nursery/failure/issues/280


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

In prior versions of Rust you have may have seen trait functions being used without
specifying the trait name. Such as

```rust
use std::clone::Clone;

let foo = String::new();
let bar = foo.clone();
```

This usage is now deprecated. It was not clear at the time how difficult this
made it to provide stability guarantees when this was first introduced. While we
might be able to guarantee that this isn't ambiguous for this version of the crate
future versions could provide new trait methods and new trait implementations,
both of which could break this kind of code.

The correct way to do this is now

```rust
use std::clone::Clone;

let foo = String::new();
let bar = Clone::clone(&foo);
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

When rustc 2015-2018 identifies method call syntax it would emit a warning to the
user.

This warning might look something like

```
warning: method call syntax for traits is deprecated
 --> src/lib.rs:2:4
  |
2 |     foo.clone()
  |     ^^^^^^^^^^^
  |         |
  |         help: Instead write "Clone::clone(&foo)""
  |
  ```

Additionally, `cargo fix` would be enhanced to identify trait method call syntax
uses and convert them into fully qualified syntax uses.

# Drawbacks
[drawbacks]: #drawbacks

Method call syntax helps reduce the verbosity of Rust code. I'd argue this comes
at too great a cost however in that we're often missing key information to
compile the code in the future.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- We have several demonstrated instances of trait method call syntax causing mass
breakage for several crates in the Rust ecosystem.
- Alternatively we could educate the Rust community that providing new trait
functions with default implementations, or adding new traits to an existing
`struct` is a breaking change and should not be done without bumping the first
non-zero digit in the crate's version number.  This would require us to release
a lot fewer patches to all crates across the ecosystem though.

# Prior art
[prior-art]: #prior-art

The author is not aware of any prior art as this is issue is unique to Rust
and the guarantees it provides.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Why was method call syntax for traits made a language feature in the first
place? Does it fill some use case that cannot be achieved through fully
qualified syntax?

# Future possibilities
[future-possibilities]: #future-possibilities

If this RFC is accepted then we could, in the next edition of Rust, transition
use of method call syntax for traits from a warning to a full on error, which
would allow us to provide stronger stability guarantees for crates.
