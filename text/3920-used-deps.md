- Feature Name: `used-deps`
- Start Date: 2026-02-12
- RFC PR: [rust-lang/rfcs#3920](https://github.com/rust-lang/rfcs/pull/3920)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

## Summary
[summary]: #summary

Extend Cargo's dependency specifications so users can mark that a dependency is used, independent of what [unused externs](https://doc.rust-lang.org/nightly/rustc/json.html#unused-dependency-notifications) says.

## Motivation
[motivation]: #motivation

Rustc can report to Cargo
[unused externs](https://doc.rust-lang.org/nightly/rustc/json.html#unused-dependency-notifications)
which Cargo can aggregate and report back which dependencies are unused.

However, not all dependencies exist for using their `extern` in the current package, including
- activating a feature on a transitive dependency
- pinning the version of a transitive dependency in `Cargo.toml` (however, normally this would be done via `Cargo.lock` or `target."cfg(false)".dependencies`)

The user needs a way to be able to augment what the compiler reports with their intentions.

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Say I have the following packages:
```toml
[package]
name = "user"

[dependencies]
abstraction = "1.0"
mechanism = { version = "1.0", features = ["semantic-addition"] }
```

```toml
[package]
name = "abstraction"

[dependencies]
mechanism = "1.0"
```

```toml
[package]
name = "mechanism"

[features]
semantic-addition = []
```

In this case, `user` references `abstraction` within its Rust source code.
The dependency on `mechanism` is only to activate the `semantic-addition` feature.

With [cargo#16600](https://github.com/rust-lang/cargo/pull/16600),
Cargo would report `mechanism` as an unused dependency:
```
[WARNING] unused dependency
  --> Cargo.toml:6:1
   |
 6 | mechanism = { version = "1.0", features = ["semantic-addition"] }
   | ^^^^^^^^^
   |
   = [NOTE] `cargo::unused_dependencies` is set to `warn` by default
[HELP] remove the dependency
   |
 6 - mechanism = { version = "1.0", features = ["semantic-addition"] }
   |
[HELP] to keep the dependency, mark it as used:
   |
 6 | mechanism = { version = "1.0", features = ["semantic-addition"], used = true }
   |                                                                +++++++++++++
   |
```

To resolve this, `user` can:
```toml
[package]
name = "user"

[dependencies]
abstraction = "1.0"
mechanism = { version = "1.0", features = ["semantic-addition"], used.reason = "feature activation" }
```

Cargo would see that a `reason` is provided for why this is `used` and silence the lint.

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Within [Specifying Dependencies](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html)

### `used`

Always consider this dependency used

Applicability:
- `[*dependencies]`: yes
- `[workspace.dependencies]`: no
- `[patch]`: no

Type:
- `used = <bool>`: `true` if used, `false` if unused
- `used.reason = <string>`: used, the description is unused and for documentation purposes only

## Drawbacks
[drawbacks]: #drawbacks

`reason` is unused by Cargo and could just as well be a comment.
In contrast, Rust's `reason` can be applied to `warn`, `deny`, and `forbid` lints and reported back in the diagnostic
([reference](https://doc.rust-lang.org/reference/attributes/diagnostics.html#lint-reasons)).

With any of the current options,
if the dependency ever becomes truly unused,
there is no way to report this to the user.

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

As we are restricted to TOML syntax, we don't have a general purpose way of handling lint control like `#[expect]`.

### `used`

Pros:
- Close to the dependency

Downsides:
- Manifest schema is being extended for control of a one off lint which can be incongruous with other lints and encourage the doing this for other lints which could overcomplicate manifest files

### Lint config

```toml
[lints.cargo]
unused_dependencies = { level = "warn", used = ["mechanism"] }
```

Pros:
- Follows a consistent pattern for where to configure lints

Downsides:
- Far away from the dependency specification, making it hard to reason about when looking at `mechanism`
- Need to decide what to do when `mechanism` becomes used or doesn't exist
- This config should not be inherited from `[workspace.lints]`
- Can't inherit `[workspace.lints]` *and* specify lint config in `[lints]` ([#13157](https://github.com/rust-lang/cargo/issues/13157))
- Blunt hammer, not distinguishing which dependencies table this applies to
  - Having it reference the dependencies table makes this more complicated and requires us to have a way to refer to `[dependencies]`

### `_dep`

```toml
_mechanism = { package = "mechanism", version = "1.0", features = ["semantic-addition"] }
```

Pros:
- Close to the dependency
- Aligns with Rust treating `_` as "don't care"
- Low overhead for design and implementation

Downsides:
- Doesn't mix well with kebab case names (e.g. `_regex-syntax`)
- The Cargo team was hesitant to officially support something similar for features ([#10794](https://github.com/rust-lang/cargo/issues/10794))
- Affects sort order, moving it away from any related packages

## Prior art
[prior-art]: #prior-art

### Rust

```rust
// Will warn
let something = foo();

// Ways of resolving
let _ = foo();
let _description = foo();
#[expect(unused_variables, reason = "description")]
let something = foo();

#[expect(unused_variables, reason = "description")]
mod {
  fn bar() {
    let something = foo();
  }
}
```

### `cargo udeps`

[Documentation](https://github.com/est31/cargo-udeps?tab=readme-ov-file#ignoring-some-of-the-dependencies)

```toml
[package.metadata.cargo-udeps.ignore]
normal = ["if_chain"]
#development = []
#build = []

[dependencies]
if_chain = "1.0.0" # Used only in doc-tests, which `cargo-udeps` cannot check.
```

### `cargo machete`

[Documentation](https://github.com/bnjbvr/cargo-machete?tab=readme-ov-file#false-positives)
```toml
[dependencies]
prost = "0.10" # Used in code generated by build.rs output, which cargo-machete cannot check

# in an individual package Cargo.toml
[package.metadata.cargo-machete]
ignored = ["prost"]

# in a workspace Cargo.toml
[workspace.metadata.cargo-machete]
ignored = ["prost"]
```

### `cargo shear`

[Documentation](https://github.com/Boshen/cargo-shear)
```toml
[package.metadata.cargo-shear]
ignored = ["crate-name"]
```

### `cfg(false)` dependencies

To specify a version requirement on a package without using it, you can do:
```toml
[target."cfg(false)".dependencies]
foo = "1.0.0"
```

This would not work for feature activations.

### `peerDependencies`

npm has the concept of
[`peerDependencies`](https://docs.npmjs.com/cli/v11/configuring-npm/package-json#peerdependencies)
for requiring a version of a package if it exists without actually building against this.
This was developed for plugins to specify the version of what they plug into ([announcment](https://nodejs.org/en/blog/npm/peer-dependencies)).

## Unresolved questions
[unresolved-questions]: #unresolved-questions

Is this the right direction?

Is `reason` justified?

## Future possibilities
[future-possibilities]: #future-possibilities

### Programmatic reason

The user being able to tell Cargo the reason in a way that it can verify it and still warn if the reason no longer applies.

For example, say we had:
```toml
[package]
name = "user"

[dependencies]
abstraction = "1.0"
mechanism = { version = "1.0", features = ["semantic-addition"], used.transitive = true }
```
Then if `abstraction` is removed, we could start warning about `mechanism` again.

For `transitive`, this could get complicated to have access to the dependency resolution graph at the time the warning is being handled.

### Potential lints that may need configuration

- [#5340: Lint against non semver-open dependencies](https://github.com/rust-lang/cargo/issues/5340)
- [#9058: Warning when large binary files are included into the bundle](https://github.com/rust-lang/cargo/issues/9058)
- [#13681: Build script allowlist mode](https://github.com/rust-lang/cargo/issues/13681)
- [#15580: Lint for redundant feature names](https://github.com/rust-lang/cargo/issues/15580)
- [#15581: Lint for negative feature names](https://github.com/rust-lang/cargo/issues/15581)
- [#15590: Lint for feature named for private dependency](https://github.com/rust-lang/cargo/issues/15590)
