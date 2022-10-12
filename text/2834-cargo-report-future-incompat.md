- Feature Name: N/A
- Start Date: 2019-12-10
- RFC PR: [rust-lang/rfcs#2834](https://github.com/rust-lang/rfcs/pull/2834)
- Rust Issue: [rust-lang/rust#71249](https://github.com/rust-lang/rust/issues/71249)

# Summary
[summary]: #summary

Cargo should alert developers to upstream dependencies that trigger
future-incompatibility warnings. Cargo should list such dependencies
even when these warnings have been suppressed (e.g. via cap-lints or
`#[allow(..)]` attributes.)

Cargo could additionally provide feedback for tactics a maintainer of
the downstream crate could use to address the problem (the details of
such tactics is not specified nor mandated by this RFC).

# Motivation
[motivation]: #motivation

From [rust-lang/rust#34596][]:

> if you author a library that is widely used, but which you are not
> actively using at the moment, you might not notice that it will
> break in the future -- moreover, your users won't either, since
> cargo will cap lints when it builds your library as a dependency.

[rust-lang/rust#34596]: https://github.com/rust-lang/rust/issues/34596

Today, cargo will cap lints when it builds libraries as dependencies.
This behavior includes future-incompatibility lints.

As a running example, assume we have a crate `unwary` with an upstream
crate dependency `brash`, and `brash` has code that triggers a
future-incompatibility lint, in this case a borrow `&x.data.0` of a packed field
(see [rust-lang/rust#46043][], "safe packed borrows").

[rust-lang/rust#46043]: https://github.com/rust-lang/rust/issues/46043

If `brash` is a non-path dependency of `unwary`, then building
`unwary` will suppress the warning associated with `brash` in its
diagnostic output, because the build of `brash` will pass
`--cap-lints=allow` to its `rustc` invocation. This means that a
future version of Rust is going to fail to compile the `unwary`
project, with no warning to the developer of `unwary`.

Example of today's behavior (where in this case, `brash` is non-path dependency of `unwary`):

```
crates % cd unwary
unwary % cargo build                                                # no warning issued about problem in the `brash` dependency.
   Compiling brash v0.1.0
   Compiling unwary v0.1.0 (/tmp/unwary)
    Finished dev [unoptimized + debuginfo] target(s) in 0.30s
unwary % cd ../brash
brash % cargo build                                                 # (but a `brash` developer will see it when they build.)
   Compiling brash v0.1.0 (/tmp/brash)
warning: borrow of packed field is unsafe and requires unsafe function or block (error E0133)
  --> src/lib.rs:13:9
   |
13 | let y = &x.data.0;
   |         ^^^^^^^^^
   |
   = note: `#[warn(safe_packed_borrows)]` on by default
   = warning: this was previously accepted by the compiler but is being phased out; it will become a hard error in a future release!
   = note: for more information, see issue #46043 <https://github.com/rust-lang/rust/issues/46043>
   = note: fields of packed structs might be misaligned: dereferencing a misaligned pointer or even just creating a misaligned reference is undefined behavior
    Finished dev [unoptimized + debuginfo] target(s) in 0.30s
brash %
```

Cargo passes `--cap-lints=allow` on upstream dependencies for good
reason, as discussed in [Rust RFC 1193][] and the comment thread from
[rust-lang/rust#59658][].
For cases like future-incompatibility lints, which
are more severe warnings for the long-term viability of a crate, we
need to provide some feedback to the `unwary` maintainer. 

But this feedback should not be *just* the raw diagnostic output for
`brash`! The developer of a crate like `unwary` typically cannot do
anything in the short term about warnings emitted by upstream crates,

 * (The `unwary` developer *can* file issues against `brash` or even
    contribute code to fix `brash`, but that does not resolve the
    immediate problem until a new version of `brash` is released by its
    maintainer.)

Therefore the diagnostics associated with building upstream
`brash` are usually just noise from the viewpoint of `unwary`'s
maintainer.

[Rust RFC 1193]: https://github.com/rust-lang/rfcs/blob/master/text/1193-cap-lints.md

[rust-lang/rfcs#1193]: https://github.com/rust-lang/rfcs/pull/1193

[rust-lang/rust#59658]: https://github.com/rust-lang/rust/issues/59658

[rust-lang/rust#27260]: https://github.com/rust-lang/rust/pull/27260

[rust-lang/cargo#1830]: https://github.com/rust-lang/cargo/pull/1830

Therefore, we want to continue passing `--cap-lints=allow` for
upstream dependencies. But we also want `rustc` to tell `cargo` (via
some channel) about when future-incompatibility lints are triggered,
and we want `cargo` to provide a succinct report of the triggers.

This RFC suggests the provided feedback take the form of a summary at
the end of cargo's build of `unwary`, as illustrated in the explanation below.

Furthermore, we want the feedback to provide guidance as to how the
`unwary` maintainer can address the issue. Here are some potential
forms this additional guidance could take.

 * cargo could respond to the future-incompatibilty signaling by querying
   the local index to find out if a newer version of the upstream crate is
   available. If a newer version is available, then it could 
   suggest to the user they might upgrade to it.
   If such an upgrade could be done via `cargo update`, then the
   output could obviously suggest that as well.

   (This is just a heuristic measure, as it would not attempt to
   check ahead of time if the newer version actually resolves the
   problem in question.)

   A further refinement on this idea would be to query
   `crates.io` itself If cargo is not running in "offline mode". But
   querying the index may well suffice in practice.

 * Cargo could suggest to the `unwary` maintainer that they file a bug
   (or search for previously-filed bugs) in the source repository for
   the upstream crate that is issuing the future-incompatibility
   warning. (That is, the `brash` author might not be aware of the
   issue; for example, if they last updated their crate before the
   lint in question was deployed on the Rust compiler.)

 * `rustc` itself could embed, for each future-incompatibility lint,
   how soon the Rust developers will turn the lint to a hard error.
   This would give the `unwary` maintainer an idea of how much time
   they have before they will be forced to address the issue (by
   posting a PR upstream, or switching to a fork of `brash`, et
   cetera).


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

After cargo finishes compiling a crate and its upstream dependencies,
it may include a final warning about *future incompatibilties*.

A future incompatibility is a pattern of code that is scheduled to be
removed from the Rust language in some future release. Such code patterns
are usually instances of constructs that can exhibit undefined behavior
(i.e. they are unsound) or do not have a well-defined semantics,
but are nonetheless in widespread use and thus need a
grace period before they are removed.

If any crate or any of its upstream dependencies has code that
triggers a future incompatibility warning, but the overall compilation
is otherwise without error, then cargo will report all instances of
crates with future incompatibilities at the end of the compilation.
When possible, this report includes the future date or release version
where we expect Rust to stop compiling the code in question.

Example:

```
crates % cd unwary
unwary % cargo build
   Compiling brash v0.1.0
   Compiling bold v0.1.0
   Compiling rash v0.1.0
   Compiling unwary v0.1.0
    Finished dev [unoptimized + debuginfo] target(s) in 0.30s

    warning: the crates brash, bold, and rash contain code that will be rejected by a future version of Rust.
    note: the crate rash will stop compiling in Rust 1.50 (scheduled for February 2021).
    note: to see what the problems were, invoke `cargo describe-future-incompatibilities`
unwary %
```

If the dependency graph for the current crate contains multiple versions of
a crate listed by the end report, then the end report should include which
version (or versions) of that crate are causing the lint to fire.

Invoking the command `cargo describe-future-incompatibilities` will make cargo
query information cached from the previous build and print out a more informative
diagnostic message:

```
unary % cargo describe-future-incompatibilities
The `brash` crate currently triggers a future incompatibility warning with Rust,
with the following diagnostic:

> warning: borrow of packed field is unsafe and requires unsafe function or block (error E0133)
>   --> src/lib.rs:12:9
>    |
> 12 | let y = &x.data.0; // UB, also future-compatibility warning
>    |         ^^^^^^^^^
>    |
>    = note: `#[warn(safe_packed_borrows)]` on by default
>    = warning: this was previously accepted by the compiler but is being phased out; it will become a hard error in a future release!
>    = note: for more information, see issue #46043 <https://github.com/rust-lang/rust/issues/46043>
>    = note: fields of packed structs might be misaligned: dereferencing a misaligned pointer or even just creating a misaligned reference is undefined behavior


The `bold` crate currently triggers a future incompatibility warning with Rust,
with the following diagnostic:

> warning: private type `foo::m::S` in public interface (error E0446)
>  --> src/lib.rs:5:5
>   |
> 5 |     pub fn f() -> S { S }
>   |     ^^^^^^^^^^^^^^^^^^^^^
>   |
>   = note: `#[warn(private_in_public)]` on by default
>   = warning: this was previously accepted by the compiler but is being phased out; it will become a hard error in a future release!
>   = note: for more information, see issue #34537 <https://github.com/rust-lang/rust/issues/34537>


The `rash` crate currently triggers a future incompatibility warning with Rust,
with the following diagnostic:

> error: defaults for type parameters are only allowed in `struct`, `enum`, `type`, or `trait` definitions.
>  --> src/lib.rs:4:8
>   |
> 4 | fn bar<T=i32>(x: T) { }
>   |        ^
>   |
>   = note: `#[deny(invalid_type_param_default)]` on by default
>   = warning: this was previously accepted by the compiler but is being phased out; it will become a hard error in a future release!
>   = note: for more information, see issue #36887 <https://github.com/rust-lang/rust/issues/36887>
```

This way, developers who want to understand the problem have a way to find out more,
without flooding everyone's diagnostics with information they cannot use with their
own local development.


Rebuilding `unwary` continues to emit the report even if the upstream
dependencies are not rebuilt.

Example:

```
unwary % touch src/main.rs
unwary % cargo build
   Compiling unwary v0.1.0
    Finished dev [unoptimized + debuginfo] target(s) in 0.30s

    warning: the crates brash, bold, and rash contain code that will be rejected by a future version of Rust.
    note: the crate rash will stop compiling in Rust 1.50 (scheduled for February 2021).
unwary %
    note: to see what the problems were, invoke `cargo describe-future-incompatibilities`
```

To keep the user experience consistent, we should probably emit the same warning at the end
even when the root crate is the sole trigger of incompatibility lints.

```
crates % cd brash
brash % cargo build
   Compiling brash v0.1.0 (/tmp/brash)
warning: borrow of packed field is unsafe and requires unsafe function or block (error E0133)
  --> src/lib.rs:13:9
   |
13 | let y = &x.data.0;
   |         ^^^^^^^^^
   |
   = note: `#[warn(safe_packed_borrows)]` on by default
   = warning: this was previously accepted by the compiler but is being phased out; it will become a hard error in a future release!
   = note: for more information, see issue #46043 <https://github.com/rust-lang/rust/issues/46043>
   = note: fields of packed structs might be misaligned: dereferencing a misaligned pointer or even just creating a misaligned reference is undefined behavior
    Finished dev [unoptimized + debuginfo] target(s) in 0.30s

    warning: the crate brash contains code that will be rejected by a future version of Rust.
brash % cargo build
    Finished dev [unoptimized + debuginfo] target(s) in 0.00s

    warning: the crate brash contains code that will be rejected by a future version of Rust.
    note: to see what the problems were, invoke `cargo describe-future-incompatibilities`
brash %
```

And as you might expect, if there are no future-incompatibilty warnings issused, then the output of `cargo` is unchanged from today.
Example:

```
crates % cd unwary
unwary % cargo build
   Compiling brash v0.2.0
   Compiling bold2 v0.1.0
   Compiling unwary v0.2.0
    Finished dev [unoptimized + debuginfo] target(s) in 0.30s
unwary %
```

Here, the unwary (sic) crate has updated its version of `brash`,
switched to `bold2` (a fork of `bold`), and replaced its internal
usage of `rash` with some local code, thus completely eliminating all
current future-incompatibility lint triggers.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

As noted above, we want to continue to suppress normal lint checks for
upstream dependencies. Therefore, Cargo will continue to pass
`--cap-lints=allow` for non-path upstream dependencies.

At the same time, we want to minimize disruption to existing users of Rust.

Therefore, the behavior of flags that directly interact with lints, like
`-Dwarnings`, will remain unchanged by this RFC.

For example, in our running example of `unwary`:
  * running either `cargo rustc -- -Dwarnings` or `RUSTFLAGS=-Dwarnings cargo build`
    will invoke `rustc` itself the same way, and each `rustc` invocation will emit
    the same set of diagnostics that it does today for each of those cases.
  * Thus, the warning lints in the downstream `brash` non-path dependency will
    be capped, and the future-incompatibility warnings associated with that `rustc`
    invocation will be hidden.
  * When `cargo` emits a future-incompatibility report at the end of the build,
    and reports that `brash` contains code that will be rejected by
    a future version of Rust, this report is *not* a lint, and does *not* interact
    with `-Dwarnings`.
  * In summary: getting a future-incompatibility report when you
    have passed `-Dwarnings` to `rustc` will *not* fail the build.

However, the Rust compiler's behavior *will* change slightly. Even when
`--cap-lints=allow` is turned on, we need Cargo to know when a
future-incompatibilty lint is triggered.

The division of responsbiilties between Cargo and the Rust compiler
may be a little subtle:

The responsibilities of the Rust compiler (`rustc`):

  * `rustc` must differentiate future-incompatibility lints (c.f.
     [PR #59658: "Minimum Lint Levels"][rust-lang/rust#59658]) from
     other lints that are expected to remain as mere warnings forever.

  * `rustc` will need to have some new mode of operation, which this
    RFC will call the where it will check for instances of
    future-incompatibility lints, *regardless* of whether
    `--cap-lints=allow` is also set. This RFC calls this the
    *future-incompatibility checking mode*.

    * In the future-incompatibility checking mode of invocation,
      `rustc` will also need to check for such lints *regardless* of
      whether the code appears in the scope of an `#[allow(..)]`
      attribute for the lint.

    * In the future-incompatibility checking mode of invocation,
      *emission* of the diagnostics themselves may still be silenced
      as specified by `--cap-lints=allow` or `#[allow(..)]` attributes.

    * That is, those flags and annotations should be interpreted by
      `rustc` as silencing the diagnostic report, but *not* as
      silencing the feedback about there existing some instance of the
      lint triggering somewhere in the crate's source code.

    * The future-incompatibility checking mode is meant as a way to
      address the bulk of issue [rust-lang/rust#34596][].

The responsibilities of Cargo:

  * Cargo is responsible for invoking `rustc` in a way that enables
    the future-incompatibility checking mode. This
    mode of invocation occurs *regardless* of whether
    `--cap-lints=allow` is also being passed when the crate is
    compiled.

  * Cargo is responsible for capturing any output from the
    future-incompatibility checking mode and summarizing it at the end
    of the whole build.

  * Cargo is responsible for storing a record of any future-incompatibility
    for a crate somewhere in the `target/` directory, so that it can
    emit the same report without having to rebuild the crate on subsequent
    rebuilds of the root crate.

  * Cargo is responsible for suggesting ways to address the problem to
    the user. The specific tactics for constructing such suggestions
    are not mandated by this RFC, but some ideas are presented as
    [Future possibiilties][future-possibilities].

## Implementation strategy: Leverage JSON error-format

The cleanest way to implement the above division of responsbilities
without perturbing *non-cargo* uses of `rustc` is probably to make 
the following change:

 * `rustc` should treat `--error-format=json` as signal that it should
   emit a future-incompatibility summary report for the crate.

It is relatively easy to extend the JSON output of `rustc` to include
a new record of any future-incompatibility lints that were triggered
when compiling a given crate.

 * However, this RFC does not *dictate* this choice of implementation
   strategy. (Other options include using some environment variable to
   opt-in to a change to `rustc`'s output, or having `rustc` emit
   future-incompatibility metadata to the filesystem.)

Some (but not all) future-incompatibility lints will have a concrete
schedule established for when they are meant to become hard errors.
(This RFC does not specify the details about how such schedules are
established or what constraints they will have to meet; it just posits
that they *will* be established by some means.) The metadata for every
future-incompatibility lint should include the anticipated version of
Rust, if known, where it will become a hard error.

 * This adds motivation for using JSON formatted diagnostics: JSON
   records are more readily extensible, and thus can support adding
   this sort of feedback in a robust fashion.

Since `cargo` is expected to continue to emit the report even when the
upstream dependencies are not rebuilt, Cargo will store the
future-incompatibility status for each crate somewhere in the `target/`
directory on the file-system.
(This should be a trivial constraint today: Cargo on the nightly channel
is already locally caching warnings emitted while building upstream 
path-dependencies.)


## Annoyance modulation

Emitting a warning on all subsequent cargo invocations until the
problem is resolved may be overkill for some users.

In particular, it may not be reasonable for someone to resolve the 
flagged problem in the short term.

In order to allow users to opt-out of being warned about future
incompatibility issues on every build, this RFC proposes
extending the `.cargo/config` file with keys that allow
the user to fine-tune the frequency of how often cargo will
print the report. For example:

```
[future_incompatibility_report]
# This setting can be used to reduce the frequency with which Cargo will report
# future incompatibility issues.
#
# The possible values are:
# * "always" (default): always emit the report if any future incompatibility 
#                       lint fires,
# * "never": never emit the report,
# * "post-cargo-update": emit the report the first time we encounter a given
#                         future incompatibility lint after the most recent
#                         `cargo update` run for a crate,
# * "daily": emit the report the first time any particular lint fires each day,
# * "weekly": emit the report the first time any particular lint fires
#             each week (starting from Monday, following ISO 8601),
# * "lunar": emit the report the first time any particular lint fires every
#            four weeks. (We recommend using this value in tandem with
#            an IDE that presents the current phase of the moon in its UI.)
frequency = "always"

# This allows a further fine-tuning for lints that have been given an
# explicit schedule for when they will be turned into hard errors.
#
# If false, such scheduled lints are treated the same as unscheduled ones.
#
# If true, such scheduled lints issue their report more frequently
# as time marches towards the release date when the warning becomes an error.
#
# Specifically,
# * 6 weeks before that release, the report is emitted at least once per week,
# * 2 weeks before that release, the report is emitted on every build.
#
# (Note that a consequence of the above definition, this value of this setting
# has no effect if the `future_incompat_report_frequency` is "always".)
telescoping_schedule = true
```

(This RFC does not actually prescribe the precise set of keys and
values laid out above. We trust the Cargo team to determine an
appropriate set of knobs to expose to the user.)

## Policy issues

We probably do not want to blindly convert all lints to
use this system. 
The mechanism suggested here may not be appropriate for every single
lint currently categorized as `C-future-incompatility` on the Rust repo.
That decision is a policy matter for the relevant teams,
and the form of such policy is out of scope for this RFC.

Whatever form that policy takes, it is worth noting: Users who encounter
upstream future-incompatibility issues may have neither free time nor external
development resources to draw upon. The rustc developers need to take some care
in deciding *when* a future-incompatibility lint should start being reported via
this mechanism.

 * If our primary goal is to minimize user frustration with our tools and
   ecosystem, then future-incompatibility reporting for a given lint should be
   turned only after much of the crate ecosystem have new fixed versions. In
   other words, we should strive for a steady-state where the typical user
   response to a future-incompatibility report is that user then runs 
   `cargo update`, or they ask for a (pre-existing) PR to be merged.

# Drawbacks
[drawbacks]: #drawbacks

The change as described requires revisions to both `rustc` and
`cargo`, which can be tricky to coordinate.

This RFC suggests an approach where the changes are somewhat loosely
coupled: Use of `--error-format=json` will enable the
future-compatibility checking mode. This avoids the need to add a new
stable command line flag to `rustc`; but it also may be a confusing
change in compiler semantics for any non-cargo client of `rustc` that
is using `--error-format=json`.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## No change needed?

Some claim that "our current approach" has been working, and therefore
no change is needed here. However, my counterargument is that the only
reason we haven't had to resolve this before is that compiler and
language teams have been forced to be very conservative in changing
existing future-incompatibility lints into hard errors, because it
takes a lot of effort to make such transitions, (in no small part
**because** of the issue described here).

In the cases where the compiler and language teams have turned such
lints into hard errors, the teams spent significant time evaluating
breakage via crater and then addressing such breakage. The changes
suggested here would hopefully encourage more Rust users *outside* of
the compiler and language teams to address future-compatibility
breakage.

## Is there something simpler?

One might well ask: Is this RFC overkill? Is there not a simpler way
to address this problem?

The following is my attempt to enumerate the simple solutions that were obvious
to me. As we will see, these approaches would have serious drawbacks.

### Can we do this in Cargo alone?

With regards to implementation, we could avoid attempt making changes
to `rustc` itself, and isolate the implementation to `cargo` alone.
The main way I can imagine doing this is to stop passing
`--cap-lints=allow`, and then having Cargo capture all diagnostic
output from the compiler and post-processing it to determine which
lints are future-incompatible warnings. However, this has a number of
problems:

 * It is fragile, since it relies on Cargo post-processing the compiler diagnostic output.

 * It is inefficient, since the compiler will now always run all the
   lints checks for all dependencies of a crate but we only care about
   a small subset of the lints.

 * It is insufficient, because it only handles instances of `--cap-lints`; it would
   fail to catch instances where an upstream dependency is using an `#[allow(..)]`
   attribute in the source to sidestep warnings.

   * If we addressed that insufficiency by unconditionally changing
     `rustc` to always emit feedback about future-incompatibilities
     regardless of `--cap-lints=allow` or `#[allow(..)]`, then that
     would probably upset people who expect those flags/annotations to
     keep the diagnostic output quiet.

   * (In other words, it would be an instance of
     "the compiler is not listening to me!")

   * That is why this RFC proposes that `--cap-lints=allow` and
     `#[allow(..)]` should continue to silence the diagnostic report
     for lints listed by those flags and annotations, and restrict the
     feedback solely to a final warning of the form "the crate brash
     contains code that will be rejected by a future version of Rust."

### Can we do this in Rust alone?

PR [rust-lang/rust#59658][] "Minimum Lint Levels" implemented a
solution in the compiler alone, by tagging the future-incompatibility
lints as special cases that would not be silenced by `--cap-lints` nor
`#[allow(..)]`. The discussion on that PR described a number of
problems with this; in essence, people were concerned about getting
spammed by lints that the downstream developer couldn't actually do
anything about.

The discussion on that PR concluded by saying that it could possibly
be reworked to reduce the amount of spam by reporting a single
instance of a lint for each dependency (rather than having a separate
diagnostic for each *expression* that triggered the lint within that
dependency).

 * The latter would indeed be an improvement on
   [PR 59658][rust-lang/rust#59658], but it still would not be an ideal
   user-experience. The change suggested by this RFC deliberately
   treats occurrences of future-incompatibility lints as separate from
   normal diagnostics: serious events worthy of being treated
   specially by Cargo, to the extent that it might e.g. do an online query
   to see if a newer version of the given crate exists. We want to
   make the process of fixing these issues as easy as we can for the
   developer, and doing that requires help from Cargo.

 * It is entirely possible that we will want to move forward with
   minimum-lint levels, independently of this RFC. The machinery proposed
   there is not in conflict with what I am proposing here; I am just saying that
   it would not be sufficient for resolving the problem at hand.

## Would extending cap-lints be preferable?

One goal of the RFC as written was to try to minimize the impact on
the Rust ecosystem. Thus it does not change the behavior of the default
output error-format, and instead leverages `--error-format=json`.
But this might be too subtle an approach.

One other way to still minimize impact would be to extend
the `--cap-lints` hierarchy so that it looks like this:

```
allow
warn-future-incompat
warn
deny-future-incompat
deny
```

Now, passing `--cap-lints=warn-future-incompat` would mean that we allow
(with no warning) all non-future-incompat lints, and warn on future-incompat ones.

Likewise, `--cap-lints=deny-future-incompat` would mean that we warn
on all non-future-incompat lints, and error on future-incompat ones.

Finally (and crucially), we would change the default for `cargo build`
to be `cap-lints=warn-future-incompat`. Then by default, developers
would be more directly informed about future incompatibilities in
their dependencies.

I opted not to take this approach in the design proposed of this RFC
because I suspect it would suffer from the same problems exhibited by
["minimum lint levels"][rust-lang/rust#59658]: it would present a
bunch of diagnostics that developers cannot immediately resolve
locally. (However, it may still be a reasonable feature to add to
`rustc` and `cargo`!)

# Prior art
[prior-art]: #prior-art

None I know of, but I'm happy to be educated.

(Has Python done anything here with the migration from Python 2 to
Python 3? I briefly did some web searches but failed to find much of
use.)

# Unresolved questions
[unresolved-questions]: #unresolved-questions

There are future-incompatibility warnings emitted by cargo itself,
such as discussed on [rust-lang/cargo#6313 (comment)][cargo 6313 comment]. I imagine it
shouldn't require much effort to incorporate support for such lints, but 
I have not explicitly addressed nor seriously investigated this.

[cargo 6313 comment]: https://github.com/rust-lang/cargo/issues/6313#issuecomment-505626509

## Implementation questions

* Is using `--error-format=json` as the way to switch `rustc` into the future-incompatibility checking mode reasonable?

  * An variant on the strategy:
    we could use the `--json CONFIG` option to `rustrc` as a way for
    `cargo` to opt into the feature.
    This way, clients already using `--error-format=json`
    would not need to know abot this change.

# Future possibilities
[future-possibilities]: #future-possibilities

The main form of follow-up work I envisage for this RFC is what
feedback that Cargo gives regarding the issues.

Cargo is responsible for suggesting to the user how they might address an
instance of a future-incompatibility lint.

Some ideas for suggestions follow.

## Query for newer/alternate versions of the crate

When crates trigger future-incompatibility warnings, Cargo could look for newer versions of the dependency on crates.io.

Example:

```
crates % cd unwary
unwary % cargo build
   Compiling brash v0.1.0
   Compiling bold v0.1.0
   Compiling rash v0.1.0
   Compiling unwary v0.1.0
    Finished dev [unoptimized + debuginfo] target(s) in 0.30s

    warning: the crates brash, bold, and rash contain code that will be rejected by a future version of Rust.
    note: the crate rash will stop compiling in Rust 1.50 (scheduled for February 2021).
    note: newer versions of bold and rash are available; upgrading to them via `cargo update` may resolve their problems.
unwary %
```

This suggestion as written only covers upgrading to newer versions.
But with more help from crates.io itself, we could go even further
here: We could suggest potential *forks* of the upstream crate that
one might switch to using. This could be useful in dealing with
abandonware.

## Suggest a bug report

If no newer version of the triggering crate is available, Cargo could
emit a template for a bug report the user could file with the
upstream crate.

Example:

```
crates % cd unwary
unwary % cargo build
   Compiling brash v0.1.0
   Compiling unwary v0.1.0
    Finished dev [unoptimized + debuginfo] target(s) in 0.30s

    warning: the crate brash contains code that will be rejected by a future version of Rust.
    note: the following gist contains a bug report you might consider filing with the maintainer of brash.
          https://gist.github.com/pnkfelix/ae03d3ea95160fb71a797b15e05f8d49
unwary %
```

In this example, the template is posted to a `gist` (The gist is using
my own github account here; to be honest I am not sure whether we
would be able to anonymously gist things from cargo in this manner,
but we should be able to find *some* pastebin service to use for this
purpose).

For ease of reference, here is the text located at the gist url above:

> This crate currently triggers a future incompatibility warning with Rust.
>
> In src/lib.rs:13:9, there is the following code:
>
> ```rust
> let y = &x.data.0;
> ```
>
> This causes `rustc` to issue the following diagnostic, from https://github.com/rust-lang/rust/issues/46043
>
> ```
> warning: borrow of packed field is unsafe and requires unsafe function or block (error E0133)
>   --> src/lib.rs:13:9
>    |
> 13 | let y = &x.data.0;
>    |         ^^^^^^^^^
>    |
>    = note: `#[warn(safe_packed_borrows)]` on by default
>    = warning: this was previously accepted by the compiler but is being phased out; it will become a hard error in a future release!
>    = note: for more information, see issue #46043 <https://github.com/rust-lang/rust/issues/46043>
>    = note: fields of packed structs might be misaligned: dereferencing a misaligned pointer or even just creating a misaligned reference is undefined behavior
> ```
>
> Since this construct is going to become a hard error in the future, we should eliminate occurrences of it.

Further refinement of this idea: If we did start suggesting bug report
templates, then Cargo might also be able to *search* for issues with
descriptions that match the template on that crate's repostory, and
advise the user to inspect that bug report to see its current status,
rather than file a new bug with the upstream crate, which might be
otherwise annoying for those maintainers.
