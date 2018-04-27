- Feature Name: Item-Level Blocks
- Start Date: 2018-03-29
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

Allow items to be grouped into "item-level blocks".

# Motivation
[motivation]: #motivation

Sometimes one wishes to guard a collection of items under some compile-time
condition. An obvious use-case for item-level blocks is for platform
abstractions. Consider the following:

```rust
// Linux

#[cfg(target_platform = "linux")]
use something::only::linux::uses;
#[cfg(target_platform = "linux")]
use more::linux::stuff;

#[cfg(target_platform = "linux")]
const SOME_SPECIFIC_PATH: &'static str = "/sys/blah/flibble/linux_specific123";

#[cfg(target_platform = "linux")]
struct LinuxThingy1 {
   ...
}

#[cfg(target_platform = "linux")]
impl LinuxThingy1 {
     ...
}

#[cfg(target_platform = "linux")]
struct LinuxThingy2 {
    ...
}

#[cfg(target_platform = "linux")]
impl LinuxThingy2 {
     ...
}

// Windows

#[cfg(target_platform = "windows")]
...

#[cfg(target_platform = "windows")]
...

#[cfg(target_platform = "windows")]
...

#[cfg(target_platform = "windows")]
...

#[cfg(target_platform = "freebsd")]
...

// FreeBSD

#[cfg(target_platform = "freebsd")]
...

#[cfg(target_platform = "freebsd")]
...

#[cfg(target_platform = "freebsd")]
...

```

Notice the repeated platform guards on each item being conditionally compiled.

Using item-level blocks, this can be written as:
```rust
// Linux

#[cfg(target_platform = "linux")] {
    use something::only::linux::uses;
    use more::linux::stuff;

    const SOME_SPECIFIC_PATH: &'static str = "/sys/blah/flibble/linux_specific123";

    struct LinuxThingy1 {
       ...
    }

    impl LinuxThingy1 {
         ...
    }

    struct LinuxThingy2 {
        ...
    }

    impl LinuxThingy2 {
         ...
    }
}

// Windows
#[cfg(target_platform = "windows")] {
    ...
}

// FreeBSD
#[cfg(target_platform = "freebsd")] {
    ...
}
```

The code is both more concise, but also easier to read due to the extra indent
level for each platform abstraction block.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

An item-level block is allowed wherever an item may appear. Item-level blocks
are useful for compile-time guarding a collection of items.

The following:

```rust
#[cfg(target_platform = "freebsd")]
use bsd;

#[cfg(target_platform = "freebsd")]
const FREEBSD_MAGIC_NUMBER: usize = 0x456;

#[cfg(target_platform = "freebsd")]
struct FreeBSDThing1 {
    ...
}

#[cfg(target_platform = "freebsd")]
struct FreeBSDThing2 {
    ...
}
```

Can be written more concisely with item-level blocks as:

```rust
#[cfg(target_platform = "freebsd")] {
    use bsd;

    const FREEBSD_MAGIC_NUMBER: usize = 0x456;

    struct FreeBSDThing1 {
        ...
    }

    struct FreeBSDThing2 {
        ...
    }
}
```

The contents of an item-level block are *not* scoped. The contents of item-level
blocks are exported at the top level of the file.

Item-level blocks may be nested, however note that only items can appear
inside. Other elements, such as control-flow expressions, may not appear inside
item-level blocks.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Implementation amounts to implementing a new kind of block, which is itself an
item. The contents of the block should be exported to the top-level of the file.

Since item-level blocks will export their contents to the top-level of a file,
if a non-item appears in a item-level block, then an appropriate error message
should be given. For example the Rust file:

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
  = note: Only items can appear in item-level blocks.

```

It should also be possible to nest item-level blocks. For example, the
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

Because an item-level block looks the same as a regular scope (i.e. `{ ... }`)
users may think that they can insert arbitrary expressions or limit the scope
of items. A good error message should dispel this concern.

# Rationale and alternatives
[alternatives]: #alternatives

The [cfg-if](https://crates.io/crates/cfg-if) crate allows the following:

```rust
cfg_if! {
    if #[cfg(target_platform = "dos")] {
        ... items
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
    pub(super) ...item
    pub(super) ...item
    ...more items
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

struct openbsd_thing {
    ...
}

#define OPENBSD_MAGIC 0x456
#endif // __SOMETHING__
#endif // __OpenBSD__
```

# Unresolved questions
[unresolved]: #unresolved-questions
