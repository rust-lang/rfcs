- Feature Name: `nit-lint-level`
- Start Date: 2024-11-15
- RFC PR: [rust-lang/rfcs#3730](https://github.com/rust-lang/rfcs/pull/3730)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)
- [Internals](https://internals.rust-lang.org/t/forbid-deny-warn-allow-and-notice/19986)

# Summary
[summary]: #summary

Add a new visible lint level below `warn` to allow linters to act like a pair-programmer / code-reviewer where feedback is evaluated on a case-by-case basis.
- `cargo` does not display these lints by default, requiring an opt-in
- This RFC assumes LSPs/IDEs will opt-in
- CIs could opt-in and open issues in the code review to show nits introduced in the current PR without merging being blocked

There is no expectation that crates will be `nit`-free.

*Note:* The name `nit` is being used for illustrative purposes and is assumed to not be the final choice.

*Note:* This RFC leaves the determination of which lints will be `nit` by
default to the respective teams. Any lints referenced in this document are
for illustrating the intent of this feature and how teams could reason about
this new lint level.

# Motivation
[motivation]: #motivation

By its name and literal usage, the `warn` level is non-blocking.
However, most projects treat `warn` as a soft-error.
It doesn't block for local development but CI blocks it from being merged.
This is an attempt to balance final correctness with rapid prototyping.
Requiring "warnings clean" code also avoids warnings fatigue where warnings
make it hard to see "relevant" compiler output, and make a codebase feel
lower-quality in a way that does not inspire people or invite people to
help solve the problem.
This convention is not new with the Rust community; many C++ projects have take
this approach before Rust came to be with "warnings clean" being a goal for
correctness.
Cargo is looking to further cement this meaning by adding
[`CARGO_BUILD_WARNINGS=deny`](https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#warnings)
for CIs to set rather than `RUSTFLAGS=-Dwarnings`.

This leaves a gap in the developer user experience for truly non-blocking lints.
- If a maintainer `allow`s a lint
  - They will never see it to benefit from it
- If contributors are expected to `#[allow]` some lints where appropriate
  - This requires judgement and/or communication on the bar for when to do this
  - The weight of `warn` may encourage contributors to do what it says, rather than `#[allow]` it
  - Sprinkling `#[allow]` in code can be noisy and the lint may not be worth it
- If a maintainer controls which lints don't block CI via `RUSTFLAGS=-A<lint>`
  - This makes it more difficult for users to reproduce this locally (which are blocking or not?).
  - `RUSTFLAGS` also comes with its own set of problems and the Cargo team is interested in finding alternatives to maintainers setting `RUSTFLAGS`,
    see [cargo#12738](https://github.com/rust-lang/cargo/issues/12738), [cargo#12739](https://github.com/rust-lang/cargo/issues/12739)

Another problem is when adopting lints.
This experience is more taken from legacy C++ code bases but the assumption is
that this can become a problem in our future as the Rust code bases grow over
time.
A complaint that can happen when adopting lints or a whole linter is the time it takes to clean up the code base to be lint free
because their only choice is to block on a lint or completely ignoring it.
If we had a non-blocking lint level, a project could switch interested lints to
that level and at least limit the introduction of new lint violations,
either viewing that as good enough or while the existing violations are resolved in parallel.

A secondary benefit of non-blocking lints is that more lints could move out of `allow` by default,
raising their visibility.
Currently, its left to each maintainer to sift through all of the
[default-`allow`ed lints](https://rust-lang.github.io/rust-clippy/master/index.html?levels=allow)
for which they may want to turn into a soft or hard error.

Another secondary benefit is that this could provide a smoother path for linters
to migrate `allow`s to `warn`s by having a period of time in a
non-blocking lint level so people can benefit immediately while having more
flexibility on when they pay the cost for turning the lint into a soft-error.

One area worth exploring for how lints are structured that we will not be
addressing is when they become `allow`ed under alternative circumstances.
These range from
[`avoid-breaking-exported-api`](https://doc.rust-lang.org/clippy/lint_configuration.html#avoid-breaking-exported-api)
which would be of interest to know about when developing new APIs
to
[`allow-dbg-in-tests`](https://doc.rust-lang.org/clippy/lint_configuration.html#allow-dbg-in-tests)
which the developer may not care to have called out.
As default lint levels are being left to the teams,
the handling of these cases are also being left to the relevant teams.
This is being included for completeness of different workflows of lints and in
case it inspires an alternative design that can better encompass cases like
this.

A linting workflow that this RFC intentionally does not try to support is for soft-errors that block on release,
in addition to soft-errors that block CI.
Historically, the software industry has had separate development and hardening phases.
The more recent trend has focused on always being in a releasable state through
the aid of code review and automated testing in CI.
While there are still times for more explicit hardening phases (gated features, human acceptance testing),
we don't see enough benefit for deferrings lints to a hardening phase to outweigh the costs of
smoothing out this workflow being misapplied.

## Example: `clap`

Each lint below from `clap`s `Cargo.toml` represents a lint that could be useful but not worthwhile enough to `allow`:
```toml
[workspace.lints.clippy]
bool_assert_comparison = "allow"
branches_sharing_code = "allow"
collapsible_else_if = "allow"
if_same_then_else = "allow"
let_and_return = "allow"  # sometimes good to name what you are returning
multiple_bound_locations = "allow"
assigning_clones = "allow"
blocks_in_conditions = "allow"
```

Also, clap's CI runs `clippy` with `-D warnings -A deprecated`
- Dependency upgrades should not be blocked on an API becoming deprecated
- The maintainers want visibility into what is deprecated though

Part of this stems from `deprecated` having two purposes:
- This API is broken and you should severely question any use of it
- This API will go away with the next major version you should migrate to the new API at some point in time

The second case is very useful for compiler-guided upgrades through breaking changes
(e.g. [winnow's migration guide](https://github.com/winnow-rs/winnow/blob/v0.6.0/CHANGELOG.md)).

Allowing dependency upgrades in the presence of deprecations is not just something that `clap` is interested in for itself but
users of clap reported that they want to handle deprecations on their own schedule ([clap#3822](https://github.com/clap-rs/clap/issues/3822)).
This led to adding a `deprecated` feature to `clap` so deprecations became opt-in
([clap#3830](https://github.com/clap-rs/clap/pull/3830)).
However, this can be easy for users to miss and won't catch new uses of the deprecated APIs.

## Example: `cargo`

In 2021 ([cargo#9356](https://github.com/rust-lang/cargo/pull/9356)), the Cargo team switched their `src/lib.rs` to:
```rust
#![allow(clippy::all)]
#![warn(clippy::needless_borrow)]
#![warn(clippy::redundant_clone)]
```
due to false positives and the level of subjectivity of the changes suggested by clippy by default.
This approach was first relaxed in 2024 ([cargo#11722](https://github.com/rust-lang/cargo/pull/11722)).

Over time and with the addition of the `[lints]` table, this eventually led to ([cargo#12178](https://github.com/rust-lang/cargo/pull/12178)):
```toml
[workspace.lints.clippy]
all = { level = "allow", priority = -1 }
dbg_macro = "warn"
disallowed_methods = "warn"
print_stderr = "warn"
print_stdout = "warn"
self_named_module_files = "warn"
```

In a recent PR, a bug that clippy would have found without our level overrides (`derive_ord_xor_partial_ord`)
was only caught in review because one of the reviewers has seen clippy report
this lint many times in other projects.
After some discussion, the `Cargo.toml` was updated to ([cargo#14796](https://github.com/rust-lang/cargo/pull/14796)):
```toml
[workspace.lints.clippy]
all = { level = "allow", priority = -2 }
correctness = { level = "warn", priority = -1 }
dbg_macro = "warn"
disallowed_methods = "warn"
print_stderr = "warn"
print_stdout = "warn"
self_named_module_files = "warn"
```

If more clippy lints were non-blocking, maybe the Cargo team would not have set
`all` to `allow` and been able to catch this more easily.
Granted, a non-blocking lint level could still lead to low quality "lint
reduction" PRs being posted and avoiding those was one of the original
motivations for the initial change.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## Example workflow

*This will be reported from perspective of a Cargo user.*

Rust users will have a new `nit` lint level for providing optional coding feedback.

If introduced today without any changes to levels for existing lints,
a maintainer may decide that they want `clippy::let_and_return` to be optional.
In their `Cargo.toml`, they would set:
```toml
[package.lints.clippy]
let_and_return = "nit"
```

A contributor may write:
```rust
fn foo() -> String {
    if condition() {
        let semantically_meaningful_name_one = bar();
        semantically_meaningful_name_one
    } else {
        let semantically_meaningful_name_two = baz();
        semantically_meaningful_name_two
    }
}
```

Rust-analyzer would give them feedback that they wrote extraneous code that could be reduced down,
with a suggested fix.
However, they feel the variable name is acting as a form of documentation and want to keep it.

When the contributor checks for lints, they run
```console
$ cargo clippy
   Compiling foo
    Finished `foo` profile [optimized + debuginfo] target(s) in 15.85s
```
No lints reported.

A non-rust-analyzer user could run the following to look for feedback:
```console
$ CARGO_BUILD_NITS=nit cargo clippy
   Compiling foo
note: creating a let-binding and then immediately returning it like `let x = expr; x` at the end of a block
...
    Finished `foo` profile [optimized + debuginfo] target(s) in 15.85s
```

They then go and post a PR.
On the PR, the code above gets the following issue opened:
```
X Check failure

Code scanning clippy

creating a let-binding and then immediately returning it like `let x = expr; x` at the end of a block (warning)

Show more details
```
This gives the contributor an opportunity to know of this potential improvement
if they missed it and for the code reviewer to double check if they agreed with
the author's determination to ignore it.

For future PRs, Github should not report this as it should recognize that the report is for existing code.

## Integrating with CI

The above potential experience is dependent on being able to integrate linting with CI to report only "new lints" or "lints for changed code".

For Github users, you can use [`clippy-sarif`](https://crates.io/crates/clippy-sarif) to report lints as SARIF alerts.
Github [tracks these alerts across runs](https://docs.github.com/en/code-security/code-scanning/integrating-with-code-scanning/sarif-support-for-code-scanning#providing-data-to-track-code-scanning-alerts-across-runs).

These can also be implemented manually with any setup.
For example, in a large, corporate, legacy C++ code base that [epage](https://github.com/epage/)
worked with, a linter used a similar database to track what lints were "new" and reported those back to
[ReviewBoard](https://www.reviewboard.org/).

## Choosing lint levels

When creating a lint or overriding a default lint level,
its important to consider the semantics, something like:

`error` is for hard errors that even prevent reporting of errors further in the build, running of tests, etc.
This should be reserved for when there is no point building or testing anything further because of how broken the code is.

`warn` generally has the semantics of a soft-error.
Its non-blocking locally but will be blocked when the code is merged upstream.
This should be used when eventual correctness or consistency is desired.

`nit` is for coding suggestions for users that may or may not improve the code or to reduce introduction of more nits on the path to making the lint a `warn`

`allow` is for when there is no right answer (e.g. mutually exclusive lints), there are false positives, the lint is overly noisy, or the lint is of limited value.

*Note:* This is for communicating the intent of this RFC and to get people started with this new feature and is not meant to precisely redefine the
[lint levels](https://rustc-dev-guide.rust-lang.org/diagnostics.html#diagnostic-levels).

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Rustc

Rustc focuses on the mechanics of a new, visible lint level `nit`.

Add support for the new lint:
```rust
#![nit(deprecated)]
```
```console
$ rustc --nit deprecated ...
```
```console
$ rustc -Ndeprecated ...
```

These will be rendered like other lint levels with a diagnostic level and coloring that conveys the non-blocking, helpful nature.
For example, this could be rendered like a `note:` or `help:` diagnostic level.

Rustc will have a dynamic lint group of all `nit`s, much like `warnings` is all lints at level `warn` at the time of processing, `-Anits`.

## Clippy

Like Rustc, Clippy would focus on the mechanism.

Add support for the new lint:
```rust
#![nit(deprecated)]
```
```console
$ rustc --nit clippy::pedantic ...
```
```console
$ rustc -Nclippy::pedantic ...
```

These will be rendered like Rustc.

These will be part of Rustc's dynamic lint group, much like `warnings`.

## Cargo

Cargo implements the semantics for this to be a first-class non-blocking lint level.

Add support for the new lint:
```toml
[lints.rust]
let_and_return = "nit"
```

Linting commands will hide these lints by default.
There won't be a summary line that nits were present.

`cargo fix` will not fix these lints by default.

A new config field will be added to mirror `build.warnings`:

### `build.nits`

- Type: string
- Default: `allow`
- Environment: `CARGO_BUILD_NITS`

Controls how Cargo handles nits. Allowed values are:

- `nit`: warnings are emitted as warnings.
- `allow`: warnings are hidden (default).

# Drawbacks
[drawbacks]: #drawbacks

`CARGO_BUILD_NITS=nit` is verbose and less obvious for people wanting to check these locally.
This is also true for `CARGO_BUILD_WARNINGS`.
CLI control could help but we deferred that for `CARGO_BUILD_WARNINGS` and the reasoning would also apply to `CARGO_BUILD_NITS`.
See [this comment](https://github.com/rust-lang/cargo/issues/14802) for some of the concerns.

Users running `CARGO_BUILD_NITS=nit cargo clippy` will have a hard time finding what lints are relevant for their change
among the sea of all of the nits.
If we had a way to filter lints by part of the API or by "whats new since run X",
then that could be resolved.

As `nit`s will always be enabled in the linter, just silenced in Cargo, the performance hit of running
them will be present unless `RUSTFLAGS=-Anits` is applied by the user.

## Hidden from users

By being hidden by default, there will be limits in how someone can see these:
- Rust-analyzer might be limited in what it can do through LSP to avoid overwhelming the user with `nit`s.
- This requires a non-standard CI setup to report these messages.
- Non-rust-analyzer users won't get feedback until CI runs and then will have to know to run their linter in a special way to reproduce CI's output locally.

That is similar to `allow` today but users except users can show these lints via `CARGO_BUILD_NITS=nit`.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This solution builds on the following past work:
- The move to statically, consolidated lint configuration via `[lints]`
  - In particular, `[lints]` design is optimized for setting only `level`.
  - This RFC works to centralize more lint configuration in a static way
- Experiments with `CARGO_BUILD_WARNINGS=deny` which further cements the relationship of lint levels with the semantic levels discusse

Benefits of the proposed solution:
- Users or tooling that request to see `nits` through Cargo can do so without
  recompilation because Rusts always repors `nits`, Cargo records this and
  replays it on cache hits, and the choice to display is made after this.
- LSPs, Sarif, etc being able to report soft-errors and nits differently to the user

For some specifics
- The default level for lints is left unchanged by this RFC to keep the scope of what is designed and reviewed to the minimum
- `CARGO_BUILD_NITS=allow` is the default to avoid "warning fatigue"
  - Users should not have to feel the need to resolve these for the sake of satisfying the tool
  - Therefore, adding support for `CARGO_BUILD_NITS=deny` in the future would run counter to the goals and intents of this RFC.  The user might as well switch the `nits` to `warn`.
- Rustc shows `nit`s by default, rather than hide them and require a separate opt-in mechanism for Cargo to see them
  - Problem: People directly using rustc or using a third-party build tool may be inundated with these
  - Lint levels may change without breaking compatibility
  - If stabilize with no `nit`s or only downgrading `warn`s to `nit`s, then there will be no difference in the amount of output
  - We already have a mechanism, let's not invent a new one
- `nits` lint group is created to give rustc-only or third-party build tools a built-in way to control these besides ignoring them like Cargo
- Cargo could print a message that `nit`s are present to help raise awareness of how to show them but
  - they will likely always be present, so this is noise to experienced users.
  - this could nerd snipe users into resolving them, especially if we say how many there are
- Cargo could print all nits if there are no other messages but
  - they will likely always be present, so we'll flood the users screen on regular builds, likely nerd sniping people into fixing them

## `clippy::pedantic`

In purpose, this has a lot of overlap with
[`clippy::pedantic`](https://rust-lang.github.io/rust-clippy/master/index.html?groups=pedantic#/print_stderr)
in that these are "too noisy" of lints.
A mechanism could be devised for hiding/showing this group within Cargo.

However,
- This is clippy specific. The conversation would instead shift to getting an agreed-to convention for this group.
- The group is linter-defined and users can't override it, lints into or out of `pedantic` and `allow` or `warn`.

# Prior art
[prior-art]: #prior-art

IntelliJ IDEA has the following [lint levels](https://www.jetbrains.com/help/idea/configuring-inspection-severities.html):
- Error: Syntax errors
- Warning: Code fragments that might produce bugs or require enhancement
- Weak Warning: Code fragments that can be improved or optimized (redundant code, duplicated code fragments, and so on)
- Server Problem
- Grammar Error
- Typo
- Consideration: Code fragments that can be improved. This severity level is not marked on the error stripe and does not have a default highlighting style, but you can select one from the list of existing styles or configure your own.
- No highlighting (but fix available)

LSP has the following [diagnostic levels](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#diagnostic):
- Error
- Warning
- Information
- Hint

with the following diagnostic tags:
- Unnecessary
- Deprecated

Github/Sarif support the following [severities](https://docs.github.com/en/code-security/code-scanning/managing-code-scanning-alerts/about-code-scanning-alerts#about-alert-severity-and-security-severity-levels)
- Error
- Warning
- Note

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- `CARGO_BUILD_NITS=nit` is modeled off of `CARGO_BUILD_WARNINGS=warn` which is just saying "treat X level violations as Y level"
  - `CARGO_BUILD_WARNINGS=warn` is "weird" but shouldn't show up too often as its the default
  - However, the frequency is swapped with `CARGO_BUILD_NITS=nit`
  - We may also want the terms to be consistent across the different level-control fields
  - Reminder: `CARGO_BUILD_WARNINGS` is unstable and we can still change it
- Should the config be under `[build]`?
  - This is also one of the unresolved issues for `CARGO_BUILD_WARNINGS`, see [cargo#14802](https://github.com/rust-lang/cargo/issues/14802)
- How should this work in rust-analyzer?
  - For Information or Hint, the rendering is left to the client
  - VSCode shows them like warnings/errors but different color scheme while filtering them out from the problems view.

## Naming

The name should clearly communicate that there is no authoriative weight
encouraging them to resolve them but that it is left to them and their reviewer
to weigh whether to resolve or ignore them.

The verb form is used for:
- attribute
- long flag

Lint names are expected to be read with their level for what should be done, e.g.
- `deny(let_and_return)` -> "deny the use of let-and-return in this code"
- `warn(let_and_return)` -> "warn on the use of let-and-return in this code"

The noun form is used for:
- Dynamic lint group

Options
- `#[nit]` / `--nit` / `-N` / `-Wnits`
  - Unsure how to word `nit(let_and_return)` as a sentence like the other lint levels
- `#[nitpick]` / `--nitpick` / `-N` / `-Wnitpicks`
  - "nitpick the let-and-return in this code"
- `#[note]` / `--note` / `-N` / `-Wnotes`
  - "note the let-and-return in this code"
- `#[notice]` / `--notice` / `-N` / `-Wnotices`
  - "notice the let-and-return in this code"
- `#[mention]` / `--mention` / `-M` / `-Wmentions`
  - "mention the let-and-return in this code"
- `#[consider]` / `--consider` / ~~`-C` (taken)~~ / `-Wconsiderations` (IntelliJ)
  - "consider the let-and-return in this code"
  - Verb and noun have a larger divergence
- `#[inform]` / `--inform` / `-I` / `-Winformation` (LSP)
  - "inform you of the let-and-return in this code"
  - Verb and noun have a larger divergence
- `#[hint]` / `--hint` / `-H` / `-Whints` (LSP)
  - "hint of a let-and-return in this code"
- `#[remark]` / `--remark` / `-R` (supposedly LLVM) / `-Wremarks`
  - "remark on the let-and-return in this code"
- ~~`#[suggest]` / `--suggest` / `-S` / `-Wsuggestions`~~
  - When read with the lint name, it sounds like its to find where you *should* do it rather than finding where you are doing it
  - Verb and noun have a larger divergence

# Future possibilities
[future-possibilities]: #future-possibilities

## First-class SARIF support

Like wanting [JUnit support for libtest](https://github.com/rust-lang/rust/issues/85563),
having SARIF support in first-party Rust linters (rustc, rustdoc, rust-clippy, or their Cargo wrappers)
would help in integrating Rust with third-party systems without needing a third-party tool like
[`clippy-sarif`](https://crates.io/crates/clippy-sarif).
See also [rust-clippy#8121](https://github.com/rust-lang/rust-clippy/issues/8121).

In moving forward with this, we'd need to figure out
- What layer this should be supported in
- The semantics for output teeing (specifying the location, separate output format from teed format, etc)
- Versioning or flavors of these documents
