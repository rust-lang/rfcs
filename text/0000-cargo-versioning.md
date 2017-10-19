- Feature Name: `cargo_versioning`
- Start Date: 2017-08-24
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Provide a mechanism for Cargo to add new functionality that older Cargo does
not understand, while providing a reasonable experience when using newer
`Cargo.toml`s with older versions of Cargo. Update the registry index format to
support this and include version numbers in package records. Introduce a schema
mechanism for older Cargo to reliably detect use of new Cargo functionality.

# Motivation

If you use new Cargo features, and people developing or depending on your crate
use an old version of Cargo, they should get a clear error message telling them
to upgrade. They should not get vague warnings and builds that fail in
mysterious ways.

Simply upgrading Cargo must not require other developers interacting with your
crate to upgrade, unless you use features incompatible with the old version of
Cargo.

A new `Cargo.toml` entry may be incompatible with older versions of
Cargo if it consists of:

  - A new key (or section), or
  - A new value type for an existing key, or
  - A string value that matches a specific pattern.

Constraints:

- When a `Cargo.toml` uses functionality that was only added in a new
  version of Cargo, old versions of Cargo that postdate the
  implementation of this RFC should gracefully identify the use of new
  functionality and produce an clear error.

- Users should not need to explicitly list Cargo version numbers.

- We should avoid gratuitously opting users into requiring a new version
  of Cargo (and therefore incompatibility with older Cargo) if they have
  not actually used any new functionality.

- Versions of Cargo prior to the introduction of this versioning
  mechanism should successfully resolve dependencies and build crates as
  well as possible using crates that do not use any new Cargo features.

- Versions of Cargo that understand this versioning mechanism should
  successfully resolve dependencies and build crates as well as possible
  using only crates using Cargo features they understand.

- This mechanism must function with crates obtained via git or alternate
  registries, not just from crates.io.

- If a `Cargo.lock` was generated with a version `N` of Cargo from a
  `Cargo.toml`, version `N+1` of Cargo should always emit exactly the
  same `Cargo.lock` (other than opaque metadata) for the identical
  `Cargo.toml`.

- Future Cargo RFCs that introduce features incompatible with should
  explicitly note how any new `Cargo.toml` entries satisfy these
  constraints.

Non-Goals:

- Making intrinsically new Cargo features work with older versions of
  Cargo (rather than just gracefully stopping).

- Changes to Cargo that aren’t opted into via keys, values or patterns
  in `Cargo.toml`.

- Retroactively improving the handling of features already added to Cargo.

# Guide-level explanation

## Elevator Pitch

If you use new Cargo features, and people developing or depending on
your crate are using an old version of Cargo, they will get a clear
error message telling them to upgrade.

Simply upgrading Cargo won’t require other developers interacting with
your crate to upgrade, unless you use features incompatible with the old
version of Cargo.

Both old and new versions of Cargo will check your crate against a
schema, which tells Cargo the version of Cargo required to handle each
field. This allows old Cargo to distinguish between fields that require
newer Cargo and entirely unknown fields.

This is a best-effort mechanism. If a version of the Cargo schema has a
bug, then Cargo will detect an incorrect version requirement and either
proceed incorrectly (producing a more inscrutable error) or stop
incorrectly (producing a false error). However, since Cargo will prefer
to obtain the schema from the index, and only use a copy in its own
source as a backup, fixing such schema bugs will automatically fix them
for all versions of Cargo.

## More Details

If a Cargo user upgrades Cargo, but continues to use `Cargo.toml` files
compatible with the previous version of Cargo (as documented by the
previous Cargo’s documentation), everything continues to work exactly as
it does today. This includes creating new crates, building crates,
resolving and downloading dependencies, and publishing existing crates.
If they are collaborating with users who have not yet upgraded Cargo,
the newer version of Cargo may introduce new metadata into `Cargo.lock`,
but running both old and new versions of Cargo will not repeatedly
change `Cargo.lock`  after that point.

A second scenario: A Cargo user upgrades Cargo and changes their
`Cargo.toml` to make use of new Cargo functionality that was not present
in the old version of Cargo, or they introduce a new crate or crate
version in their dependencies that makes use of new Cargo functionality.
In this case, if a collaborator was still using an old version of Cargo
(that postdates this RFC), that collaborator would get a clear error
message telling them to upgrade.

## How Future RFCs Should Think About It

When writing a Cargo RFC that defines new fields in `Cargo.toml`,
consider how that would look to older Cargo. Always consider what would
happen if older Cargo attempted to build a package using the new fields.
Declare the new fields (or types or patterns for existing fields) in the
Cargo schema. If ignoring the field produces no issues, then the field
need not be declared incompatible with old Cargo. This includes cases
where the new field provides information required to run a new Cargo
mechanism that didn’t exist in old Cargo at all.

For example, a new field that declares a new kind of build script would
require a new version of Cargo, because ignoring the field will either
prevent the crate from building or cause it to silently break. However,
if new Cargo adds a new field providing data for a corresponding new
Cargo target, old Cargo may be able to safely ignore that new field
since it does not support the new target.

# Reference-level explanation

Define a TOML schema format, and write a complete schema for
`Cargo.toml` in that format, as understood by current Cargo. Call that
schema version 0.

The schema format defines, for every possible key or family of keys (the
latter defined by a pattern), a list of types allowed for its values;
for string values, it can additionally define a pattern for allowed
values. Each of these gets mapped to a Cargo version. To support
unstable Cargo features, they may also get mapped to a stability flag.

Within the index format, add a new directory for crates that use newer
schemas. Any crate that only uses schema version 0 goes into the old
directory (and index format); any crate that uses a newer schema goes
into the new directory and new index format.

The new index format has the same “one JSON dictionary per line”
structure, containing a key `"schema"` with a string value containing a
minimum Cargo version number, and a key `"data"` with a dictionary value
consisting of all other data within the entry.

The new index format also introduces a new, more flexible sharding
scheme. To find an entry under the new index directory, Cargo must look
for the full crate name at each level, and if not found, split off the
next two (or less) characters, treat that as a subdirectory, and try
again. (Cargo must also canonicalize names to lowercase and change `_`
to `-` first. If Cargo finds the full crate name as a subdirectory, it
must recurse within that directory, to allow for two-character crate
names.) This allows any number of levels of sharding, since different
parts of the namespace may have varying numbers of crates.  (This will
not hurt the performance of Cargo, as Cargo must already parse every
tree object regardless. Cargo could also cache a pre-parsed version of
the index after each update.)

Versions of Cargo prior to the introduction of the schema mechanism will
ignore all crates in the new format. Versions of Cargo that understand
the schema mechanism will read the new index format, but will ignore all
entries with version numbers newer than itself without producing an
error. (Note, in particular, that they *must not* attempt to parse
anything under the `"data"` key.) Cargo must ignore entries with version
numbers newer than itself, and must not produce an error.

If dependency resolution fails, Cargo may provide a warning if crates it
could not find an appropriate version of have entries with Cargo
versions it does not understand, suggesting that the user may need a
newer version of Cargo. (However, we should not gratuitously produce
this warning just because we’ve seen newer entries, to avoid giving the
impression that Rust users *must* always run the latest version.) Cargo
could potentially attempt to look at the crate versions of those entries
to see if they’d satisfy dependency resolution, though it *must not*
break if it cannot parse those entries to determine their version.

crates.io should parse crates and their `Cargo.toml` on publish,
checking them against the schema, and insert them in the index with the
appropriate version number. crates.io should reject crates that don’t
parse with any version of ethe schema.

When Cargo obtains a crate from the new index, it should produce an
error, not a warning, if it encounters unknown fields (or field types or
patterns). For crates obtained via alternate registries or directly from
version control or filesystem paths, Cargo may attempt to use the schema
provided by crates.io-index to parse the `Cargo.toml` file and
distinguish types of errors, but may have to continue and warn rather
than stopping with an error.

# Drawbacks

The schema autodetection mechanism ties the schema solely to the use of
keys (and types and patterns) in `Cargo.toml`. This does not allow for
new features that involve no changes to `Cargo.toml`, such as exposing
new environment variables. We could potentially account for such
changes, but that would add significant complexity and fragility. We
could also require that such features have an accompanying change in
`Cargo.toml` to enable them, which would tie them to a schema version.

# Rationale and Alternatives

We could require explicit specification of version numbers in
`Cargo.toml` files, and propagate those versions into the index. This
would have the advantage of making it easier to detect the intended
Cargo version for a package, even in the absence of a registry. However,
this would introduce a significant burden on crate authors to keep this
version number accurate, and in practice this seems likely to diverge
from reality.

We could have Cargo do all the version detection at `cargo publish` time
rather than having crates.io do that work. However, relying solely on
client-side validation seems error-prone.

We could simplify the schema format by only taking fields and field
types into account, and not patterns. However, that would limit the
flexibility of introducing new features. Patterns allow, for instance,
introducing new dependency types, or new syntax, or new enumeration
values.

# Unresolved questions

This RFC intentionally does not specify the exact details of the TOML
schema format.

This RFC does not specify whether one schema file would define multiple
versions, or whether each Cargo version would have one schema file.
Having a single file to define the schema across multiple versions seems
like an optimization, but not a hard requirement.
