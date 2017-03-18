- Feature Name: `cargo-schema-version`
- Start Date: 2017-03-12
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Explicitly version the interface Cargo presents to crates, so newer crates do
not mislead older versions of Cargo.

# Motivation
[motivation]: #motivation

In the past, Cargo has made semantic changes to the interface it presents to
crates, such that while new Cargo understands old crates, old Cargo doesn't
understand new crates.  For instance, new Cargo will automatically use a file
named `build.rs` by convention, whereas old Cargo required specifying `build =
"build.rs"` explicitly.  If old Cargo attempted to use new crates relying on
these features, it would fail at build time with difficult-to-diagnose errors.

In order to introduce further changes to Cargo in the future, we propose
introducing a Cargo interface/schema version, and a means for both `Cargo.toml`
files and crate registry indexes (such as the one managed by crates.io) to
incorporate that version.  Older versions of Cargo can then detect that version
and skip crates requiring new Cargo at dependency resolution time, or emit
meaningful errors at build time.

Schema versioning represents a dependency for multiple subsequent Cargo
features, including:

- stdlib-aware Cargo (RFC #1133)
- Improvements to cross-compilation support
- Dependencies on the version or features of the Rust language
- Dependencies on external system libraries (Issue rust-lang/cargo/#3816)
- Dependencies on arbitrary tools to be used at compile-time (such as bindgen,
  gcc, or even rustc itself)
- Non-cargo tools parsing `Cargo.toml` and/or the index (such as to generate
  Linux distribution packages, or to integrate into an [external build
  system](https://github.com/rust-lang/rust-roadmap/issues/12))

While some of these changes could potentially be accomplished by adding new
fields to `Cargo.toml` that old versions of Cargo would ignore, this presents a
significant downside in usability and functionality: A crate author using these
features *cannot* ensure that a user with an old version of Cargo will build
the code in the manner intended.  Instead, the crate author must either attempt
to cope with the failure modes of all past versions of Cargo (and the various
fields they ignore), or attempt some kind of ad-hoc mechanism for enforcing
failure in such cases.  However, any such mechanism cannot take effect any
earlier than at build time, and thus makes it impossible for Cargo to fall back
to a different version of that crate which *would* have worked.  Such a
mechanism would also likely express a stricter version requirement on the Cargo
tool than necessary, and break compatibility with non-Cargo tools parsing
`Cargo.toml` files.  The schema version introduced in this RFC allows Cargo to
take that version into account in dependency resolution.

This change will also allow for future experiments with new "unstable" Cargo
features not yet ready for use in stable crates, without allowing those
features to leak into stable crates or have their behavior set in stone.  Any
future RFC introducing such behavior should consider whether crates.io should
prohibit the use of such features entirely, or keep them separated and tagged
appropriately in indexes.

# Detailed design
[design]: #detailed-design

Introduce a new Cargo schema version, initially `1.0.0`.  Cargo will increase
the minor version when introducing any crate-visible change to Cargo behavior
that old versions of Cargo must not ignore.  (For instance, this would include
many new `Cargo.toml` stanzas, new environment variables provided to build
scripts, or assumption of some configuration through convention.)  Cargo would
increase the major version if it stops handling (or handles incompatibly) some
behavior that old Cargo handles.  (A major version increase seems unlikely to
ever happen.)  The patch version will always remain `0`.

Introduce a mechanism for `Cargo.toml` files to specify a minimum major or
major.minor version for the schema, using the semantics of the `^` operator
from Cargo semver dependencies.

Versions of Cargo predating the introduction of schema versions must not
silently ignore the schema requirement; thus, we will use a format change that
older Cargo will not understand.  `package.name` is currently a mandatory
field; if missing, old Cargo will stop and reject the package.  So, we will
move `package.name` (and the other contents of `package`) underneath a new
`package.major` or `package.major.minor` key.

Packages compatible with schema `0.0` of Cargo (the last Cargo version that
doesn't support a schema version number) will continue to write `[package]` as
they do today.  Packages that require schema `1.0` or newer will write:

```toml
[package.1]
name = "crate-name"
```

And packages that require schema `1.5` or newer will write:

```toml
[package.1.5]
name = "crate-name"
```

Semantically, `package` can contain either a single numeric key or the key
"name".  If it has a single numeric key, use that as the minimum schema major
version; that key can either contain a single numeric key or the key "name".
If it has a single numeric key, use that as the minimum schema minor version;
that key must contain the key "name".

Formalizing this schema as a grammar (for clarity expressed over the parsed and
normalized hierarchical structure of TOML, rather than the raw text), we have:

```ebnf
<pkg-meta> ::= { name = ..., version = ..., ... }
<package> ::= <pkg-meta>
            | { <number> = <pkg-meta> }
            | { <number> = { <number> = <pkg-meta> } }
<Cargofile> ::= { package = <package>, ... }
```

Concurrently, we propose an update to the registry index format (used on
crates.io) to separate packages compatible with version `0.0` from those
incompatible with it.  This will prevent old Cargo from locking in a resolution
only to encounter a `Cargo.toml` it cannot comprehend.  Crates not specifying a
minimum schema version will still generate index entries in the existing
format.  Crates specifying a minimum schema version will have their index
entries appear in a new file `cratename.idx` alongside the existing index, with
entries in the following format:

```json
{ "schema": "major.minor", "data" : { ... normal index entry ... } }
```

Old Cargo will ignore these new files entirely.  New Cargo will read both the
new and old index files, and completely ignore any entry whose schema it does
not understand.

Optionally, versions of Cargo that understand schema versions may wish to
provide a warning to the user if dependency resolution fails to find an
acceptable version of a dependency, and that dependency contains index entries
with newer schema versions than those understood by the running Cargo.

For forwards compatibility with future changes to schema versioning, such as
unstable features, versions of Cargo that understand schema versions must also
skip any index records with a `schema` key that has a richer non-semver/string
value.

# How We Teach This
[how-we-teach-this]: #how-we-teach-this

Cargo documentation (including on crates.io) should mention the schema version
when discussing the Cargo.toml format, and should provide a list of known
schema versions and the functionality associated with them.  Separately,
documentation of that functionality should mention the minimum schema version
required to use that functionality.  Any mentions of schema version
requirements should link to the explanation of schema versions.

Explanations of such features should tie in with mentions of semantic
versioning (semver) and desirable compatibility properties across the Cargo
ecosystem.  In particular, documentation of schema versioning should explain
why all crates should not automatically use the latest schema version, just as
some crates intentionally preserve compatibility with older versions of Rust.

The need for such documentation and associated examples will increase as new
Cargo features arrive that require an updated schema version; RFCs introducing
such features should include discussion of schema version requirements in their
"How We Teach This" section.

Cargo can also provide built-in guidance associated with those features.  When
a user attempts to use a new feature without declaring the associated schema
version (for instance, a new section in `Cargo.toml`), Cargo can suggest
increasing the schema version requirement.  Not all features will make such
detection straightforward, but for those that do, Cargo can provide gentle
guidance in that direction to allow users to naturally discover this mechanism
when needed.

This will not fundamentally change how we teach Rust; it represents a minor
detail of Cargo usage.

The terms "schema version" and "interface version" both seem reasonable for
this concept.  "Interface version" seems less like jargon, but seems more
likely to be confused with API interfaces; "schema version" avoids that
confusion, but seems more like jargon, and sounds like something describing the
`Cargo.toml` format alone rather than the entirety of what Cargo presents to
crates.  We recommend using the term "schema version" as the canonical *name*
for this concept, but freely using words like "interface" or "contract" as part
of descriptions of the concept and what precisely it describes the version of.

# Drawbacks
[drawbacks]: #drawbacks

Introducing a schema version for Cargo means that Cargo promises additional
stability going forward.  In practice, the amount of churn in Cargo has already
drastically decreased corresponding to its critical importance in the crate
ecosystem; however, this change would represent a more formal stability
promise.  This represents both a drawback and a step forward.

# Alternatives
[alternatives]: #alternatives

It might be possible to introduce versioning of tools like cargo and rustc
without preventing old Cargo from parsing the non-dependency information of a
crate, such as by introducing a namespace for tool names.  RFC 1707 took this
approach, with a `tool:` prefix.  However, this would require case-by-case
evaluation of every feature with old Cargo to observe its behavior, and would
make it more difficult to modify the semantics of such dependencies, such as to
handle cross-compilation robustly.

Cargo could use the version number of Cargo itself, rather than a separate
"schema" version.  Doing so would simplify Cargo, but in the crate ecosystem
that would increase the occurrence of "spuriously tight" dependencies that
depend on a Cargo newer than necessary for the features in use, as the version
would conflate the library API and the interface to crates.  That would also
make it more difficult to build other tools on or around Cargo and crates, as
well as making it likely that crate authors will accidentally overstate their
requirements.

In the registry format, rather than introduce a new index format, records with
a schema could appear in the same files as records without a schema.  However,
this would break old versions of Cargo, which will fail to parse any records
from the file at all rather than just ignoring those they do not understand.

crates.io and other registry indexes could provide full `Cargo.toml` files (as
Haskell's "Hackage" does), rather than a JSON-formatted subset of the data.
This would simplify the introduction of future extensions, by avoiding the need
to handle modifications to the index schema separately from modifications to
`Cargo.toml`.  However, this would potentially increase the storage and
download size of registry indexes.

We could support multiple schema versions simultaneously, such that old Cargo
will use an old schema, and new Cargo will use a new schema.  However, that
would introduce substantial additional complexity, and would require an
analogous mechanism for the registry index.

We could prune old deprecated syntax before introducing version 1.0.  However,
doing so would complicate the introduction of version 1.0, and would lead to
extensive controversy over what to remove.  Rust traditionally has a
deprecation cycle prior to removal; Cargo should follow a similar model, to the
extent it removes syntax at all.

The schema version could appear elsewhere in `Cargo.toml`.  RFC 1709 suggested
introducing a `package.schema-version` key.  However, tests with current Cargo
show that Cargo ignores most unknown keys with at most a warning, and cannot
take them into account during dependency resolution.  In particular, current
Cargo would ignore any of the following changes:

- A new key under `[package]` (such as `package.schema`)
- A new top-level section (such as `[schema]`).
- The absence or renaming of a `[dependencies]` section (Cargo will assume the
  crate has no dependencies)
- Any approach that was not reflected in the index

The schema version could use a quoted semver string, such as `[package.'1.5']`,
rather than separate keys for the major and minor numbers.  However, that would
make the syntax more cumbersome for the common case, and would not
significantly simplify parsing.
