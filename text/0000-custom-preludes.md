- Feature Name: custom_prelude
- Start Date: 2015-02-19
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Add a `#[prelude]` attribute that can be applied to a single module at
crate root to serve as a custom prelude for that crate.

# Motivation

There are a variety of motivations for allowing prelude customization:

* Applications and libraries are often domain-specific, and end up
  using the same set of APIs from other libraries over and over
  again. Allowing a custom prelude would drastically improve
  ergonomics in such cases, setting up an ambient namespace that is
  specialized for their domain.

* The lack of prelude customization ends up discouraging certain API
  designs that would otherwise be fine, e.g. not using extension
  traits, or not modularizing code in order to cut down on import
  work.

* Rust has long been moving away from special treatment of built-in
  types or of `std`, but today the `std` prelude is still a bit of
  magic. That means that non-`std` libraries are at a significant
  ergonomic disadvantage -- they always feel a bit "second class".

* Some libraries are already following a "prelude pattern", in which
  they export a `prelude` module which is glob-imported by clients --
  but these clients must do the import in *each* of their modules,
  making it not a true prelude.

# Detailed design

The design is a quite simple extension of the current prelude injector:

* Add a `#[prelude]` attribute that can be applied to a single module
  at crate root to serve as a custom prelude for that crate.

* Inject the public contents of the `#[prelude]` the given module as a
  glob in all modules defined in the crate (except for the prelude
  module itself). This should work via the injector that is already in place.

* If the `#[prelude]` attribute does not appear, and the crate does
  not otherwise opt out of the standard prelude, act as if
  `#[prelude]` had been applied to `std::prelude::vX` where `X` is the
  current prelude version.

By restricting the `#[prelude]` attribute to crate root, this RFC
ensures that it is still straightforward to determine which names are
in scope in a given source location. In particular, this determination
only involves the file defining a module and the file defining the
crate itself.

## Library exmaple

```rust
// Export a prelude for clients, includes only things defined in this crate
pub mod prelude {
    pub use my_module::{MyType, MyOtherType};
    pub use my_other_module::SomeExtensionTrait;
}

#[prelude]
mod local_prelude {
    pub use std::prelude::v1::*;
    pub use prelude::*; // use our own prelude
    pub use upstream_lib::prelude::* // use preludes from upstream libraries
}
```

## Application example

```rust
#[prelude]
mod prelude {
    // std actually provides multiple preludes now
    pub use std::prelude::v1::*;
    pub use std::io::prelude:*;

    // use some upstream library preludes
    pub use my_favorite_crate::prelude::*;

    // ad hoc imports from upstream libs
    pub use another_upstream::SomeType;

    // local definitions
    pub use my_module::MyType;
}
```

# Drawbacks

Adds a bit more complexity around name binding in Rust, in particular
making it mildly less local.

In an extreme case, a very large custom prelude could make it
substantially harder to reason about names in practice, but that
choice is made per-crate and does not affect downstream code.

# Alternatives

None currently proposed.
