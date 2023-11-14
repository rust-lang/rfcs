- Feature Name: `path_bases`
- Start Date: 2023-11-13
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Introduce shared base directories in Cargo configuration files that in
turn enable base-relative `path` dependencies.

# Motivation
[motivation]: #motivation

While developing locally, users may wish to specify many `path`
dependencies that all live in the same local directory. If that local
directory is not a short distance from the `Cargo.toml`, this can get
unwieldy. They may end up with a `Cargo.toml` that contains

```toml
foo = { path = "/home/jon/dev/rust/foo" }
bar = { path = "/home/jon/dev/rust/bar" }
baz = { path = "/home/jon/dev/rust/ws/baz" }
```

This is not only frustrating to type out, but also requires many changes
should any component of the path change. For example, if `foo`, `bar`,
and `ws/baz` were to move under a sub-directory of `libs`, all the paths
would have to be updated. If they are used in more than one local
project, each project would have to be updated.

As related issue arises in contexts where an external build system may
make certain dependencies available through vendoring. Such a build
system might place vendored packages under some complex path under a
build-root, like

```
/home/user/workplace/feature-1/build/first-party-package/first-party-package-1.0/x86_64/dev/build/private/rust-vendored/
```

If a developer wishes to use such an auto-vendored dependency, a
contract must be established with the build system about exactly where
vendred dependencies will end up. And since that path may not be near
the project's `Cargo.toml`, the user's `Cargo.toml` may end up with
either an absolute path or a long relative path, both of which may not
work on other hosts, and thus cannot be checked in (or must be
overwritten in-place by the build system).

The proposed mechanism aims to simplify both of these use-cases by
introducing named "base" paths in the Cargo configuration
(`.cargo/config.toml`). Path dependencies can then be given relative to
those base path names, which can be set either by a local developer in
their user-wide configuration (`~/.cargo/config.toml`), or by an
external build system in a project-wide configuration file.

This effectively makes a "group" of path dependencies available at some
undisclosed location to `Cargo.toml`, which then only has to know the
layout to path dependencies _within_ that directory, and not the path
_to_ that directory.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

If you often use path dependencies that live in a particular location,
or if you want to avoid putting long paths in your `Cargo.toml`, you can
define path _base directories_ in your [Cargo
configuration](https://doc.rust-lang.org/cargo/reference/config.html).
Your path dependencies can then be specified relative to those
directories.

For example, say you have a number of projects checked out in 
`/home/user/dev/rust/libraries/`. Rather than use that path in your
`Cargo.toml` files, you can define it as a "base" path in
`~/.cargo/config.toml`:

```toml
[base_path]
dev = "/home/user/dev/rust/libraries/"
```

Now, you can specify a path dependency on a library `foo` in that
directory in your `Cargo.toml` using

```toml
[dependencies]
foo = { path = "foo", base = "dev" }
```

Like with other path dependencies, keep in mind that both the base _and_
the path must exist on any other host where you want to use the same
`Cargo.toml` to build your project.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Configuration

`[base_path]`

* Type: string
* Default: see below
* Environment: `CARGO_BASE_PATH_<name>`

The `[base_path]` table defines a set of path prefixes that can be used to
prepend the locations of `path` dependencies. Each key in the table is the name
of the base path and the value is the actual file system path. These base paths
can be used in a `path` dependency by setting its `base` key to the name of the
base path to use.

```toml
[base_path]
dev = "/home/user/dev/rust/libraries/"
```

The "dev" base path may then be referenced in a `Cargo.toml`:

```toml
[dependencies]
foo = { path = "foo", base = "dev" }
```

To produce a `path` dependency `foo` located at
`/home/user/dev/rust/libraries/foo`.


## Specifying Dependencies

A `path` dependency may optionally specify a base path by setting the `base` key
to the name of a base path from the `[base_path]` table in the configuration.
The value of that base path in the configuration is prepended to the `path`
value to produce the actual location where Cargo will look for the dependency.

If the base path is not found in the `[base_path]` table then Cargo will
generate an error.

```toml
[dependencies]
foo = { path = "foo", base = "dev" }
```

Given a `[base_path]` table in the configuration that contains:

```toml
[base_path]
dev = "/home/user/dev/rust/libraries/"
```

Will then produce a `path` dependency `foo` located at
`/home/user/dev/rust/libraries/foo`.

# Drawbacks
[drawbacks]: #drawbacks

1. There is now an additional way to specify a dependency in
   `Cargo.toml` that may not be accessible when others try to build the
   same project. Specifically, it may now be that the other host has a
   `path` dependency available at the same relative path to `Cargo.toml`
   as the author of the `Cargo.toml` entry, but does not have the `base`
   defined (or has it defined as some other value).

   At the same time, this might make path dependencies _more_ re-usable
   across hosts, since developers can dictate only which _bases_ need to
   exist, rather than which _paths_ need to exist. This would allow
   different developers to host their path dependencies in different
   locations from the original author.
2. Developers still need to know the path _within_ each path base. We
   could instead define path "aliases", though at that point the whole
   thing looks more like a special kind of "local path registry".
3. This introduces yet another mechanism for grouping local
   dependencies. We already have [local registries, directory
   registries](https://doc.rust-lang.org/cargo/reference/source-replacement.html),
   and the [`[paths]`
   override](https://doc.rust-lang.org/cargo/reference/overriding-dependencies.html#paths-overrides).
   However, those are all intended for immutable local copies of
   dependencies where versioning is enforced, rather than as mutable
   path dependencies.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This design was primarily chosen for its simplicity — it adds very
little to what we have today both in terms of API surface and mechanism.
But, other approaches exist.

Developers could have their `path` dependencies point to symlinks in the
current directory, which other developers would then be told to set up
to point to the appropriate place on their system. This approach has two
main drawbacks: they are harder to use on Windows as they [require
special privileges](https://docs.microsoft.com/en-us/windows/security/threat-protection/security-policy-settings/create-symbolic-links),
and they pollute the user's project directory.

For the build-system case, the build system could place vendored
dependencies directly into the source directory at well-known locations,
though this would mean that if the source of those dependencies were to
change, the user would have to re-run the build system (rather than just
run `cargo`) to refresh the vendored dependency. And this approach too
would end up polluting the user's source directory.

An earlier iteration of the design avoided adding a new field to
dependencies, and instead inlined the base name into the path using
`path = "base::relative/path"`. This has the advantage of not
introducing another special keyword in `Cargo.toml`, but comes at the
cost of making `::` illegal in paths, which was deemed too great.

Alternatively, we could add support for extrapolating environment
variables (or arbitrary configuration values?) in `Cargo.toml` values.
That way, the path could be given as `path =
"${base.name}/relative/path"`. While that works, it's not trivially
backwards compatible, may be confusing when users try to extrapolate
random other configuration variables in their paths, and _seems_ like a
possible Pandora's box of corner-cases.

The [`[paths]`
feature](https://doc.rust-lang.org/cargo/reference/overriding-dependencies.html#paths-overrides)
could be updated to lift its current limitations around adding
dependencies and requiring that the dependencies be available on
crates.io. This would allow users to avoid `path` dependencies in more
cases, but makes the replacement more implicit than explicit. That
change is also more likely to break existing users, and to involve
significant refactoring of the existing mechanism.

We could add another type of local registry that is explicitly declared
in `Cargo.toml`, and from which local dependencies could then be drawn.
Something like:

```toml
[registry.local]
path = "/path/to/path/registry"
```

This would make specifying the dependencies somewhat nicer (`version =
"1", registry = "local"`), and would ensure a standard layout for the
locations of the local dependencies. However, using local dependencies
in this manner would require more set-up to arrange for the right
registry layout, and we would be introducing what is effectively a
mutable registry, which Cargo has avoided thus far.

Even with such an approach, there are benefits to being able to not put
complex paths into `Cargo.toml` as they may differ on other build hosts.
So, a mechanism for indirecting through a path name may still be
desirable.

Ultimately, by not having a mechanism to name paths that lives outside
of `Cargo.toml`, we are forcing developers to coordinate their file
system layouts without giving them a mechanism for doing so. Or to work
around the lack of a mechanism by requiring developers to add symlinks
in strategic locations, cluttering their directories. The proposed
mechanism is simple to understand and to use, and still covers a wide
variety of use-cases.

# Prior art
[prior-art]: #prior-art

Python searches for dependencies by walking `sys.path` in definition
order, which [is pulled
from](https://docs.python.org/3/tutorial/modules.html#the-module-search-path)
the current directory, `PYTHONPATH`, and a list of system-wide library
directories. All imports are thus "relative" to every directory in
`sys.path`. This makes it easy to inject local development dependencies
simply by injecting a path early in `sys.path`. The path dependency is
never made explicit anywhere in Python. We _could_ adopt a similar
approach by declaring an environment variable `CARGO_PATHS`, where every
`path` is considered relative to each path in `CARGO_PATHS` until a path
that exists is found. However, this introduces additional possibilities
for user confusion if, say, `foo` exists in multiple paths in
`CARGO_PATHS` and the first one is picked (though maybe that could be a
warning?).

NodeJS (with npm) is very similar to Python, except that dependencies
can also be
[specified](https://nodejs.org/api/modules.html#modules_all_together)
using relative paths like Cargo's `path` dependencies. For non-path
dependencies, it searches in [`node_modules/` in every parent
directory](https://nodejs.org/api/modules.html#modules_loading_from_node_modules_folders),
as well as in the [`NODE_PATH` search
path](https://nodejs.org/api/modules.html#modules_loading_from_the_global_folders).
There does not exist a standard mechanism to specify a path dependency
relative to a path named elsewhere. With CommonJS modules, JavaScript
developers are able to extrapolate variables directly into their
`require` arguments, and can thus implement custom schemes for getting
customizable paths.

Ruby's `Gemfile` [path
dependencies](https://bundler.io/man/gemfile.5.html#PATH) are only ever
absolute paths or paths relative to the `Gemfile`'s location, and so are
similar to Rust's current `path` dependencies.

The same is the case for Go's `go.mod` [replacement
dependencies](https://golang.org/doc/modules/managing-dependencies#tmp_10),
which only allow absolute or relative paths.

From this, it's clear that other major languages do not have a feature
quite like this. This is likely because path dependencies are assumed
to be short-lived and local, and thus having them be host-specific is
often good enough. However, as the motivation section of this RFC
outlines, there are still use-cases where a simple name-indirection
could help.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- What should the Cargo configuration table and dependency key be called? This
  RFC calls the configuration table `base_path` to be explicit that it is
  dealing with paths (as `base` would be ambiguous) but calls the key `base` to
  keep it concise.
- Is there other reasonable behavior we could fall back to if a `base`
  is specified for a dependency, but no base by that name exists in the
  current Cargo configuration? This RFC suggests that this should be an
  error, but perhaps there is a reasonable thing to try _first_ prior to
  yielding an error.

# Future possibilities
[future-possibilities]: #future-possibilities

It seems reasonable to extend `base` to `git` dependencies, with
something like:

```toml
[base_path]
gh = "https://github.com/jonhoo"
```

```toml
[dependency]
foo = { git = "foo.git", base = "gh" }
```

However, this may get complicated if someone specifies `git`, `path`,
_and_ `base`.

It may also be useful to be able to use `base` for `patch` and `path`.
