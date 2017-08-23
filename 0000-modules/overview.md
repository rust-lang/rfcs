# Overview of the new system

This is intended to be a "guide-level" overview of the new module system, to
give readers a sense of how things will work after this RFC is fully
implemented.

This document is divided into two sections: one about structuring your project,
and the other about crafting your API. This distinction is intentional: more
users contribute to binary projects than to libraries, and it is especially
true that new users are more likely to start with binaries than libraries.
Binaries do not have significant public APIs. Libraries, in contrast, care that
their public APIs are well-structured & also stable across versions.

This system is intended to be incremental - users contributing to binary
projects only need to worry about half of the system, whereas the second half
can be reserved to users contributing to libraries. The first section, about
internal project structure, applies to both binaries and libraries. The second
section applies only to users writing libraries.

## Structuring your project

### Adding an external dependency to your project

To add a new dependency to a project managed with cargo, users need to edit
their `Cargo.toml`. This is the only thing they need to do to make that
dependency available to them:

```toml
[dependencies]
serde = "1.0.0"
```

Once a user has done this, they will be able to use anything from that
dependency's public API in their project, using the `use` statement or absolute
paths. These paths look exactly like they look today:

```rust
use std::collections::HashMap;
use serde::Serialize;
::std::iter::Iterator
::serde::de::DeserializeOwned
```

The major difference from today is that `extern crate` declarations are no
longer necessary. This change however is covered in [RFC #2088][extern-crate],
not directly by this RFC.

### Adding a new file to your crate

When a user wants to break their project into multiple files, they can do so by
creating a new `.rs` file in the directory that contains their crate root (or a
subdirectory thereof). For most users, this would be the `src` directory.

This file will automatically be picked up to create a new **module** of the
user's crate. The precise mechanism is discussed in [another
section][loading-files] of the RFC. In brief, cargo will find the file and tell
rustc to treat it as a new module using an interface that is also available to
non-cargo users. Cargo users who do not want this behavior to be automatic can
instead specify their modules using a field in their `Cargo.toml`.

The major difference from today is that a `mod` statement is no longer
necessary to get rustc to look up and load your files. The `mod` keyword still
exists to support inline modules (`mod foo { ... }`).

### Making items visible to the rest of your crate

By default, all modules are visible to the rest of the crate. However, all
items inside them - like types, functions, traits, etc - are private to that
module. To make them visible to outside of the module, users add a visibility
modifier. There are two visibility modifiers in the future Rust:

* `pub` - this means them item is visible in the public API of this crate
* `local` - this means the item is visible only inside this crate. It can take
a modifier to restrict it even further, like `local(super)`.

For users writing binaries, the difference between `pub` and `local` does not
matter. For users writing libraries, `pub` items which are not actually
accessible from outside the crate will be linted against.

```rust
// Private to this module
struct Foo;

// Publicly visible
pub struct Bar;
```

There are two significant differences from today:

* All modules which are automatically loaded are public to the rest of the
crate, rather than taking a visibility modifier.
* The new `local` visibility is added, equivalent to `pub(crate)`, and
restricted visibilites are moved to the `local` visibility instead of `pub`.

### Importing items from other parts of your crate

Once a user has a public item in another module, they can import it using the
`use` statement, just like items from external dependencies. Paths to items in
modules of this crate work like this:

* All these paths begin with the `local` keyword, because they are local paths.
* All modules in the `src` directory are mounted in the root module, all
modules in other directories are mounted inside the module for that directory.
* Items are mounted in the module that defines them.

So if a user has a structure like this:

```
// src/foo.rs
pub struct Bar;
```

They can access it with `use` and absolute paths like this:

```rust
use local::foo::Bar;
::local::foo::Bar;
```

The major difference from today is that the `local` keyword is used to prefix
items inside this crate, so that users can distinguish external dependencies
from modules in this crate by only looking at the import statement.

## Crafting your API

### Exporting things from your crate

When a user writing a library wants to declare something exported from their
crate, they can use the `pub` visibility modifier to declare it as a part of
their public API:

```rust
// This is exported
pub struct Foo;
```

Items marked `pub` inside of files which are not public are not visible outside
of this crate, because the full path to that item is not `pub`. Instead, they
need to provide a public mod statement to make that module public:

```rust
// In `src/lib.rs` Imagine that there is a `src/foo.rs` as well.
pub mod foo;
```

Mod statements exist in this system to control the visibility of modules. If
the `local` visibility is sufficient for your use cases, you do not need to use
`mod` statements.

### Hiding items that shouldn't be exported

In a library, items which are marked `pub` must be publicly accessible in the
library's API; a lint warns users when they are not. For items which are not
intended to be a part of the API, the `local` visibility makes them visible
only inside the library:

```rust
// only visible "locally" - inside this crate
local struct Foo;
```

Even in a public module, a local item won't be visible in the public API. The
local visibility can also take restrictions, to make it more restricted than
the default localness, such as:

```rust
// Only visible in the parent module
local(super) struct Foo;
```

The `local` visibility takes the same role that `pub(restricted)` takes in the
current system.

### Re-exporting items to simplify your public API

Sometimes users want to make items public using a path hierarchy that's
different from the true path hierarchy which makes up the crate. This can be to
simplify the API, or to maintain backwards compatibility. Users can do this by
using the `export` keyword with a relative path (just like all non-`use`
paths):

```rust
local mod foo;
local mod bar;

pub export foo::Foo;
pub export bar::Bar;
```

This will create a new public path to the item, even if the canonical path to
the item is not public. The item itself needs to be marked `pub`, or this is an
error.

This replaces the functionality of `pub use`.

## Opt-outs

There are two primary mechanisms for opting out of this system: one, for
temporarily avoiding compiling a file that would otherwise be compiled, during
development, and the other, for making mod statements with visibility mandatory
for your project.

### The `#[ignore]` attribute

If you want a file to be ignored, you can use the `#[ignore]` attribute on the
top of it. This file will not be compiled and you will not get errors from it
unless they were very early parsing errors. That is, this file will not need to
pass name resolution, typechecking, or ownership & borrow checking. We will
make a best effort to make the ignore attribute read as early as possible.

### The `load-files` directive

If you want module statements to always be required to load files in a crate,
every target section of the Cargo.toml has a `load-files` directive, which can
be set to false. This will prevent cargo from telling Rust about module files,
making `mod` statements necessary to find files.

[extern-crate]: https://github.com/rust-lang/rfcs/pull/2088
[loading-files]: detailed-design/loading-files.md
