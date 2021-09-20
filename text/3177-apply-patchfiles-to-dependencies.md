- Feature Name: Apply patchfiles to dependencies
- Start Date: 2021-09-16
- RFC PR: [rust-lang/rfcs#3119](https://github.com/rust-lang/rfcs/pull/3119)
- Rust Issue: [rust-lang/rust#88867](https://github.com/rust-lang/rust/issues/88867)

# Summary
[summary]: #summary

Propose to add an ability for Cargo to patch dependencies with patch files.

# Motivation
[motivation]: #motivation

> Why are we doing this? What use cases does it support? What is the expected outcome?

The addition of the [patch] manifest section has made making localised changes to crates used by an
individual project much easier. However, this still requires checking out the source code for the entire
crate even if only a few lines of the source actually need to be modified.

This approach is fairly reasonable for individual Rust projects, however when embedded in a much larger
build system that pulls hundreds of various projects together this can quickly become unwieldy (e.g.
Buildroot, OpenWrt build system, etc). Rather than storing the entire source for any packages requiring
modification, these build system instead store a set of patch files against a specific release version for
the package in question.

This would additionally provide users with a way to update dependencies of dependencies, in the case of
a dependency that also is a dependency of other crates, for example:

```
   my-crate -> libraryA
            -> libraryA_feature1 -> libraryA
            -> libraryA_feature2 -> libraryA
```

Patching libraryA sometimes mean that libraryA_feature1 and libraryA_feature2 also need to be
updated to point to the modified code of LibraryA, especially if sharing traits or structures. By
providing a way to apply a patchfile that only changes the libraryA dependency of the features crates.

The expected outcome of this example is that the user need not copy the sources of the feature crates
to fix an issue in libraryA, as well as preventing users of the library to simply vendor the selected
libraries and fix this issue locally.


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

> Explain the proposal as if it was already included in the language and you were teaching it to another Rust programmer. That generally means:

> - Introducing new named concepts.
> - Explaining the feature largely in terms of examples.
> - Explaining how Rust programmers should *think* about the feature, and how it should impact the way they use Rust. It should explain the impact as concretely as possible.
> - If applicable, provide sample error messages, deprecation warnings, or migration guidance.
> - If applicable, describe the differences between teaching this to existing Rust programmers and new Rust programmers.

> For implementation-oriented RFCs (e.g. for compiler internals), this section should focus on how compiler contributors should think about the change, and give examples of its concrete impact. For policy RFCs, this section should provide an example-driven introduction to the policy, and explain its impact in concrete terms.

You are using a library called `foo` while developing a Rust software, while doing so you find a bug. You decide to
try and fix it, first you obtain the source code by cloning the repository. After fixing the bug you want
to try it out in your software.

for this you use the [patch] section of the Cargo.toml file to override the dependency version you have previously used, with the fixed version you have locally as this:

``` toml
# your crate
[dependencies]
foo = "1.0"

[patch.crates-io]
foo = { path = "../path/to/foo" }
```
It works flawlessly! You continue to develop and come upon a nice library called bar that contains a
feature you want. You add this to your Carg.toml file like this:


``` toml
# your crate
[dependencies]
foo = "1.0"
bar = "2.0"

[patch.crates-io]
foo = { path = "../path/to/foo" }
```
But `bar` itself depends on the library `foo`, and even re-exports some of the `foo` components! Suddenly
you have two structs that are identical, but come from different versions of `foo`, your locally patched
one, and the one that `bar` depends on. To fix this, you decide to download `bar` and change so that `bar`
now patches its version of `foo`, and that is the only change.

``` toml
# bar
[dependencies]
foo = "1.0"

[patch.crates-io]
foo = { path = "../path/to/foo" }
```

In your own Cargo.toml file you now also need to patch the version of `bar` like this:

``` toml
# your crate
[dependencies]
foo = "1.0"
bar = "2.0"

[patch.crates-io]
foo = { path = "../path/to/foo" }
bar = { path = "../path/to/bar" }
```

This works! Now they both depend on the same version of `foo`, but you had to clone and keep the source
of `bar`, even though nothing really changed in `bar`. You quickly realize that you want to add more
libraries, but they also depend on `foo`!, this would mean keeping all of those sources cloned somewhere,
as well as when they receive updates, actually update them, and apply your change there as well.

Luckily you can use patchfiles to mitigate this issue. Your go into your modified `bar` folder and run
the following command:

```
git diff > bar-update-foo-dependency.patch
```

You copy that file into your project directory under a folder called `patches` to keep things tidy.
In your Cargo.toml you now enter:

``` toml
# your crate
[dependencies]
foo = "1.0"
bar = { version =  "2.0", patches = ["patches/bar-update-foo-dependency.patch"] }

[patch.crates-io]
foo = { path = "../path/to/foo" }
```

Voila! You no longer need to keep that source of `bar` that you barely even touched. Upon running
`cargo build` the bar dependency will be downloaded, the patch will be applied to the source of `bar` and
it will utilize your changed library in this case.

This same method can also be used to backport important bugfixes to old versions of dependencies that you do not have the time
to upgrade to, since they include way to many breaking changes. Where you would fetch the source, use something similar to:

``` bash
git clone <sourcecode>
git checkout <old-version-hash>
git cherry-pick <bugfix-hash>
git diff HEAD~1 > fix-foobarize-bug.patch
```

And then update your dependency to apply the patch you just created onto that old version of the
dependency

It could also be used to change the behavior of a dependency in a way that is only useful for
your application and you know would never be merged into the original software.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

> This is the technical portion of the RFC. Explain the design in sufficient detail that:

> - Its interaction with other features is clear.
> - It is reasonably clear how the feature would be implemented.
> - Corner cases are dissected by example.

> The section should return to the examples given in the previous section, and explain more fully how the detailed proposal makes those examples work.

1. The feature would add an extra step to the build process, right after the download of the crate from
crates.io

2. It would apply the patch from the patchfiles in the order they are defined in the patches section of
the dependency declaration, if a patch does not apply cleanly, it would result in an compilation error
that shows which patch that did not apply cleanly

3. The resulting crate after patches would be evaluated and new dependencies would be downloaded if needed

4. the software is built


# Drawbacks
[drawbacks]: #drawbacks

> Why should we *not* do this?

 - The patches could become unruly, complicate the dependency tree in ways that are hard to predict
 - Instead of fixing the software, developers might stop at just implementing the patchfile and then not care
 - A patch could in theory be applied twice, if the dependency is updated, but the patchfiles are still used
 - Patching libraries, that patches libraries can create interesting dependency problems that are hard to
 debug


# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

> - Why is this design the best in the space of possible designs?
> - What other designs have been considered and what is the rationale for not choosing them?
> - What is the impact of not doing this?

Currently, the copying of sources to apply changes can lead to sources being copied, and to not maintain the
multitudes of sources trivially modified, sometimes the entire software trees are just vendored, similar to
how old-school C projects would do it.

Patchfiles have a standardised formatting, with a lot of tools and developers knowing both how to use
and read them, and could easily apply patches from multiple merge requests without having to create a
new repository that combines all of those merge requests.

The need for forking a library where the developer has been unreachable for a long time is reduced, since the
patches can be shared until the original developer has time to merge them into the original code.

Not allowing patchfiles leads to copying of sources codes, and this practice sometimes creates forks that
might get out of touch with the ecosystem version, either by falling behind, or by implementing features on a copy
of the source code that does not contain the git history, and thus would be problematic to merge back into the upstream

# Prior art
[prior-art]: #prior-art

> Discuss prior art, both the good and the bad, in relation to this proposal.
> A few examples of what this can include are:

> - For language, library, cargo, tools, and compiler proposals: Does this feature exist in other programming languages and what experience have their community had?
> - For community proposals: Is this done by some other community and what were their experiences with it?
> - For other teams: What lessons can we learn from what other communities have done here?
> - Papers: Are there any published papers or great posts that discuss this? If you have some relevant papers to refer to, this can serve as a more detailed theoretical background.

> This section is intended to encourage you as an author to think about the lessons from other languages, provide readers of your RFC with a fuller picture.
> If there is no prior art, that is fine - your ideas are interesting to us whether they are brand new or if it is an adaptation from other languages.

> Note that while precedent set by other languages is some motivation, it does not on its own motivate an RFC.
> Please also take into consideration that rust sometimes intentionally diverges from common language features.

This would automate a lot of the process that has been used in mailing lists, instead of sharing the entire modified code, only share the modifications, so that anyone can implement them.

It is also a common process of other major software projects such as Yocto, Buildroot and OpenWrt, where patches can be used to make software play nice with each other

# Unresolved questions
[unresolved-questions]: #unresolved-questions

> - What parts of the design do you expect to resolve through the RFC process before this gets merged?
> - What parts of the design do you expect to resolve through the implementation of this feature before stabilization?
> - What related issues do you consider out of scope for this RFC that could be addressed in the future independently of the solution that comes out of this RFC?


* TODO *

# Future possibilities
[future-possibilities]: #future-possibilities

> Think about what the natural extension and evolution of your proposal would
> be and how it would affect the language and project as a whole in a holistic
> way. Try to use this section as a tool to more fully consider all possible
> interactions with the project and language in your proposal.
> Also consider how this all fits into the roadmap for the project
> and of the relevant sub-team.

> This is also a good place to "dump ideas", if they are out of scope for the
> RFC you are writing but otherwise related.

> If you have tried and cannot think of any future possibilities,
> you may simply state that you cannot think of anything.

> Note that having something written down in the future-possibilities section
> is not a reason to accept the current or a future RFC; such notes should be
> in the section on motivation or rationale in this or subsequent RFCs.
> The section merely provides additional information.


* TODO *
