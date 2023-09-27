- Feature Name: N/A
- Start Date: 2020-11-04
- RFC PR: [rust-lang/rfcs#3013](https://github.com/rust-lang/rfcs/pull/3013)
- Rust Issue: [rust-lang/rust#82450](https://github.com/rust-lang/rust/issues/82450)

# Checking conditional compilation at compile time

# Summary

Rust supports conditional compilation, analogous to `#ifdef` in C / C++ / C#. Experience has shown
that managing conditional compilation is a significant burden for large-scale development. One of
the risks is that a condition may contain misspelled identifiers, or may use identifiers that are
obsolete or have been removed from a product. For example:

```rust
#[cfg(feature = "widnows")]    // notice the typo!
fn do_windows_thing() { /* ... */ }
```

The developer intended to test for the feature named `windows`. This could easily have been detected
by `rustc` if it had known the set of all valid `feature` flags, not only the ones currently
enabled.

This RFC proposes adding new command-line options to `rustc`, which will allow Cargo (and other
build tools) to inform `rustc` of the set of valid conditions, such as `feature` tests. Using
conditions that are not valid will cause a diagnostic warning. This feature is opt-in, for backwards
compatibility; if no valid configuration options are presented to `rustc` then no warnings are
generated.

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
* Boolean operators on conditions, such as `not(...)`, `all(...)`, and `any(...)`.

## Checking conditions names

`rustc` can optionally verify that condition names used in source code are valid. _Valid_ is
distinct from _enabled_. A _valid_ condition is one that is allowed to appear in source code; the
condition may be enabled or disabled, but it is still valid. An _enabled_ condition is one which has
been specified with a `--cfg foo` or `--cfg 'foo = "value"'` option.

For example, `rustc` can detect this bug, where the `test` condition is misspelled as `tset`:

```rust
if cfg!(tset) {    // uh oh, should have been 'test'
   ...
}
```

To catch this error, we give `rustc` the set of valid condition names:

```bash
rustc --check-cfg 'names(name1, name2, ..., nameN)' ...
```

The `--check-cfg` option does two things: First, it turns on validation for the set of condition
names (and separately for values). Second, it specifies the set of valid condition names (values).

Like many `rustc` options the `--check-cfg` option can be specified in a single-argument form, with
the option name and its argument joined by `=`, or can be specified in a two-argument form.

### Well-known condition names

`rustc` defines a set of well-known conditions, such as `test`, `target_os`, etc. These conditions
are always valid; it is not necessary to enable checking for these conditions. If these conditions
are specified in a `--check-cfg names(...)` option then they will be ignored. This set of well-known
names is a part of the stable interface of the compiler. New well-known conditions may be added in
the future, because adding a new name cannot break existing code. However, a name may not be removed
from the set of well-known names, because doing so would be a breaking change.

These are the well-known conditions:

* `feature`
* `linux`
* `test`
* `target_os`
* `target_arch`
* `windows`
* TODO: finish enumerating this list during implementation

## Checking key-value conditions

For conditions that define a list of values, such as `feature`, we want to verify that any
`#[cfg(feature = "v")]` test uses a valid value `v`. We want to detect this kind of bug:

```rust
if cfg!(feature = "awwwwsome") {    // should have been "awesome"
    ...
}
```

This kind of bug could be due to a typo or a bad PR merge. It could also occur because a feature
was removed from a `Cargo.toml` file, but source code still contains references to it. Or, a
feature name may have been renamed in one branch, while a new use of that feature was added in
a second branch. We want to catch that kind of accident during a merge.

To catch these errors, we give `rustc` the set of valid values for a given condition name, by
specifying the `--check-cfg` option. For example:

```bash
rustc --check-cfg 'values(feature, "derive", "parsing", "printing", "proc-macro")' ...

# specifying values for different names requires more than one --cfg option
rustc --check-cfg 'values(foo, "red", "green")' --check-cfg 'values(bar, "up", "down")'
```

## Checking is opt-in (disabled by default)

The default behavior of `rustc` is that conditional compilation names and values are not checked.
This maintains compatibility with existing versions of Cargo and other build systems that might
invoke `rustc` directly. All of the information for checking conditional compilation uses new
syntactic forms of the existing `--cfg` option.

Checking condition names is independent of checking condition values, for those conditions that
use value lists.

### Example: Checking condition names, but not values

```bash
# This turns on checking for condition names, but not values, such as 'feature' values.
rustc --check-cfg 'names(is_embedded, has_feathers)' \
      --cfg has_feathers \
      --cfg 'feature = "zapping"'
```

```rust
#[cfg(is_embedded)] // this is valid, and #[cfg] evaluates to disabled
fn do_embedded() {}

#[cfg(has_feathers)] // this is valid, and #[cfg] evaluates to enabled
fn do_features() {}

#[cfg(has_mumble_frotz)] // this is INVALID
fn do_mumble_frotz() {}

#[cfg(feature = "lasers")] // this is valid, because values() was never used
fn shoot_lasers() {}
```

### Example: Checking feature values, but not condition names

```bash
# This turns on checking for feature values, but not for condition names.
rustc --check-cfg 'values(feature, "zapping", "lasers")' \
      --cfg 'feature="zapping"'
```

```rust
#[cfg(is_embedded)]         // this is valid, because --check-cfg names(...) was never used
fn do_embedded() {}

#[cfg(has_feathers)]        // this is valid, because --check-cfg names(...) was never used
fn do_features() {}

#[cfg(has_mumble_frotz)]    // this is valid, because --check-cfg names(...) was never used
fn do_mumble_frotz() {}

#[cfg(feature = "lasers")]  // this is valid, because "lasers" is in the
                            // --check-cfg values(feature) list
fn shoot_lasers() {}

#[cfg(feature = "monkeys")] // this is INVALID, because "monkeys" is not in the
                            // --check-cfg values(feature) list
fn write_shakespeare() {}
```

### Example: Checking both condition names and feature values

```bash
# This turns on checking for feature values and for condition names.
rustc --check-cfg 'names(is_embedded, has_feathers)' \
      --check-cfg 'values(feature, "zapping", "lasers")' \
      --cfg has_feathers \
      --cfg 'feature="zapping"' \
```

```rust
#[cfg(is_embedded)]         // this is valid, and #[cfg] evaluates to disabled
fn do_embedded() {}

#[cfg(has_feathers)]        // this is valid, and #[cfg] evaluates to enabled
fn do_features() {}

#[cfg(has_mumble_frotz)]    // this is INVALID, because has_mumble_frotz is not in the
                            // --check-cfg names(...) list
fn do_mumble_frotz() {}

#[cfg(feature = "lasers")]  // this is valid, because "lasers" is in the values(feature) list
fn shoot_lasers() {}

#[cfg(feature = "monkeys")] // this is INVALID, because "monkeys" is not in
                            // the values(feature) list
fn write_shakespear() {}
```

## Cargo support

Cargo is ideally positioned to enable checking for `feature` flags, since Cargo knows the set of
valid features. Cargo will invoke `rustc --check-cfg 'values(feature, "...", ...)'`, so that
checking for features is enabled. Optionally, Cargo could also specify the set of valid condition
names.

Cargo users will not need to do anything to take advantage of this feature. Cargo will always
specify the set of valid `feature` flags. This may cause warnings in crates that contain invalid
`#[cfg]` conditions. (Rust is permitted to add new lints; new lints are not considered a breaking
change.) If a user upgrades to a version of Cargo / Rust that supports validating features, and
their crate now reports errors, then they will need to align their source code with their
`Cargo.toml` file in order to fix the error. (Or use `#[allow(...)]` to suppress it.) This is a
benefit, because it exposes potential existing bugs.

## Supporting build systems other than Cargo

Some users invoke `rustc` using build systems other than Cargo. In this case, `rustc` will provide
the mechanism for validating conditions, but those build systems will need to be updated in order
to take advantage of this feature. Doing so is expected to be easy and non-disruptive, since this
feature does not change the meaning of the existing `--cfg` option.

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
      --check-cfg 'values(feature, "lighting", "bump_maps", "mip_maps", "vulkan")'
```

In this command-line, Cargo has specified the full set of _valid_ features (`lighting`,
`bump_maps`, `mip_maps`, `vulkan`) while also specifying which of those features are currently
_enabled_ (`lighting`, `bump_maps`).

## Command line arguments reference

`rustc` accepts the `--check-cfg` option, which specifies whether to check conditions and how to
check them. The `--check-cfg` option takes a value, called the _check cfg specification_. The
check cfg specification is parsed using the Rust metadata syntax, just as the `--cfg` option is.
(This allows for easy future extensibility, and for easily specifying moderately-complex data.)

Each `--check-cfg` option can take one of two forms:

1. `--check-cfg names(...)` enables checking condition names.
2. `--check-cfg values(...)` enables checking the values within list-valued conditions.

### The `names(...)` form

This form uses a named metadata list:

```bash
rustc --check-cfg 'names(name1, name2, ... nameN)'
```

where each `name` is a bare identifier (has no quotes). The order of the names is not significant.

If `--check-cfg names(...)` is specified at least once, then `rustc` will check all references to
condition names. `rustc` will check every `#[cfg]` attribute, `#[cfg_attr]` attribute, and
`cfg!(...)` call against the provided list of valid condition names. If a name is not present in
this list, then `rustc` will report an `invalid_cfg_name` lint diagnostic. The default diagnostic
level for this lint is `Warn`.

If `--check-cfg names(...)` is not specified, then `rustc` will not check references to condition
names.

`--check-cfg names(...)` may be specified more than once. The result is that the list of valid
condition names is merged across all options. It is legal for a condition name to be specified
more than once; redundantly specifying a condition name has no effect.

To enable checking condition names with an empty set of valid condition names, use the following
form. The parentheses are required.

```bash
rustc --check-cfg 'names()'
```

Note that `--check-cfg 'names()'` is _not_ equivalent to omitting the option entirely.
The first form enables checking condition names, while specifying that there are no valid
condition names (outside of the set of well-known names defined by `rustc`). Omitting the
`--check-cfg 'names(...)'` option does not enable checking condition names.

Conditions that are enabled are implicitly valid; it is unnecessary (but legal) to specify a
condition name as both enabled and valid. For example, the following invocations are equivalent:

```bash
# condition names will be checked, and 'has_time_travel' is valid
rustc --cfg 'has_time_travel' --check-cfg 'names()'

# condition names will be checked, and 'has_time_travel' is valid
rustc --cfg 'has_time_travel' --check-cfg 'names(has_time_travel)'
```

In contrast, the following two invocations are _not_ equivalent:

```bash
# condition names will not be checked (because there is no --check-cfg names(...))
rustc --cfg 'has_time_travel'

# condition names will be checked, and 'has_time_travel' is both valid and enabled.
rustc --cfg 'has_time_travel' --check-cfg 'names(has_time_travel)'
```

### The `values(...)` form

The `values(...)` form enables checking the values within list-valued conditions. It has this
form:

```bash
rustc --check-cfg `values(name, "value1", "value2", ... "valueN")'
```

where `name` is a bare identifier (has no quotes) and each `"value"` term is a quoted literal
string. `name` specifies the name of the condition, such as `feature` or `target_os`.

When the `values(...)` option is specified, `rustc` will check every `#[cfg(name = "value")]`
attribute, `#[cfg_attr(name = "value")]` attribute, and `cfg!(name = "value")` call. It will
check that the `"value"` specified is present in the list of valid values. If `"value"` is not
valid, then `rustc` will report an `invalid_cfg_value` lint diagnostic. The default diagnostic
level for this lint is `Warn`.

The form `values()` is an error, because it does not specify a condition name.

To enable checking of values, but to provide an empty set of valid values, use this form:

```bash
rustc --check-cfg `values(name)`
```

The `--check-cfg values(...)` option can be repeated, both for the same condition name and for
different names. If it is repeated for the same condition name, then the sets of values for that
condition are merged together.

> The `--check-cfg names(...)` and `--check-cfg values(...)` options are independent. `names`
> checks the namespace of condition names; `values` checks the namespace of the values of
> list-valued conditions.

### Valid values can be split across multiple options

The valid condition values are the union of all options specified on the command line.
For example, this command line:

```bash
# legal but redundant:
rustc --check-cfg 'values(animals, "lion")' --check-cfg 'values(animals, "zebra")'

# equivalent:
rustc --check-cfg 'values(animals, "lion", "zebra")'
```

This is intended to give tool developers more flexibility when generating Rustc command lines.

### Enabled condition names are implicitly valid

Specifying an enabled condition name implicitly makes it valid. For example, the following
invocations are equivalent:

```bash
# legal but redundant:
rustc --check-cfg 'names(animals)' --cfg 'animals = "lion"'

# equivalent:
rustc --check-cfg 'names()' --cfg 'animals = "lion"'
```

### Enabled condition values are implicitly valid

Specifying an enabled condition _value_ implicitly makes that _value_ valid. For example, the
following invocations are equivalent:

```bash
# legal but redundant
rustc --check-cfg 'values(animals, "lion", "zebra")' --cfg 'animals = "lion"'

# equivalent
rustc --check-cfg 'values(animals, "zebra")' --cfg 'animals = "lion"'
```

Specifying a condition value also implicitly marks that condition _name_ as valid. For example,
the following invocations are equivalent:

```bash
# legal but redundant:
rustc --check-cfg 'names(other, animals)' --check-cfg 'values(animals, "lion")'

# so the above can be simplified to:
rustc --check-cfg 'names(other)' --check-cfg 'values(animals, "lion")'
```

### Checking condition names and values is independent

Checking condition names may be enabled independently of checking condition values.
If checking of condition values is enabled, then it is enabled separately for each condition name.

Examples:

```bash

# no checking is performed
rustc

# names are checked, but values are not checked
rustc --check-cfg 'names(has_time_travel)'

# names are not checked, but 'feature' values are checked.
# note that #[cfg(market = "...")] values are not checked.
rustc --check-cfg 'values(feature, "lighting", "bump_maps")'

# names are not checked, but 'feature' values _and_ 'market' values are checked.
rustc --check-cfg 'values(feature, "lighting", "bump_maps")' \
      --check-cfg 'values(market, "europe", "asia")'

# names _and_ feature values are checked.
rustc --check-cfg 'names(has_time_travel)' \
      --check-cfg 'values(feature, "lighting", "bump_maps")'
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
  (i.e. there is at least one `--cfg 'names(...)'` option and an invalid condition name is found
  during compilation.

* `invalid_cfg_value`: Indicates that source code contained a condition value that was invalid.
  This diagnostic will only be reported if the command line options enable checking condition values
  for the specified condition name (i.e. there is a least one `--check-cfg 'values(c, ...)'` for
  a given condition name `c`).

All of the diagnostics defined by this RFC are reported as warnings. They can be upgraded to
errors or silenced using the usual diagnostics controls.

## Examples

Consider this command line:

```bash
rustc --check-cfg 'name(feature)' \
      --check-cfg 'values(feature,"lion","zebra")' \
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

> Note: The `--check-cfg names(feature)` option is necessary only to enable checking the condition
> name, as in the last example. `feature` is a well-known (always-valid) condition name, and so it
> is not necessary to specify it in a `--check-cfg 'names(...)'` option. That option can be
> shortened to > `--check-cfg names()` in order to enable checking condition names.

## Drawbacks

* Adds complexity, in the form of additional command-line options. Fortunately, this is
  complexity that will be mainly be exposed to build systems, such as Cargo.
* As with all lints, correct code may be trigger lints. Developers will need to take time to
  examine them and see whether they are legitimate or not.
* To take full advantage of this, build systems (including but not limited to Cargo) must be
  updated. However, for those systems that are not updated, there is no penalty or drawback,
  since `--check-cfg` is opt-in.

* This lint will not be able to detect invalid `#[cfg]` tests that are within modules that
  are not compiled, presumably because an ancestor `mod` is disabled due to a.  For example:

  File `lib.rs` (root module):
  ```rust
  #[cfg(feature = "this_is_disabled_but_valid")]
  mod foo
  ```

  File `foo.rs` (nested module):
  ```rust
  #[cfg(feature = "oooooops_this_feature_is_misspelled_and_invalid")]
  mod uh_uh;
  ```

  The invalid `#[cfg]` attribute in `foo.rs` will not be detected, because `foo.rs` was not
  read and parsed. This is a minor drawback, and should not prevent users from benefitting
  from checking in most common situations.

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

This RFC specifies the exact syntax of this feature in source code and in the
command-line options for `rustc`. However, it does not address how these will be used
by tools, such as Cargo. This is a split between "mechanism" and "policy"; the mechanism
(what goes in `rustc`) is specified in this RFC, but the policies that control this
mechanism are intentionally left out of scope.

We expect the stabilization process for the mechanism (the support in `rustc`) to stabilize
relatively quickly. Separately, over a much longer time frame, we expect the polices that
control those options to stabilize more slowly. For example, it seems uncontroversial for
Cargo to enable checking for `feature = "..."` values immediately; this could be
implemented and stabilized quickly.

However, when (if ever) should Cargo enable checking condition _names_? For crates that
do not have a `build.rs` script, Cargo could enable checking condition names immediately.
But for crates that do have a `build.rs` script, we may need a way for those scripts to
control the behavior of checking condition names.

One possible source of problems may come from build scripts (`build.rs` files) that add `--cfg`
options that Cargo is not aware of. For example, if a `Cargo.toml` file did _not_ define a feature
flag of `foo`, but the `build.rs` file added a `--cfg feature="foo"` option, then source code
could use `foo` in a condition. My guess is that this is rare, and that a Crater run will expose
this kind of problem.

# Future possibilities

* Should these checks be enabled by default in Cargo?
* How many public crates would fail these checks?
* If these checks are enabled by default in Cargo, should they be warnings or errors?
