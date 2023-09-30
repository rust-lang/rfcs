- Feature Name: (fill me in with a unique ident, `my_awesome_feature`)
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Categories for crates are now deprecated and implicitly added as keywords instead. A new set of policies is added to allow the crates.io team to curate the way keywords are presented, replacing features such as the "Popular Categories" list on crates.io with a "Popular Keywords" instead.

# Motivation
[motivation]: #motivation

Currently, Cargo supports two types of metadata when tagging the purpose/scope of a crate:

1. Categories, which are restricted to a curated set of allowed categories.
2. Keywords, which are unrestricted and user-defined.

However, the reality is that the actual curation of categories is extremely loose, and the only process for adding a category is to open a PR for the crates.io repository. The required review is effectively asking for an "LGTM" reply from one of the crates.io team members, as seen in [the history for the `categories.toml` file on the repository](https://github.com/rust-lang/crates.io/commits/main/src/boot/categories.toml).

Ultimately, the most important distinction between categories and keywords has really been that the crates.io team has control over the presentation of categories, although a more important distinction is that categories cannot be removed, only added.

Recently, [a discussion](https://github.com/rust-lang/crates.io/discussions/6762) was opened in the crates.io repository on whether the cryptocurrencies category should be removed from crates.io, due to the plethora of issues surrounding them. This should not be treated as a reason for adopting the RFC (although it was a motivation to write it), but instead as something that brought up the fact that categories cannot be removed by policy.

Because categories cannot be removed, they also cannot be renamed or otherwise curated by the community. However, by switching to keywords, we can effectively solve this problem; community members can simply start publishing their crates under different keywords and older, unsupported crates wouldn't have any issue remaining published under their older versions.

To make the change backwards-compatible, any crates using categories in their manifests will implicitly add those strings to keywords instead. Ultimately, switching to a unified system solves a couple problems:

1. Instead of being gatekept by a PR to the crates.io repository, categories can organically be adopted by community members in the form of keywords. Which keywords are most popular and useful can be decided organically.
2. Keywords can still be given descriptions and other metadata on crates.io, although no distinction between these "special" keywords and other keywords is made in cargo itself. This allows making changes to the way crates are presented without having to worry about backwards compatibility.
3. Adding, removing, and modifying the curated set of keywords is no longer a technical choice, but a cultural one.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Categories in `Cargo.toml` are now deprecated. Setting `categories` will now trigger a warning and suggest to use `keywords` instead. To ensure that crates still build correctly in these cases, the provided `categories` are implicitly converted into `keywords`.

As part of this conversion, both the parent and child categories are added as keywords, meaning that for example the `development-tools::testing` category gets converted into a `development-tools` keyword and a `testing` keyword. When computing the number of keywords used, only unique keywords are added, meaning that the following manifest contains only three keywords:

```toml
[package]
categories = ["development-tools::testing"]
keywords = ["testing", "tests"]
```

To accomodate this change, the number of keywords allowed on `crates.io` is increased to `15`; this allows for 5 parent categories, 5 child categories, and 5 pre-existing keywords. External registries are free to change this limit as they please, just like before.

Keywords also have their length limit raised to 25 characters, which accomodates the previous largest category, `procedural-macro-helpers`.

On crates.io, the "categories" section of the sidebar is removed and all keywords are shown with hashtags in a crate header, like they are currently. On the pages for popular keywords, a curated description from the crates.io team may be shown alongside a list of aliases and commonly paired keywords. Aliases are special because they allow the crates.io team to implicitly include multiple keywords in the same list, for example, including both crates with `tests` and `testing` keywords in listings for either keyword.

The exact policy for how this curated metadata can be added to keywords can be modified by the crates.io team without an RFC, although an initial policy is described below in the reference-level explanation.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Currently, Cargo does not place any limits on the contents or quantity of categories and keywords and instead relies on registries (like crates.io) to perform validation; this will not change. To accomodate older external registries, Cargo will continue to send categories and keywords separately if they are both provided and rely on registries to merge them together. Regardless of whether the registry treats categories as special, Cargo should emit a warning if a user provides categories, encouraging them to switch to keywords; this may eventually have the option to `cargo fix` to automatically convert the categories to keywords.

To allow simpler code, crates.io may eventually decide to stop validating categories specially and implicitly convert all provided categories into keywords. The actual requirements for the format of keywords will not change, although their length limit will be increased to 25 characters. Additionally, the number of allowed keywords will be increased to 15 to accomodate the lack of categories, as well as the implicit conversion of categories into keywords.

## Adding keyword metadata

This policy can be changed at the discretion of the crates.io team without an RFC. It's really just a starting set of guidelines.

crates.io can now add metadata to keywords according to this policy. Immediately after the adoption of this RFC, this would likely start by converting the `categories.toml` configuration into a `keywords.toml` configuration, with PRs affecting this file subject to the policy.

Keywords with the same name as a crate (for example, `serde` or `tokio`) may be automatically marked as "crate keywords" if the majority of the crates with the keywords have the crate with the same name as the keyword as a non-development dependency, optional or otherwise. At the discretion of the crates.io team, keywords may be explicitly marked as non-crate keywords, to account for cases of popular crates with generic names (for example, `sha2`). This can help avoid marking a particular crate as "canonical" just because it's popular and has a generic name.

Non-crate keywords can have the following metadata:
* A description of what the keyword is for.
* A list of "commonly paired" keywords, which operate similar to subcategories. (Unlike subcategories, a keyword can be commonly paired with more than one parent keyword.)
* A list of alias keywords, whose listings are merged with the parent keyword. (If "tests" is an alias of "testing," then any listing for "test" will redirect to that of "testing," and all crates with the keyword "test" will be shown under the "testing" list.)

Keywords that are deemed "noteworthy" can have metadata added to them. A keyword is noteworthy if it:

1. Does not violate the Rust Code of Conduct in any way. (Extra scrutiny is given to keywords compared to individual crates, since they reflect on the values of the Rust project more broadly.)
2. Is not a crate keyword. (These use the crate for their metadata instead.)
3. Has at least five "noteworthy" crates from separate authors. (Intentionally vague to allow unpopular but important keywords to gain metadata.)
4. Serve a purpose that is not covered by an existing noteworthy keyword. (For these cases, marking the keyword as a "commonly paired" keyword might be more appropriate, as described below. A good metric for this would be whether the five noteworthy crates share a second keyword in common that itself is noteworthy.)
5. Could feasibly have at least one commonly paired keyword that is noteworthy. ("Feasibly" is also left highly subject to interpretation. The idea here is to ensure that these are not best replaced with pairings.)

For determining the noteworthiness of commonly paired keywords, the requirements are slightly reduced:

* They only have to serve a purpose that isn't covered by other pairings, instead of any keyword. (In this case, an alias might be more appropriate.)
* They don't need to have noteworthy pairings themselves.

When determining noteworthiness, potential aliases can be taken into account. For example, if `math`, `maths`, and `mathematics` are used as keywords, but the community hasn't yet decided upon a canonical keyword, one can be chosen arbitrarily and metadata can be added to it instead.

Additionally, keywords that are seen as noteworthy are allowed to be shown on the "popular keywords" list on the crates.io home page, or displayed prominently above other keywords on crate pages.

These guidelines are meant to provide a *minimum* requirement to add a noteworthy keyword, but meeting these requirements does not ensure that a keyword is added. These keywords may still be vetoed at the discretion of the crates.io team, and this may also result in changes to the policy.

Since this is a big cultural change to the way crates are presented, it's going to take a bit of experimentation and time to fully refine these guidelines. This is why the crates.io team should be allowed to change these guidelines as they see fit without a full RFC, since ultimately, they're the ones in charge of actually responding to changes to keyword metadata and they can only do what they have the capacity to do.

# Drawbacks
[drawbacks]: #drawbacks

The biggest drawback is that this can potentially add substantial extra work for the crates.io team, since there could potentially be several more keywords with metadata than existing categories. However, [because crates.io currently uses the `itree` PostgreSQL extension to handle nested categories](https://github.com/rust-lang/rfcs/pull/3488#issuecomment-1741110992), this change could potentially reduce maintenance load on that team.

However, the requirements here aren't substantially different than those expected of users proposing categories before, just more clarified. Arguably, the onus is more on the users proposing keywords than the crates.io members accepting metadata changes, since it's substantially more difficult to make a case than to accept or reject one.

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

It seems unlikely that keyword functionality could be extended beyond the points this RFC suggests. Given the fact that existing package repositories take a rather lax mentality toward metadata, arguably, having metadata for arbitrary keywords at all is going above and beyond the status quo.

It's probably not a best idea to expand the scope of what's offered.
