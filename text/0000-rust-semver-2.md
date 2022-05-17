- Feature Name: rust-semver-2
- Start Date: 2022-05-11

# Summary
[summary]: #summary

This RFC defines the Rust's SemVer 2 rules. It's define version requirement operator that can be used in Cargo to define the version of dependencies that Cargo can choose. The rules use [SemVer 2].

Empirical change from Rust SemVer 1:

* Remove ranges `>=`, `>`, `<=`, `<`
* Remove logical "and" `,`
* Add logical "or" `||`
* `~` can now update MAJOR
* `^` will never consider a pre-release compatible with any other pre-release or release
* `~0` or `^0.0` are not any more valid version requirement
* Remove sugar `1.*` or `1.0.*`

# Motivation
[motivation]: #motivation

The current rules of version requirement in Cargo are unclear, contains exceptions and `^` behavior with pre-release is unexpected. We want:

  * to trust `^` to do the right thing, since it's the default behavior of Cargo
  * to prevent user to make mistake
  * simple rules
  * crates that apply these rules

Cargo never officially state most of current behavior of version requirement resolution. [SemVer 2] have been used as reference to define it with some addition in Cargo doc, It's unclear what rules follow Cargo cause there have been no formal decision to clearly decide what Cargo should do. [Cargo Specifying Dependencies] define compatibly rules of `^` for release, there are clear and logic for a release but never mention pre-release existence, [rules for pre-release] are well hidden and doesn't fully describe the current observed behavior of `^1.0.0-alpha`, the current behavior create a lot of problems when a user put a pre-release version like `1.0.0-alpha` in their `Cargo.toml`.

This lead to [RFC 3263 motivation]. The main proposed solution was to change the default of Cargo that consider `1.0.0-alpha` as `^1.0.0-alpha` to `=1.0.0-alpha`. But while this work to solve a specific problem, this introduces an exception to Cargo behavior for pre-release. SemVer 2.0 said "pre-release version indicates that the version is unstable and might not satisfy the intended compatibility requirements as denoted by its associated normal version." this clearly indicate there is no compatibility obligation between pre-release and release. Despite that the current behavior of `^` do this assumption and consider higher pre-release version and release compatible ! This mean currently `^1.0.0-alpha` match `1.0.0-beta` or even `1.0.0` (up to `1.*.*`) this behavior come from NPM rules.

There is a trap when using range operator, precedence in SemVer 2.0 say that `1.0.0 < 1.1.0 < 1.1.1 < 2.0.0-alpha < 2.0.0`. In theory this mean that range would include pre-release. Let's say user want something either version `1` or `2` their would write `>=1, <3`, but this could be interpreted as include pre-release between `1` and `3` so include `2.0.0-alpha` and worse `3.0.0-alpha` even if user know this trap and try using `>=1, <3-0` it could still match `2.0.0-alpha` or `2.9.9-alpha`! We need a solution to this problem. It's unclear what Cargo do, for example, doc say that `>=1.2.3, <2.0.0` match "Any SemVer-compatible version of at least the given value.", this according to SemVer INCLUDE pre-release of `2.0.0` since cargo doc didn't precise anything about release or pre-release behavior. Currently, the behavior of Cargo are more or less a copy of what NPM do. NPM behavior is complex it's allow pre-release on certain condition notably when the range has a pre-release too: `>=1.0.0-alpha && <2` would match `1.0.0-alpha`, but this does not look consistent with something like `>=1.0.0-alpha && <1` that would not match `1.0.0-alpha`. Cargo never talk about pre-release and range. Instead of having complex rules to avoid this problem, we should have rules that can be instantly be clear to anybody if possible.

Some maintainers are just not using pre-release feature at all because it's currently annoying in Rust. They just prefer to avoid them entirely. Sometimes a duplicate of crate is publish like a standalone crates [`clap-v3`]. Some maintainer use pre-release but are not happy about it. [Clap] 3 pre-release experience reveal they needed to carefully deal with default `^` operator behavior, by changing every dep to `=` for pre-release and again for the final release changing every `=` operator back to `^`. [Rocket] fall into this trap and a new pre-release break a previous pre-release because of the `^` current behavior and there is no good solution to fix the breaking. The only thing to do is to avoid it next time by using `=` operator in requirement version of their pre-release internal dependencies. It's annoying to be afraid of using pre-release feature of SemVer because there are very useful when `MAJOR >= 1` in Rust. This make maintainers of Rust crate that want to introduce a preview version a more complicated job. User will be afraid to use pre-release version if trap like this make their project break, this mean less user will test pre-release. Maintainer do not like to have to deal with this issue. We need rules that make pre-release more usable in practice without the trap of range.

Pre-release tag are allowed to be very flexible, almost too much. SemVer 2.0 implicitly say that pre-release MAY be compatible with associate stable version but this mean we must not expect it. This mean that behavior actual of `^` to take the higher version with the same MAJOR is broken on pre-release in Rust, this operator is expected to not allow breaking change by Rust user. This is why we should restrict this behavior, and have a rule to define compatible version between pre-release. The problem is that actually there is no rule for pre-release tag. We should have a rule that are both logical and used by most. A [list of most used pre-release tag] in dependencies requirement version of all available crate in [`crates.io`] including yanked crate. We can see the top 3 are `alpha.1`, `alpha.2`, `alpha`. Most people use `alpha`, `beta`, etc... or `rc.1`, `rc.2`, etc... or `rc1`, `rc2`, etc... convention. Rust ecosystem seem for the most part using a logical way to define compatible pre-release with the first identifier. On the contrary some crate use very weird pre-release tag [`air-interpreter-wasm`] have more than 800 version and most of them are pre-release tag than doesn't follow any compatible logic.

Rust ecosystem have always followed SemVer. When a version break SemVer rules it can be yanked so Rust ecosystem is pretty healthy about compatibility versioning. This is show by the almost absence of range operator use because maintainer simply trust SemVer compatible rules with `^` behavior, there is 1.50% of dependence requirement version on [`crates.io`] that are using range operator, this includes every version of every crate available on [`crates.io`] excluding yanked version. Rust being a strongly typed language there is way less occasion to be able to use two different major versions of a crate. This mean Rust ecosystem use case of range is very limited. We can reasonably think most of the use of range operator in Rust could be replaced by simple Component requirement and caret operator.

Some stats of [`crates.io`], excluding yank version:

* 3_835_442 dependence requirements in total
* 57_412 (1.50%) dependence requirements that use range operator.
* 68_167 (1.78%) dependence requirements that use tilde operator.
* 22_737 (0.59 %) dependence requirements that use multiple requirement.

This list of [unique multiple requirement] show that most use of multiple requirement either, could be replaced by `~` or `^`, look unclear because they allow multiple breaking change version it is on purpose or not ?

Non-exhaustive list of case of misuse of range operator in Rust crate:

  * [`alice`](https://crates.io/crates/alice/0.1.0-alpha.1/dependencies): `clap = ">= 2.33, <2.34"` 
  * [`ascon-aead`](https://crates.io/crates/ascon-aead/0.1.2/dependencies): almost all dependencies use range while it's should use `^` operator the very next version 0.1.3 removed all these ranges and replace them by `^`. This show ranges operator are not only a trap for pre-release but also for release, they are easily badly used. There are 9874 requirement versions than include a single range without bound like this.
  * [`slog-envlogger`](https://crates.io/crates/slog-envlogger/2.0.0-1.0/dependencies): Use range to opt in for pre-release the next `2.0.0-3.0` version of this crate switched to use `~` that was doing the equivalent but is simpler.

List of "good" range use case used in Rust ecosystem:
  
  * [`webbrowser`](https://crates.io/crates/webbrowser/0.7.1/dependencies): while `>=0.3, <=0.6` is okish it's unclear what user want, why exclude `0.6.1` of `0.6` ? `ndk-glue` have a `0.6.2`, it's unclear if this is on purpose or not.

There is currently crate on `crates.io` version and requirement version that break syntax of SemVer 2:

 * [tma 0.1.0](https://crates.io/crates/tma/0.1.0/dependencies) dependency `^0-.11.0` is not a valid pre-release tag
 * [bluetooth_client 0.0.1-001](https://crates.io/crates/bluetooth_client/0.0.1-001) `001` is not a valid pre-release tag
 * [hxgm30-client 0.3.0-alpha.01](https://crates.io/crates/hxgm30-client/0.3.0-alpha.01) `alpha.01` is not a valid pre-release tag
 * [lmdb-rkv-sys 0.9.4](https://crates.io/crates/lmdb-rkv-sys/0.9.4/dependencies) dependency `^0.51-oldsyn` is not a valid version
 * [raft 0.5.0](https://crates.io/crates/raft/0.5.0/dependencies) dependency `~2.0-2.0` is not a valid requirement version
 * [solstice-2d 0.1.2](https://crates.io/crates/solstice-2d/0.1.2/dependencies) dependency `^0.1-alpha.0` is not a valid version
 * [volatile 0.4.0-alpha.00](https://crates.io/crates/volatile/0.4.0-alpha.00) and [volatile 0.4.0-alpha.01](https://crates.io/crates/volatile/0.4.0-alpha.01) `alpha.00` and `alpha.01` are not a valid pre-release tag.

The final question is, What do we need/want ? What operator Rust community want for SemVer ? We should not take rules that doesn't fit Rust user expectation. We must choose rules that fit Rust need. Do we really need a range operator in Rust ? What features Rust user need in version requirement ?

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

### Rust SemVer 2
[rules]: #rules

Rust SemVer 2 rules are defined on the top of [SemVer 2] plus the following rules:

12. When MAJOR is zero, MINOR is considered as MAJOR, PATCH is considered as MINOR, there is no number considered as PATCH, `0.MAJOR.MINOR`. When MAJOR and MINOR are both zero, PATCH is considered as MAJOR, there is no number considered as MINOR or PATCH, `0.0.MAJOR`.

13. A Requirement Version defines when a VERSION is "matched", unless specified requirement version MUST be the combination between an OPERATOR terminated by a REQVERSION, for example `^1.0.0` is a requirement version that have OPERATOR `^` and REQVERSION `1.0.0`.

14. A REQVERSION numbers text representation can omit field, MAJOR number can't be omitted, if a pre-release or a build tag is present REQVERSION MUST NOT omit any numbers.

15. A pre-release version can only be compatible with itself, so when MAJOR, MINOR, PATCH and PRERELEASE are equals.

16. The OPERATOR "exact", with `=` REQVERSION match only itself, so when MAJOR, MINOR, PATCH and PRERELEASE are equals.

17. The OPERATOR "caret", with `^` REQVERSION match the highest COMPATIBLE VERSION of REQVERSION, `^` operator is the default operator when a version requirement don't have operator.

18. The OPERATOR "tilde", with `~` REQVERSION operator match the highest VERSION up to the precision of REQVERSION. This operator MAY match INCOMPATIBLE version.

19. The LOGICAL OPERATOR "or". `||` operator is a union between two requirement versions. `||` MUST be preceded by a requirement version and terminated by another requirement version. `||` matches if any of the two requirement version match. `||` can be chained. It's RECOMMENDED to write text requirement version representation from the smaller REQVERSION on the left to the higher REQVERSION on the right ordering with precedence rules.

#### OPTIONAL

20. A Wildcard `*` MAY be used as REQVERSION, it's a sugar for `~0.0.0 || ~0.0 || ~1`.

#### Examples

The following tree show what `^` consider as compatible version, when a version is indented by space this mean it's compatible with the parent version.

```none
0.0.0-0
0.0.0-alpha
0.0.1
0.0.2
0.1.0
  0.1.1
  0.1.2
0.2.0-beta
0.2.0
0.3.0
1.0.0
  1.0.1
  1.1.0
    1.1.1
    1.1.2
  1.2.0
    1.2.1
2.0.0-alpha.0
2.0.0-alpha.1
2.0.0-beta.0.0
2.0.0-beta.1.0
2.0.0-beta.1.1
2.0.0
```

The following example of requirement versions are INVALID:

```none
0
0.0
2-alpha
2.0-beta
~0
~0.0
^0
^0.0
=*
^*
~*
* || ~1.0.0-beta.0
```

The following show some `~` and `||` usage:

* `~1` is equivalent to `~x.y.z` with `x >= 1`, `y >= 0` and `z >= 0`
* `~1.1` is equivalent to `~1.y.z` with `y >= 1` and `z >= 0`
* `~1.0.9` is equivalent to `~1.0.z` with `z >= 9`
* `~1.0.0-alpha` is equivalent to `~1.0.0-IDENTIFIERS` with `IDENTIFIERS >= alpha`
* `~1.0.0-alpha.0` is equivalent to `~1.0.0-alpha.IDENTIFIERS` with `IDENTIFIERS >= 0` like `1.0.0-alpha.1` or `1.0.0-alpha.the.turbofish.remains.undefeated`.
* `~0.1` is equivalent to `~0.y.z` with `y >= 1` and `z >= 0`
* `~0.0.9` is equivalent to `~1.0.z` with `z >= 9`
* `~0.0.0-0` is equivalent to `~0.0.0-0.PREMINOR` with `PREMINOR >= 0` so any pre-release of `0.0.0`
* `^1.0.0 || ^2.0.0` match all release of either `1` or `2`
* `~1.7.0 || ~1.8.0 || ~1.9.0` match all releases between `1.7` and `1.10` excluded. 
* `~1.2.0 || ^1.3.0` should be written `^1.2.0`
* `~1.2.0 || ^1.4.0` is valid but SHOULD not be needed if a crate respect SemVer.
* `1.0.0 || || 2.0.0`, `||1.0.0`, `1.0.0||` `||^1.0.0`, `^1.0.0||` are not a valid
* `~1.0.0-0 || ^1` match any pre-release or release of `1`. This should be used with care.
* `~0.0.0 || ~0.0 || ~1` match any release. It's call the wildcard.

#### ABNF
[rust-semver-2-abnf]: #rust-semver-2-abnf

The following is the [ABNF] rules of Rust SemVer 2

```abnf
; Rust SemVer 2
reqversion = "*" / logical-or

logical-or = req-core *("||" req-core)

req-core = *SP [operator *SP] (version / numbers-partial) *SP

operator = "=" / "^" / "~"

; 0.0 and 0 are not accepted
numbers-partial = numbers / (num-ident dot num-ident-non-zero) / num-ident-non-zero

; SemVer 2
version = numbers ["-" pre-release] ["+" build]

numbers = num-ident dot num-ident dot num-ident

pre-release = pre-ident *(dot pre-ident)
pre-ident = alphanum-ident / num-ident

build = build-ident *(dot build-ident)
build-ident = 1*(ALPHA / DIGIT / ident-join)

; need at least one alpha or ident-join
alphanum-ident = *DIGIT (ALPHA / ident-join) *(ALPHA / DIGIT / ident-join)

; leading zero are not accepted like 01 or 001
num-ident = num-ident-non-zero / "0"
num-ident-non-zero = digit-non-zero *DIGIT
digit-non-zero = "1" / "2" / "3" / "4" / "5" / "6" / "7" / "8" / "9"

ident-join = "-"
dot = "."
```

We thank authors of [`bap`] and [`abnfgen`] to have provided free tool to test this ABNF. [`abnfgen`] can be used to generate tests for parser that want to implement Rust SemVer 2.

### Cargo
[cargo]: #cargo

`Cargo.toml` has 2 new fields:

* `package.rust-semver`, define what Rust SemVer version Cargo should use for this `Cargo.toml`, the default is `1` for 2021 edition of Rust.

* `dependencies.foo.allow-advanced-operator`, the default is `warn` for `rust-semver = "1"` and `deny` for `rust-semver = "2"`:

  * `warn` will emit a warning if a requirement version use `~` or `||` operator.
  * `allow` will accept tilde `~` or `||` operator.
  * `deny` will emit an error if a requirement version use `~` or `||` operator.

`package.version` will now default to `0.0.0`.

Cargo should emit a warning when `^` should be used instead of `~`. It is the case when only the MAJOR and MINOR are specified in `~` like `~1.2` == `^1.2` or `~0.2.5` == `^0.2.5`.

### Pre-release Guideline
[pre-release-guideline]: #pre-release-guideline

A pre-release tag MAY follow such convention:

* Alpha pre-release are considered very unstable that is similar to `0.0.z`. Alpha version should not have any compatibility expectation. It's recommended for an Alpha pre-release to set version like `1.0.0-alpha.PREMAJOR` where `PREMAJOR` is a numeric identifier incremented at each Alpha pre-release. `1.0.0-alpha.0` and `1.0.0-alpha.1` don't have any compatibility expectation.
* Beta pre-release are considered unstable that are similar to `0.y.z` when `y > 0`. Beta version may have compatibility expectation. It's recommended for a Beta pre-release to set version like `1.0.0-beta.PREMAJOR.PREMINOR` where `PREMAJOR` is a numeric identifier incremented at each breaking change and `PREMINOR` is a numeric identifier incremented at each non-breaking change. `1.0.0-beta.0.1` MAY be compatible with `1.0.0-beta.0.0` or `1.0.0-beta.2.0` MAY be compatible with `1.0.0-beta.2.4`
* If a pre-release is considerate to be the last pre-release before a release we call them Candidate Release and if a crate desire hint their users about Candidate Release, it's recommended to use build tag like `1.0.0-beta.4.0+rc`. There is no compatible expectation between a Candidate Release and the final release. They may be any number of Candidate Release.

The following example use spaces to show version that MAY be compatible, using `~` to opt in:

```none
1.0.0-alpha.0
1.0.0-alpha.1
1.0.0-alpha.2
1.0.0-alpha.3
1.0.0-alpha.4
1.0.0-beta.0.0 => ~1.0.0-beta.0.0
  1.0.0-beta.0.1
  1.0.0-beta.0.2
1.0.0-beta.1.0+rc => ~1.0.0-beta.1.0
  1.0.0-beta.1.1
1.0.0-beta.2.0  => ~1.0.0-beta.2.0
  1.0.0-beta.2.1
  1.0.0-beta.2.2
  1.0.0-beta.2.3+rc
  1.0.0-beta.2.4+rc
1.0.0
```

As you can see `1.0.0-beta.1.0+rc` was a release candidate, but we change our mind at `1.0.0-beta.1.1`. If a crate use this pre-release guideline user MAY use `~` operator to receive Beta upgrade, for example `~1.0.0-beta.0.0` would match `1.0.0-beta.0.0`, `1.0.0-beta.0.1`, `1.0.0-beta.0.2` pre-release. A crate should state its pre-release policy for example at end of a `readme.md` file, it's perfectly allowed to not follow this guideline about pre-release policy. A user should not make any assumption of pre-release policy of a crate if the crate doesn't specify it.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Cargo should stick to use the same [`semver`] crate version to not change its behavior for crates that doesn't specify `rust-semver = 2`. We call Rust SemVer 1 the actual behavior of Cargo (there is no formal definition yet). [`semver`] crate version should match the evolution of Rust SemVer version. So have a `2` version that implement Rust SemVer 2 rules.

Ideally, when using `rust-semver = 2` Cargo would detect `<`, `<=`, `>=`, `>` and `,` usage to offer a clear error message about their removal in Rust SemVer 2 and explain how to replace then with the new operator.

`crates.io` or any alternative registry SHOULD disallow using any `*` in requirement version.

# Drawbacks
[drawbacks]: #drawbacks

These changes make Cargo registry live with mixed rules, it's currently the case some crates are not valid. Cargo would need to differentiate crates that use the new rules. This drawback can be reduced by not removing any existent operator and not remove `1.0.*` sugar.

For a user the drawbacks are:

  * Change rules from Rust SemVer 1
  * Replace features from Rust SemVer 1
  * It doesn't have the same rules as NPM (Actually already true)

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Rationale
[rationale]: #rationale

The philosophy of Rust SemVer 2 is to use the opposite mindset of Rust SemVer 1, where `&&` is about [intersection], `||` is about [union]. This is the main difference between the two designs, this RFC try to argue that union is simpler and more flexible than intersection for user. Rust SemVer 2 try to be KISS, quoting a famous French personality:

> Perfection is achieved, not when there is nothing more to add, but when there is nothing left to take away. [Antoine de Saint-Exupéry]

Operator range make our life complicated in Rust, remove them for Rust SemVer 2 mean:

* No more trap with pre-release, `^` have a simple behavior that follow compatible versioning rules with clear rules that allow to not be afraid that a pre-release is included implicitly. Or on the contrary that a final version is included implicitly. `^` alone represent 97% of dependence operator of Rust.

* It's remove the ambiguity of range including pre-release or not.

* No more possible unclear use of range operator like:

  * `>=1` this represents 17.2% of range use of Rust ecosystem, mean that a majority of people probably make a mistake here. Replaceable with `~1` now (use of tilde will at least emit a warning by default).
  * `>=1, <2.0` instead of `^1`.
  * `>=1.2, <1.3` instead of `~1.2.0`

* That we need to change tilde behavior. Without compatibly rule for pre-release and without range operator, they are no way to opt-in for a range of pre-release identifiers. Change rule of tilde also remove the exception to the tilde operator for MAJOR number. This make `~` more general and more clear, it's allow a user to opt in to potentially breaking change versions of a crate. `allow-advanced-operator` make Cargo to warn a user that it's dangerous to use this operator and link to documentation to help a user to not use tilde operator if this can be avoided. Only advanced user should use it, so there is an opt-in option. It's also nice cause the behavior of tilde slightly change, so we can also warn the user of the change in the same time. It is worth mentioning that `~` behavior is not documented for pre-release, so it's unclear what `~` actually do for pre-release.

* Include `||` operator will cover the case where user want to support several non-compatible version. The only drawback is this could be very explicit if a crate release a hundred of major version; but currently there have never been such case, on the contrary, most crate in Rust try to not break without good reason. There is very low chance that users would ever need more than 2 major releases. Previously a user would have done `>= 0.6, <0.9` now a user can just write `0.6 || 0.7 || 0.8` it's will naturally avoid non-compatible version such as pre-release. It's even better because we can now make a jump that was impossible before we can now do `0.6 || 0.8`. Of course, this operator need to be used with care. It's a very specific use case where two major release are considered compatible by the user. Instead of writing:

  * `>=1.0.0, <2.0.0-0` prefer `1`
  * `>=1.4, <2.0` prefer `1.4`
  * `>=1.0, <3.0` prefer `1 || 2`
  * `>=1.5, <1.6` prefer `~1.5.0`
  * `>=0.7, <0.9` prefer `~0.7.0 || ~0.8.0`
  * `>=1.0.0-0, <2.0.0-0` prefer `~1.0.0-0 || 1` (A user probably never want that)
  * `>=1.0, <2.3` prefer `1 || ~2.0.0 || ~2.1.0 || ~2.2.0` (A user probably never want that)

* Without range operator the "and" operator (`&&` or `,`) is not needed. All operators only allow to go to higher or equal version. This mean we don't need to have `(` and `)` to handle prevalence of logical operator. `||` is the only logical operator similar to before where that was `&&` the only available logical operator. Also, `,` is quite unclear for new user of Rust, we should probably have used `&&` before.

Rust can remove range operator cause Rust's tool force to respect SemVer, so we generally never need them. Any small breaking change is often detected instantly in Rust, user will implicitly get the incompatible version and Rust being a strongly typed language user will directly spot the breaking change. Maintainers will likely just yank the release and the problem will be gone. But NPM needs to deal with the incredible flexibility of JavaScript. JavaScript try as hard as possible to run no matter what. This mean that even if on paper two releases are incompatible in practice a user can hope it will "work". Also, the speed of JavaScript release is also higher, there is a lot of user, a lot of movement, more major release, more quickly. The tool for these two languages are likely to need different approach. It's not rare for a JavaScript project to want to handle few major releases while in Rust it's very rare. So rare that it's hard to find example of it. The way the two languages use SemVer is very different. NPM need range feature and so try to make them usable despite the pre-release nightmare. This RFC try to argue that we don't need to range feature in Rust, and so we can avoid the complicated rule needed to protect user from range trap.

It's hard to choice how we consider a pre-release compatible with another pre-release. There is a lot of convention existing and there may not be respected since Rust ecosystem has already many pre-release versions. Rust SemVer 2 choices to let this choice to advanced user. Have guideline about RECOMMENDED behavior will allow to have a standard convention about it, but a user will need to explicitly ask Cargo to opt in for pre-release convention using `~` operator. It's recommended that user verify twice that a crate clearly state its pre-release compatible convention. In case of doubt, a user should not use advanced operator.

Removing the sugar of `1.0.*` is because `~` behavior is now exactly this. We don't need to introduce exception when `~` do the job and better since `*` can't be release in pre-release identifiers. Specially `1.*.0` case is considered not valid make the rule annoying to implement, while `~` can't represent `1.*.0` so no need of specially handle of `~` in the code.

`*` is just a Q&D feature, it's a very bad practice to not at least choice a major version for a dependence, a registry should not accept `*` as doc of Cargo say "Note: [`crates.io`] does not allow bare `*` versions.". The sugar `*` is acceptable since the tilde equivalent is `~0.0.0 || ~0.0 || ~1`, the complexity clearly represent why it's wrong to use `*` in production.

## Alternatives
[alternatives]: #alternatives

* We could only change the `^` rules about match all pre-release and release of a version. This let open other problem of current Rust SemVer 1. User can implicitly make mistake, trying to prevent user to make a mistake follow Rust philosophies, Cargo should reflect that.

* We could follow the exact same rule than NPM or similar other tools that manage requirement version. We could think "NPM do it, why shouldn't we too ?". Because NPM (or other tools) users have problems and needs very different from Cargo users. NPM handle of SemVer 2 are not necessary good solution for other ecosystem.

# Prior art
[prior-art]: #prior-art

There is a formal RFC in preparation in [SemVer#584]. This proposition try to reach a consensus on range operator. Range operator have a lot of problems when interacting with pre-release. 

# Unresolved questions
[unresolved-questions]: #unresolved-questions

* Should `||` operator be written `or`. `^1 or ^2`, it's mostly equivalent, matter of taste, but `||` have the advantage to not have any character allowed in SemVer ABNF.
* Should we keep range operator as optional opt-in feature ? If yes we would probably need `&&`, `(` and `)`.
* Do we need a way to say any release or any pre-release `*-*` ?
* Do we keep `1.*` and `1.0.*` sugar ?
* What is the percentage of requirement version use range or tilde usages in NPM ecosystem ? This to compare with Rust one.
* The BNF of SemVer 2, allow `1.0.0-------------` or even `1.0.0------------+----------`, should we limit this behavior ? `1.0.0-alpha-beta` could be miss leading, SemVer should probably have used `_` instead or just not allowed `-` in identifier. `crates.io` should probably refuse such version and Rust SemVer 2 could force to have alpha between `-`. Do we allow `_` like `-` in identifiers ?
* Should we define limit to identifier and number of version ?
* Should the ABNF, use recursively for `||` rule instead of just being a list ? It's not needed for now.
* Do we restrict space number in ABNF ? Disallow space in most place ? Use WSP that include tabulation ?
* Rule 15 is redundant with SemVer 2 but clarify the situation, should we remove it ?

# Future possibilities
[future-possibilities]: #future-possibilities

Since, we introduce `||` operator, it would be easy to add `&&` operator later, even `(`, `)` if needed. With this RFC as base we could make upgrade to Rust SemVer more easily. Most of this RFC could be reused to official define Rust SemVer 1, an example with some bonus so 1.1.

21. The LOGICAL OPERATOR "and". `&&` (or `,`) operator is an intersection between two requirement versions. `&&` MUST be preceded by a requirement version and terminated by another requirement version. `&&` matches if both of the two requirement version match. `&&` can be chained. It's RECOMMENDED to write text requirement version representation from the smaller REQVERSION on the left to the higher REQVERSION on the right ordering with precedence rules.

22. `&&` have higher priority than `||`, you can use parenthesis `(` and `)` to encapsulate a requirement version.

23. The OPERATOR "greater", with `>` REQVERSION operator match any greater VERSION of REQVERSION. This operator MAY match INCOMPATIBLE version.

24. The OPERATOR "greater or equal", with `>=` REQVERSION operator match any greater or equal VERSION of REQVERSION. This operator MAY match INCOMPATIBLE version.

25. The OPERATOR "greater release", with `>>` REQVERSION operator match any greater release VERSION of REQVERSION. This operator MAY match INCOMPATIBLE version.

26. The OPERATOR "greater or equal release", with `>>=` REQVERSION operator match any smaller or equal release VERSION of REQVERSION. This operator MAY match INCOMPATIBLE version.

27. The OPERATOR "smaller", with `<` REQVERSION operator match any smaller VERSION of REQVERSION. This operator MAY match INCOMPATIBLE version.

28. The OPERATOR "smaller or equal", with `<=` REQVERSION operator match any smaller or equal VERSION of REQVERSION. This operator MAY match INCOMPATIBLE version.

29. The OPERATOR "smaller release", with `<<` REQVERSION operator match any smaller release VERSION of REQVERSION. This operator MAY match INCOMPATIBLE version.

30. The OPERATOR "smaller or equal release", with `<<=` REQVERSION operator match any smaller or equal release VERSION of REQVERSION. This operator MAY match INCOMPATIBLE version.

31. A REQVERSION numbers text representation last field can be a wildcard `*`, it would be equivalent to any value, if a pre-release or a build tag is present REQVERSION MUST NOT omit any numbers.

```abnf
; Rust SemVer 1.1
reqversion = wildcards / req-core

wildcards = "*" / (num-ident dot "*") / (num-ident dot num-ident dot "*")

req-core = logical-or / logical-and / parenthesis / req-axiom

logical-or = req-axiom "||" req-core
logical-and = req-axiom ("&&" / ",") req-core
parenthesis = *SP "(" *SP req-core *SP ")" *SP

req-axiom = *SP [operator] *SP (version / numbers-partial) *SP

operator = "=" / "^" / "~" / ">" / ">>" / ">=" / ">>=" / "<" / "<<" / "<=" / "<<="

; 0.0 and 0 are not accepted
numbers-partial = numbers / (num-ident dot num-ident-non-zero) / num-ident-non-zero

; SemVer 2
version = numbers ["-" pre-release] ["+" build]

numbers = num-ident dot num-ident dot num-ident

pre-release = pre-ident *(dot pre-ident)
pre-ident = alphanum-ident / num-ident

build = build-ident *(dot build-ident)
build-ident = 1*(ALPHA / DIGIT / ident-join)

; need at least one alpha or ident-join
alphanum-ident = *DIGIT (ALPHA /ident-join) *(ALPHA / DIGIT / ident-join)

; leading zero are not accepted like 01 or 001
num-ident = num-ident-non-zero / "0"
num-ident-non-zero = digit-non-zero *DIGIT
digit-non-zero = "1" / "2" / "3" / "4" / "5" / "6" / "7" / "8" / "9"

ident-join = "-"
dot = "."
```

Thus, it's add a lot of rules and make the grammar more complex, need a form of recursively.

[SemVer 2]: https://semver.org/spec/v2.0.0.html

[Cargo Specifying Dependencies]: https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#specifying-dependencies
[rules for pre-release]: https://doc.rust-lang.org/cargo/reference/resolver.html#pre-releases

[RFC 3263 motivation]: https://github.com/rust-lang/rfcs/blob/8a020f872763f83834b3a03070e417257cebc8a1/text/3263-precise-pre-release-deps.md#motivation
[SemVer#584]: https://github.com/semver/semver/pull/584

[`crates.io`]: https://crates.io

[Clap]: https://crates.io/crates/clap
[`clap-v3`]: https://crates.io/crates/clap-v3/versions
[Rocket]: https://crates.io/crates/rocket
[`air-interpreter-wasm`]: https://crates.io/crates/air-interpreter-wasm/versions
[`semver`]: https://crates.io/crates/semver

[list of most used pre-release tag]: https://gist.github.com/Stargateur/b7feeeb6b22cfbe2afee5744a4a30326#file-unique-name-list-of-pre-release-tag-dependencies
[unique multiple requirement]: https://gist.github.com/Stargateur/b7feeeb6b22cfbe2afee5744a4a30326#file-multiple-requirements

[`bap`]: https://tools.ietf.org/tools/bap/
[`abnfgen`]: http://www.quut.com/abnfgen/

[Antoine de Saint-Exupéry]: https://en.wikipedia.org/wiki/Antoine_de_Saint-Exup%C3%A9ry
[KISS]: https://en.wikipedia.org/wiki/KISS_principle
[ABNF]: https://datatracker.ietf.org/doc/html/rfc5234
[intersection]: https://en.wikipedia.org/wiki/Intersection_(set_theory)
[union]: https://en.wikipedia.org/wiki/Union_(set_theory)