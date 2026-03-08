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
 - In code, by means of `[lints]` in Cargo.toml.
     - This **does not** support use with `cfg`
     - This **does** allow fine grained control over code sections
     - This **does** allow fine grained control over individual lints
     - This **cannot** be easily tweaked at runtime without having to edit code
     - This **can** be easily shared between crates (via workspaces)
 - In code, by means of `[profiles.foo.rustflags]` and `-Afoobar`
     - This **does not** support use with `cfg`
     - This **does not** allow fine grained control over code sections
     - This **does** allow fine grained control over individual lints
     - This **cannot** be easily tweaked at runtime
     - This **can** be easily shared between crates (via workspaces)
 - In the CLI, by means of `RUSTFLAGS=-Afoobar` and friends. 
     - This **does not** support use with `cfg`
     - This **does not** allow fine grained control over code sections
     - This **does** allow fine grained control over individual lints
     - This **can** be easily tweaked at runtime
     - This **is always** shared between crates
 - In the CLI, by means of `RUSTFLAGS=-Dwarnings` or `CARGO_BUILD_WARNINGS=deny`
     - This **does not** support use with `cfg`
     - This **does not** allow fine grained control over code sections
     - This **does not** allow fine grained control over individual lints
     - This **can** be easily tweaked at runtime
     - This **is always** shared between crates
  - In the CLI, by choosing to call `cargo clippy`
      - (This is technically a modality too)

At first glance, it appears that fine grained control is available at both "code editing time" and at runtime, however `-Afoobar` is not pleasant to use at all when you are configuring hundreds of lints. What is missing is a way to toggle groups of lints on and off at runtime, where these groups can be controlled by the developer at a fine grained level in source code somewhere.

Furthermore, `-Afoobar`, either via `[profiles]` or via `RUSTFLAGS` works poorly with Cargo: most solutions for doing this at runtime can trigger recompilation of the entire crate. `[lints]` was developed in part as a way to avoid this problem.

By and large, people currently use a mix of `-Dwarnings` and separately calling `cargo clippy` as a way to run different sets of lints on the same codebase. This proposal aims to expand this ability.


## When is a lint noticed by the user?

One of the subtleties here is that different lint levels have different bars of noticeability depending on *where* they are run. A `warn` rustc lint will be noticed locally, and perhaps noticed locally when running `clippy` (depending on workflow!), but will not be noticed if run just in CI. A `deny` lint will be noticed in CI but may be bothersomely in the way when running locally.

"PR-integrated CI linters" like [clippy-action] and [clippy-sarif] which leave notes on PRs change the dynamic here a little bit, such that "warn" lints run by CI may still be noticed by the user (but potentially only on code they are changing).

Tools like rust-analyzer can also change the dynamic here, such that "warn" lints do not inundate the user but rather are noticed as a soft note/highlight somewhere.

In essence, lints can have different effects in different contexts.

## Lint modalities and their use cases

Overall, it's quite common in codebases to want to have different "modes" for lints for the different contexts a linter might be run in.
### Deny in CI

The most common use case is wanting to have the codebase be lint-free but not hinder development while hacking on something, but have the lints gate landing on `main`. Workflows around this typically involve running CI with `-Dwarnings` (or the new `CARGO_BUILD_WARNINGS=deny`), with contributors often running `cargo check` / `cargo clippy` locally and ensuring they are warnings-clean before opening a PR.

### Noisier PR-integrated linters or IDEs

If using a PR-integrated CI linter, your bar for non-blocking noisy informative lints can be lower since the linter will only flag things in code touched by the current PR. One may wish to enable far more pedantic lints in such CI.

_Ideally_ one can do this in the same CI task as "deny in CI".

Something similar can happen for IDEs which have a relatively muted lint display (often a small :warning: icon near the offending line of code): it's somewhat fine to inundate the programmer with lints because they'll only see the ones affecting the files they are editing, not every file. This expands the scope of lints a user is exposed to without necessarily showing them more than a manageable number of lints.


### Check at release time

In some cases, a lint would be too noisy to deny in CI, but people expect to have the release be lint-free and use that as an opportunity to clean things up. Such a workflow typically involves calling `cargo clippy -D{lints}  --fix ` and then `cargo clippy -D{lints}` to catch any stragglers. I've mostly seen this around lints that are _automatable_: It's just not worth it to ask contributors to fix this each PR, but it is worth it to run a pass right before release. 

This isn't a lint, but similar workflows can be found around `rustfmt`'s "format code in doc comments" mode: not worth it to require everyone to do all the time, but worth running before releases.

Currently, this requires a manual specification of all the relevant lints.


### Only on non-test-code

Some lints protect production code from things like panics and bad API choices, things which aren't as much of a big deal (or even, counterproductive to prevent) for test code. It's common to do something like `#[cfg_attr(test, allow(...))]`, however this can't be combined with the Cargo `[lints]` table, making it less useful as a feature.

Typically you want something that applies to `cargo test`, `cargo bench`, and test/bench targets during `cargo check --all-targets`.

Clippy has a patchwork of config options that disable lints in tests, like `allow-unwrap-in-tests`, however not all lints have this, and [they don't work consistently in all test code](https://github.com/rust-lang/rust-clippy/issues/13981). So far most codebases I have worked on end up with a lot of allows in test code for lints that would be easier to global allow.


### "Teaching" lints

A proposal that comes up semi-regularly is for there to be lints that teach you more about your code. These may not even point out _mistakes_, but rather teach you things about code you are writing. While lint profiles doesn't solve this problem entirely, being able to toggle sets of lints for the purposes of "I am learning Rust and want some more helpful nudges" helps solve part of the problem for designs here.




# Design

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
[lints.profiles.ci]
inherits = "default"
[lints.profiles.ci.clippy]
pedantic = "allow"
correctness = "deny"
```

Lint profiles can control lint groups and lints as usual. They can also "inherit" from an existing profile, which means that they copy over the settings from that profile, applying further lint levels on top.


Lint profiles can be controlled from the CLI:

```
$ cargo build --lints ci
```

Open question: what should the flag be called? Should it also be available as an environment variable? `--lints` is short but perhaps too short, `--lint-profiles` also seems nice.

Open question: Should it be possible to also access the default profile via `[lints.profiles.default]`? What's the behavior of specifying both? Probably doesn't matter too much.

## Interaction with workspaces

Lint profiles set via CLI flag are only relevant for the current workspace. Dependencies outside of the workspace cannot be expected to use the same naming scheme for profiles, and it is unlikely that users will wish to run lints on third-party dependencies. `--lints ci` will set lints to the levels defined by the `ci` profile for all crates in the workspace, for crates with such a profile (falling back to `default` otherwise).

Not having such a profile is not a hard error in this mode since it's acceptable to not have the profile defined on every workspace crate.

Open question: When should it be a hard error to specify `--lints foo` for a nonexistant profile `foo`? 

Open question: Would it hurt to *by default* inherit lint profiles from the workspace? A straightforward implementation would break current behavior of `[lints]` (which does not autoinherit, though perhaps it ought to?), but we could make this behavior kick in only when you specify `--lint someprofile` and `someprofile` is defined on the workspace but not the individual crate.


### Workspace inheritance

The following patterns are all legal:

```toml
# Inherit all lints and lint profiles from workspace
[lints]
workspace = true

# Inherit the ci profile from the workspace
[lint.profiles.ci]
workspace = true

# Inherit just the default profile (`[lints.lintname]` / `[lints.clippy.lintname]`) from the workspace
[lints.profiles.default]
workspace = true

# Inherit the ci profile from the workspace but override some things
[lint.profiles.ci]
workspace = true
[lint.profiles.ci.clippy]
ptr_eq = "deny" # this is a lint
correctness = "warn" # this is a lint group
```

Use of `workspace = true` does not prevent addition of new profiles or tweaking of existing ones. 

Note that currently, `[lints] workspace = true` cannot be combined with explicitly specified lints.


## Applying `-Dwarnings` to a build

This straight up applies `-Dwarnings`. The only allowed values here are `"deny"`, `"warn"`, and `"allow"`.


```toml
[lints.profiles.ci]
warn = "deny"
```

Open question: Someone should pick how this prioritizes with [`build.warnings`](https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#warnings). I don't see any particular choice as having more merit, but a choice must be made.

Open question: `warn` or `warnings`?

## Changing lint levels in bulk based on existing profile

This provides a more flexible way to do `-Dwarnings`-style transforms when inheriting.

```toml
[lints.profiles.ci]
inherits = { profile = "default", warn = "deny" }
[lints.profiles.ci.rust]
deprecated = "warn"
[lints.profiles.ci.clippy]
some-noisy-lint = "warn"
some-other-noisy-lint = "warn"
```

This creates a profile that inherits from the default profile, but with all warnings replaced with hard errors, and further tweaks to some other lints.

This can be used to turn `warn` or `deny` lint level into any other lint level _when inheriting_. This has no impact on lint levels specified in the source code, or warnings that come from places other than the warning system (in other words, this does not apply `-Dwarnings`)

This could work well with PR-integrated CI linters, where you want to simultaneously:

 - Ensure the code is lint-free with the regular (default) profile
 - Ensure additional nitpicky lints _show up_ on PRs so that people try to fix them (but don't block landing)


## Test modalities

There are two ways of making test modalities work here. One is less powerful but simpler, the other will complicate the implementation.

Open question: Pick one

### Option 1: A magic `test` profile

By default, all crates have the implicit equivalent of this:

```toml
[lints.profiles.test]
inherits = "default"
```

This profile is what is chosen when running `cargo test`/`cargo bench` or building `test` (including integration, unit, and doc tests) and `bench` targets during `cargo check --all-targets`.

Explicity filling in the `test` profile allows for selecting a different set of lints to include there. For example, one may choose to:

```toml
[lints.profiles.test.clippy]
indexing-slicing = "allow"
unwrap-used = "allow"
expect-used = "allow"
```

Open question: A potential tweak would be to introduce separate `bench`/`doc` lint profiles as well. I don't quite see this as necessary, but it's a simple enough extension.

This profile selection can still be overridden with `--lints somethingelse`

This is pretty straightforward, but a lot of the other configurability in this feature is lost on tests. With a special `test` profile, one can't, for example, have a distinction between test and non-test lints work for PR integrated CI linters.

### Option 2: Test-only contexts

Instead, a different way to do this would be to have lint inheritance work contextually; so you can define a test profile but only inherit from it in a test context.

```toml
[lints]
inherits = { profile = "testprofile", context = "test" }

[lints.profiles.testprofile.clippy]
indexing-slicing = "allow"
unwrap-used = "allow"
expect-used = "test"
```

(The name "testprofile" is used here so it's unambiguous when "test" is used as a keyword)


Open question: Does configuring the default profile happen on `[lints]`, `[lints.profiles.default]`, or both?


`context` can be `normal`, `all` (default), or `test`. Other contexts can be added if desired.

In this case the `testprofile` profile is automatically inherited from when constructing the lint set for `cargo test`, `cargo bench`, or test targets in `cargo test --all-targets`, but ignored otherwise.


This is more powerful but it complicates lint profile inheritance and lint profiles need to keep a list of their lint sets for test and non-test contexts. The benefit of this is that it allows full configurability for test profiles, for example, one can do this:

```toml
[lints]
inherits = { profile = "testprofile", context = "test" }

[lints.profiles.testprofile.clippy]
indexing-slicing = "allow"
unwrap-used = "allow"
expect-used = "allow"

[lints.profiles.ci]
inherits = { profile = "testprofile", context = "test" }
[lints.profiles.ci.rust]
deprecated = "warn"
```

Now the CI profile is also able to ignore certain lints in test mode.


This does open up the question of multiple inheritance: if we want to inherit both a normal and a test context profile, we may need some form of multiple inheritance. There are three ways to do this that fit well with this design:

 - `inherits` accepts an array: `inherits = ["default", {profile = "testprofile", context = "test"}]`
 - `inherits` instead takes a profile name: `inherits.default = true`, `inherits.testprofile = {context = "test"}`
 - We add a separate `inherits-test` key: `inherits-test = "testprofile"`

The former two allow multiple inheritance in general, though we could disallow that. The first and third fits well with the existing syntax of `inherits`, which is modeled after _regular_ profile inheritance. The second feels a bit cleaner.

Open question: should this only apply to inheritance, or if it should be possible to directly specify lints as being contextual, like so:

```toml

[lints.clippy]
indexing-slicing = [{profile = "normal", level = "warn", profile = "test", level = "allow"}]
```

It seems like the underlying data model will likely need the ability to store multiple contextual levels per lint, so it might be worth allowing the user to do that too.

Open question: For `#[test]` in `--lib` test targets, should these lint levels apply _only_ to the tests, or all of the code? The latter is the easy option, the former needs extensions to rustc. I don't really see a strong motivation for this, but I'm calling it out as something to think about.

 [clippy-action]: https://github.com/giraffate/clippy-action
 [clippy-sarif]: https://crates.io/crates/clippy-sarif
 
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

Open question: Maybe we want to merge it with profiles anyway? Or perhaps provide a way to set a profile's default lint profile? I haven't seen a strong motivation for this yet.


# Drawbacks

I think the main drawback here is that this is Cargo.toml-focused, so it requires people to buy in to doing lints via `[lints]` and not any other method if they wish to have the benefits. It also

# Rationale and alternatives

Overall I think there are a lot of reasons to have "modalities" for lints, some already in use, some attained by a patchwork of features, and some that people would likely use if they were more convenient. Having profiles directly addresses them by giving the user a way to define and name a modality, so they or their CI/IDE/tooling/whatever can pick the right one based on the context.


One major alternative is allowing `[lints]` to exist in `[profiles]` (having them inherit the default profile if so). This could work, but it doesn't allow some of the things this RFC does, like disabling lints in test mode, or inheriting with a warn -> deny transform.

[RFC 3730: Add a semantically non-blocking lint level][RFC 3730] attempts to address the problem of wanting lints to be blocking in some contexts and non blocking in others, essentially by extending `-Dwarnings` to be a bit more flexible, and adding an IDE-useful "nit" lint level that only applies to new code. I think this attempts to solve a bunch of the same problems as this, but it solves them more narrowly: it doesn't help non-IDE users as much, and doesn't solve things like the test mode problem.

Both of these solutions are likely to be simpler, though.

# Prior art

The [custom named Cargo profiles](https://rust-lang.github.io/rfcs/2678-named-custom-cargo-profiles.html) RFC is probably the main prior art here. I think this feature works well, including profile inheritance.



# Unresolved questions

I've marked most unresolved questions as "open question" in the text of this RFC. I'd rather not duplicate them here, but I can do so if people like.

Picking a way to handle tests is one major question.

There's also a bunch of tricky design around how inheritance works around the "default" profile.


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