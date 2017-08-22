# The `local` keyword

This RFC introduces a `local` keyword, which means "local to this crate." This
keyword is used in two contexts:

* It is a new visibility, equivalent to today's `pub(crate)`.
* The items inside of this crate are located at an absolute path under the
special `local` module, instead of at the absolute root.

We make a handful of other changes to paths and visibility to incorporate this
new keyword into the system smoothly.

## Detailed design

### Changes to absolute paths

As a result of [RFC #2088][extern-crate], `extern crate` will be deprecated. In
conjunction with that RFC, all `--extern` dependencies are mounted at the
absolute path root. However, that no longer corresponds to the root file of
this crate (usually `lib.rs` or `main.rs`). Instead, that root *contains* the
root file of this crate in a module named `local`.

`use` statements continue to take paths from the absolute root, but this no
longer corresponds to the crate root. Other paths continue to be relative by
default, but can be made absolute with an initial `::`.

The use of the `local` keyword has several advantages:

1. Users can always tell whether a path comes from an external dependency or
from within this crate. This increases the explicit, local information. Today
you have to know if `::foo` refers to a top-level module or a dependency.
2. It is no longer the case that within the root module, relative paths and
absolute paths correspond. This correspondence has led users to believe that
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
4. Several other languages work this way; most use the name of this package as
the distinguisher, rather than a keyword, but a keyword has these advantages:
    * If a crate is renamed, you do not need to rename all internal imports.
    * Crates currently can depend on other crates with the same name; this is
    commonly employed when you want a library with the same name as a binary
    (for example, cargo). Using a keyword avoids breaking this pattern.

#### Special path prefixes

The special path prefixes are `local`, `super`, and `self` (`super` and `self`
are implemented today). These prefixes are supported in both relative and
absolute paths. They can be prefixed with the `::` absolute root in either
context. That is, all of these are valid forms:

```rust
local::module::Item;
super::sibling::Item;
self::child::Item;

::local::module::Item;
::super::sibling::Item;
::self::child::Item;

use local::module::Item;
use super::sibling::Item;
use self::child::Item;

use ::local::module::Item;
use ::super::sibling::Item;
use ::self::child::Item;
```

### Changes to visibility

The `local` visibility is very similar to `pub`, except that it is not exported
in the public API of the crate. This gives it the same semantics as
`pub(crate)` has today.

Local, however, can take restrictions, using the same parenthical syntax
suppoted on `pub` today:

* `local(self)` - visible to this module and its children
* `local(super)` - visible to this module's parent and its children
* `local(crate)` - visible to this crate; the `crate` modifier is redundant,
and is linted against by dead code (but supported, for possible use in macros).
* `local(in $path)` - visible to everything under the given path, which must be
a parent of this module (such as `local(in super::super)`

Visibility restrictions on `pub` are deprecated in favor of using visibility
restrictions on `local` instead. The result of this is that `local` always
means locally visible (with modifiers giving different definitions of `local`),
while `pub` always means visible in the public API of this crate.

#### The private `pub` lint

We also introduce a lint, called `nonpublic_pub` which is error by default when
compiling a library, and allow by default when compiling a binary. The
`nonpublic_pub` lint checks that all items marked `pub` are actually, somehow,
accessible in the public API. This could mean that they are accessible at their
canonical location (where they are defined), but it could also mean that they
are re-exported publically at another point if the module they are in is not
public.

For example, this would trigger the lint, because there is no public path to
`Foo`:

```rust
mod foo {
    pub struct Foo;
}
```

But either of these would be fine:

```rust
pub mod foo {
    pub struct Foo;
}

// Foo is public at ::foo::Foo
```

```rust
mod foo {
    pub struct Foo;
}

pub export foo::Foo;

// Foo is public at ::Foo
```


## Drawbacks

This section of the RFC deprecates the current absolute path syntax for local
dependencies, as well as the current pub(restricted) syntax. This results in
some churn as users have to migrate to the new syntax. However, it does follow
the standard epoch deprecation procedures.

The `local` keyword is not a perfect fit, because it is plausible to believe it
means something more narrow than "local to this crate" - like local to this
module, or local to this function. However, it has some narrow constraints that
make it hard to find a better alternative.

## Alternatives

We've considered multiple alternative keywords for this purpose, instead of
`local`, such as `protected`, `internal`, `crate`, `lib` or a shorthand like
`loc` or `vis`. There are some constraints though:

* Shorter tends to be better than longer.
* It needs to be sensible as the prefix to a path (making sense as a spatial
designator is preferable.)
* It needs to be sensible as a visibility (an adjective is strongly
preferable).
* It can't be an existing crate on crates.io (unless its been reserved for this
purpose, as `local` has been).
* It needs to have support for the `(restricted)` syntax.

Based on these criteria, we've chosen `local`, though we aren't wedded to it
and are open to an alternative which fits these criteria better.

There is an overall alternative, proposed in a previous RFC, in which `pub`
means what `local` does in this RFC, and `export` means "part of the public
API." This path was not pursued, because it is a more radical and complex
breakage.

[extern-crate]: https://github.com/rust-lang/rfcs/pull/2088
