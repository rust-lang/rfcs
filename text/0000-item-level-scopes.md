- Feature Name: Item-Level Scopes
- Start Date: 2018-03-29
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

Allow Items to be grouped into "Item-level scopes".

# Motivation
[motivation]: #motivation

Sometimes one wishes to import a handful of functions/data structures from
other modules, but only under some compile-time condition. For example, we
might like to conditionally import some stuff only if we are in debug mode like
this:

```rust
#[cfg(debug_assertions)]
use a;
#[cfg(debug_assertions)]
use b;
#[cfg(debug_assertions)]
use c;
#[cfg(debug_assertions)]
use d
```

The above example is quite verbose because the `#[cfg...]` line must be
repeated for each `use` line. This RFC proposes that the following should be
possible:

```rust
#[cfg(debug_assertions)] {
    use a;
    use b;
    use c;
    use d;
}
```

The latter is more concise and thus easier to read and write.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

An Item-level scope is allowed wherever an Item may appear. Item-level scopes
are useful for (e.g.) compile-time guarding a collection of `use` statements.

The following:

```rust
#[cfg(debug_assertions)]
use a;
#[cfg(debug_assertions)]
use b;
#[cfg(debug_assertions)]
use c;
#[cfg(debug_assertions)]
use d
```

Can be written more concisely as:

```rust
#[cfg(debug_assertions)] {
    use a;
    use b;
    use c;
    use d;
}
```

Item-level scopes may be nested, however note that only Items can appear
inside. Other elements, such as control-flow expressions, may not appear inside
Item-level scopes.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Implementation amounts to implementing a new kind of scope, which is itself an
Item.

If a non-Item appears in a Item-level scope, then an appropriate error
message should be given. For example the Rust file:

```rust
#[cfg(debug_assertions)] {
    use a;
    for _ in 0..3 {
        println!("x");
    }
}
```

Should fail with an error message like:

```
error: expected item, found `for`
 --> src/main.rs:3:5
  |
3 |     for _ in 0..3 {
  |     ^^
  |
  = note: Only items can appear in item-level scopes.

```

It should also be possible to nest Item-level scopes. For example, the
following should be valid:

```rust
#[cfg(debug_assertions)] {
    use a;
    use b;
    #[cfg(target_os = "linux")] {
        use c;
        use d;
    }
}
```

# Drawbacks
[drawbacks]: #drawbacks

Because an Item-level scope looks the same as a regular scope (i.e. `{ ... }`)
users may think that they can insert arbitrary expressions. A good error
message should dispel this concern.

# Rationale and alternatives
[alternatives]: #alternatives

The [cfg-if](https://crates.io/crates/cfg-if) crate allows the following:

```rust
cfg_if! {
    if #[cfg(debug_assertions)] {
        use a;
        use b;
        use c;
        use d;
    }
}
```

However, this is more verbose, and requires using an external crate.

The closest you can get without using a crate is by using a submodule trick:

```rust
#[cfg(debug_assertions)]
use self::imports::*;
#[cfg(debug_assertions)]
mod imports {
    pub(super) use a;
    pub(super) use b;
    pub(super) use c;
    pub(super) use d;
}
```

Which seems overly verbose and complicated.

# Prior art
[prior-art]: #prior-art

This feature should be welcomed by C/C++ programmers who are used to the
following idiom:

```C
#if defined(__OpenBSD__)
#include <something.h>
#ifdef __SOMETHING__
#include <another.h>
#endif // __SOMETHING__
#endif // __OpenBSD__
```

# Unresolved questions
[unresolved]: #unresolved-questions
