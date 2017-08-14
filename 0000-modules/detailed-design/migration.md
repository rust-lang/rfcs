# Migration

The changes proposed by this RFC are significant breaking changes to the
language. To complete them entirely we will need to pass a checkpoint/epoch
boundary as described in [RFC #2052][checkpoints]. However, we will begin with
a multi-phase deprecation process to ease the transition.

We will also introduce a tool that will not only automatically move a crate
onto a new system, but also *improve clarity* of the user's code by
intentionally drawing the distinction between the `export` and `pub`
visibilities that today does not firmly exist.

## Initial, compatible mode

In the first release supporting this RFC, we will begin supporting all of the
feature set of this RFC, without breakages to existing functionality. That is:

* `--extern` and `--module` arguments will be automounted, and cargo will pass
`--module` arguments in its expected way.
* The `crate::` path prefix will be accepted.
* The `export` visibility will be accepted.
* The "vis path" re-export syntax will be accepted.

However, existing semantics will not be broken, meaning:

* Module-internal paths will be accepted from the crate root without the
`crate::` prefix.
* `pub` will continue to have its current semantics if a crate does not ever
use the `export` visibility

In other words, as soon as this RFC is implemented, users will be able to
transition to the new system, but they will not be immediately *required* to.

The only breakage during this phase arises if users have files with valid Rust
module names inside of the directory than defines a crate, which is not a
compilable module of that crate. Users are encouraged to fix that by renaming
those files so that they will not be picked up by cargo, or else deleting them
if they are not necessary. **Note** that this does not include overlapping
crates which are both managed by cargo, as when cargo detects that it will
instead pass no `--module` arguments.

### Using `export` changes semantics

One aspect of this RFC requires a bit more of a complex transition than simply
"add new features, then deprecate the old ones." That is the change in the
semantics of the `pub` keyword.

Under the current checkpoint/epoch, the `pub` keyword will continue to mean
exactly what it means prior to this RFC **unless** the crate under compilation
contains at least one `export` statement. If an `export` statement is found in
the crate, `pub` is interpreted to have its semantics defined under this RFC
(public to this crate).

## Deprecations

Deprecation will proceed in two phases:

1. First, we will deprecate code which is now superfluous as a result of this
RFC.
2. Then, at a later point, we will issue more opinionated deprecations, pushing
users to write code which is compatible with the future checkpoint transition.

### Phase 1 - deprecating dead code

#### Dead `extern crate`

We will issue deprecation warnings on extern crate statements which do not have
a `#[macro_use]` attribute. In the majority of cases, users will need to delete
these statements and take no other actions. In some cases, they will also need
to do something else as well:

* If they used `extern crate as` syntax, they will need to add an alias to
their dependency object in their Cargo.toml.
* If they mounted the extern crate somewhere other than the crate root, they
will need to use an appropriate import at that location.

#### Dead `mod`

We will issue deprecation warnings on mod declarations for which there is a
corresponding `--module` argument. For private or `pub(crate)` modules without
attributes, users will need to delete these statements and nothing else. In
other cases, they will need to do other things as well:

* If they used a visibility modifier on the module, they will need a re-export.
* If they used an attribute, they will need to move it inside the module.

Note that modules that do not have a `--module` argument are *not* considered
dead code, and continue to be used to load additional files. If cargo was not
able to generate a module list for a crate, and the user hasn't specified one,
mod statements will continue to be the way to find modules.

#### Dead `pub(crate)`

If the user is using `export` semantics, `pub` becomes equivalent to
`pub(crate)`. We will issue dead code warnings on the `(crate)` restriction if
the crate is being compiled under this RFC's semantics.

### Cargo errors when unable to generate the module list

When cargo recognizes that a crate contains the source of another inside of it,
it will not generate a module list. When it does so, it issues a warning,
encouraging users to do one of two things:

* Set their module list manually in the `Cargo.toml`.
* Rearrange their package so that their crates do not have overlapping source
directories.

### Phase 2 - opinionated deprecations

#### Deprecating all `extern crate` and `mod` statements

All `extern crate` and `mod` statements (without blocks) will be deprecating.

Users will need to be using cargo's default module loading or a manual module
list to avoid warnings.

### Deprecating crate paths not through `crate::`

Absolute paths to items in this crate which do not pass through `crate` will
receive a warning.

### Warning to move to `export` in libraries

When compiling a library, if still using the pre-`export` semantics, users will
receive a warning providing them documentation on the change to `export` and
encouraging them to make that transition.

#### Blockers on phase 2

In the second phase, all `extern crate` statements will be deprecated. Phase 2
as proposed by this RFC is, for this reason, blocked on deprecating the
`#[macro_use]` attribute on extern crates.

#### Staggering phase 2

Its possible that some of the phase 2 deprecations will be issuable sooner than
others. Its not necessary that phase 2 be initiated as a single unit, but
instead its deprecations could be introduced across multiple releases.

### Phase 3 - next checkpoint

Once phase 2 has been completely implemented, in the next checkpoint, all of
the phase 2 warnings will be made into hard errors.

## Tooling

Concurrent with the first release under this RFC, we will also release a tool
which will automatically transform projects to be consistent with this RFC (and
therefore work, without warnings, on both the current and future checkpoint).

The exact interface and distribution of this tool is left unspecified for the
implementation, but as a rough sketch, it would perform these transformations:

* If a package contains both a `lib.rs` and a `main.rs`, it will move the
binary crate into the `bin` directory.
* It will remove unnecessary `mod` and `extern crate` statements.
* It will transform `use` imports and absolute paths to include the `crate`
prefix when necessary.
* It will selectively replace `pub` with `extern` *only* when the type is
exposed in the actual external API of the crate. (For binaries, it will not
perform this change at all.) This way, the tool will automatically make code
more self-documenting.

Our aspiration is that most users will have access to this tool and be able to
use it, making this transition automatic for most users.

[checkpoints]: https://github.com/rust-lang/rfcs/pull/2052
