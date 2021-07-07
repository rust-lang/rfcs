- Feature Name: `cargo-feature-migrations`
- Start Date: 2021-07-01
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Extend Cargo to allow some limited forms of migrations of feature sets to be expressed.
This will allow adding new features to make existing functionality optional without causing needless breakage.

# Motivation
[motivation]: #motivation

## Problem

Today, Cargo's features most easily support a workflow where features added in new versions of a crate gate new functionality.

> Example: In a new version of a crate, someone creates a new function, and adds it behind a new feature.
> Neither the function nor feature existed in the old version of the crate.

The problem is there is another quite different use case of features: making previously mandatory functionality optional.

> Example: In a new version of a create, some creates a new feature for a function that *already* existed.
> Before, the export of that function was mandatory, but now it is optional because it can be disabled.

The workflow isn't supported so well: all Cargo offers for it is the notion of "default features", which isn't sufficient for reasons that will be elaborated below.

This second use-case is really important — in fact, perhaps more important.
The key thing aspect about features to realize is that the ultimate *reason* they are used is not to control a crate's interface (exports), but its *dependencies* (imports).
All things equal, no one minds if a crate provides some extra functionality they don't need.
That's harmless.
They do, however, mind if that extra functionality depends on another crate they cannot provide, e.g. because of platform considerations (running bare metal, etc.).
That means if someone adds new functionality that doesn't require new deps (case 1), there's little reason to bother gating it under new features.
Conversely, if someone wants to make a dep like `std` optional (case 2), there is a lot of benefit to add `std` feature to do so.

Let's return to Cargo's notion of "default features".
What is it for, and why is it not sufficient?
It's purpose is simple enough: if we create a new feature that gates existing functionality (again, case 2), existing consumers of the crate won't know about the new feature, so we need to provide them some avoidance to continue to receive that existing functionality without changing their "request".
Default features allow a new feature to be on by default, so the old functionality continues to work.
New consumers that do, in fact, want to avoid the new feature (and, moreover, avoid the newly-optional dependency obligations), can opt out of the default features so their feature list isn't embellished.

The problem is, what happens if we later have *another* feature making existing functionality optional?
Concretely, let's say:

- 1.0 has no features, and functions `foo` and `bar`
- 1.1 has a default feature `foo-feature` gating `foo`
- 1.2 has default features `foo-feature` and `bar-feature` gating `foo` and `bar`, respectively.

When we introduced 1.1, there were no default features to opt out of.
But when we introduce 1.2, there could be consumers *already* opting out of default features to skip `foo-feature` / `foo`.
How can we ensure those consumers nonetheless still get access to `bar`, while still allowing new consumers to take advantage of `bar-feature` to opt out of `bar`?
Too bad, we cannot.

## New motivation

This gap in Cargo's functionality has taken on new urgency with [RFC #3140](https://github.com/rust-lang/rfcs/pull/3140), which would propose a feature for the `alloc` crate of the latter sort, making existing functionality optional.
This would be the first example of a Cargo feature on a standard library crate slated for user consumption (as opposed to being some artifact of the current build system with no plans for stabilization).
If we want to eventually have stable user-facing features in library libraries, it's very important we have a good design so that we don't end up accidentally introduction breaking changes in the standard library.
If we fail and have an accidental breaking change, the damage will be immense because there is no way for users to opt out of the latest version!

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

I recommend first reading the [Rationale and alternatives](#rationale-and-alternatives) section.
I don't think this design is overwrought, but it does represent a new way of thinking about these sorts of issues that might feel unfamiliar.
I'm quite aware that "migrations" has negative connotations to many people stemming from their experiences with databases.

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
you mean

>I don't need any of the features that exist *as of version '1.1'*.

For users, that should be it!

For crate authors, yes, now the work of migrations comes in.
If in version 1.2 a `std` feature is added, then we need to say that users coming from 1.1 should have it enabled.
We can do it like this:
```toml
[feature-migrations."1.1"]
"all()" = [ "std" ]
```
Yes, that `all()` is a pretty obscure syntax.
It means the "empty intersection"; too bad I cannot use lists (of features) as TOML object keys.
It is supposed to match syntax I proposed in https://github.com/rust-lang/rfcs/pull/3143#issuecomment-868829430.
I am fine if we have some sugar for this common case.

For the more involved case from the discussion of alternatives, where we formerly had:
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
```toml
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
It can be hard to track down myriad crates and compel them to use quixotic `default-features = false` if they were happily working without.
Crate authors often might not appreciate the nagging either. 
Conversely if crates where compelled by build errors rather than humans to use `features = [ "std" ];"`, and compelled immediately rather than some time after they had shared create, I don't think they would find that nearly as annoying.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

1. `Cargo.toml` has a new section in the form `feature-migrations`
   The format is
   ```toml
   [feature-migrations."<version>"]
   "<feature-pseudo-cfg>" = "<feature-list>"
   ```
   where

    - `<version>` is a string containing a prior crate version
    - ```bnf
      <feature-pseudo-cfg> ::= <feature-name>
                            |  all(<possbily-empty-comma-separated-list-of-feature-names>)
      ```
2. When resolving features, use the migrations to extend the requested feature set.

   If the set of features on the left match the current set, *including feature's* dependencies,
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

The most popular proposal was some way to opt out of features, i.e. to say:

> Give me the default features *without* necessarily features `x` or `y`.

This does solve the problem: without users opting out of all features, just the ones they can name, there is no risk new features they will need being disabled.
However, it causes other problems.

First of all, this complicates the "additive" features mental model.
We've worked very hard to teach people how features are additive: it should always be safe to add more features.
We can't give that up across-the-board without destroying feature resolution, so it's important that "without necessarily features `x` or `y`" doesn't mean the crate will for sure not get `x` or `y`, but just that they are not requesting them.
I worry this will be very subtle and hard to teach.

Second of all, there is an especially unintuitive case of the former where a non-opted-out default feature depends on an opted-out feature.
Consider this example:

 - Consumer depends on "default features without `foo`".

 - `bar` is another default

 - `bar` depends on `foo`.

In this case, the not-mentioned feature still drags in the opted-out feature.
This might happen because the user simply forgot to include both features.
It might also happen because the problematic `bar -> foo` dependency only exists in a newer version, and not the one the user was using when they wrote the dependency spec.
Finally, it might be that that `bar` itself, and thus necessarily also the `bar -> foo` dependency, is newer than the dependency spec.
In any case, the result is that *all* build plans using a problematic version of the package have the explicitly opted-out feature `foo`, which is liable to confuse the user even more.

One might think there is a fix making the out-out a hard rejection, so such that no plan is allowed to have those features.
But that creates other problems, namely that those same sneaky new deps would rule out all plans causing solving to fail.
Moreover, this sort of "negative reasoning" undermines the entire "additive" comparability story Cargo features are supposed to have.
This will make the overall Cargo solving brittle, and also make it impossible to express the situation where one *is* in fact, agnostic to whether some unneeded feature is enabled for some other consumer's sake.<sup>[1](#assert-disabled)</sup>

Finally, and is a matter of taste, I find writing down features that I *don't* need poor UX.
We say "pay for what you use" in Rust, but writing down features that we, by definition, don't care about means cluttering our minds and `Cargo.toml`s.
I would only want to propose negative reasoning as an absolute last resort.

<a name="assert-disabled">1:</a> N.B. I do think is useful to be able assert some features aren't enabled. but I that such "global reasoning" should just be allowed in the workspace root, and not dependency crates, for sake of modularity / compositionality.

## Always at least one feature

In https://github.com/rust-lang/rfcs/pull/3140#issuecomment-862109208, @Nemo157 wrote:

> The way this is solved in other crates is to move everything behind a feature and make that default on.
> You can then in the future subset that feature as necessary, and make it activate those new sub-features.

To expound on that bit, the idea is that the empty feature set should always correspond to the empty crate.
A consequence of this is that "realistic" consumers will always opt into at least one feature.

This has a lot of nice properties.
First of all, we don't even need any notion of "default features" anymore for sake of compatability.
Since there is at least one feature in every prior release of a crate (that's non-empty), we simply prevent breaking changes by never removing old features.
When we want to make existing functionality more optional, we just split existing features up:

> Example: The old "everything else" feature gets a dependency on the new optional feature, and the new "everything else" feature, which is correspondingly narrower:
>
> - In code: `#![cfg(everything-else-0)]` attributes become either `#![cfg(std)]` or else `#![cfg(everything-else-1)]`
>
> - In `Cargo.toml`:
>   ```toml
>   [features]
>   std = []
>   everything-else-1 = []
>   everything-else-0 = ["std", "everything-else-1"]
>   ```

Importantly, there is no negative reasoning, or opting out, which avoids all the pitfalls of the previous solution.

Of course, this method also has some serious ergonomic drawbacks.
It would be easily to forget to create the "everything else" feature, and users who aren't familiar with the problem will scoff at what appears to be pointless boilerplate.
Naming these "everything else" features is also a bit tedious, whether one is familiar with the issue or not.

Finally, a single minimal "everything else" feature isn't even enough to prepare oneself for all possible futures where features could be partitioned.

> Example:
>
> Start as in the motivation
>
> - `Cargo.toml`:
>   ```toml
>   [features]
>   foo-feature = []
>   bar-feature = []
>   ```
>
> - Code:
>   ```rust
>   #[cfg(feature = "foo-feature")]
>   fn foo() { .. }
>   #[cfg(feature = "bar-feature")]
>   fn bar() { .. }
>   #[cfg(all(feature = "foo-feature", feature = "bar-feature"))]
>   fn baz() { .. }
>   ```
> Since all items are already non-trivially gated, we don't need any "everything else" feature.
>
> Now, later, we want to make a `baz-feature` that depends on the other two, and gates `baz` itself:
>
> - `Cargo.toml`:
>   ```toml
>   [features]
>   foo-feature = []
>   bar-feature = []
>   baz-feature = ["foo-feature", "bar-feature"]
>   ```
>
> - Code:
>   ```rust
>   #[cfg(feature = "foo-feature")]
>   fn foo() { .. }
>   #[cfg(feature = "bar-feature")]
>   fn bar() { .. }
>   #[cfg(feature = "baz-feature")]
>   fn baz() { .. }
>   ```

When the user migrates to the new versions, uses of `foo-feature` and `bar-feature` *alone* should still be fine;
neither of them were using the `baz` function so nothing goes wrong.
It's only the consumers of the *combination* of `foo-feature` and `bar-feature` that might be using `baz`, so only they need continue to depend additionally on `baz-feature`.
But there is no way to express that:
Yes, `baz` was feature gated all along, but it being "more" than the combination what either `foo-feature` or `bar-feature` each individually provide hits the same issue as ungated functionality being "more" than the empty crate interface.
The only way to be "future-proof" is to ensure everything is gated on exactly one feature---so making features like `foo-feature` "defensively" in advanced---but that could mean `O(2^n)` "defensive" "everything-else"-like features!

Our actual plan for "feature migrations" works remarkably the same as this plan "underneath the hood".
The difference is just about ergonomics, which is addresses by freeing the user from needing to preemptively and defensively create (and name) these extra features.

## Migrations and compatibility don't mix!

A final short note.
Some might find the whole premise funny, because two versions of anything being "compatible" ought to be that no "migrations" are needed by definitions!
To avoid going on a tangent, my definition of "compatible" says anytime there is a single, canonical way to replace one component with another, they are compatible.
The single, canonical way doesn't need to be some notion of "do nothing".

# Prior art
[prior-art]: #prior-art

Database migrations are the obvious prior art.

Otherwise, I will reuse this section to talk about the underlying math.
It is "prior" in a Platonic sense, at least :).

First, let us characterize basic features as they exist today.
We have individual features that depend on other features, and also sets of features.
Importantly, the features sets exist in a way that respects those dependecies.
Concretely, that means if `bar` depends on `foo`, it makes no sense to distinguish `[ "bar" ]` from `[ "bar" "foo" ]`.
What that means is that we have a ["free *meet-semilattice* over a partial order"](https://ncatlab.org/nlab/show/semilattice#the_free_joinsemilattice_on_a_poset).
(Nevermind the link saying "join" not "meet", the concepts are exactly symmetrical.)
These lattices are actually bounded and distributive, which isn't structure we need to care about, but does make for a nicer "extruded hypercube" mental imagery as depicted in the images in https://en.wikipedia.org/wiki/Birkhoff%27s_representation_theorem.
I mention this for sake anyone (like myself :)) that rather imagine geometric shapes than algebraic machinations.

Mathematically, it's best to look at every crate's features as an independent mathematical construct.
When we say "don't remove features, keep the same names", what we are really doing is defining the "base migration" between each versions' features.
Mathematically, it that is a *homomorphism* between them.
What sort of homomorphism?
Because we are frequently mapping the empty feature set to something else, e.g. `[]` to `["std"]`,
We also don't care whether meets are mapped to meets, per the "foo bar baz" example where `["foo-feature", "bar-feature"]` became `["foo-feature", "bar-feature", "baz-feature"]`.
Indeed it's rather important we allow that, because in some sense our migration is claiming the old crate version had the "wrong" meet.
I think that means we just care about preserving the underling partial order --- that is the partial order gotten by merely forgetting the lattice structure and still including the generated meets, and not the original standalone feature dependency partial order we generated the lattice from.
The type of "matching rules" system that is proposed preserve the partial order.
Preserving an order is otherwise known as being "monotonic".
Our migrations are monotonic because they only map feature sets to equal or larger sets that contain the old sets.

Back to the `["foo-feature", "bar-feature"]` to `["foo-feature", "bar-feature", "baz-feature"]` problem.
Recall for "at least one feature" alternative, we said that every item had to gated on not just at least one but exactly one feature
(though those features could have dependencies on one another).
What that meant mathematically was that the `cfg` for each item had to be [*meet-irreducible*](https://en.wikipedia.org/wiki/Birkhoff%27s_representation_theorem#The_partial_order_of_join-irreducible).
That is the precise criterion for when a crate is truly "future proof" today, absent the proposed new functionality.

Note it is OK if the migration homomorphisms are not injective.
That would mean that some features are collapsed together, and we loose distinctions.
This violates the spirit of the feature distinction, but need not violate the letter of compatibility rules as long as everything from before in the crate interface is still available.

For further reading, https://arxiv.org/abs/2004.05688 also discusses the mathematical formalization of package management.
The formalism it uses is not the same as this one, but it is related.

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

In https://github.com/rust-lang/rfcs/pull/3143#issuecomment-868829430 I gave another use-case for `"and(...)"` TOML keys that represent conjunctions of features.

Moreover, I think the mathematical framework based around order theory I used when designing this feature will be useful when considering the design of other parts of Cargo and it's interaction with Rust.
Longstanding unimplemented features like:

- Compatibility testing (https://github.com/rust-lang/cargo/issues/5500)

- The portability lint (https://github.com/rust-lang/rfcs/blob/master/text/1868-portability-lint.md)

- Public and private dependencies (https://github.com/rust-lang/rfcs/blob/master/text/1977-public-private-dependencies.md)

are all challenging problems that are amendable to this sort of analysis.
I am less concerned with whatever "bells and whistles" Cargo has, than that there is some sort of holistic approach for both deciding on a fundamental level what we want from Cargo, and what core functionality should exist in service of that goal.
I think this sort of theory can help, and this is a far less costly and more urgent problem than those other three with which to evaluate it.
