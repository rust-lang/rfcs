- Feature Name: `custom_lint_profiles`
- Start Date: 2026-03-08
- RFC PR: [rust-lang/rfcs#3926](https://github.com/rust-lang/rfcs/pull/3926)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

## Summary
[summary]: #summary


This proposes the ability to add "lint profiles" to Cargo.toml to allow for wholesale toggling of lint levels in predefined ways.

In essence, it is a much more powerful version of the coarse lint modality currently offered by `-Dwarnings`.

# Motivation



## The many ways of toggling lints

There are multiple ways that lint levels can be toggled in modern Rust. For the purpose of this design we assume usage of Cargo; though some of these work outside of the Cargo world too.

 - In code, by means of `#[allow]` and friends.
     - This **does** support use with `cfg`
     - This **does** allow fine grained control over code sections
     - This **does** allow fine grained control over individual lints
     - This **cannot** be easily tweaked at runtime without having to edit code
     - This **cannot** be easily shared between crates
     - Changing this invalidates the build cache for the edited file/crate.
 - In code, by means of `[lints]` in Cargo.toml.
     - This **does not** support use with `cfg`
     - This **does not** allow fine grained control over code sections
     - This **does** allow fine grained control over individual lints
     - This **cannot** be easily tweaked at runtime without having to edit code
     - This **can** be easily shared between crates (via workspaces)
     - Changing this invalidates the build cache for the edited crate (or the entire workspace if this was in the workspace)
 - In code, by means of `[profiles.foo.rustflags]` (unstable) and `-Afoobar`
     - This **does not** support use with `cfg`
     - This **does not** allow fine grained control over code sections
     - This **does** allow fine grained control over individual lints
     - This **cannot** be easily tweaked at runtime
     - This **can** be easily shared between crates (via workspaces)
     - Changing this invalidates the entire build
 - In the CLI, by means of `RUSTFLAGS=-Afoobar` and friends. 
     - This **does not** support use with `cfg`
     - This **does not** allow fine grained control over code sections
     - This **does** allow fine grained control over individual lints
     - This **can** be easily tweaked at runtime
     - This **is always** shared between crates
     - Changing this invalidates the entire build
 - In the CLI, by means of `RUSTFLAGS=-Dwarnings` or `CARGO_BUILD_WARNINGS=deny`
     - This **does not** support use with `cfg`
     - This **does not** allow fine grained control over code sections
     - This **does not** allow fine grained control over individual lints
     - This **can** be easily tweaked at runtime
     - This **is always** shared between crates
     - Changing this invalidates the entire build
  - In the CLI, by choosing to call `cargo clippy`
      - (This is technically a modality too)
      - This usually rebuilds the workspace.

At first glance, it appears that fine grained control is available at both "code editing time" and at runtime, however `-Afoobar` is not pleasant to use at all when you are configuring hundreds of lints. What is missing is a way to toggle groups of lints on and off at runtime, where these groups can be controlled by the developer at a fine grained level in source code somewhere.

Furthermore, `-Afoobar`, either via `[profiles]` or via `RUSTFLAGS` works poorly with Cargo: most solutions for doing this at runtime can trigger recompilation of the entire crate graph. `[lints]` was developed in part as a way to avoid this problem.

By and large, people currently use a mix of `-Dwarnings` and separately calling `cargo clippy` as a way to run different sets of lints on the same codebase. This proposal aims to expand this ability.


## When is a lint noticed by the user?

One of the subtleties here is that different lint levels have different bars of noticeability depending on *where* they are run. A `warn` rustc lint will be noticed locally, and perhaps noticed locally when running `clippy` (depending on workflow!), but will not be noticed if run just in CI. A `deny` lint will be noticed in CI but may be bothersomely in the way when running locally.

"PR-integrated CI linters" like [clippy-action] and [clippy-sarif] which leave notes on PRs change the dynamic here a little bit, such that "warn" lints run by CI may still be noticed by the user (but potentially only on code they are changing).

Tools like rust-analyzer can also change the dynamic here, such that "warn" lints do not inundate the user but rather are noticed as a soft note/highlight somewhere.

In essence, lints can have different effects in different contexts.



 [clippy-action]: https://github.com/giraffate/clippy-action
 [clippy-sarif]: https://crates.io/crates/clippy-sarif

## Lint modalities and their use cases

Overall, it's quite common in codebases to want to have different "modes" for lints for the different contexts a linter might be run in.
### Deny in CI

The most common use case is wanting to have the codebase be lint-free but not hinder development while hacking on something, but have the lints gate landing on `main`. Workflows around this typically involve running CI with `-Dwarnings` (or the new `CARGO_BUILD_WARNINGS=deny`), with contributors often running `cargo check` / `cargo clippy` locally and ensuring they are warnings-clean before opening a PR.

### Noisier PR-integrated linters or IDEs

If using a PR-integrated CI linter, your bar for non-blocking noisy informative lints can be lower since the linter will only flag things in code touched by the current PR. One may wish to enable far more pedantic lints in such CI.

_Ideally_ one can do this in the same CI task as "deny in CI".

Something similar can happen for IDEs which have a relatively muted lint display (often a small :warning: icon near the offending line of code): it's somewhat fine to inundate the programmer with lints because they'll only see the ones affecting the files they are editing, not every file. This expands the scope of lints a user is exposed to without necessarily showing them more than a manageable number of lints.


### Upgrade workflows

Large projects, as is recommended, often pin a Rust version for their clippy CI, so that their developers are not hit by piles of failures just because the compiler updated.

Usually, once a new Rust compiler is released, these projects will spend some time updating everything and make a new PR.

It is sometimes nice to be able to do this at a crate-by-crate level. Especially large projects would like to be able to update their CI toolchain without needing to fix lints everywhere. Having more flexible control over lint levels would allow them to e.g. disable new lints by default, but allow individual crates to opt in to the newer lints, giving a smoother migration path that can be handled at an appropriate pace for individual subcomponents.

### Check at release time

In some cases, a lint would be too noisy to deny in CI, but people expect to have the release be lint-free and use that as an opportunity to clean things up. Such a workflow typically involves calling `cargo clippy -D{lints}  --fix ` and then `cargo clippy -D{lints}` to catch any stragglers. I've mostly seen this around lints that are _automatable_: It's just not worth it to ask contributors to fix this each PR, but it is worth it to run a pass right before release. 

This isn't a lint, but similar workflows can be found around `rustfmt`'s "format code in doc comments" mode: not worth it to require everyone to do all the time, but worth running before releases.

Currently, this requires a manual specification of all the relevant lints.


### Only on certain types of targets

Some lints protect production code from things like panics and bad API choices, things which aren't as much of a big deal (or even, counterproductive to prevent) for test code. It's common to do something like `#[cfg_attr(test, allow(...))]`, however this can't be combined with the Cargo `[lints]` table, making it less useful as a feature.

Typically you want something that applies to `cargo test`, `cargo bench`, and test/bench targets during `cargo check --all-targets`.

Clippy has a patchwork of config options that disable lints in tests, like `allow-unwrap-in-tests`, however not all lints have this, and [they don't work consistently in all test code](https://github.com/rust-lang/rust-clippy/issues/13981). So far most codebases I have worked on end up with a lot of allows in test code for lints that would be easier to global allow.

Similarly, someone may wish to only enable certain lints on bin targets.


### "Teaching" lints

A proposal that comes up semi-regularly is for there to be lints that teach you more about your code. These may not even point out _mistakes_, but rather teach you things about code you are writing. While lint profiles doesn't solve this problem entirely, being able to toggle sets of lints for the purposes of "I am learning Rust and want some more helpful nudges" helps solve part of the problem for designs here.

# Guide level explanation


Currently, in Cargo.toml, it is possible to control lints with the `[lints]` section, like this:

```toml
[lints.rust]
unsafe_code = "forbid"
[lints.clippy]
pedantic = "warn"
enum_glob_use = { level = "deny", priority = 0 }
```

In the context of this proposal, this set of lint settings will be called the "default" profile.


This proposal adds the ability to define custom lint profiles:

```toml
[lints.profile.ci]
inherits = "default"
[lints.profile.ci.clippy]
pedantic = "allow"
correctness = "deny"
```

Lint profiles can control lint groups and lints as usual. They can also "inherit" from an existing profile, which means that they copy over the settings from that profile, applying further lint levels on top.


Lint profiles can be controlled from the CLI:

```
$ cargo build --lints ci
```


Open question: What should the `--lints` flag be called? `--lints`? `--lint-profiles`?

Open question: `profile` is ambiguous with build profiles. Should we pick a different name?

## Inheritance and workspaces

Lint profiles can be inherited from the same package and from the workspace. The profile specified in `[lints]` is called `default`.

Open question: Should it be possible to also access the default profile via `[lints.profile.default]`? What's the behavior of specifying both? Probably doesn't matter too much. Either way, we should reserve the profile name `default`.



```toml
# Simple inheritance

[lints]
unused = "allow"
missing-debug-implementations = "warn"
[lint.profile.ci]
inherits = "default" # inherits from the default profile
unused = "deny" # overrides `unused` preference

# Workspace examples

# Inherit all lints and lint profiles from workspace
[lints]
workspace = true

# Inherit just the ci profile from the workspace
[lint.profile.ci]
workspace = true

# Inherit just the default profile (`[lints.lintname]` / `[lints.clippy.lintname]`) from the workspace
[lints.profile.default]
workspace = true

# Inherit the ci profile from the workspace but override some things
[lint.profile.ci]
workspace = true
[lint.profile.ci.clippy]
ptr-eq = "deny" # this is a lint
correctness = "warn" # this is a lint group
```

Note that if you wish to inherit from a profile defined in the workspace, you must first inherit the profile via `lints.profile.profilename.workspace = true`, and then you can inherit from `profilename`.


## Changing warn levels wholesale

It is possible to use lint profiles to map warnings to another level.

For example, the following is equivalent to using `-Dwarnings` in CI: 


```toml
[lints.profile.ci]
warn = "deny"
```

The only allowed values here `"deny"`, `"warn"`, and `"allow"`.

Open question: Someone should pick how `warn = "deny"` prioritizes with [`build.warnings`](https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#warnings). I don't see any particular choice as having more merit, but a choice must be made.

Open question: Should it be `warn = "deny"` or `warnings = "deny"`?

This can be used when inheriting profiles to map `warn` to `deny` for the lint groups that are inherited:


```toml
[lints]
some-lint = "warn"
some-other-lint = "warn"
[lints.profile.ci]
inherits = { profile = "default", warn = "deny" }
[lints.profile.ci]
deprecated = "warn"
clippy.some-noisy-lint = "warn"
clippy.some-other-noisy-lint = "warn"
```

This creates a profile that inherits from the default profile, but with all warnings replaced with hard errors, and further tweaks to some other lints.

This has no impact on lint levels specified in the source code, or warnings that come from places other than the warning system (in other words, this does not apply `-Dwarnings`)


This is useful with PR-integrated CI linters, where you want to simultaneously:

 - Ensure the code is lint-free with the regular (default) profile
 - Ensure additional nitpicky lints _show up_ on PRs so that people try to fix them (but don't block landing)



### Workspace inheritance

The following patterns are all legal:

```toml
# Inherit all lints and lint profiles from workspace
[lints]
workspace = true

# Inherit the ci profile from the workspace
[lint.profile.ci]
workspace = true

# Inherit just the default profile (`[lints.lintname]` / `[lints.clippy.lintname]`) from the workspace
[lints.profile.default]
workspace = true

# Inherit the ci profile from the workspace but override some things
[lint.profile.ci]
workspace = true
[lint.profile.ci.clippy]
ptr_eq = "deny" # this is a lint
correctness = "warn" # this is a lint group
```

Use of `workspace = true` does not prevent addition of new profiles or tweaking of existing ones. 

Note that currently, `[lints] workspace = true` cannot be combined with explicitly specified lints.




## Test-only lint levels

There are two ways of making test modalities work here. One is less powerful but simpler, the other will complicate the implementation. This RFC has not yet selected one, but I prefer Option 1.

Open question: Should we go with Option 1 or 2 for tests?


### Option 1: A `test` subprofile

Each lint profile contains a "test" sub-profile, which behaves like a normal profile, but is applied when you build `cfg(test)` binaries with that profile in test mode. These are integration tests, unit tests, doctests, and benchmarks.



```toml
[lints.profile.ci.test]
some-lint = "allow"
```

This profile can inherit from other profiles normally:

```toml
[lints.profile.testprofile]
some-lint = "allow"

[lints.profile.ci.test]
some-other-lint = "allow"
inherits = "testprofile"
```

When building with `--lints ci`, all the lints specified in the `ci` profile will be used, plus any overrides from the testing subprofile.

When inheriting a profile into a regular profile, its `test` sub-profile is also inherited.

(It's unclear if this really needs to support inheritance.)

Open question: Should we introduce `bench`/`doc` lint profiles as well? I don't quite see this as necessary, but it's a simple enough extension.

Open question: Should we support inheritance with test profiles?

### Option 2: Cfg-gated lints

This works similarly to how you can specify cfg-specific dependencies

```toml
[lints.profile.ci.'cfg(test)']
indexing-slicing = "allow"
```

This is a lot more flexible, but it might be too flexible: is there actually a use case for target-specific lints? There is some use case for different lints based on `cfg(test)`, `cfg(doc)`, `cfg(bench)`, etc, but less so for `cfg(windows)`.

On the other hand, this does not allow for inheritance, which is simpler. Similar to dependencies, this just lets you conditionally specify additional members.

# Reference-level explanation

## General feature

A lint profile can be specified as

```toml
[lints.profile.nameofprofile]
workspace = true
lintname = "allow" # allow, warn, deny, forbid
clippy.lintname = "allow" # same
warn = "deny" # allow, warn, deny
inherits = "default" # a name of a profile
# or
inherits = {profile = "default", warn = "deny"}  # name of a profile, and an allow/warn/deny value
# Option 1
test = {} # can contain all the same fields as above, except for `test` itself
# Option 2
'cfg(test)'.lintname = "allow"

```

The "default" lint profile specified as `[lints]` can also have the same fields.

The profile can be selected via a `--lints` flag available in all commands that produce a build: `build`, `test`, `run`, `check`, `bench`.


Custom profiles cannot be named `default`. The name `default` is reserved for referencing the default profile.

Rustc cannot add lints named `workspace`, `inherits`, `warn`, or `test`.

Note that currently [workspace overriding is not supported][ws-override]. More on that below.

## Inheritance

Internally, each lint profile is *resolved* to a list of lints and lint levels, plus an optional `warn = "somelevel"` setting. In case we go with Option 1 for tests, each profile contains an additional "test" profile. If we go with option 2, it also contains a list of lints and lint levels with `cfg` predicates. This lint is sorted in definition order.

When profiles are inherited via `inherits` or `workspace = true`, it is the resolved profile that is inherited. Further overrides are applied on top of that resolved profile, producing a new resolved profile. We do not have multiple inheritance: a single resolved profile is inherited and further overrides can be applied on top of it.

Loops are not allowed during inheritance.

`lints.workspace = true` copies all resolved lint profiles (including the default one) from the workspace.

When using `{inherits = "someprofile", warn = "deny"}`, the resolved profile has all `warn` entries replaced with `deny` entries before being inherited.


When running with a resolved profile, it is simply a matter of applying the `-A`/`-W`/etc flags specified by the lint list in the resolved profile. Inheritance is already "handled" once you compute a resolved profile.

The interaction of testing solutions with resolved profiles will be covered below.

### The workspace override issue

If we choose to fix the [workspace override][ws-override] issue in this RFC, then the following will work:

```toml
[lints]
workspace = true
some-lint = "allow"


[lints.profile.ci] # assume the workspace has a `ci` profile
some-other-lint = "allow"
```

Here, the crate will have a default and `ci` resolved profile copied from the workspace, with overrides applied on top.



## Interaction with workspaces

Lint profiles set via CLI flag are only relevant for the current workspace. Dependencies outside of the workspace cannot be expected to use the same naming scheme for profiles, and it is unlikely that users will wish to run lints on third-party dependencies. `--lints ci` will set lints to the levels defined by the `ci` profile for all crates in the workspace, for crates with such a profile (falling back to `default` otherwise).

Not having such a profile is not a hard error in this mode since it's acceptable to not have the profile defined on every workspace crate.

Open question: When should it be a hard error to specify `--lints foo` for a nonexistant profile `foo`? 

Open question: Would it hurt to *by default* inherit lint profiles from the workspace? A straightforward implementation would break current behavior of `[lints]` (which does not autoinherit, though perhaps it ought to?), but we could make this behavior kick in only when you specify `--lint someprofile` and `someprofile` is defined on the workspace but not the individual crate.

## Interaction with rustc

This design lives entirely in Cargo: In essence it uses the existing `[lints]` infrastructure (which just feeds `rustc` a bunch of flags).

`#[allow()]`s (etc) in Rust cannot reference lint profiles, they interact with the `[lints]` table much the same as they do today.

It would be an interesting extension to allow rustc to be provided with custom named lint groups that can be toggled wholesale, see "future work" below.

## Interaction with regular (build) profiles

Cargo already allows one to configure `[profile.dev]` `[profile.release]` and custom named profiles for choosing compilation flags.

This iteration of this proposal does not have any overlap between lint profiles and regular profiles, you can select them in free variation. Lint profile names do not need to match regular profile names.

The way I see it is that the regular compilation profile is about what kind of artifact you want, whereas the lint profile is more about the actual compilation experience wrt warnings. Lints don't affect the final artifact.

Regular profiles already have inheritance (etc), so it _is_ tempting to merge the two, and integrates nicely. But this will tie lint profiles to regular profiles which may lead to people needing to define profiles like `release-default` and `release-ci` to get different sets of lints.

Overall I see the use case of toggling lints to be different from that of toggling other compilation flags.

Open question: Maybe we want to merge lint profiles with profiles anyway? Or perhaps provide a way to set a profile's default lint profile? I haven't seen a strong motivation for this yet.

## Testing: Testing subprofiles (Option 1)

Each profile's `test` subprofile is resolved by taking the "parent" profile and then overlaying any overrides from the test subprofile. It is applied whenever something is being built with `--cfg test` while using that profile. The `test` subprofile is itself resolved

When inheriting a profile, the resolved `test` subprofile is also inherited, and further overrides can be applied on top.

It's not clear if `test` profiles need to support inheritance, but if we decide to support that, then an `inherits = ` key in a test subprofile will *switch* the base profile used for inheritance to being the specified profile, rather than the parent profile.

## Testing: CFG'd lints (option 2)


As noted before, these inherit with their predicates. When computing the set of lint levels, the resolved profile is taken, and all `cfg-`predicated lint levels that apply to the current build setting are applied in definition order (inherited cfgs apply first).


# Drawbacks

I think the main drawback here is that this is Cargo.toml-focused, so it requires people to buy in to doing lints via `[lints]` and not any other method if they wish to have the benefits.

This is also not a simple system: is the complexity worth it?

Similarly, the Cargo team is worried about adding too many CLI flags, since each flag affects the discoverability of other flags.

# Rationale and alternatives

Overall I think there are a lot of reasons to have "modalities" for lints, some already in use, some attained by a patchwork of features, and some that people would likely use if they were more convenient. Having profiles directly addresses them by giving the user a way to define and name a modality, so they or their CI/IDE/tooling/whatever can pick the right one based on the context.


One major alternative is allowing `[lints]` to exist in `[profiles]` (having them inherit the default profile if so). This could work, but it doesn't allow some of the things this RFC does, like disabling lints in test mode, or inheriting with a warn -> deny transform.

[RFC 3730: Add a semantically non-blocking lint level][RFC 3730] attempts to address the problem of wanting lints to be blocking in some contexts and non blocking in others, essentially by extending `-Dwarnings` to be a bit more flexible, and adding an IDE-useful "nit" lint level that only applies to new code. I think this attempts to solve a bunch of the same problems as this, but it solves them more narrowly: it doesn't help non-IDE users as much, and doesn't solve things like the test mode problem.

Both of these solutions are likely to be simpler, though.

# Prior art

The [custom named Cargo profiles](https://rust-lang.github.io/rfcs/2678-named-custom-cargo-profiles.html) RFC is probably the main prior art here. I think this feature works well, including profile inheritance.


These designs are superficially similar, but not super similar in the details. Cargo profiles are workspace-level, whereas this is package-level. The inheritance works similarly, where each profile is a list of key-value pairs, and when inherited that list of pairs is inherited with overrides being applied on top.

# Unresolved questions


These are all mentioned inline in the RFC under "open question", but duplicated here for referencing. I prefer having these discussions inline in the RFC.

 - What should the `--lints` flag be called? `--lints`? `--lint-profiles`?
 - `profile` is ambiguous with build profiles. Should we pick a different name?
 - Should it be possible to also access the default profile via `[lints.profile.default]`?
 - Someone should pick how `warn = "deny"` prioritizes with [`build.warnings`](https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#warnings).
 - Should it be `warn = "deny"` or `warnings = "deny"`?
 - Should we go with Option 1 or 2 for tests?
 - In Option 1, should we introduce `bench`/`doc` lint profiles as well?
 - In Option 1, should we support inheritance with test profiles?
 - Maybe we want to merge lint profiles with profiles anyway? 
 - When should it be a hard error to specify `--lints foo` for a nonexistant profile `foo`? 
 - Would it hurt to *by default* inherit lint profiles from the workspace?

Other unresolved questions:

Should using this feature require an MSRV bump? Technically crates consuming your crate do not need to care about the `[lints]` section, but older versions of Cargo are likely to misinterpret `lints.profile`. Needs investigation.


# Future work


## Custom lint groups


A thing this feature does *not* let one do is toggle multiple lints at once in code sections, a feature that is useful to have. A *potential* extension of this feature would be to allow profiles to be defined as lint groups so that one can write `#[allo w(customprofile)]`. There's a lot of subtletlies of such a design, including:

 - How does one provide this specification to rustc?
 - Profiles contain `allow`s and `warns` and other things, they are not _just_ a grouping of lints. What does it mean to `#[warn(group)]` for a group that contains some `allow`s and some `warn`s? Does it turn on every lint explicitly mentioned in the profile? Does it turn on every `allow` lint from the profile? There's not an easy answer here.


It's not yet clear to me how feasible this is, or if we should have such a feature, but it's worth listing.


## Teaching lints

In the past people wished for tooling that produces lints that potentially tell new users about subtleties in their code, subtleties that are not really *problems* to be fixed, but interesting things to be noted. These would be opted in to by individual users and as they get used to a concept, disabled globally one by one. Lint profiles allow one to better handle toggles like this, but it is not in an of itself a major step in this direction.


## Further lint levels

Currently there are only two choices for lint UX: either the lint is a hard error or it is not, and it shows up the same way either way in the CLI. IDEs may choose to offer some customizeability but it's typically global.

It would be nice to have more choices for the UX of lints, and, ideally, those choices could be targeted to specific noisy-and-less-important lints. A "nit" lint level, similar to that proposed in [RFC 3730], could serve this purpose.

An example of improved lint UX (in CLI) for a noisy-and-less-important lint would be that instead of showing each instance of the lint, the error message could be something like:

```
warn: found 5 instances of the `needless_borrow` lint:
 
 - src/foo.rs:32
 - src/bar.rs:45
 - ...
 
```

This works well with lint groups since you may then upgrade nits to warnings when you decide to try and fix these. PR-integrated linters can also choose to have different behavior with these if desired.


 [RFC 3730](https://github.com/rust-lang/rfcs/pull/3730)