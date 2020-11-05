# Checking conditional compilation at compile time

# Summary

Rust supports conditional compilation, analogous to `#ifdef` in C / C++ / C#. Experience has shown
that managing conditional compilation is a significant burden for large-scale development. One of
the risks is that a condition may contain misspelled identifiers, or may use identifiers that are
obsolete or have been removed from a product. For example:

```rust
#[cfg(feature = "widnows")]
fn do_windows_thing() { /* ... */ }
```

The developer intended to test for the feature named `windows`. This could easily have been detected
by `rustc` if it had known the set of all valid `feature` flags, not only the ones currently
enabled.

This RFC proposes adding new command-line options to `rustc`, which will allow Cargo (and other
build tools) to inform `rustc` of the set of valid feature flags. Using conditions that are not
valid will cause a diagnostic warning. This feature is opt-in, for backwards compatibility;
if no valid configuration options are presented to `rustc` then no warnings are generated.

# Motivation

* Stronger assurance that large code bases are correct.
* Protect against typos, bad merges, etc.
* Detect dead code, typically caused by feature flags that have been removed from a crate's
  manifest, but which still have `#[cfg(...)]` attributes that mention those features.

# Guide-level explanation

## Background

Rust programs can use conditional compilation in order to modify programs based on the features a
user has selected, the target CPU architecture, the target OS, or other parameters under control
of the user. Rust programs may use conditional compilation in these ways:

* By applying the `#[cfg(c)]` attribute to language elements, where `c` is a condition.
* By applying the `#[cfg_attr(c, attr)]` attribute to language elements, where `c` is a conditional
  and `attr` is an attribute to apply.
* By using the `cfg!(c)` built-in macro, where `c` is a condition. The compiler replaces the macro
  call with a `true` or `false` literal.

A _condition_ can take one of two forms:

* A single identifier, such as `#[cfg(test)]` or `#[cfg(linux)]`. These are Boolean conditions;
  they are either enabled or disabled.
* A condition may test whether a given value is present in a named list of values. For example,
  `#[cfg(feature = "lighting")]` tests whether the `lighting` feature is enabled. Note that a given
  condition name may have any number of enabled values; for example, it is legal to invoke
  `rustc --cfg feature="lighting" --cfg feature="bump_maps"`.

## Checking conditions names

`rustc` can optionally verify that all condition names used in source code are valid. "Valid" is
distinct from "enabled". A _valid_ condition is one that is allowed to appear in source code; the
condition may be enabled or disabled, but it is still valid. An _enabled_ condition is one which has
been specified with a `--cfg foo` or `--cfg 'foo="value"'` option.

For example, `rustc` can detect this bug, where the `test` condition is misspelled as `tset`:

```rust
if cfg!(tset) {    // uh oh, should have been 'test'
   ...
}
```

To catch this error, we give `rustc` the set of valid condition names:

```bash
rustc --cfg 'valid(name1, name2, ..., nameN)' ...
```

The `--cfg 'valid(...)'` option specifies the set of valid condition names.

### Examples

```bash
rustc --cfg 'valid(cats, dogs, foo, bar)' ...
```

The `--cfg 'valid(...)'` option may be repeated. If it is repeated, then the condition names of
all of the options are merged into a single set. For example:

```bash
rustc --cfg 'valid(cats)' --cfg 'valid(dogs)' ...
```

### Well-known condition names

`rustc` defines a set of well-known conditions, such as `test`, `target_os`, etc. These conditions
are always valid; it is not necessary to enable checking for these conditions. If these conditions 
are specified in a `--cfg valid(...)` option, they will be ignored. This set of well-known names is
a part of the stable interface of the compiler. New well-known conditions  may be added in the
future, because adding a new name cannot break existing code. However, a name may not be removed
from the set of well-known names, because doing would be a breaking change.

These are the well-known conditions:

* `feature`
* `linux`
* `test`
* `target_os`
* `target_arch`
* `windows`
* TODO: finish enumerating this list

## Checking key-value conditions

For conditions that define a list of values, such as `feature`, we want to verify that any
`#[cfg(feature = "v")]` test uses a valid value `v`. We want to detect this kind of bug:

```rust
if cfg!(feature = "awwwwsome") {    // should have been "awesome"
    ...
}
```

This kind of bug could be due to a typo or a bad PR merge. It could also occur because a feature
was removed from a `Cargo.toml` file, but source code still contains references to it.

To catch these errors, we give `rustc` the set of valid values for a given condition name.
To do so, we extend the syntax of the `--cfg` option. For a given condition name `c` (such as
`feature`), we specify the set of valid values as:

```bash
rustc --cfg 'valid_values(c, "value1", "value2", ... "valueN")'
```

### Examples

```bash
rustc --cfg 'valid_values(feature, "derive", "parsing", "printing", "proc-macro")'

# or equivalently:
rustc --cfg 'valid_values(feature, "derive")' \
    'valid_values(feature, "parsing")' \
    'valid_values(feature, "printing")' \
    'valid_values(feature, "proc-macro")'

rustc --cfg 'valid_values(target_os, "linux", "macos", "ios", "windows", "android")'

# specifying values for different names requires more than one --cfg option
rustc --cfg 'valid_values(foo, "red", "green")' --cfg 'valid_values(bar, "up", "down")'
```

## Checking is opt-in (disabled by default)

The default behavior of `rustc` is that conditional compilation names and values are not checked.
This maintains compatibility with existing versions of Cargo and other build systems that might
invoke `rustc` directly. All of the information for checking conditional compilation uses new
syntactic forms of the existing `--cfg` option.

Checking condition names is independent of checking condition values, for those conditions that
use value lists. Examples:

### Example: Checking condition names, but not values

```bash
# This turns on checking for condition names, but not values, such as 'feature' values.
rustc --cfg 'valid(is_embedded, has_feathers)' \
      --cfg has_feathers \
      --cfg 'feature="zapping"'
```

```rust
#[cfg(is_embedded)] // this is valid, and #[cfg] evaluates to disabled
fn do_embedded() {}

#[cfg(has_feathers)] // this is valid, and #[cfg] evalutes to enabled
fn do_features() {}

#[cfg(has_mumble_frotz)] // this is INVALID
fn do_mumble_frotz() {}

#[cfg(feature = "lasers")] // this is valid, because valid_values() was never used
fn shoot_lasers() {}
```

### Example: Checking feature values, but not condition names

```bash
# This turns on checking for feature values, but not for condition names.
rustc --cfg 'valid_values(feature, "zapping", "lasers")' \
      --cfg 'feature="zapping"'
```

```rust
#[cfg(is_embedded)] // this is valid, because --cfg valid(...) was never used
fn do_embedded() {}

#[cfg(has_feathers)] // this is valid, because --cfg valid(...) was never used
fn do_features() {}

#[cfg(has_mumble_frotz)] // this is valid, because --cfg valid(...) was never used
fn do_mumble_frotz() {}

#[cfg(feature = "lasers")] // this is valid, because "lasers" is in the valid_values(feature) list
fn shoot_lasers() {}

#[cfg(feature = "monkeys")] // this is INVALID, because "monkeys" is not in
                            // the valid_values(feature) list
fn write_shakespear() {}
```

### Example: Checking both condition names and feature values

```bash
# This turns on checking for feature values and for condition names.
rustc --cfg 'valid_values(feature, "zapping", "lasers")' \
      --cfg 'valid(is_embedded, has_feathers)'
      --cfg has_feathers \
      --cfg 'feature="zapping"' \
```


```rust
#[cfg(is_embedded)] // this is valid, and #[cfg] evaluates to disabled
fn do_embedded() {}

#[cfg(has_feathers)] // this is valid, and #[cfg] evalutes to enabled
fn do_features() {}

#[cfg(has_mumble_frotz)] // this is INVALID
fn do_mumble_frotz() {}

#[cfg(feature = "lasers")] // this is valid, because "lasers" is in the valid_values(feature) list
fn shoot_lasers() {}

#[cfg(feature = "monkeys")] // this is INVALID, because "monkeys" is not in
                            // the valid_values(feature) list
fn write_shakespear() {}
```

## Cargo support

Cargo is ideally positioned to enable checking for `feature` flags, since Cargo knows the set of
valid features. Cargo will invoke `rustc --cfg 'valid_values(feature, "...", ...)'`, so that
checking for features is enabled. Optionally, Cargo could also specify the set of valid condition
names.

## Command line flags for checking conditional compilation

To use this feature, you give `rustc` the set of condition names that are valid, by using the
`--cfg valid(...)` flag. For example:

```bash
rustc --cfg 'valid(test, target_os, feature, foo, bar)' ...
```

> Note: This example shows `test` and `target_os`, but it is not necessary to specify these. All
> condition names that are well-known to `rustc` are always permitted.

For condition values, specify the set of values that are legal for a given condition name. For
example:

```bash
 --cfg 'valid_values(feature, "foo", "bar", ..., "zot")'
 ```

## What do users need to do to use this?

Most of the time, users will not invoke `rustc` directly. Instead, users run Cargo, which handles
building command-lines for `rustc`. Cargo handles building the lists of valid condition names.

> This reflects the final goal, which is turning on condition checking in Cargo. However, we will
> need to do a gradual deployment:
> 1. Users on `nightly` builds can run `cargo -Z check-cfg` to opt-in to checking.
> 2. After stabilization, users can opt-in to conditional checking by using `cargo build --check-cfg`.
> 3. Eventually, if the community experience is positive, Cargo could enable this check by default.

For users who are not using Cargo, they will need to work with their build system to add support
for this. Doing so is out-of-scope for this RFC.

# Reference-level explanation

## What Cargo does

When Cargo builds a `rustc` command line, it knows which features are enabled and which are
disabled. Cargo normally specifies the set of enabled features like so:

```bash
rustc --cfg 'feature="lighting"' --cfg 'feature="bump_maps"' ...
```

When conditional compilation checking is enabled, Cargo will also specify which features are
valid, so that `rustc` can validate conditional compilation tests. For example:

```bash
rustc --cfg 'feature="lighting"' --cfg 'feature="bump_maps"' \
      --cfg 'valid(feature, "lighting", "bump_maps", "mip_maps", "vulkan")'
```

In this command-line, Cargo has specified the full set of _valid_ features (`lighting`,
`bump_maps`, `mip_maps`, `vulkan`) while also specifying which of those features are currently
_enabled_ (`lighting`, `bump_maps`).

## Command line arguments reference

All information would be passed to Rustc by enhancing the forms accepted for the existing `--cfg`
option. To enable checking condition names and to specify the set of valid names, use this form:

```bash
rustc --cfg 'valid(c1, c2, ..., cN)'
```

Where `c1..cN` are condition names. These are specified as bare identifiers, not quoted strings. If
this option is not specified, then condition checking is not performed. If this option is specified,
then each usage of this option (such as a `#[cfg]` attribute, `#[cfg_attr]` attribute, or
`cfg!(...)` call) is checked against this list. If a name is not present in this list, then a
diagnostic is issued. The default diagnostic level for an invalid name is a warning. This can be
promoted to an error by using `#![deny(invalid_cfg_name)]` or the equivalent command line option.

If the `valid(...)` option is repeated, then the set of valid condition names is the union of
those specified in all options.

To enable checking condition values, specify the set of valid values:

```bash
rustc --cfg 'valid_values(c,"value1","value2", ... "valueN")'
```

where `c` is the condition name, such as `feature` or `target_os`, and the `valueN` terms are the
values that are valid for that condition. The `valid_values` option can be repeated, both for the
same condition name and for different names. If it is repeated for the same condition name, then the
sets of values for that condition are merged together.

> The `valid` and `valid_values` options are independent. The `valid` option controls
> whether condition names are checked, but it does not require that condition values be checked.

### Valid values can be split across multiple options

`rustc` processes all `--cfg` options before compiling any code. The valid condition names and
values are the union of all options specified on the command line. For example, this command line:

```bash
rustc --cfg 'valid_values(feature,"lion","zebra")'

# is equivalent to:
rustc --cfg 'valid_values(feature,"lion")' \
      --cfg 'valid_values(feature,"zebra")'
```

This is intended to give tool developers more flexibility when generating Rustc command lines.

### Enabled conditions are implicitly valid

Specifying an enabled condition name implicitly makes it valid. For example, the following
invocations are equivalent:

```bash
rustc --cfg 'valid_values(feature,"lion","zebra")' --cfg 'feature="lion"'

# equivalent:
rustc --cfg 'valid_values(feature,"zebra")' --cfg 'feature="lion"'
```

Specifying a condition value also implicitly marks that condition _name_ as valid. For example,
the following invocations are equivalent:

```bash
# specifying valid_values(foo, ...) implicitly adds 'foo' to the set of valid condition names
rustc --cfg 'valid(foo,bar)' --cfg 'valid_values(foo,"lion")' --cfg 'foo="lion"'

# so the above can be simplified to:
rustc --cfg 'valid(bar)' --cfg 'valid_values(foo,"lion")' --cfg 'foo="lion"'
```

## Stabilizing

Until this feature is stabilized, it can only be used with a `nightly` compiler, and only when
specifying the `rustc -Z check-cfg ...` option.

Similarly, users of `nightly` Cargo builds must also opt-in to use this feature, by specifying
`cargo build -Z check-cfg ...`.

Experience gained during stabilization will determine how this feature is best enabled in the final
product. Ideally, once the feature is stabilized in `rustc`, the `-Z check-cfg` requirement will
be dropped from `rustc`. Stabilizing in Cargo may require a stable opt-in flag, however.

## Diagnostics

Conditional checking can report these diagnostics:

* `invalid_cfg_name`: Indicates that a condition name was not in the set of valid names.
  This diagnostic will only be reported if the command line options enable checking condition names
  (i.e. there is at least one `--cfg 'valid(...)'` option and an invalid condition name is found
  during compilation.

* `invalid_cfg_value`: Indicates that source code contained a condition value that was invalid.
  This diagnostic will only be reported if the command line options enable checking condition values
  for the specified condition name (i.e. there is a least one `--cfg 'valid_values(c, ...)'` for
  a given condition name `c`).

All of the diagnostics defined by this RFC are reported as warnings. They can be upgraded to
errors or silenced using the usual diagnostics controls.

## Examples

Consider this command line:

```bash
rustc --cfg 'valid(feature)' \
      --cfg 'valid_values(feature,"lion","zebra")' \
      --cfg 'feature="lion"'
      example.rs
```

This command line indicates that this crate has two features: `lion` and `zebra`. The `lion`
feature is enabled, while the `zebra` feature is disabled. Consider compiling this code:

```rust
// this is valid, and tame_lion() will be compiled
#[cfg(feature = "lion")]     
fn tame_lion(lion: Lion) { ... }

// this is valid, and ride_zebra() will NOT be compiled
#[cfg(feature = "zebra")] 
fn ride_zebra(zebra: Zebra) { ... }

// this is INVALID, and will cause a compiler error 
#[cfg(feature = "platypus")] 
fn poke_platypus() { ... }

// this is INVALID, because 'feechure' is not a known condition name,
// and will cause a compiler error.
#[cfg(feechure = "lion")]
fn tame_lion() { ... }
```

> Note: The `--cfg valid(feature)` argument is necessary only to enable checking the condition
> name, as in the last example. `feature` is a well-known condition name, and so it is not necessary
> to specify it in a `--cfg 'valid(...)'` option. That option can be shorted to `--cfg valid()` in
> order to enable checking condition names.

## Drawbacks
There are no known drawbacks to this proposal.

## Rationale and alternatives
This design enables checking for a class of bugs at compile time, rather than detecting them by 
running code.

This design does not break any existing usage of Rustc. It does not change the meaning of existing
programs or existing Rustc command-line options. It is strictly opt-in. If the verification that
this feature provides is valuable, then it could be promoted to a warning in the future, or
eventually an error. There would need to be a cleanup period, though, where we detected failures in
existing crates and fixed them.

The impact of not doing this is that a class of bugs may go undetected. These bugs are often easy
to find in relatively small systems of code, but experience shows that these kinds of bugs are much
harder to verify in large code bases. Rust should enable developers to scale up from small to large
systems, without losing agility or reliability.

## Prior art

Rust has a very strong focus on finding defects at compile time, rather than allowing defects to be
detected much later in the development cycle. Statically checking that conditional compilation is
used correctly is consistent with this approach.

Many languages have similar facilities for conditional compilation. C, C++, C#, and many of their
variants make extensive use of conditional compilation. The author is unaware of any effort to
systematically verify the correct usage of conditional compilation in these languages.

## Unresolved questions

During the RFC process, I expect to resolve the question of what the exact rustc command-line
parameters should be, at least enough to enable the feature for nightly builds. We should avoid
bikeshedding on the exact syntax, but should agree on the semantics. We should agree on what is
checked, what is not checked, how checking is enabled, and how it is performed. We should agree on
what information is passed from Cargo to Rustc.

During the implementation and before stabilization, I expect to resolve the question of how many errors this actually detects. How many crates on crates.io actually have invalid `#[cfg]` usage?
How valuable is this feature?

One possible source of problems may come from build scripts (`build.rs` files) that add `--cfg`
options that Cargo is not aware of. For exaple, if a `Cargo.toml` file did _not_ define a feature
flag of `foo`, but the `build.rs` file added a `--cfg feature="foo"` option, then source code
could use `foo` in a condition. My guess is that this is rare, and that a Crater run will expose
this kind of problem.

# Future possibilities

* Should these checks be enabled by default in Cargo?
* How many public crates would fail these checks?
* If these checks are enabled by default in Cargo, should they be warnings or errors?
