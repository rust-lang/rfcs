- Feature Name: source_replacement_ambiguity
- Start Date: 2022-07-05
- RFC PR: [rust-lang/rfcs#3289](https://github.com/rust-lang/rfcs/pull/3289)
- Tracking Issue: [rust-lang/cargo#10894](https://github.com/rust-lang/cargo/issues/10894)

# Summary
[summary]: #summary

When Cargo is performing an API operation (`yank`/`login`/`publish`/etc.) to a source-replaced `crates-io`, require the user to pass `--registry <NAME>` to specify exactly which registry to use. Additionally, ensure that the token for `crates-io` is never sent to a replacement registry.

# Motivation
[motivation]: #motivation

There are multiple issues that this RFC attempts to resolve around source-replacement.

* When Cargo is performing an API operation, source replacement is only respected for `crates-io`, not alternative registries. This is inconsistent.
* The [error message](https://github.com/rust-lang/cargo/issues/6722) for attempting to publish to a replaced crates-io is confusing, and there is no workaround other than temporarily removing the source replacement configuration.
* When performing an API operation other than `publish` with a replaced `crates-io` source, the `crates-io` credentials are sent to the replacement registry's API. This is a security risk.
* It's unclear which credentials should be used when fetching a source-replaced authenticated alternate registry ([RFC 3139][3139]).

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When the `crates-io` source is replaced, the user needs to specify `--registry <NAME>` when running an API operation to disambiguate which registry to use. Otherwise, `cargo` will issue an error.

`cargo` only sends the token associated with a given registry to that registry and no other (even if source replacement is configured).

When replacing a source with a registry, the `replace-with` key can reference the name of a registry in the `[registries]` table.

## Example scenarios

### Local source replacement (vendoring)
A repository has a local `.cargo/config.toml` that vendors all dependencies from crates.io. Fetching and building within the repository would work as expected with the vendored sources.

If the user decides to publish the crate, `cargo publish --registry crates-io` will ignore the source-replacement and publish to crates.io.

### `crates-io` mirror registry
A server has been set up that provides a complete mirror of crates.io. The user has configured a `~/.cargo/config.toml` that points to the mirror registry in the `[registries]` table. The mirror requires authentication to access (based on [RFC 3139][3139]).

The user can log in to the mirror using `cargo login --registry mirror`. Fetching and building use the mirror.

The user decides to publish the crate to crates.io, and does `cargo login --registry crates-io` to log in to crates.io. Source replacement is ignored, and the token is saved.

Next, the user runs `cargo publish --registry crates-io` to publish to crates.io. Cargo ignores source replacement when building and publishing the crate to crates.io.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

### Change 1: respect `--registry`
When running an API operation (`login`, `logout`, `owner`, `publish`, `search`, `yank`), Cargo always uses the registry specified by `--registry <NAME>`, and never a source-replacement.

### Change 2: error for replaced crates-io
When running an API operation (as defined above) and ALL of the following are true:
* `crates-io` has been replaced by a remote-registry source.
* command line argument `--registry <NAME>` is not present.
* command line argument `--index <URL>` is not present.
* `Cargo.toml` manifest key `publish = <NAME>` is not set (only applies for publishing).

`cargo` issues an error:
```
error: crates-io is replaced: use `--registry replacement` or `--registry crates-io`
```

### Change 3: credentials are only sent to the same registry
If the `crates-io` source is replaced with another remote registry, the credentials for
`crates-io` are never sent to the replacement registry. This makes `crates-io` consistent
with alternative registries and ensures credentials are only sent to the registry they are
associated with.

### Change 4: `[source]` table can reference `[registries]` table
The `replace-with` key in the `[source]` table can reference a registry defined in the `[registries]` table.

For example, the following configuration would be valid:

```
[source.crates-io]
replace-with = "my-registry"

[registries.my-registry]
index = "https://my-registry-index/"
```

This is necessary to allow the `--registry <NAME>` command-line argument to work with source-replaced registries. It also allows additional configuration (such as a token) to be specified for a source-replacement registry without duplicating configuration between `[registries]` and `[source]` tables.

# Drawbacks
[drawbacks]: #drawbacks

Behavior is changed around where credentials are sent, which could break some workflows.

If a mirror of crates.io is set up with `config.json` containing `"api": "https://crates.io"`, then the current system of sending the crates.io token to the replaced source would work correctly, and this RFC would break it.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Alternative: ignore source replacement for API operations
When doing an API operation with a replaced `crates-io`, `cargo` would ignore source replacement without additional arguments. This is how alternative registries currently work.

If the user wants to use the replacement, they could pass `--registry <NAME>`, but would not be required to do so.

A new option `--respect-source-config` could be added to make cargo follow the source replacement for API operations (similar to what we already have for `cargo vendor`).

This may be too confusing for users since it silently changes behavior. The RFC proposes a solution that requires the user to be explicit about which registry to use in the ambiguous situation (crates-io replacement).

## Alternative: disallow source replacement for API operations

Attempting an API operation on a replaced source would be an error. The user could use `--registry crates-io` to explicitly bypass the source replacement.
```
Error: <operation> is not supported on replaced source `crates-io-mirror`; use `--registry crates-io` for the original source
```

# Prior art
[prior-art]: #prior-art

Other package managers don't seem to have a source replacement feature.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

Should the `--registry <NAME>` command line argument be allowed to reference the name of a `source` from the `[source]` table as well? This makes it more flexible, but adds potentially unnecessary complexity.

Cargo's tests rely on the ability to replace the crates.io source and have the crates.io credentials go to the replaced source. We need a way for these tests to continue working. 

# Future possibilities
[future-possibilities]: #future-possibilities

Can't think of anything.

[3139]: https://rust-lang.github.io/rfcs/3139-cargo-alternative-registry-auth.html
