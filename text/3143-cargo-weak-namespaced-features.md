- Feature Name: `weak-dep-features` and `namespaced-features`
- Start Date: 2021-06-10
- RFC PR: [rust-lang/rfcs#3143](https://github.com/rust-lang/rfcs/pull/3143)
- Tracking Issues: [rust-lang/cargo#5565](https://github.com/rust-lang/cargo/issues/5565) and [rust-lang/cargo#8832](https://github.com/rust-lang/cargo/issues/8832)

# Summary

This RFC proposes to stabilize the `weak-dep-features` and `namespaced-features` enhancements to Cargo. These introduce the following additions to how Cargo's [feature system] works:

Weak dependency features adds the ability to specify that features of an [optional dependency] should be enabled only if the optional dependency is already enabled by another feature.

Namespaced features separates the namespaces of dependency names and feature names.

These enhancements are already implemented, but testing is limited because the syntax is only available on the nightly channel and is currently not allowed on [crates.io].
See [Weak dependency features] and [Namespaced features] for more information on how to use them on nightly.

[feature system]: https://doc.rust-lang.org/cargo/reference/features.html
[crates.io]: https://crates.io/
[Weak dependency features]: https://doc.rust-lang.org/cargo/reference/unstable.html#weak-dependency-features
[Namespaced features]: https://doc.rust-lang.org/cargo/reference/unstable.html#namespaced-features
[optional dependency]: https://doc.rust-lang.org/cargo/reference/features.html#optional-dependencies

# Motivation

These enhancements to Cargo's feature system unlock the ability to express certain rules for features that are currently difficult or impossible to achieve today.
These issues can crop up for many projects that make use of optional dependencies, and are well-known pain points.
Introducing these enhancements can alleviate some of those pain points.

## Weak dependency feature use cases

Sometimes a package may want to "forward" a feature to its dependencies. This can be done today with the `dep_name/feat_name` syntax in the `[features]` table.
However, one drawback is that if the dependency is an optional dependency, this will implicitly enable the dependency, which may not be what you want.
The weak dependency syntax provides a way to control whether or not the optional dependency is automatically enabled in that case.

For example, if your crate has optional `std` support, you may need to also enable `std` support on your dependencies.
But you may not want to enable those dependencies just because `std` is enabled.

```toml
[dependencies]
serde = { version = "1.0", optional=true, default-features = false }

[features]
# This will also enable serde, which is probably not what you want.
std = ["serde/std"]
```

## Namespaced features use cases

Currently, optional dependencies automatically get a feature of the same name to enable that dependency.
However, this presents a compatibility hazard because the existence of that optional dependency may be an internal detail that a package may not want to expose.
This can be mitigated somewhat through the use of documentation, but remains as an uncomfortable point where users may enable optional dependencies that you may not want them to have direct control over.
Namespaced features provides a way to "hide" an optional dependency so that users cannot directly enable an optional dependency, but only through other explicitly defined features.

Also, removing the restriction that feature names cannot conflict with dependency names allows you to use more natural feature names. For example, if you have an optional dependency on `serde`, and you want to enable `serde` on your other dependencies at the same time, today you can't define a feature named `serde`, but instead are required to specify an alternate name like `serde1`, for example:

```toml
[features]
# This is an awkward name to use because dependencies and features
# share the same namespace.
serde1 = ["serde", "chrono/serde"]
```

Another example, here `lazy_static` is required when `regex` is used, but you don't want users to know about the existence of `lazy_static`.

```toml
[dependencies]
# This implicitly exposes both `regex` and `lazy_static` externally. However,
# enabling just `regex` will fail to compile without `lazy_static`.
regex = { version = "1.4.1", optional = true }
lazy_static = { version = "1.4.0", optional = true }

[features]
# Another circumstance where you have to pick a name that doesn't conflict,
# which may be confusing.
regexp = ["regex", "lazy_static"]
```

# Guide-level explanation

The following is a replacement of the corresponding sections in the [features guide].

[features guide]: https://doc.rust-lang.org/cargo/reference/features.html

### Optional dependencies

Dependencies can be marked "optional", which means they will not be compiled by default.
They can then be specified in the `[features]` table with a `dep:` prefix to indicate that they should be built when the given feature is enabled.
For example, let's say in order to support the AVIF image format, our library needs two other dependencies to be enabled:

```toml
[dependencies]
ravif = { version = "0.6.3", optional = true }
rgb = { version = "0.8.25", optional = true }

[features]
avif = ["dep:ravif", "dep:rgb"]
```

In this example, the `avif` feature will enable the two listed dependencies.

If the optional dependency is not specified anywhere in the `[features]` table, Cargo will automatically define a feature of the same name.
For example, let's say that our 2D image processing library uses an external package to handle GIF images.
This can be expressed like this:

```toml
[dependencies]
gif = { version = "0.11.1", optional = true }
```

If `dep:gif` is not specified in the `[features]` table, then Cargo will automatically define a feature that looks like:

```toml
[features]
# Cargo automatically defines this if "dep:gif" is not specified anywhere else.
gif = ["dep:gif"]
```

This is a convenience if the name of the optional dependency is something you want to expose to the users of the package.
If you don't want users to directly enable the optional dependency, then place the `dep:` strings in another feature that you do want exposed, such as in the `avif` example above.

You can then use `cfg` macros to conditionally use these features just like any other feature.
For example, `cfg(feature = "gif")` or `cfg(feature = "avif")` can be used to conditionally include interfaces for those image formats.

> **Note**: Another way to optionally include a dependency is to use [platform-specific dependencies].
> Instead of using features, these are conditional based on the target platform.

[platform-specific dependencies]: https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#platform-specific-dependencies

#### Dependency features in the `[features]` table

Features of dependencies can also be enabled in the `[features]` table.
This can be done with the `dependency-name/feature-name` syntax which says to enable the specified feature for that dependency. For example:

```toml
[dependencies]
jpeg-decoder = { version = "0.1.20", default-features = false }

[features]
# Enables parallel processing support by enabling the "rayon" feature of jpeg-decoder.
parallel = ["jpeg-decoder/rayon"]
```

If the dependency is an optional dependency, this syntax will also enable that dependency.
If you do not want that behavior, the alternate syntax `dependency-name?/feature-name` with the `?` character tells Cargo to only enable the given feature if the dependency is activated by another feature.
For example:

```toml
[dependencies]
# Defines an optional dependency.
serde = { version = "1.0", optional=true, default-features = false }

[features]
# This "std" feature enables the "std" feature of serde, but only if serde is enabled.
std = ["serde?/std"]
```

# Reference-level explanation

## Index changes

For reference, the current index format is documented [here](https://doc.rust-lang.org/cargo/reference/registries.html#index-format).

A new `"features2"` field is added to the package description, which is an object with the same form as the `"features"` field.
When reading the index, Cargo merges the values found in `"features2"` into `"features"`.
This helps prevent breaking versions of Cargo older than 1.19 (published 2017-07-20), which will return an error if they encounter the new syntax, even if there is a `Cargo.lock` file.
These older versions will *ignore* the "features2" field, allowing them to behave correctly assuming there is a `Cargo.lock` file (or the packages they need do not use the new syntax).

During publishing, [crates.io] is responsible for separating the new syntax into the `"features2"` object before saving the entry in the index.
Other registries do not need to bother as versions of Cargo older than 1.19 do not support other registries (though they may separate them if they wish).

Cargo does not add the "implicit" features for optional dependencies to the features table when publishing, that is still inferred automatically when reading the index.

Additionally, a new `"v"` field is added to the index package structure, which is an integer that indicates a "version" of the schema used.
The default value is `1` if not specified, which indicates the schema before `"features2"` was added.
The value `2` indicates that this package contains the new feature syntax, and possibly the `"features2"` key.
During publishing, registries are responsible for setting the `"v"` field based on the presence of the new feature syntax.

The version field is added to help prevent older versions of Cargo from updating to newer versions of package that it doesn't understand.
Cargo, since 1.51, already supports the `"v"` field, and will ignore any entries with a `"v"` value greater than 1.
This means that running `cargo update` with a version older than 1.51 (published 2021-03-25) may not work correctly when updating a package that starts using the new syntax. This can have any of the following behaviors:

1. It will update to the new version and work just fine if nothing actually uses the new feature syntax.
2. It will skip the package if something requires one of the new features.
3. It will update and successfully build, but build with the wrong features (because the new features aren't enabled correctly).
4. It will update and the build will fail, because a new feature that is required isn't enabled.
5. The update will fail if a matching version can't be found, since the required features aren't available.

Package authors that want to support versions of Cargo older than 1.51 may want to avoid using the new feature syntax.

## Internal resolver changes

Internally, Cargo will switch to always using the "new" feature resolver, which can emulate the old resolver behavior if a package is using `resolver="1"` (which is the default for editions prior to 2021).
This should not be perceptible to the user, but is a major architectural change in Cargo.

# Drawbacks

* This adds complication to the features syntax. It can be difficult for someone unfamiliar with the `Cargo.toml` format to understand the syntax, and it can be difficult to search the internet and documentation for special sigils.
* It may encourage continuing to add complexity to feature expressions. The Cargo Team wants to avoid having syntax that only experts can understand, and additions like this take us further down that road.
* Cargo has a long history of treating optional dependencies as sharing the same namespace as features. It can take time for seasoned Rust developers to pick up the new syntax and to unlearn how Cargo used to work.
* The errors that versions of Cargo older than 1.51 may generate when trying to use a dependency using the new syntax can be confusing.
* Since the `dep:` syntax no longer explicitly enables a feature of the same name, there may be some scenarios where it can be difficult to write `cfg` expressions to target exactly what the developer wants to make conditional.
  For example:

  ```toml
  [features]
  foo = ["dep:a", "dep:b"]
  bar = ["dep:b"]
  ```

  Here, the developer may want to write code that is conditional on the presence of the `b` dependency.
  With this new system, they may need to write `cfg(any(feature="foo", feature="bar"))` instead of the previously simpler syntax of `cfg(feature="b")`.
  It is intended that in the future, syntax such as [`cfg(accessible(::b))`](https://github.com/rust-lang/rust/issues/64797) will help simplify this situation.
  Another alternative is to rearrange the features, for example making `foo` depend on `bar` in the example above, and then use `cfg(feature="bar")` to check for the presence of `b`.
* The new feature resolver may not emulate the old resolver behavior perfectly.
  A large number of tests of been done to try to ensure that it works the same, but there are some unusual configurations that have not been exercised.
  There is a moderately high risk that this may introduce unintended changes in resolver behavior or other bugs.

# Rationale and alternatives

* The Cargo Team considered many different variants of the syntax expressed here.
  We feel that this hit a desired balance of expressiveness and terseness, but it certainly won't be the perfect match for everyone.
* Instead of introducing the `dep:` syntax, Cargo could continue to keep dependencies and features in the same namespace, and instead introduce other mechanisms such as "private" features to hide optional dependencies.
  However, several people felt that there is a conceptual benefit to splitting them into separate namespaces, and that provided a path to prevent exposing the optional dependencies and being able to use more natural feature names.
* A new publish API could be added (endpoint `api/v2/crates/new`) to ensure that Cargo is not speaking to a registry that does not understand the new syntax.
  This was pursued in [PR #9111](https://github.com/rust-lang/cargo/pull/9111), but it was considered not necessary.
  [crates.io] is the only registry that can support older versions of Cargo.
  Other registries that don't support the new syntax may reject publishing with the new syntax (if they perform validation), or they may accept it (if the don't validate), in which case it should just work.
  The `"v"` field addition is only necessary for Cargo versions between 1.51 and whenever this is stabilized, and most use cases of other registries are generally expected to have stricter control over which versions of Cargo are in use.

# Prior art

[RFC 2957](https://rust-lang.github.io/rfcs/2957-cargo-features2.html#prior-art) contains a survey of other tools with systems similar to Cargo's features.
Some tools treat the equivalent of "features" and "dependencies" together, and some treat them separately.

## Prior issues

The following issues in Cargo's issue tracker cover the initial desires and proposals that lead to this design:

* [#8832](https://github.com/rust-lang/cargo/issues/8832) Tracking issue for weak dependency features
    * [#3494](https://github.com/rust-lang/cargo/issues/3494) Original issue proposing weak dependency features
* [#5565](https://github.com/rust-lang/cargo/issues/5565) Tracking issue for namespaced features
    * [#1286](https://github.com/rust-lang/cargo/issues/1286) Original issue proposing namespaced features

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None at this time.

# Future possibilities
[future-possibilities]: #future-possibilities

* The `package-name/feature-name` syntax may be deprecated in the future. The new syntax is more flexible, and by encouraging only using the new syntax, that can help simplify learning materials and ensure developers don't make mistakes using the old syntax.
