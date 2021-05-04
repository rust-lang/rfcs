- Feature Name: `preview_features`
- Start Date: 2021-04-29
- RFC PR: [rust-lang/rfcs#3120](https://github.com/rust-lang/rfcs/pull/3120)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)
- Cargo Implementation PR: [rust-lang/cargo#9401](https://github.com/rust-lang/cargo/pull/9401)

# Summary
[summary]: #summary

Making select unstable features available on stable Rust behind a
preview flag.

# Motivation
[motivation]: #motivation

Most Rust features start their lives as unstable, which restricts them
to only be available on the nightly release channel. This choice was
made early on to reinforce Rust's [commitment to stability as a
deliverable][stability]. Once a feature is available on nightly, the
hope is that it attracts developers who want the feature and are willing
to restrict their crate's to only compiling on nightly. Those developers
try out the feature, report problems, and thus improve the design,
implementation, and stability of the feature. Eventually, once (if) the
feature has seen sufficient evidence of its correctness, completeness,
and usefulness, it is stabilized, which makes the feature available on
the beta release channel, and eventually stable itself. This process
generally serves Rust well -- it ensures that only features whose design
and implementation have been battle-tested end up on stable.

The main factor that determines if, and how quickly, a feature is
stabilized, is the evidence-gathering process. For highly desired
features like const generics or generic associated types, there is no
shortage of users willing to try out the feature and provide experience
reports with using it. That, in turn, provides more feedback for honing
its design and implementation in the early phases, and provides more
evidence for maturity and usefulness in the later phases. That evidence
is ultimately what is needed to build the confidence needed to stabilize
a feature, so such features have a robust path to stabilization. Less
highly desired features get less attention, and may spend much longer
before the get stabilized if at all. This is, arguably, the process
working as designed -- features that users want get prioritized.

There is, however, a class of features that suffers under this process
-- those that impact a large number of _users_ without impacting a large
number of Rust _developers_. Consider a feature like Cargo's
[`-Zstrip`], which strips debug symbols from binaries and thus
significantly reduces their size. It's likely not a feature a large
number of individual Rust developers care about, but once it's made
available, developers who disseminate programs written in Rust can much
more easily distribute smaller binaries to all of their users, who may
in turn have a better experience from the faster downloads. Another
example is [`-Zpatch-in-config`], which allow non-Cargo build systems to
inject `[patch]` sections into Rust crates when they're built. Few
individual developers need this feature, but the developers of those
build systems may be able to use that feature to significantly improve
the experience of working with first-party Rust dependencies under their
build tool, which may in turn improve the user experience of large
number of Rust developers that in turn rely on that build system.

The reason this class of features is underserved by the current process
is two-fold. First, since few developers need them _directly_, the
perceived need for the feature is less than that of a high-profile
feature like const generics, even if the actual _impact_ of the feature
may be comparable. Second, and more importantly, users who are in the
position to make use of such high-impact changes are often hesitant to
abandon the stability guarantees of Rust's stable channel. While Rust's
nightly releases are _generally_ quite stable, and flags like
`-Zallow-features` exist to limit the unstableness of the nightly
compiler, at such large scale even the slight risk from switching to
nightly is often unacceptable. For this reason, the developers who could
make use of these high-impact features may be unable to actually use
those features until they're stable (see [rationale and
alternatives][rationale-and-alternatives] for alternatives). But since
stabilization requires evidence of impact and maturity, these features
can get caught in a Catch-22 situation where evidence won't be provided
until they're stabilized, and they can't be stabilized without evidence.

This RFC proposes a mechanism for improving this situation for a subset
of such features _without_ incurring the wide-spread breakage potential
and ecosystem fragility that Rust's stability guarantee is intended to
guard against. It exploits the fact that some of these underserved
high-impact features are only needed in contained contexts where any
breakage would be entirely localized, and in those cases it is
acceptable to allow selective opting out of the stability guarantees.

[stability]: https://blog.rust-lang.org/2014/10/30/Stability.html#why-not-allow-opting-in-to-instability-in-the-stable-release
[`-Zstrip`]: https://github.com/rust-lang/rust/issues/72110
[`-Zpatch-in-config`]: https://github.com/rust-lang/cargo/issues/9269

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Most unstable features are only available on the nightly release
channel. This is so that unstable features can continue to change as
their design and implementation evolve, or even be removed, without
breaking the backwards compatibility guarantees that Rust guarantees in
its stable releases. However, certain unstable features may be available
as _preview features_ on Rust's beta and stable release channels to
gather feedback on the feature from more users (such as those who cannot
use nightly). You can enable a preview feature using the
`--enable-preview=some-feature` option, and do not have to be on a
nightly version of the compiler to do so.

The idea behind preview features is to solicit feedback on features that 
can have a high impact even if enabled in just _one_ place, and
therefore will not generally cause widespread breakage if they are later
removed. Thus, a feature that a crate and all of its dependencies must
make use of to be impactful is unlikely to be made available as preview,
whereas a feature that is hugely impactful when used in a crate that
builds a binary might.

Features are only available in preview for a relatively short time
window (usually a bit longer than one release cycle); once the window
passes, a feature will either be stabilized or return to nightly-only.
During the preview period, the hope is to gather enough evidence from
real-world usage to support stabilization, so if you do find value in a
preview feature, please remember to contribute back a report of your
experience!

Since preview features may _not_ be stabilized after the preview period
ends, users of preview features should be prepared to stop using the
feature again in the future. This is another reason why only features
that are useful at the "end" of the compilation pipeline are likely to
be considered for preview.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Process

To make an unstable feature available under preview, an FCP should take
place for the responsible team. The customary waiting period is not
required for preview features. When evaluating whether to allow a
feature for preview, the team members should evaluate whether the
feature:

1. **Has wide, fan-out impact.** That is, if the feature is enabled in
   _one_ place (e.g., a single build system or a single crate) would
   meaningfully and positively impact a large number of developers.
   "Large" is subjective of course, but 100+ developers is a good place
   to start.
2. **Has a stable implementation.** The feature is unlikely to see
   significant changes before landing, especially in its interface. This
   could be because it is simple enough that this is trivial to
   determine, or it can be because the feature has gone through enough
   revisions that it's reasonable to conclude. This is important because
   if that's not the case, chances are it's best for the feature to be
   tested on nightly for the time being anyway.
3. **Does not cascade.** The feature is impactful when enabled only at
   the "top level", be it at a crate with no dependents or in an
   external system. This is an important condition, as features that do
   not meet this requirement very quickly start violating [Rust's
   stability guarantees][stability].

Preview features are always approved for only a limited period of time,
usually until the end of the _next_ release cycle. This process is
automated -- when a feature is marked for preview, it is also given an
expiry, and any build after the listed expiry will treat the preview as
inactive.

Over the course of the preview period, the hope is that users provide
experience reports for the feature on stable, which can then serve as
evidence supporting stabilization. Once a feature enters preview, teams
should strive to make a stabilization decision by the time the preview
period ends. If the decision is to stabilize the feature, the preview
window should be extended until the next beta release that includes the
stabilization so that users experience no gap in support.

## In Cargo

Cargo traditionally exposes unstable features through the `-Z` flag,
though the `[unstable]` table in `.cargo/config.toml` can also be used.
Through these, users can enable new syntax in `Cargo.toml`, experimental
command-line options, and other opt-in behavior changes that may not
have an external interface.

This RFC proposes that `cargo` accept a new command-line flag,
`--enable-preview`, and a new section in `.cargo/config.toml`:
`[enable-preview]`. These are both available on beta/stable, and allow
the user to list unstable features by name, or command-line options by
their option. Passing in this flag makes `cargo` pretend that the
corresponding feature/option is stable. That is,
`--enable-preview=im-a-teapot` is equivalent to `-Zim-a-teapot`, and
`--enable-preview=--out-dir` is equivalent to `-Zunstable-options` with
the restriction that only the unstable option `--out-dir` is permitted.

Preview feature enabling is a binary setting, and does not accept values
or additional options. This is because a preview should happen only when
a feature is seeking stabilization, at which point its interface should
be entirely stable. For example, an experimental command-line option
currently exposed as `-Zstrip=[none|symbols|debuginfo]` could not be
placed in preview, as it is not yet clear how the feature _would_ be
exposed for stabilization (it can't be with `-Z`).

Cargo should forward any `--enable-preview` features it does not
recognize to `rustc`.

## In Rust

Rust exposes unstable features in two primary ways: `-Z` compiler flags
(like Cargo) and through `#![feature]` crate-level attributes. To
support preview features, `rustc`, like `cargo`, will gain an
`--enable-preview` flag on both beta and stable. That flag takes a list
of features by name and command-line options by their option, and makes
`rustc` pretend as if each passed-in feature/option is already stable.

# Initial candidates

This is a list of initial candidate preview features [solicited from
internals.rust-lang.org][thread]:

 - `rustc -Zstrip`: https://github.com/rust-lang/rust/issues/72110
 - `rustc -Zoom=panic`: https://github.com/rust-lang/rust/issues/43596
 - `cargo -Zpatch-in-config`: https://github.com/rust-lang/cargo/issues/9269

[thread]: https://internals.rust-lang.org/t/survey-of-high-impact-features/14536

# Drawbacks
[drawbacks]: #drawbacks

First and foremost, this addition enables users to opt out of the Rust
stability guarantees on the stable release channel, a choice that was
[explicitly rejected][stability] in the past. Below is the rationale
given, and why the mechanism proposed in this RFC may be acceptable
after all.

> First, as the web has shown numerous times, merely advertising
> instability doesn't work. Once features are in wide use it is very
> hard to change them -- and once features are available at all, it is
> very hard to prevent them from being used. Mechanisms like "vendor
> prefixes" on the web that were meant to support experimentation
> instead led to de facto standardization.

This RFC [specifically targets](#preview) features that need only be
used in a small number of places to have a large impact, and that do not
require cascading changes. Thus, any feature that makes it into preview
should hopefully only be made use of in a handful of locations. Since
preview periods are inherently time-limited, those early adopters are
also required to be prepared for the feature leaving preview. Taken
together, this should mean that exposing a feature under preview will
not be equivalent to long-term stabilization.

> Second, unstable features are by definition work in progress. But the
> beta/stable snapshots freeze the feature at scheduled points in time,
> while library authors will want to work with the latest version of the
> feature.

The RFC requires features to be specifically _chosen_ for preview under
the requirement that they have a fairly stable implementation and
interface already. A feature _may_ still end up changing while under
preview, but any wide-reaching impacts should (hopefully) already have
been nailed out on nightly before the feature is proposed for preview.

> Finally, we simply cannot deliver stability for Rust unless we enforce
> it. Our promise is that, if you are using the stable release of Rust,
> you will never dread upgrading to the next release. If libraries could
> opt in to instability, then we could only keep this promise if all
> library authors guaranteed the same thing by supporting all three
> release channels simultaneously.

Once a user makes use of a preview feature, they _will_ now experience
some of that dread; the preview period will eventually end, and the
feature may then be removed. But, this is something the user is aware
of, and knows the timeline for, the moment they choose to opt into the
preview. Furthermore, it's something that their consumers should not
_also_ be forced to opt into due to the no-cascading requirement, so
undoing the change _should_ be a local action.

Another drawback of this approach is that since stable and beta
necessarily lag behind nightly, fixes and improvements to a preview
feature will take a while to reach preview testers. This in turn means
that testers will be testing a potentially known-buggy version of a
feature, and their input may be outdated.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This RFC attempts to balance the need for testing of unstable features
on a stable release channel against Rust's "stability as a deliverable"
policy. There are many other points in the design-space:

**`RUSTC_BOOTSTRAP` + `-Zallow-features`:** The Rust toolchain already
supports using unstable features on the stable compiler through the
`RUSTC_BOOSTRAP` environment variable used to build the standard library
with the latest stable compiler on each stable release. Users can
further limit which features can be used by passing a list of features
to `-Zallow-features` to avoid opting into _all_ unstable features,
which is [particular important for build systems][test-unstable]. We
could promote this as _the_ way to test unstable features on stable.

However, `RUSTC_BOOTSTRAP` is a bit of a blunt instrument -- users have
little insight into how stable the features they opt into are. They also
need to remember to include `-Zallow-features` to avoid feature creep.
Furthermore, since the list of features aren't restricted, users can
inadvertently opt into features that are then painful to remove later
on, such as if they propagate to changes in their consumers. Preview
features are essentially a list of features that the Rust maintainers
have carefully decided are reasonable to test on stable, so that
individual developers do not need to make that determination themselves.

[test-unstable]: https://internals.rust-lang.org/t/mechanism-for-beta-testing-unstable-features/14280

**Use nightly:** Rust already has an avenue for testing unstable
features: the nightly release channel. Since nightly releases are made,
well, nightly, it contains the very latest fixes and features for
unstable features, so any experience reports of bugs or poor UX are
likely to be current and actionable. The nightly channel also inherently
informs users that they are getting on a fast-moving train. We could
tell users who want to test unstable features that they must do so on
nightly, with no exceptions.

However, for larger organization using Rust, this is a non-starter.
While Rust's nightly releases are _generally_ quite stable, the
frequency of stable-to-nightly regressions is much higher than that of
stable-to-stable regressions. That, in turn, makes nightly a risky
proposition for large-scale users for whom stability is a must.
Furthermore, since nightly changes constantly, users with particularly
large code bases may have a hard time finding _any_ nightly release that
has no known bugs that affect any of their potentially millions of lines
of code while also supporting a particular feature they wish to test.
And finally, using nightly shares the same challenge as
`RUSTC_BOOTSTRAP` where users opt into _all_ unstable features, rather
than a carefully curated list.

**Use beta:** This has been [suggested in the past][unstable-beta] as a
compromise between allowing unstable features on stable and requiring
testers to use nightly. The idea is to not allow unstable features on
stable, but to allow them on the beta channel which should be _more_
stable than nightly. Furthermore, since beta is closer to nightly,
unstable features on beta would be _more_ up to date, which should
ensure that fewer experience reports are based on an outdated
implementation.

However, the use of beta inherits problems from both of the preceding
options. First, opting into _all_ unstable features means that users can
easily inadvertently start using features that will be hard to remove,
or whose implementation is still very much in flux. And second, since
beta is automatically cut from nightly, its stability varies over time
-- when a beta is freshly cut, it is no more stable than nightly is.
Thus, stability-sensitive users are likely to be wary of deploying beta
widely. As an added challenge, the beta channel is supposed to serve as
a testing ground for the next stable release. This means we want it both
to be as identical as possible to the coming stable, and to encourage
users to rely on its stability just as they do on that of stable.
Allowing unstable features on beta but _not_ stable violates that.

[unstable-beta]: https://internals.rust-lang.org/t/allow-unstable-features-on-beta/2986

**A new release channel:** Since none of the current release channels
are a great fit for allowing unstable features, the idea of introducing
a new, fourth release channel [has been proposed][new-channel]. We would
introduce a channel that is more stable than nightly (and early betas),
less stale than stable, and allows unstable features, that is
specifically geared towards testing unstable features. We could even
_require_ that `-Zallow-features` is set on such a channel, so that
users do not blindly use that channel instead of nightly and then start
reporting already-fixed issues.

However, not only is a new release channel a fairly large endeavour,
it's also not clear that it buys us much. If the channel is a fork of
stable with unstable features enabled, then the challenge still arises
that users can opt into _any_ feature, including those that shouldn't
really be considered ready for use yet. Furthermore, users would be
forced to wait for a release cycle and a half for fixes to unstable
features to reach them, unless we _also_ establish a `testing-beta`,
with all of the implications that carries. If the channel is instead a
fork of nightly, well, then users can just use nightly instead. A fork
off of beta splits the difference, but then means testing will vary
in stability over time, just like beta does. There's also the question
of whether backports should be made to such a channel, which would
further increase the effort required.

[new-channel]: https://internals.rust-lang.org/t/the-case-for-a-new-relese-channel-testing/14412

**Not doing this:** The status quo has worked for Rust up until now --
testing happens on nightly, and that's that. And, as set out in the
[motivation section][motivation], that has worked well for many of
Rust's features so far. However, it means that high-impact,
low-visibility features end up with less testing, and thus a lower
chance of being stabilized, even though those features may be highly
important to big-fan-out-users, and potentially even blockers for
adoption. The concern, more concretely, is that features that larger
users, like organizations, external build systems, or non-Rust projects
that _use_ Rust projects highly desire, or even _need_, fall by the
wayside, and ultimately hold back Rust's adoption in those ecosystems.
Which would be a shame. Some of the trade-offs involved are discussed in
depth in [this internals thread on getting more testing of unstable
features][more-testing].

[more-testing]: https://internals.rust-lang.org/t/getting-more-testing-of-unstable-features/4954

# Prior art
[prior-art]: #prior-art

## NodeJS

NodeJS library features are marked with a ["stability"
indicator][node-stability], but it is only used for documentation, and
not for enforcement. Language features are rarer, but when support for
ECMAScript modules was being tested, their opted for an [explicit opt-in
flag][node-experimental]: `--experimental-modules`. That flag did not
guarantee stable behavior, but was available on stable releases.

[node-experimental]: https://nodejs.medium.com/announcing-a-new-experimental-modules-1be8d2d6c2ff
[node-stability]: https://nodejs.org/api/documentation.html#documentation_stability_index

## Java

Java has [incubator modules] and [preview features] for library features
and language features respectively. The former take the form of modules
under the `jdk.incubator.` prefix in its module name (it can also be
used for tool names). Such modules are included with the standard stable
JDK releases, and are explicitly renamed (requiring user action) when
stabilized. The specification also states:

> If an incubating API is **not** standardized or otherwise promoted
> after a small number of JDK feature releases, then it will no longer
> be able to incubate, and its packages and incubator module will be
> removed.

Which is similar to the time bound for preview features in this RFC.
Furthermore:

> An incubating feature need not be retained forever in the JDK Release
> Project in which it was introduced, nor in every release of downstream
> binaries derived from that Release Project. For example, an incubating
> feature may evolve, or even be removed, between different update
> releases of a JDK Release Project. Beyond this explicit statement of
> when evolution is permitted, this proposal deliberately provides no
> further guidance. Such decisions are best left to the individual
> feature owner.

The latter (preview features) are summarized in [JEP12][preview
features] as

> A preview feature is a new feature of the Java language, Java Virtual
> Machine, or Java SE API that is fully specified, fully implemented,
> and yet impermanent. It is available in a JDK feature release to
> provoke developer feedback based on real world use; this may lead to
> it becoming permanent in a future Java SE Platform.

and later adds

> whose design, specification, and implementation are complete, but
> which would benefit from a period of broad exposure and evaluation
> before either achieving final and permanent status in the Java SE
> Platform or else being refined or removed.

Which is very similar to the desire for preview features in this RFC,
though with an even stronger requirement for implementation stability.
The spec also suggests a timeline that resembles the one proposed in
this RFC (two release cycles):

> By "complete", we do not mean "100% finished", since that would imply
> feedback is pointless. Instead, we mean "100% finished within 12
> months". This timeline reflects our experience that two rounds of
> previewing is the norm, i.e., preview in Java `$N` and `$N+1` then
> final in `$N+2`.

In particular, the spec recommends that preview features are _high
quality_, _not experimental_, and _universally available_.That is, the
feature should meet the same level of "technical excellence and finesse"
as a final and permanent feature, and the feature should not be
experimental, risky, incomplete, or unstable. The "universally
available" bit isn't very relevant in our case, and requires that all
preview features are available in all Java SE implementations.

Java's Preview features are all disabled by default, and can be enabled
explicitly with `--enable-preview`, though that flag enables the use of
_all_ preview features. This RFC instead proposes that individual
features can be enabled individually.

Any use of a Java preview feature generates a non-suppressible warning.

The JEP12 section on [process] is also a good read.

[preview features]: https://openjdk.java.net/jeps/12
[incubator modules]: https://openjdk.java.net/jeps/11
[process]: https://openjdk.java.net/jeps/12#Process-issues

## Ruby and Python

Ruby and Python publish preview releases that may included
not-yet-stabilized features. As far as I can tell, there is no formal
process around opting into these beyond "run preview".

## Go

Go does not appear to have any mechanism for previewing and testing
language features. Library features are previewed using `x` modules,
such as [`x/tools`](https://pkg.go.dev/golang.org/x/tools) and
[`x/net`](https://pkg.go.dev/golang.org/x/net), which carry no stability
guarantees.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- How long should the preview period be?
- What features should be chosen initially?
- How strict should we be about adding new features?
- How does a feature get proposed for preview in the firstplace?
- Should we require that the API of preview features not change during
  the preview period?
- Is an FCP appropriate for choosing whether to land a feature in
  preview?
- Should preview features be advertised more widely? Where?
- Should use of a preview feature give a lint warning? Should it be
  possible to suppress?

# Future possibilities
[future-possibilities]: #future-possibilities

We could consider extending preview features to cover editions as well:
`--enable-preview=edition2021` would enable `--edition=2021` on
stable/beta before we actually release the 2021 edition.

Given sufficient resources, making a feature available in preview could
be extended to mean committing to backporting fixes to that feature onto
stable/beta as well, so that stale feedback from stable/beta during the
feedback cycle is mitigated.
