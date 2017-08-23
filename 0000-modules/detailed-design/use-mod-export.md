# Use, mod, and export

## Detailed design

### The `use` statement

Like implementation blocks, `use` statements are not "items" in the sense that
they do not create new names in the item hierarchy, and for that reason do not
have their own visibility modifier.

Instead, `use` statements just bring names into scope for the resolution of
paths within the same scope that they are declared in (whether that be module,
function, etc), without creating declared items of those names at this point in
the module hierarchy.

`use` statmeents with a visibility (such as `pub use`) are deprecated, and
imports that have taken advantage of the item-like behavior of use (such as
`use super::*` picking up imports in the parent module) will receive warnings
that can be fixed by explicitly importing the items picked up this way.

### The `mod` statement

In this system, the `mod` statement still exists; it is primarily used as a
way to control the visibility of submodules.

If a module is not in the `--modules` list, it will still be added by `mod`
statements, just as it would be today. But if it is in the modules list, the
mod statement can be used instead to control the visibility of the module. For
example:

```rust
pub mod foo;
local(super) mod bar;
local(self) mod baz;
```

If a module is in the `--modules` list, and the user has declared it with a
`local` visibility, this is redundant (as `--modules` loaded modules are
`local` inherently). A lint warns on dead code for `local` modules that are
already loaded from the `--modules` list, the `redundant_mod_statements`.

#### Cfg attributes on `mod` statements

This RFC specifies that cfg attributes on mod statements will be treated
analogously to `#!` attributes on the inside of modules. There is a
possibility, in conjunction with the changes to how files are loaded, that
these attributes will have surprising behavior.

For example, consider this code:

```rust
#[cfg(test)]
local mod tests;
```

In a naive implementation, this could remove the mod statement, but because the
module is loaded through the `--modules` list, the module itself is still
loaded, even outside of the test cfg. This RFC specifies that this cfg
attribute is treated as applied to *the module*, not just this statement.

### The `export` statement

The `export` statement is used for re-exporting items located at a different
point. It takes a path, which, like all paths other than those in `use`
statements, is a relative path by default. `export` replaces `pub use` for this
purpose:

```rust
pub export foo::Bar;
local export foo::Baz;
```

It is an error to export an item at a greater visibility than its declared
visibility on its canonical declaration.

The reason to introduce this, instead of continuing to use `pub use`, is that
it takes a relative path, which is usually what users want for re-exports, and
it allows `use` to be simpler (not have item semantics, not possible to take a
visibility), leading to a more incremental learning process.

### Required visibility on `mod` and `export`

Because the `mod`-semicolon statement (not `mod` with a block) and the `export`
statement are both about configuring the visibility of an API, they will both
require a visibility statement, rather than having a default visibility. For
example, these will become errors:

```rust
mod foo;
export foo::Bar;
```

And you will have to specify their visibilities, as in:

```rust
local(self) mod foo;
pub export foo::Foo;
```

Possibly after use under the new system shows certain visibilities are
overwhelmingly common, we can add a default visibility during the next
checkpoint or later.

On the current checkpoint, bare `mod` statements will receive a warning. They
will have the exact same behavior, with no warning, by amending them to
`local(self)`.

## Drawbacks

The main drawbacks of this specific section of the RFC are that it deprecates
several things:

* Use statements as items and `pub use` statements are deprecated
* Mod statements without visibilities are deprecated

It also introduces a new form, `export`, which today is just a "use statement
with a visibility." While this change has motivation, it comes at the cost of
some churn.

## Alternatives

The changes here are specifically designed to interact well with the other
changes to how files are loaded and the new `local` visibility. The main
alternative that only impacts this part of the proposal would be to not
introduce the `export` statement, and instead keep the current behavior of `pub
use`.
