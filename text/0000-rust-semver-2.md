**This RFC is not really mean for RFCS repository of rust yet, it's mean as a real RFC as request for comment. This RFC try to open the debate about current rules of version requirement. It doesn't focus on if it's possible or not to apply it to Cargo yet. This RFC is far from being stable**

- RFC Version: 2.0.0-alpha.0
- Feature Name: rust-semver-2
- Start Date: 2022-05-11

# Summary
[summary]: #summary

This RFC defines the Rust's SemVer 2 rules. It's define version requirement operator that can be used in Cargo to define the version of dependencies that Cargo can choose. The rules use [SemVer 2].

# Motivation
[motivation]: #motivation

Cargo never officially state most of current behavior of version requirement resolution. [SemVer 2] have been used as reference to define it with some addition in Cargo doc It's unclear what rules follow Cargo cause there have been no formal decision to clearly decide what Cargo should do. [Cargo Specifying Dependencies] define compatibly rules of `^` for release, there are clear and logic for a release but never mention pre-release existence, [rules for pre-release] are well hidden and doesn't fully describe the current observed behavior of `^1.0.0-alpha`, the current behavior create a lot of problems when a user put a pre-release version like `1.0.0-alpha` in their `Cargo.toml`.

This lead to [RFC 3263 motivation]. The main proposed solution was to change the default of Cargo that consider `1.0.0-alpha` as `^1.0.0-alpha` to `=1.0.0-alpha`. But while this work to solve a specific problem, this introduces an exception to Cargo behavior for pre-release and this actually reveal the real problem that we never decided compatibility rule for pre-release. SemVer 2.0 said "pre-release version indicates that the version is unstable and might not satisfy the intended compatibility requirements as denoted by its associated normal version." this clearly indicate there is no compatibility obligation between pre-release and final version. Despite that the current behavior of `^` do this assumption and consider higher pre-release version and final version compatible ! This mean currently `^1.0.0-alpha` match `1.0.0-beta` or even `1.0.0` (up to `1.*.*`) this behavior come from NPM rules.

There is a trap when using range operator, precedence in SemVer 2.0 say that `1.0.0 < 1.1.0 < 1.1.1 < 2.0.0-alpha < 2.0.0`. In theory this mean that range would include pre-release. Let's say user want something either version `1` or `2` their would write `>=1 && <3`, but this could be interpreted as include pre-release between `1` and `3` so include `2.0.0-alpha` and worse `3.0.0-alpha` even if user know this trap and try using `>=1 && <3-0` it would still match `2.0.0-alpha` or `2.9.9-alpha`! We need a solution to this problem. Since Cargo didn't define clearly the behavior of pre-release behavior. It's unclear what Cargo do for example, doc say that `>=1.2.3, <2.0.0` match "Any SemVer-compatible version of at least the given value.", this according to SemVer INCLUDE pre-release of `2.0.0`. Currently, the behavior of Cargo are more or less a copy of what NPM do. NPM behavior is complex it's allow pre-release on certain condition notably when the range has a pre-release too: `>=1.0.0-alpha && <2` would match `1.0.0-alpha`, but this does not look consistent with something like `>=1.0.0-alpha && <1` that would not match `1.0.0-alpha`. Cargo never talk about pre-release and range. Instead of having complex rules to avoid this problem, we should have rules that can be instantly be clear to anybody if possible.

Some maintainers are just not using pre-release feature at all because it's currently annoying in Rust. They just prefer to avoid them entirely. Sometimes a duplicate of crate is publish like a standalone crates [`clap-v3`]. Some maintainer use pre-release but are not happy about it. [Clap] 3 pre-release experience reveal they needed to carefully deal with default `^` operator behavior, by changing every dep to `=` for pre-release and again for the final release changing every `=` operator back to `^`. [Rocket] fall into this trap and a new pre-release break a previous pre-release because of the `^` current behavior and there is no good solution to fix the breaking. The only thing to do is to avoid it next time by using `=` operator in requirement version of their pre-release internal dependencies. It's annoying to be afraid of using pre-release feature of SemVer because there are very useful when `MAJOR > 0` in Rust. This make maintainers of Rust crate that want to introduce a preview version a more complicated job. User will be afraid to use pre-release version if trap like this make their project break, this mean less user will test pre-release. Maintainer do not like to have to deal with this issue. We need rules that make pre-release more usable in practice without the trap of range.

Rust ecosystem have always followed SemVer. When a version break SemVer rules it can be yanked so Rust ecosystem is pretty healthy about compatibility versioning. This is show by the almost absence of range operator use because maintainer simply trust SemVer compatible rules with `^` behavior, there is 1.49% of dependence requirement version on [`crates.io`] that are using range operator, this includes every version of every crate available on [`crates.io`] even the yanked. Rust being a strongly typed language there is way less occasion to be able to use two different major versions of a crate. This mean Rust ecosystem use case of range is very limited. We can reasonably think most of the use of range operator in Rust could be replaced by simple Component requirement and caret operator. Non-exhaustive list of case of misuse of range operator in Rust crate:

  * [`alice`](https://crates.io/crates/alice/0.1.0-alpha.1/dependencies): `clap = ">= 2.33, <2.34"` 
  * [`ascon-aead`](https://crates.io/crates/ascon-aead/0.1.2/dependencies): almost all dependencies use range while it's should use `^` operator the very next version 0.1.3 removed all these ranges and replace them by `^`. This show ranges operator are not only a trap for pre-release but also for release, they are easily badly used. There are 9874 requirement versions than include a single range without bound like this.
  * [`slog-envlogger`](https://crates.io/crates/slog-envlogger/2.0.0-1.0/dependencies): Use range to opt in for pre-release the next `2.0.0-3.0` version of this crate switched to use `~` that was doing the equivalent but is simpler.

List of "good" range use case used in Rust ecosystem:
  
  * [`webbrowser`](https://crates.io/crates/webbrowser/0.7.1/dependencies): while `>=0.3, <=0.6` is okish it's unclear what user want, why exclude `0.6.1` of `0.6` ? `ndk-glue` have a `0.6.2`, it's unclear if this is on purpose or not.

There is currently crate on `crates.io` version and requirement version that break syntax of SemVer:

 * https://crates.io/crates/tma/0.1.0/dependencies dep `^0-.11.0` is not a valid pre-release tag
 * https://crates.io/crates/bluetooth_client/0.0.1-001 `001` is not a valid pre-release tag
 * https://crates.io/crates/hxgm30-client/0.3.0-alpha.01 `alpha.01` is not a valid pre-release tag
 * https://crates.io/crates/lmdb-rkv-sys/0.9.4/dependencies dep `^0.51-oldsyn` is not a valid version
 * https://crates.io/crates/raft/0.5.0/dependencies dep `~2.0-2.0` is not a valid requirement version
 * https://crates.io/crates/solstice-2d/0.1.2/dependencies dep `^0.1-alpha.0` is not a valid version
 * https://crates.io/crates/volatile/0.4.0-alpha.00 and https://crates.io/crates/volatile/0.4.0-alpha.01 `alpha.00` and `alpha.01` are not a valid pre-release tag.

Pre-release tag are allowed to be very flexible, almost too much. SemVer 2.0 implicitly say that pre-release MAY be compatible with associate stable version but this mean we must not expect it. This mean that behavior actual of `^` to take the higher version with the same MAJOR is broken on pre-release in Rust, this operator is expected to not allow breaking change by Rust user. This is why we should restrict this behavior, and have a rule to define compatible version between pre-release. The problem is that actually there is no rule for pre-release tag. We should have a rule that are both logical and used by most. A [list of most used pre-release tag] in dependencies requirement version of all available crate in [`crates.io`] including yanked crate. We can see the top 3 are `alpha.1`, `alpha.2`, `alpha`. Most people use `alpha`, `beta`, etc... or `rc.1`, `rc.2`, etc... or `rc1`, `rc2`, etc... convention. Rust ecosystem seem for the most part using a logical way to define compatible pre-release with the first identifier. On the contrary some crate use very weird pre-release tag [`air-interpreter-wasm`] have more than 800 version and most of them are pre-release tag than doesn't follow any compatible logic.

Finally, the real question is, what do we need ? What do we want ? What operator Rust community want for SemVer ? We should not take previous rules that doesn't fit Rust user expectation. We must choose rules that fit Rust need. Do we really need a range operator in Rust ?

What features Rust user need in version requirement ? This RFC is bias toward this:

  * We need to be able to trust Cargo default behavior, user want thing that work naturally
  * We want to trust `^` and `~` to do the right thing
  * We need to have rules that SHOULD avoid cargo update break our build (Cargo update or a fresh lock file like in workflow of CI/CD of `gitlab.com` or `github.com` action)
  * We need to avoid implicitly include pre-release version
  * We need a way to use a pre-release version without fear of unexpected breaking change with cargo update, we want stability above all even for pre-release like how we expect `0.5.x` version to not break our build.
  * We need simple Rules
  * We need crates that apply these rules

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

`package.rust_semver = "2"` use Rust's SemVer, these rules are defined on the base of [SemVer 2] plus the following rules:

  12. When the MAJOR is 0, MINOR number is considered as the MAJOR number and the PATCH number is considered as the MINOR number, there is no number considered as PATCH:

      * `0.5.5` is compatible with `0.5.2`.
      * `0.5.5` is not compatible with `0.6.0`.

  13. A VERSION text representation can omit field, when omitted field will default to `0`, MAJOR can't be omitted, if a pre-release is present version need to be complete:

      * `0` is invalid. /* this is due to 0.MAJOR.MINOR exception */
      * `0.5` mean `0.5.0`
      * `1` mean `1.0.0`
      * `1.1` mean `1.1.0`
      * `2-alpha` or `2.0-beta` is not valid

  14. A "Pre-release" version can only be compatible with another pre-release version. The first identifier of a pre-release tag is call PREMAJOR for two pre-release versions to be compatible MAJOR, MINOR, PATCH and PREMAJOR must be equal. In `alpha.0`, `alpha` is the PREMAJOR. The rest of following identifiers is only used to determine order between same PREMAJOR `alpha` < `alpha.0.1` < `alpha.1.0` < `alpha.alpha` as define in rule 11.4. A pre-release compatible version MUST not be breaking change:

      * `1.0.0-alpha.1` is compatible with `1.0.0-alpha.0`
      * `1.0.0-alpha` is compatible with `1.0.0-alpha.0`
      * `1.0.0-alpha.0` is not compatible with `1.0.0-alpha`
      * `1.0.0-beta.0` is not compatible with `1.0.0-alpha.0`
      * `1.0.0-alpha0` is not compatible with `1.0.0-alpha.0`
      * `1.0.0` is not compatible with `1.0.0-alpha.0`

  15. A "Requirement Version" defines when version is "matched", unless specified requirement version MUST be the combination between an OPERATOR terminated by a REQVERSION, for example `^1.0.0` is a requirement version that have OPERATOR `^` and REQVERSION `1.0.0`, this requirement version will define how Cargo will choose the best suitable version of a crate to use.

  16. The OPERATOR "exact", `=` operator match if `VERSION == REQVERSION` that mean when `MAJOR == REQMAJOR` and `MINOR == REQMINOR` and `PATCH == REQPATCH` and `PRERELEASE == REQPRERELEASE`:

      * `1.0.0` match `=1.0.0`
      * `1` match `=1.0.0`
      * `1.0.1` doesn't match `=1.0.0`.
      * `1.0.0-alpha` match `1.0.0-alpha`
      * `1.0.0-alpha.0` doesn't match `1.0.0-alpha`

  17. The OPERATOR "caret", `^` operator match the highest compatible version with the VERSION associate with the OPERATOR, `^` operator is the default operator when a version requirement don't have operator in Cargo.

      * `1.2.3` match `^1.0.0`
      * `1.0.0-alpha.1` match `^1.0.0-alpha.0`
      * `1.0.0-alpha.0` match `^1.0.0-alpha`
      * `1.0.0-alpha` doesn't match `^1.0.0-alpha.0`
      * `2.0.0` doesn't match `^1.2.3`
      * `0.5.0` doesn't match `^0.4.0`
      * `1.0.0` doesn't match `^1.0.0-alpha.0`
      * `1.0.0-alpha0` doesn't match `^1.0.0-alpha`
      * `1.0.0-beta` doesn't match `^1.0.0-alpha`

  18. The OPERATOR "tilde", `~` operator match the highest compatible version up to the precision of the associate VERSION.

      * `~1` is equivalent to `~1.y.z` with `y >= 0` and `z >= 0`
      * `~1.1` is equivalent to `~1.y.z` with `y >= 1` and `z >= 0`
      * `~1.0.9` is equivalent to `~1.0.z` with `z >= 9`
      * `~1.0.0-alpha` is equivalent to `~1.0.0-alpha.PREMINOR` with `PREMINOR` being any pre-release tag like `1.0.0-the.turbofish.remains.undefeated` or just empty
      * `~1.0.0-1.2.3` is equivalent to `~1.0.0-1.2.PREPATCH` with `PREPATCH >= 3` like `1.0.0-1.2.4`.
      * `~0` is equivalent to `~0.0.z` with `z >= 0` /* this is due to 0.MAJOR.MINOR exception */
      * `~0.1` is equivalent to `~0.1.z` with `z >= 0` /* this is due to 0.MAJOR.MINOR exception */
      * `~0.0.2` is equivalent to `~0.0.z` with `z >= 2`
      * `~0.0.0-0` is equivalent to `~0.0.0-0.PREMINOR` with `PREMINOR` being any pre-release tag.
      * `~1.2` is equivalent to `~1.y.z` with `y >= 2` and `z >= 0`

      It's RECOMMENDED to use the `^` operator when `~` or `^` would have the same matching behavior. It is the case when only the MAJOR is specified in `~`, `~1` is equivalent to `^1.0.0`, `^1.0` or `^1`. The same apply for pre-release when only PREMAJOR is specified, `~1.2.3-alpha` is equivalent to `^1.2.3-alpha`.

  19. The OPERATOR "or". `||` operator requirement is the combination between two requirement versions. `||` MUST be preceded by a requirement version and terminated by another requirement version. `||` matches any of the two requirement version. `||` can be chained. It's RECOMMENDED to write requirement version from the smaller on the left to the higher on the right ordering with precedence rules.

      * `^1.0.0 || ^2.0.0` match all release of either `1` or `2`
      * `~1.7.0 || ~1.8.0 || ~1.9.0` match all release between `1.7` and `1.9` included. 
      * `~1.2.0 || ^1.3.0` should be written `^1.2.0`
      * `~1.2.0 || ^1.4.0` is valid but SHOULD not be needed if a crate respect SemVer.
      * `1.0.0 || || 2.0.0`, `||1.0.0`, `1.0.0||` `||^1.0.0`, `^1.0.0||` are not a valid syntax

  20. A Wildcard `*` is a special requirement operator, it's not associate with any VERSION, this operator match ANY the release. `1.0.*` is not a valid syntax, `~` operator SHOULD be used instead `~1.0.0`.

      * `*` match `0.4`
      * `*` match `1`
      * `*` doesn't match `2.0.0-alpha`
      * `=*`, `^*` and `~*` are not valid

  21. A pre-release wildcard can be written as `*`, this mean you can write `1.0.0-*` this will match anything pre-release tag this EXCLUDING empty release tag:

      * `1.0.0-*` match `1.0.0-alpha`.
      * `1.0.0-*` match `1.0.0-alpha-0`.
      * `1.0.0-*` match `1.0.0-beta`
      * `1.0.0-*` doesn't match `1.0.0`
      * `1.0.0-*` doesn't match `1.5.5-alpha`.
      * `*-*` match any pre-release /* do we allow this ? */
      * `* || *-*` match anything
      * `1.0.0-* || ^1.0.0` match any pre-release of `1.0.0` or any compatible version of `1.0.0`
      * `1.0.0-alpha.*` is not valid
      * `^1.0.0-*` is not valid
      * `~1.0.0-*` is not valid

`Cargo.toml` have a new option in package field `package.dep_prerelease` by default it's `warn`. `warn` Cargo will emit a warning if a requirement dep include a pre-release. `deny` Cargo will emit an error. `allow` Cargo will accept pre-release.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Cargo should stick to use the same [SemVer crate] version to not change its behavior for crates that doesn't specify `rust_semver = 2`. We call Rust SemVer 1 the actual behavior of Cargo (there is no formal definition yet). [SemVer crate] version should match the evolution of Rust SemVer version. So have a `2` version that implement Rust SemVer 2 rules.

Ideally, when using `rust_semver = 2` Cargo would detect `<`, `<=`, `>=`, `>` and `,` usage to offer a clear error message about their removal in Rust SemVer 2.

`crates.io` or any alternate registry SHOULD disallow using any `*`.

# Drawbacks
[drawbacks]: #drawbacks

These change make Cargo registry live with mixed rules, it's currently the case some crates are not valid. Cargo would need to differentiate crates that use the new convention since we remove `,` operator. This drawback can be reduced by not removing any existent operator and not remove `1.0.*`. That can clearly be considered, but is still not the focus of this RFC.

For a user the drawbacks are:

  * Change rules
  * Replace features
  * It doesn't have the same rules as NPM

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

There are 57412 dependence requirements in [`crates.io`] that use range operator this represents 1.50% of the total. And there are 68167 dependence requirements that use tilde operator this represents 1.78% of the total. This excluding yank version this time.

Operator range make our life complicated in Rust, remove them for Rust SemVer 2 mean:

  * No more trap with pre-release, `^` and `~` have a simple behavior that follow compatible versioning rules with clear rules that allow to not be afraid that a pre-release is included implicitly. Or on the contrary that a final version is included implicitly.

  * No more wrong use of range operator like `version = ">=1"`

  * That we need to define compatible rule for pre-release. Without compatibly rule for pre-release and without range operator we could only use exact operator on pre-release. The additional rule that consider the first identifier as PREMAJOR allows ruling out the need of range to handle pre-release opt in. A user who wanted to opt in for alpha release of a crate can now do it with `^`, it will work as expect and the user should not have any breaking change. Previously a user would have done `>= 2.0.0-alpha, < 2.0.0-b` now a user can just write `2.0.0-alpha` it's will naturally work as expected if maintainer follow the Rust SemVer 2 rule. It's a trust contract between users and maintainers. All already existing pre-release will probably not all respect pre-release compatibility rules, but new one will very likely.

  * Include `||` operator will cover the case where user want to support several non-compatible version. The only drawback is this could be very explicit if a crate release a hundred of major version; but currently there have never been such case, on the contrary, most crate in Rust try to not break without good reason. There is very low chance that users would ever need more than 2 major releases. Previously a user would have done `>= 0.6, <0.9` now a user can just write `0.6 || 0.7 || 0.8` it's will naturally avoid non-compatible version such as pre-release. It's even better because we can now make a jump that was impossible before we can now do `0.6 || 0.8`. Of course, this operator need to be used with care. It's a very specific use case where two major release are considered compatible by the user.

    * Instead of writing `>1.0 && <3.0` write `1.0 || 2.0`
    * Instead of writing `>1.4 && <2.0` write `1.4`
    * Instead of writing `>1.5 && <1.6` write `~1.5`
    * Instead of writing `>1.7 && <1.9` write `~1.7 || ~1.8`
    * Instead of writing `>1.0.0-0 && <2.0.0-0` write `1.0.0-* || ^1` 
    * Instead of writing `>1.0 && <2.3` write `1.0 || ~2.0 || ~2.1 || ~2.2` (That would be incredibly rare in Rust)

  * Without range operator, the "and" operator (`&&` or `,`) is not needed. All operators only allow to go to higher or equal version. This mean we don't need to have `(` and `)` to handle prevalence of logical operator. `||` is the only logical operator similar to before where that was `&&` the only available logical operator. Also, `,` is quite unclear for new user of Rust, we should probably have used `&&` before.

Rust can remove range operator cause Rust's tool force to respect SemVer. Any small breaking change is often detected instantly in Rust, user will implicitly get the incompatible version and Rust being a strongly typed language user will directly spot the problem. Maintainers will likely just yank the release and the problem will be gone. But NPM needs to deal with the incredible flexibility of JavaScript. JavaScript try as hard as possible to run no matter what. This mean that even if on paper two releases are not compatible in practice a user can hope it will "work". Also, the speed of JavaScript release is also higher, there is a lot of user, a lot of movement, more major release, more quickly. The tool for these two languages are likely to need different approach. It's not rare for a JavaScript project to be able to handle few major releases while in Rust it's very rare. So rare that it's hard to find example of it. The way the two languages use SemVer is very different. NPM need range feature and so try to make them usable despite the pre-release nightmare. This RFC try to argue that we don't need to range feature in Rust, and so we can avoid the complicated rule needed to protect user from range trap.

We could want to keep the range operator as optional, the user would opt in (`package.allow_range_reqversion = true`) to be able to use range. 

To choice how we consider a compatible pre-release with another pre-release is not arbitrary. It's follow the ordering precedence define by SemVer. Take the first identifier as PREMAJOR seem like a natural choice that a lot of maintainers are doing. The problem is that there was no previous advice about this before and even when maintainers use this pattern isn't certain that make pre-release really compatible. That where the `dep_prerelease` come handy, it will allow Cargo to warn user about that and check if the pre-release they want to use follow this pattern, in case of doubt it could probably be better to use the `=` operator to avoid any surprise. This RFC by defining these rules will allow to have more and more case where a user can opt for a pre-release without expecting breaking when running Cargo update or just from a fresh cargo build. If a maintainer doesn't want to have pre-release compatible version or expect a lot of breaking change it's RECOMMENDED to use a numerical identifier for the PREMAJOR like `1.0.0-0`, `1.0.0-1`, `1.0.0-2-rc`.

`package.dep_prerelease` serve a clear purpose to explicitly know if a crate want to use pre-release. A maintainer can opt in this for its pre-release version than opt-out. Cargo will warn the maintainer of a mistake about having a pre-release dep.

`*` is just a Q&D feature, it would be a very bad practice to not at least choice a major version for a dependence when you make a release and as doc said "Note: [`crates.io`] does not allow bare * versions.". Removing the sugar of `1.*` serve two purposes, first `~` behavior is exactly this, secondly it doesn't follow SemVer ABNF. We don't need to introduce exception when `~` do the job. Specially `1.*.0` case is considered not valid make the rule annoying to implement. The only thing `~` can't do is what wildcard operator define in the rule, "match any release". So you can't express the notion of "any non-pre-release version" with `~` alone. Having `*` handle this special case isolate the feature and cost very little to the ABNF. It's allow to just add a rule `<valid rust semver> = <valid semver> | "*"` it doesn't change `valid semver` rule it's encapsulate it. We could also say that `~` alone do that, but this would contradict the compatible rule and make an expectation on an operator parsing. `~0` should not be currently accepted as it's make no sense.

An alternative to this RFC could be to follow the exact same rule than NPM or similar other tools that manage requirement version. We could think, NPM do it, why shouldn't we too ? Because NPM users have problems and needs very different from Cargo users. NPM handle of SemVer are not necessary good solution for other ecosystem.

# Prior art
[prior-art]: #prior-art

There is a formal RFC in preparation in [SemVer#584]. This proposition try to reach a consensus on range operator. Range operator have a lot of problems when interacting with pre-release. 

# Unresolved questions
[unresolved-questions]: #unresolved-questions

  * Should `||` operator be written `or`. `^1 or ^2`, it's mostly equivalent, matter of taste, but `||` have the advantage to not have any character allowed in SemVer ABNF.
  * Should we keep range operator as optional opt-in feature ? If yes we would probably need `&&`, `(` and `)`.
  * Should `*` take the highest of either pre-release or release version available instead of just pick release ?
  * Should we define recommended pre-release convention ? A simple recommendation could be, use `alpha` then `beta` and so on. An alternative recommendation could be use `rc0` then `rc1` and so on. We could also propose to merge them saying to use alpha convention until it's reasonable to think a release is for very soon and here you use release candidate convention, `alpha` then `beta` then `rc` then `x.0.0` release. The majority of the most used crates use these conventions. I think it would be a very idea to define guideline for pre-release convention. This would make these rule more easy to understand, examples are always more simple to understand.
  * Should we keep the syntax sugar `1.*` ? The problem of this sugar is that while `1.*` is valid `1.*.0` is not, also it's equivalent to tilde operator. It's look there are more cons than pros to this sugar.
  * What is the percentage of requirement version use range or tilde usages in NPM ecosystem ? This to compare with Rust one.
  * Should rule 13 allow `0` be `0.0.0` ? This make an exception to an exception.
  * Do we really need pre-release wildcard rule 21 ? This look complex to use correctly. This only exist for user that want "the last possible pre-release or release", should we advise use git feature of cargo for that ? Even rule 20 do we really need wildcard ?

# Future possibilities
[future-possibilities]: #future-possibilities

Since, we introduce `||` operator, it would be easy to add `&&` operator later, even `(`, `)` if needed. With this RFC as base we could make upgrade to Rust SemVer more easily.

[SemVer 2]: https://semver.org/spec/v2.0.0.html
[SemVer crate]: https://crates.io/crates/semver
[Cargo Specifying Dependencies]: https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#specifying-dependencies
[rules for pre-release]: https://doc.rust-lang.org/cargo/reference/resolver.html#pre-releases
[RFC 3263 motivation]: https://github.com/rust-lang/rfcs/blob/8a020f872763f83834b3a03070e417257cebc8a1/text/3263-precise-pre-release-deps.md#motivation
[SemVer#584]: https://github.com/semver/semver/pull/584
[Clap]: https://crates.io/crates/clap
[`clap-v3`]: https://crates.io/crates/clap-v3/versions
[Rocket]: https://crates.io/crates/rocket

[`crates.io`]: crates.io

[list of most used pre-release tag]: https://gist.github.com/Stargateur/b7feeeb6b22cfbe2afee5744a4a30326#file-unique-name-list-of-pre-release-tag-dependencies
[`air-interpreter-wasm`]: https://crates.io/crates/air-interpreter-wasm/versions