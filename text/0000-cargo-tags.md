- Feature Name: (fill me in with a unique ident, `my_awesome_feature`)
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Categories and keywords for crates are now deprecated, replaced by a unified "tags" instead. A new set of policies is added to allow the crates.io team to curate the way tags are presented, replacing features such as the "Popular Categories" list on crates.io with a "Popular Tags" instead.

# Motivation
[motivation]: #motivation

Currently, Cargo supports two types of metadata when tagging the purpose/scope of a crate:

1. Categories, which are restricted to a curated set of allowed categories.
2. Keywords, which are unrestricted and user-defined.

However, the reality is that the actual curation of categories is extremely loose, and the only process for adding a category is to open a PR for the crates.io repository. The required review is effectively asking for an "LGTM" reply from one of the crates.io team members, as seen in [the history for the `categories.toml` file on the repository](https://github.com/rust-lang/crates.io/commits/main/src/boot/categories.toml).

Ultimately, the most important distinction between categories and keywords has really been that the crates.io team has control over the presentation of categories, although a more important distinction is that categories cannot be removed, only added.

Recently, [a discussion](https://github.com/rust-lang/crates.io/discussions/6762) was opened in the crates.io repository on whether the cryptocurrencies category should be removed from crates.io, due to the plethora of issues surrounding them. This should not be treated as a reason for adopting the RFC (although it was a motivation to write it), but instead as something that brought up the fact that categories cannot be removed by policy.

Because categories cannot be removed, they also cannot be renamed or otherwise curated by the community. However, by switching to keywords, we can effectively solve this problem; community members can simply start publishing their crates under different keywords and older, unsupported crates wouldn't have any issue remaining published under their older versions.

To make the change clearer, the unified categories and keywords are called "tags" instead, and any crates using keywords or categories in their manifests will implicitly add those strings to tags instead. Ultimately, switching to a unified, unrestricted system solves a couple problems:

1. Instead of being gatekept by a PR to the crates.io repository, categories can organically be adopted by community members in the form of keywords. Which keywords are most popular and useful can be decided organically.
2. Keywords can still be given descriptions and other metadata on crates.io, although no distinction between these "special" keywords and other keywords is made in cargo itself. This allows making changes to the way crates are presented without having to worry about backwards compatibility.
3. Adding, removing, and modifying the curated set of keywords is no longer a technical choice, but a cultural one.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Categories and keywords in `Cargo.toml` are now deprecated. Setting `categories` or `keywords` will now trigger a warning and suggest to use `tags` instead. These combined `tags` now accept the formats used by both `categories` and `keywords`, meaning:

* It's now possible to add a single double-colon inside a tag to indicate a parent tag. This has the effect of adding two tags to a crate: for example, adding the `development-tools::testing` tag adds the crate to both the `development-tools` tag and the `development-tools::testing` tag. The part after the double-colon is called a subtag.
* On each side of the double-colon, the length of text may be up to 25 characters. That means that, including the double-colon, tags can be up to 52 characters long. (This is to accomodate the largest category before the unification, `development-tools::procedural-macro-helpers`, although rounding up to a nice number for the actual limit.)

On crates.io, the "categories" section of the sidebar is removed and all tags are shown with hashtags in a crate header, like keywords are now. On the pages for popular tags, a curated description from the crates.io team may be shown alongside a list of common subtags and similar tags.

The exact policy for how this curated metadata can be added to tags can be modified by the crates.io team without an RFC, although an initial policy is described below in the reference-level explanation.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The `Cargo.toml` manifest gains a new key, `package.tags`, which is an array of strings with the following restrictions:

* The strings are nonempty and consist of "tag components" separated by double-colons as specified below. While crates.io limits the number of components to at most two, this can be raised for custom registries.
* Tag components must start with a letter and only contain letters, numbers, `_`, or `-`. (This is the same as keywords and crate names.)
* crates.io limits the length of tag components to 25 characters and the number of tags to 10, but this can be raised for custom registries.

The `package.categories` and `package.keywords` keys are now deprecated, and future versions of Cargo emit a warning whenever they are used. If either of these keys is present, their contents are implicitly treated as tags and count toward the ten-tag limit.

## Adding tag metadata

This policy can be changed at the discretion of the crates.io team without an RFC. It's really just a starting set of guidelines.

crates.io can now add metadata to tags according to this policy. Immediately after the adoption of this RFC, this would likely start by converting the `categories.toml` configuration into a `tags.toml` configuration, with PRs affecting this file subject to the policy.

Tags with the same name as a crate may be automatically marked as "crate tags" if the majority of the crates with the tag have the tagged crate as a non-development dependency, optional or otherwise. At the discretion of the crates.io team, tags may be explicitly marked as non-crate tags, to account for cases of popular crates with generic names. This can help avoid marking a particular crate as "canonical" just because it's popular and has a generic name.

Tags that are deemed "noteworthy" can have metadata added to them. A tag is noteworthy if it:

1. Does not violate the Rust Code of Conduct in any way. (Extra scrutiny is given to tags compared to individual crates, since they reflect on the values of the Rust project more broadly.)
2. Is not a crate tag. (These use the crate for their metadata instead.)
3. Has at least five "noteworthy" crates from separate authors. (Intentionally vague to allow unpopular but important tags to gain metadata.)
4. Serve a purpose that is not covered by an existing tag. (For these cases, subtags are more appropriate. A good metric for this would be whether the five noteworthy crates share a second tag in common that itself is.)
5. Could feasibly have at least one subtag that is noteworthy. ("Feasibly" is also left highly subject to interpretation. The idea here is to ensure that these are not best replaced with subtags.)

For determining the noteworthiness of subtags, the conditions are slightly altered:

* They only have to serve a purpose that isn't covered by other subtags, instead of any tag.
* They don't even feasibly need to have noteworthy subtags, since nested subtags aren't allowed.

When determining noteworthiness, similar tags can be taken into account. For example, if `math`, `maths`, and `mathematics` are used as tags, but the community hasn't yet decided upon a canonical tag, one can be chosen arbitrarily and metadata can be added to it instead.

Metadata can take one of the following forms:

* A short description can be added to the tag.
* A list of similar tags can be added, grouping together crates under all these tags in the same list.

Additionally, tags that are seen as noteworthy are allowed to be shown on the "popular tags" list on the crates.io home page, or displayed prominently above other tags on crate pages.

These guidelines are meant to provide a *minimum* requirement to add a noteworthy tag, but meeting these requirements does not ensure that a tag is added. These tags may still be vetoed at the discretion of the crates.io team, and this may also result in changes to the policy.

Since this is a big cultural change to the way crates are presented, it's going to take a bit of experimentation and time to fully refine these guidelines. This is why the crates.io team should be allowed to change these guidelines as they see fit without a full RFC, since ultimately, they're the ones in charge of actually responding to changes to tag metadata and they can only do what they have the capacity to do.

# Drawbacks
[drawbacks]: #drawbacks

The biggest drawback is that this can potentially add substantial extra work for the crates.io team, since there could potentially be several more tags with metadata than existing categories.

However, the requirements here aren't substantially different than those expected of users proposing categories before, just more clarified. Arguably, the onus is more on the users proposing tags than the crates.io members accepting metadata changes, since it's substantially more difficult to make a case than to accept or reject one.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

One potential alternative would be to allow arbitrary categories from users, but only show metadata for categories that are known to crates.io. However, this would be extremely similar to the existing proposal, with the added confusion of keywords just being a worse version of categories. Unifying these two seems like the best choice for any proposal of this form.

The likely biggest proposal would be to retain the status quo and simply keep everything as-is. The biggest problem with this is the lack of a proper specification for when categories can be added, in addition to the fact that categories can't be removed. Additionally, the lack of nesting inside keywords means that users actually can't create their own categories outside of what crates.io offers, which means that these curated categories always will be needed. Ultimately, this distinction is trying to explicitly separate technical and cultural tagging of crates, which seems bad from a technical perspective. But what do I know?

# Prior art
[prior-art]: #prior-art

Looking at other package repositories:

* [PyPI](https://pypi.org) offers curated [classifiers](https://pypi.org/classifiers) without metadata. They don't seem to have an official policy on adding new classifiers, having instead taken the original list [from external sources](https://peps.python.org/pep-0301/#distutils-trove-classification).
* [Hackage](https://hackage.haskell.org) has both keywords and uncurated categories without any metadata.
* [NPM](https://www.npmjs.com) has keywords.
* [pub.dev](https://pub.dev) has keywords under the name "topics".
* [Maven Central](https://mvnrepository.com/repos/central), [NuGet](https://www.nuget.org), [Packagist](https://packagist.org) have keywords under the name "tags".
* [Dub](https://code.dlang.org), [Go](https://pkg.go.dev), [RubyGems](https://rubygems.org) have no keywords or categories.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None currently.

# Future possibilities
[future-possibilities]: #future-possibilities

It seems unlikely that tagging functionality could be extended beyond the points this RFC suggests. Given the fact that existing package repositories take a rather lax mentality toward metadata, arguably, having metadata for arbitrary tags at all is going above and beyond the status quo.

It's probably not a best idea to expand the scope of what's offered.
