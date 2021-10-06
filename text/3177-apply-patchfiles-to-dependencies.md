- Feature Name: Apply patchfiles to dependencies
- Start Date: 2021-09-16
- RFC PR: [rust-lang/rfcs#3177](https://github.com/rust-lang/rfcs/pull/3177)
- Rust Issue: [rust-lang/rust#4648](https://github.com/rust-lang/cargo/issues/4648)

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
providing a way to apply a patchfile that only changes the libraryA dependency of the features crates, 
the amount of "private" forks can be reduced.

The expected outcome of this example is that the user need not copy the sources of the feature crates
to fix an issue in libraryA, as well as preventing users of the library to simply vendor the selected
libraries and fix this issue locally. 

It is not expected that crates are allowed to be published to crates.io with patchfiles, 
but to be used while doing development. 


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

> Explain the proposal as if it was already included in the language and you were teaching it to another Rust programmer. That generally means:

> - Introducing new named concepts.
> - Explaining the feature largely in terms of examples.
> - Explaining how Rust programmers should *think* about the feature, and how it should impact the way they use Rust. It should explain the impact as concretely as possible.
> - If applicable, provide sample error messages, deprecation warnings, or migration guidance.
> - If applicable, describe the differences between teaching this to existing Rust programmers and new Rust programmers.

> For implementation-oriented RFCs (e.g. for compiler internals), this section should focus on how compiler contributors should think about the change, and give examples of its concrete impact. For policy RFCs, this section should provide an example-driven introduction to the policy, and explain its impact in concrete terms.

The idea behind this is to centralize software belonging to a specific crate more, instead of requiring the user to create multiple git repositories for the
changes to libraries, those changes can in the cases where it makes sense, be kept to within the project itself in the form of unidiff patchfiles, providing an 
easier way to get an overview of what a specific project changes.

One example where multiple uneccesarry git-repositories ( git repositories whose changes is never useful for anyone else ) is illustrated below. 

## Patch dependencies of dependencies

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
feature addition to foo that you want. You add this to your Cargo.toml file like this:


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
now depends on your fixed version of `foo`, and that is the only change.

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
bar = "2.0"

[patch.crates-io]
foo = { path = "../path/to/foo" }
bar = { version = "2.0", patchfiles = ["patches/bar-update-foo-dependency.patch"] }
```

Voila! You no longer need to keep that cloned source of `bar` that you barely even touched. Upon running
`cargo build` the bar dependency will be downloaded, the patch will be applied to the source of `bar` and
it will utilize your changed `foo` library in this case.

The `version` field would specify which versions of `bar` to try and apply the patch to, and each patchfile will be applied in the order they are 
defined upon that original version. If no `version` is defined, it will default to what is in the the git or path supplied, and if none is supplied it will default to "*"

## Backport bugfixes into project

This same method can also be used to backport important bugfixes to old versions of dependencies that you do not have the time
to upgrade to, since they include way to many breaking changes and additional features, just updating the software to the newest version is 
not a priority right now. You can now instead just get the specific bugfix included in your software for what you are maintaining. 

``` bash
git clone <sourcecode>
git checkout <old-version-hash>
git cherry-pick <bugfix-hash>
git diff HEAD~1 > fix-foobarize-bug.patch
```

And then update your dependency to apply the patch you just created onto that old version of the
dependency.

``` toml
# your crate
[dependencies]
# apply the created patchfile to the legacy version of foo, that the maintainer do not wish to fix.
foo = "1.0"

[patch.crates-io]
foo = { path = "../path/to/foo", patchfiles = ["patches/fix-foobarize-bug.patch"]  }
```

It could also be used to change the behavior of a dependency in a way that is only useful for
your application and you know would never be merged into the original software. The patchfiles will be applied
after `git` or `path` has been resolved if there are any present. Otherwise since patchfiles do not contain any version information, the `version` tag needs
to be present. 

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

> This is the technical portion of the RFC. Explain the design in sufficient detail that:

> - Its interaction with other features is clear.
> - It is reasonably clear how the feature would be implemented.
> - Corner cases are dissected by example.

> The section should return to the examples given in the previous section, and explain more fully how the detailed proposal makes those examples work.

There was initial work created in PR: 

The Cargo `[patch]` section gets information about which source to patch from the Cargo.toml inside the `path` or `git` tags. Since a patchfile can change this information, 
and do not contain this information in itself, then either the version to patch needs to be gathered from a `path` or `git` tag, but if none of those are supplied, the 
`version` tag _must_ be specified. The 'version' tag will in this RFC will not interfere with the workings of how versions were previously handled. But is suggested in 
future work. 

To maintain compatibility with how patches applied previously and to limit the scope of this RFC, the added `version` key in the `[patch]` section will only be allowed 
and required when no `path` or `git` exists.

The feature would add an extra step to the build process, right after the download of the crate from
crates.io, path or git. 

It would apply the patch from the patchfiles in the order they are defined in the patches section of
the dependency declaration, if a patch does not apply cleanly, it would result in an compilation error
that shows which patch that did not apply cleanly to which library. This would make sure that patchfiles are not to dangerous. 

The patch apply logic should be written as a rust code specifically for this, either as a library or as part of the code. This to make sure that 
the patch apply logic is not to lenient or tries to smart things. The reason for this is to ensure and control how much patchfiles can affect. As well as to implement
logic that makes sure that Cargo is not trying to patch files outside of the working directory since the unidiff format specifies which files to diff. 

Once patched Cargo would check if the dependency graphs need to be rebuilt ( Should be possible by checking for modified Cargo.toml files from the previous run )

The resulting crate after patches would be evaluated and new dependencies would be downloaded if needed, if the new dependency tree contains dependencies that should be 
patched, but was not previously, we would have to rerun the patch apply on top of those new dependencies, and again refresh the dependency trees. Since this would be a 
recursive function, a maximum amount of recursion is needed here to not end in an infinite patchloop.

4. The software is built


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

Currently, the copying of sources to apply miniscule changes, that is not very useful for anyone else, can lead to sources being copied, 
and to not maintain the multitudes of sources trivially modified, sometimes the entire software trees are just vendored to save time and 
allow the local-development to continue, similar to how old-school C projects would do it.

Patchfiles have a standardised formatting called unidiff, with a lot of tools and developers knowing both how to use
and read them, and could easily apply patches from multiple merge requests without having to create a
new repository that combines all of those merge requests. Those changes are also commonly shared in mailing lists, and can also be used to 
try out multiple different changes at the same time without creating a git branch, and the quite advanced merging that can occur in such cases.

The need for forking a library where the developer has been unreachable for a long time is reduced, since the
patches can be shared until the original developer has time to merge them into the original code. There are a few examples of this happening in the
rust ecosystem, where the divergence of the fork and the original means that both libraries suffer in the end.

Not allowing patchfiles leads to copying of sources codes, and this practice sometimes creates forks that
might get out of touch with the ecosystem version, either by falling behind, or by implementing features on a copy
of the source code that does not contain the git history, and thus would be problematic to integrate into the ecosystem again. 
Maintaining a set of patchfiles locally in the project repository would reduce the friction of getting them into the community.

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

This would automate a lot of the process that has been used in mailing lists, instead of sharing the entire modified code, only share the modifications, 
so that anyone can try them, and mix and match them locally when revieving a multitude of different patches together.

It is also a common process of other major software projects such as Yocto, Buildroot and OpenWrt, where patches can be used to make software 
play nice with each other for specific usecases or projects. However, as a first step in this way, cargofiles that contain patchfiles should not be allowed
onto crates.io, until the ramifications of such a feature addition has been more tested, and more understood in the rust community setting.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

> - What parts of the design do you expect to resolve through the RFC process before this gets merged?
> - What parts of the design do you expect to resolve through the implementation of this feature before stabilization?
> - What related issues do you consider out of scope for this RFC that could be addressed in the future independently of the solution that comes out of this RFC?

This RFC is to discuss how the implementation should look like, and how to resolve dependency graph updates that would happen as a consequence of the Cargo.toml 
dependency tree changing dynamically through the patching sequence. 

However, it is considered out of scope for this RFC to solve all of those edge-cases, since not allowing these patchfiles to be included in crates that are 
published to crates.io would place these issues locally, and not affect users.

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

With the addition of the `version` key in the patches section, one can more clearly specify on which the version something should be patched, a future RFC could allow that 
key on top of the `path` or `git` keys in the patch section, and that would override what is found when resolving the `path` or `git` sources. It would make the process 
more intuitive for users trying to use the patch section in general.

Future possibilites related to this are mostly tooling, such as Cargo addon softwares that automates the patchfile creation and Cargo project file manipulations would help
users to utilize this featur. 
