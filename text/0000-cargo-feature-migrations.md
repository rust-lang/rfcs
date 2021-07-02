- Feature Name: (fill me in with a unique ident, `my_awesome_feature`)
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Extend Cargo to allow some limited forms of migrations of feature sets to be expressed.
This will allow adding new features to make existing functionality optional without causing needless breakage.

# Motivation
[motivation]: #motivation

## Problem

Today, Cargo's features most easily support workflow where features added in a new versions of a crate gate new functionality.
For example, in a new version of a crate, someone crates a new function, and adds it behind a new feature; neither the function nor feature existed in the old version of the crate.
The problem is there is another quite different use case of features: making previously mandatory functionality optional.
For example, in a new version of a create, some creates a new feature for a function that *already* exists, so that it can be disabled.
The workflow isn't supported so well, the only avoidance Cargo supports for it is the "default features" set, which isn't sufficient for reasons that will be provided below.

This second use-case is really important --- in fact, perhaps more important.
The important thing to realize about features is that the ultimate *reason* they are used is not controlling a creates interface (outputs), but it's *dependencies* (inputs).
No one minds if a crate provides harmless functionality they don't need.
They do, however, mind if it depends on another crate they cannot provide, e.g. because of platform considerations (bare metal, etc.).
That means if someone adds new functionality that doesn't require new deps (common for case 1), there's little reason to bother adding features.
Conversely, if someone wants to make a dep like `std` optional, there is a lot of benefit to add `std` feature to do so.

Let's return to "default features".
What is it for, and why is it not sufficient?
Simple enough, if we create a new feature gating existing functionality, existing consumers of the functionality won't know about the new feature, so we need to provide some avoidance so they continue to receive the functionality without changing their "request".
Default features allow a new feature to be on by default, so the old functionality continues to work.
New consumers that don't want to use the now-optional feature (and moreover incur the now-optional dependencies obligations), can opt out of default features and then provide a feature list that isn't embellished.

The problem is, what happens if we later have *another* feature making existing functionality optional?
Concretely, let's say 1.0 has no features with functions `foo` and `bar`, 1.1 has a default feature `foo-feature` gating `foo`, and 1.2 has default features `foo-feature` and `bar-feature` gating `foo` and `bar`.
When we introduced 1.1, there were no default features to opt out of.
But when we introduce 1.2, there could be consumers *already* opting out of default features to skip `foo-feature` / `foo`?
How can we ensure those consumers nonetheless still get access to `bar`, while still allowing new consumers to take advantage of `bar-feature` to opt out of `bar`?
Too bad, we cannot.

## New motivation

This gap in Cargo's functionality has taken on new urgency with https://github.com/rust-lang/rfcs/pull/3140, which would propose a feature for the `alloc` crate of the latter sort, making existing functionality optional.
This would be the first example of a Cargo feature on a standard library crate slates for user consumption (as opposed to being some artifact of the current build system with no plans for stabilization).
It's very important we have a good design so that we don't end up accidentally introduction breaking changes in the standard library, because there is no way for users to opt out of the latest version!

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

I recommend first reading the [rationale-and-alternatives] section.
I don't think this design is overwrought, but it does represent a new way of thinking about these sorts of issues that might feel unfamiliar.
I fully acknowledge "migrations" is a scary word for many people due to their experiences with databases.

## Main feature

The basic insight were going for is that users, when writing down their dependencies, don't want to think about all possible future (compatible) versions of a crate.
In fact, they probably don't want to think about any past versions either!
As a first approximation, most deps specs mean "give me the version I used during development or something compatible with it!"

We want to embrace that, and interpret all feature "requests" in terms of the base version being requested.
So when you write
```toml
[dependencies.other-crate]
version = "1.1"
features = []
```
you mean "I don't need any of the features that exist *as of version '1.1'*".
For users, that should be it!

For crate authors, yes, now the work of migrations comes in.
If in version 1.2 a `std` feature is added, then we need to say that users coming from 1.1 should have it enabled.
We can do it like this
```
[feature-migrations."1.1"]
"all()" = [ "std" ]
```
Yes, that `all()` is pretty obscure.
It means the "empty union"; no bad I cannot use feature lists as keys.
It is supposed to match syntax I proposed in https://github.com/rust-lang/rfcs/pull/3143#issuecomment-868829430.
I am fine if we have some sugar for this common case.

For the more common case from the discussion of alternatives, where we formerly had:
```rust
// 1.1
#[cfg(feature = "foo-feature")]
fn foo();
#[cfg(feature = "bar-feature")]
fn bar();
#[cfg(all(feature = "foo-feature", feature = "bar-feature"))]
fn baz();
```
but now have
```rust
// 1.2
#[cfg(feature = "foo-feature")]
fn foo();
#[cfg(feature = "bar-feature")]
fn bar();
// baz-feature depends on other two
#[cfg(feature = "baz-feature")]
fn baz();
```
we can have migration
```
[feature-migrations."1.1"]
"all(foo, bar)" = [ "baz" ]
```
And that's it!
Any migration that matches is applied, they all only "add" features to the requested set.

## Wither default features?

Finally, note that we handled creating a `std` feature without breakage or using `default-features`.
I suspect most uses of default-features today are also to preserve comparability more than they are a value judgment on what features "ought" to be the default.
Given the issues with the default features, and that that new migrations solve the compatabilty problem alone, I would be happy to see default features deprecated.

The biggest beneficiary of this would be the "no std" and other exotic platforms ecosystems.
It can be hard to track down myriad crates and get them to use quixotic `default-features = false` if they were happily working without.
Conversely if crates had to use `features = [ "std" ];"` from the get-go, I don't think it would be that annoying.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

1. `Cargo.toml` has a new section in the form `feature-migrations`
   The format is
   ```
   [feature-migrations.<version>]
   <feature-union> = <feature-list>
   ```
   where

    - `<version>` is a string containing a prior crate version
    - ```
      <feature-union> ::= <feature-name>
                       |  all(<possbily-empty-comma-separated-list-of-feature-names>)
      ```
2. When resolving features, use the migrations to extend the requested feature set.

   If the unions on the left match the current set, *including feature's* dependencies,
   Also depend on the features on the right.
   Features on the right are "already migrated", and shouldn't be fed back into the algorithm.
   Any match on the left *within a version* is fair game, that means both `all(foo,bar)` and `all(foo,bar,baz)` would apply if the feature set was `[ "foo", "bar", and "baz" ]`.

   As to which version block to use, start with the base version of the dependency spec, and then work backwards among compatible versions (e.g 1.2, 1.1, 1.0, but not 0.9).
   As long as some feature hasn't been migrated, keep on going backwards,
   but features that have already been matched in a later migration *won't* fire this time.
   That means if we already matched `all(foo,bar,baz)` in a later version, and now see `all(foo,bar)` in an earlier version, we *won't* apply the migration.

# Drawbacks
[drawbacks]: #drawbacks

I am a little wary of putting migrations *in* `Cargo.toml`s, especially as crates can be published out of order (new 1.x after 2.0 for example).
Migrations represent a relationship between versions, so ideally would be stored separately.
But this creates numerous engineering challenges, including out-of-band "meta-versioning" issues as the solver works from migration data which itself changes over time.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

A few ideas came up in the https://github.com/rust-lang/rfcs/pull/3140 thread that should be discussed here.

## Feature opt-outs

The most popular feature was some way to opt out of features, i.e. to say "give me the default features" *without* necessarily features 'x' or 'y'".
This does solve the problem: without users opting out of all features, just the ones they can name, there is no risk new features they will need being disabled.
However, it causes other problems.

First of all, this complicates the "additive" features mental model.
We've worked very hard to teach people how features are additive: it should always be safe to add more features.
We can't give that up across the board without destroying feature resolution, so it's important that "without necessarily features 'x' or 'y'" doesn't mean the crate will for sure not get "x" or "y", but just that they are not requesting them.
I worry this will be very subtle and hard to teach.

Second of all, there is an especially unintuitive case of the former where a non-opted-out default feature depends on an opted-out feature.
E.g. the consumer depends on "default features without 'foo'", but "bar" is another default feature that depends on "foo".
In this case, the not mentioned feature still drags in the opted out feature.
This might happen because the user forgot to include both features.
It might also happen because only in the new version, and in not the one the user was using when they wrote the spec, did the feature gain the problematic dep on the excluded feature.
Finally, it might be that that other feature and its dependency are both newer.
In any case, this means that *all* build plans (with the given version of the package) have the opted-out feature, not just some, which is liable to make things more confusing.

One might think there is a fix making the out-out a hard rejection, so such that no plan is allowed to have those features.
But that creates other problems, namely that those same sneaky new deps would disallow all plans entirely.
Moreover, this sort of "negative reasoning" undermines the entire "additive" comparability story Cargo features are supposed to have.
This will make everything brittle, and make it impossible to express when you *are* in fact, agnostic to whether some unneeded feature is enabled due to something else.

> I do think is useful to assert some features aren't enabled, but that should be done in the workspace root, not dependency crates, for sake of modularity.

Finally, and is a matter of taste, I find writing down features that I *don't* need poor UX.
We say "pay for what you use" in Rust, but writing down features that we, by definition, don't care about means cluttering our minds and `Cargo.toml`s.
I would only want to propose negative reasoning as an absolute last resort.

## Always at least one feature

In https://github.com/rust-lang/rfcs/pull/3140#issuecomment-862109208, @Nemo157 wrote:

> The way this is solved in other crates is to move everything behind a feature and make that default on.
> You can then in the future subset that feature as necessary, and make it activate those new sub-features.

To expound on that bit, the idea is that the empty feature set should always correspond to the empty crate.
Realistic consumers, will always opt-into at least one feature.

This has a lot of nice properties.
First of all, we don't even need any notion of "default features" anymore for compatability's sake.
Since there was at least one feature from the get-go, we simply prevent breaking changes by not removing old features.
When we want to make existing functionality more optional, we just split existing features up:
the old "everything else" feature gets a dependency on the new optional feature, and the new "everything else" feature, which is correspondingly narrower.
For example, `everything-else-0` in the old version becomes `std`, `everything-else-1`, and `everything-else-0 = ["std", "everything-else-1"]`.
Importantly, there is no negative reasoning, or opting out, which avoids all the pitfalls of the previous solution.

Of course, this method also has some serious ergonomic drawbacks.
It would be easily to forget to create the "everything else" feature, and users who aren't familiar with the problem would see it as more pointless boilerplate.
Naming the "everything else" features is also a bit tedious.

Finally, a single minimal "everything else" feature isn't even enough to prepare oneself for all possible future split features.
As in the motivation, assume we have functions `foo` and `bar` gated on `foo-feature` and `bar-feature`, respectively.
Assume also we have a `baz` that requires `foo-feature` and `bar-feature`.
Now, later, we want to make a `baz-feature` that depends on the other two, and gates `baz` itself.
When the user migrates to the new versions, uses of `foo-feature` and `bar-feature` *alone* should still be fine;
neither of them were using `baz` and so there is no issue.
It's only the consumers of the *combination* of `foo-feature` and `bar-feature` that might be using `baz`, so only they should we conservatively insure also depend on `everything-else-0`.
But there is no way to express that:
Yes, `baz` was feature gated, but it being "more" than just the `foo-feature` and `bar-feature` bare minimum hits the same issue as ungated functionality being "more" than the empty crate.
The only way to be "future proof" is to ensure everything is gated on exactly one feature, but that could mean up to 2 ^ n "defensive" "everything-else"-like features!

Our actual plan for "migrations" works remarkably the same as this plan "underneath the hood".
It simply tries to increase the ergonomics by freeing the user from needing to preemptively and defensively create these extra features and name them.

## Migrations and compatibility don't mix!

A final short note.
Some might find the whole premise funny, because two versions of anything being "compatible" ought to be that no "migrations" are needed by definitions!
To avoid going on a tangent, my definition of "compatible" says anytime there is a single, canonical way to replace one component with another, they are compatible.
The single, canonical way doesn't need to be some notion of "do nothing".

# Prior art
[prior-art]: #prior-art

Database migrations is the obvious prior art.

Otherwise, I will reuse this section to talk about the underlyingmath.

Features depend on other features, and also we have sets of features but sets that respect those dependecies.
That means if `bar` depends on `foo`, it makes no sense to distinguish `[ "bar" ]` from `[ "foo" ]`.
What that means is that we have a ["free *join-semilattice* over a partial order"](https://ncatlab.org/nlab/show/semilattice#the_free_joinsemilattice_on_a_poset).
These lattices are actually bounded and distributive, which isn't structure we need to care about, but does make for a nicer "extruded hypercube" mental imagery as depicted in the images in https://en.wikipedia.org/wiki/Birkhoff%27s_representation_theorem, for anyone that rather imagine geometric shapes than algebraic machinations.

Mathematically, it's best to look at every crate's features as an independent mathematical construct.
When we say "don't remove features, keep the same names", what we are really doing is defining the "base migration" between each versions' features.
Mathematically, it that is a *homomorphism* between them.
What sort of homomorphism?
Because we are frequently mapping the empty feature set to something else, e.g. `[]` to `["std"]`,
We also don't care whether joins are mapped to joins, per the "foo bar baz" example where `["foo-feature", "bar-feature"]` became `["foo-feature", "bar-feature", "baz-feature"]`.
I think that means we just care about preserving the underling partial order (including unions, not the standalone feature dependency partial order we generated it the lattice from).
The type of "matching rules" system that is proposed does do that by being monotonic.

Note it is OK if the migration homomorphisms are not injective.
That would mean that some features are collapsed together, and we loose distinctions.
This violates the spirit of the feature distinction, but need not violate the letter of compatibility rules as long as everything from before in the crate interface is still available.

It's of a different, but related, sort than mentioned here, but https://arxiv.org/abs/2004.05688 also discusses the mathematical formalism of package management.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Syntax bikeshed, `all()` could be regular `cfg` syntax or something else.

- Maybe we just worry about migrating the no-feature case for now.

- Does it matter how the migrations from old versions change over time?
  E.g. how the migrations from 1.1 -> 1.2 -> 1.3 might not match the migrations from 1.1 -> 1.3 directly?
  (Note that multi-step migrations might be possible if Cargo combines depenency specs to avoid duplicate crates / public dep violations mid-solving.)
  The "path dependency" of migrating different ways could make for unexpected behavior as the solver tries different versions.

# Future possibilities
[future-possibilities]: #future-possibilities

Think about what the natural extension and evolution of your proposal would
be and how it would affect the language and project as a whole in a holistic
way. Try to use this section as a tool to more fully consider all possible
interactions with the project and language in your proposal.
Also consider how this all fits into the roadmap for the project
and of the relevant sub-team.

This is also a good place to "dump ideas", if they are out of scope for the
RFC you are writing but otherwise related.

If you have tried and cannot think of any future possibilities,
you may simply state that you cannot think of anything.

Note that having something written down in the future-possibilities section
is not a reason to accept the current or a future RFC; such notes should be
in the section on motivation or rationale in this or subsequent RFCs.
The section merely provides additional information.
