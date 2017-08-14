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

### Making items public to the rest of your crate

By default, all modules are public to the rest of the crate. However, all
items inside them - like types, functions, traits, etc - are private to that
module. To make them public to the rest of the crate, the can use the `pub`
keyword:

```rust
// Private to this module
struct Foo;

// Public to the crate
pub struct Bar;
```

Users can make it public to only part of your crate using the `pub(restricted)`
syntax that already exists.

There are two significant differences from today:

* All modules which are automatically loaded are public to the rest of the
crate, rather than taking a visibility modifier.
* The `pub` keyword means *public to this crate* - the equivalent of today's
`pub(crate)`. A new keyword is introduced to mean a part of the public API;
this is discussed in a subsequent section.

### Importing items from other parts of your crate

Once a user has a `pub` item in another module, they can import it using the
`use` statement, just like items from external dependencies. Paths to items in
modules of this crate work like this:

* All these paths begin with the `crate` keyword, which means "this crate."
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
use crate::foo::Bar;
::crate::foo::Bar;
```

The major difference from today is that the `crate` keyword is used to prefix
items inside this crate, so that users can distinguish external dependencies
from modules in this crate by only looking at the import statement.

## Crafting your API

### Exporting things from your crate

When a user writing a library wants to declare something exported from their
crate, they can use the `export` visibility modifier to declare it as an
exported item:

```rust
// This is exported
export struct Foo;
```

Items marked `export` inside of files which have not been exported are not
visible outside of this crate, because the full path to that item is not
`exported`.

If a user wants to make one of their file submodules a part of their API, they
can do so using the `export keyword (no `use`), followed by the name of the
module, in that module's parent:

```rust
// In `src/lib.rs` Imagine that there is a `src/foo.rs` as well.
export foo;
```

(This is actually an instance of the re-export functionality described in the
next section.)

The major difference from today is that `export` has been added as a keyword,
meaning the same thing that `pub` does today.

### Re-exporting items to simplify your public API

Sometimes users want to make items public using a path hierarchy that's
different from the true path hierarchy which makes up the crate. This can be to
simplify the API, or to maintain backwards compatibility. Users can do this by
using the `export` keyword with a relative path (just like all non-`use`
paths):

```rust
export foo::Foo;
export bar::Bar;
```

This will create a new exported path to the item, even if the canonical path to
the item is not exported. The item itself needs to be exported, or this is an
error.

This replaces the functionality of `pub use` - users can re-export with a
visibility modifier and a path without a `use` statement.

## Deprecations

Over time, this RFC proposes to deprecate and ultimately remove this syntax
that exists today:

* `mod $ident;` - mod statements used to pick up new `.rs` files
* `extern crate $ident;` - extern crate statements
* `pub use` - re-exports using the `use` keyword
* `pub(crate)` - the `crate` visibility is no longer necessary

All paths inside a module are relative to that module, just like today.
However, you can make them absolute using the `::` prefix.

Paths prefixed `::` are from the absolute root, which is not (unlike today) the
root of this crate. To access the root of this crate, you need to prefix paths
`crate`.

[extern-crate]: https://github.com/rust-lang/rfcs/pull/2088
[loading-files]: detailed-design/loading-files.md
