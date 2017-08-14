# Visibility and Re-Exporting

This RFC changes the hierarchy of visibility keywords in Rust, introducing a
new `export` visibility. It also introduces a new mechanism for re-exporting,
replacing `pub use`.

## Detailed design

### Visibility modifiers

The set of valid visibility modifiers is changed to be:

* `pub(self)` - visible to this module & its children
* `pub(super)` - visible to the parent module & its children
* `pub(in $path)` - visible to the given path, which must be a parent of this
module
* `pub` - visible to this entire crate
* `export` - visible in the API of this crate

The first three exist today and have no change in meaning, but there has been a
shift in the final two, represented in this table:

Today      | After this RFC
---------- | --------------
pub(crate) | pub
pub        | export

In addition, we introduce a `public-in-private` error for items marked
`export`. If a canonical item (that is, not a re-export) is marked `export` but
not actually reachable by a public path (whether through its canonical path or
through a re-export), the compiler raises a hard error about failing to
actually export that item.

Enforcing this convention will enable greater clarity around intent - if a user
means for an item to be in the public API, they will have demarcated it
distinctly. It will also be less likely for users to accidentally expose
something in their API, because their 'default' move will be to mark things
`pub` instead of `export`, and if they mark something `export`, they *know* it
is part of their public API.

Additionally, it will become more meaningful to search a crate for all
instances of `export` than it today is to search for all instances of `pub`.
After this change, that search will tend to give you a good sense of the entire
API this crate exposes, whereas today it includes many items which are marked
`pub` but are not exposed.

Finally, though redefining `pub` may seem like a very significant breakage,
practical breakages will be limited by the [migration][migration] mechanism.
Repurposing the `pub` syntax will also limit some of the "long tail" effects of
breakages. For example, if outdated documentation or Q&A content uses the `pub`
visibility in a code snippet, that code will continue to compile, whereas if a
new keyword were used, that code would not work on a newer Rust checkpoint.

The distinction between the two definitions of `pub` really only impacts
library authors who are in the process of migrating across versions. In
binaries, or even parts of libraries that aren't part of the external API, the
change in the meaning of `pub` does not make code stop working.

### New re-export mechanism

We also introduce a new mechanism for re-export, having the syntax:

```rust
$visibility $path;
```

This is similar to the `pub use` system we have today, but the `use` keyword is
omitted.

Other than the syntactic difference, another big difference is that these paths
are relative paths, the same paths used everywhere inside of modules (except
for `use` statements).

Most commonly, the visibility used will be the export visibility. When you
compare a facaded module like `futures::future`, for example, it will go from
this:

```rust
mod and_then;
mod map;
mod select;
mod then;

pub use self::and_then::AndThen;
pub use self::map::Map;
pub use self::select::Select;
pub use self::then::Then;
// etc
```

To this:

```rust
export and_then::AndThen;
export map::Map;
export select::Select;
export then::Then;
```

#### Exporting modules

Because the blockless mod statement is deprecated, we need a new mechanism for
making modules a part of the public API. In this system, modules are made
public by exporting them:

```rust
// Exports the `foo` module
export foo;
```

In this way, the syntax for mounting a module which is another file at this
location in the public API is the same as the syntax for mounting any other
item from other files at this point in the public API.

#### Private-in-public & re-exporting

It is not permissible to use this mechanism to re-export something at a greater
visibility than its explicitly declared visibility. That is, this would be
invalid:

```
mod foo {
    pub struct Foo;
}

// This is an error: you have exported something which is only marked `pub`
export foo::Foo;
```

The exception to this is modules which have not been explicitly created with
the `mod` keyword. Because they have no explicit statement of their visibility,
it is permitted (and idiomatic) to use a re-export to make a module more
visible by exporting it, as discussed in the previous section.

You also cannot re-export something at the same location if it has been
explicitly declared there - again, this means modules are exempt from this
rule. For example, this code would be invalid:

```rust
export struct Foo;

// This is an error: you have re-exported something at a path it already is
// mounted at.
export Foo;
```

#### Deprecated `pub use`

As a result of this RFC, visibility attributes on `use` will be phased out.
Ultimately, `use` will not take a visibility modifier, like impl blocks.

## Drawbacks

The biggest drawback of this change is that redefining `pub` involves a complex
[migration][migration]. This is the only change in this RFC that does not
follow a strict "add and deprecate cycle." For reasons outlined in the detailed
design section, we believe this is the right choice, but it carries some costs.

## Alternatives

An alternative is to use a new keyword (such as `public`) and deprecating `pub`
entirely. The migration here would be less complex, but more disruptive, in
that every `pub` statement would need to change, and not only those that are
meant to be `export`.

We could also introduce `export` as a new kind of statement, similar to `use`,
but for the purpose of re-exporting. We explored some schemes along these
lines, and found that the visibility-based system proposed above had the most
appealing properties, such as grepability and clarity of naming.

We could make the error on a non-exposed `export` item a warning instead of a
hard error.

[migration]: 0000-modules/detailed-design/migration.md
