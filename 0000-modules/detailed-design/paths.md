# Paths

Today, the `::` "root" path refers to the crate root. Under this RFC, this will
be changed, so that instead it refers to a root which contains all `--extern`
dependencies, as well as the current crate under the special `crate` module.

This means that imports from within this crate will be distinguished from
imports from other crates by being initialized with the `crate` keyword:

```rust
// Import from a dependency
use std::fs::File;
// Import from within this crate
use crate::toplevel::Item;
```

## Detailed design

As a result of [RFC #2088][extern-crate], `extern crate` will be deprecated. In
conjunction with that RFC, all `--extern` dependencies are mounted at the
absolute path root. However, that no longer corresponds to the root file of
this crate (usually `lib.rs` or `main.rs`). Instead, that root *contains* the
root file of this crate in a module named `crate`.

`use` statements continue to take paths from the absolute root, but this no
longer corresponds to the crate root. Other paths continue to be relative by
default, but can be made absolute with an initial `::`.

The use of the `crate` keyword has several advantages:

1. Users can always tell whether a path comes from an external dependency or
from within this crate. This increases the explicit, local information. Today
you have to know if `::foo` refers to a top-level module or a dependency.
2. It is no longer the case that within the root module, relative paths and
absolute paths correspond. This correspondence has lead users to believe that
they will always correspond, causing significant confusion when they don't
correspond in submodules.
3. Our survey of the ecosystem suggests that external imports are more common
than internal imports and we believe this will continue to be true for the
reasons listed below. Thus if we are going to distinguish external dependencies
from internal ones, it makes more sense to put the distinguishing mark on
internal dependencies.
    * We encourage users to modularize their code using the `crates.io`
    ecosystem, leading to a proliferation of external dependencies.
    * Many large projects are broken into several crates, making even
    intra-project dependencies "external" dependencies for the purposes of
    paths.
4. `crate` is already a reserved word, the current purpose of which is being
deprecated. This is easier to transition to than using new syntactic forms
which would complicate the grammar & require more learning about the meaning of
sigils.
5. Several other languages work this way; most use the name of this package as
the distinguisher, rather than a keyword, but a keyword has these advantages:
    * If a crate is renamed, you do not need to rename all internal imports.
    * Crates currently can depend on other crates with the same name; this is
    commonly employed when you want a library with the same name as a binary
    (for example, cargo). Using a keyword avoids breaking this pattern.

`crate` will act like a "special path prefix," similar to `self` and `super`.
Relative paths can begin with `crate` without beginning with a `::` prefix, to
make them relative to the crate module.

## Drawbacks

This will require migrating users away from the current path syntax in which
`crate` is not required.

These paths will be longer, being slightly less convenient to type. This is a
trade off to make it more explicit whether an import is from this or another
dependency.

## Alternatives

The primary alternative is to distinguish external dependencies instead of
distinguishing internal crate-rooted dependencies. For example, you could
imagine this instead:

```rust
use extern::serde::Serialize;
use module::Item;
```

Various syntaxes, more radically departing from the current syntax, have also
been considered, such as:

```rust
use [serde]::Serialize;
use :serde::Serialize;
from serde use Serialize;
```

We believe because of the slight weighting toward extern dependencies, the
similarity to other languages and - most importantly - the symmetry this
introduces for `use` statements in the root and other modules (reducing path
confusion), distinguishing local imports is the most pragmatic choice.

We have also considered even more radical departures, such as making `use` take
local paths by default, but ultimately decided not to make such an enormous
departure from the current system.

## Potential future extensions

This RFC doesn't include any other sugar in the `use` syntax, because it
already is a large and complicated change. However, the cost of introducing
`crate` can be mitigated (and imports made shorter, in general) through some
sort of multi-tiered nesting, as in:

```rust
use crate::{
    module::Item,
    other_module::{AnotherItem, some_other_function},
};
```

[extern-crate]: https://github.com/rust-lang/rfcs/pull/2088
