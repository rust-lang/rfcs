- Start Date: 2023-09-10
- RFC PR: [rust-lang/rfcs#3488](https://github.com/rust-lang/rfcs/pull/3488)

# Summary

The first part of this RFC contains a summary of the "categories" and "keywords"
concepts, their combinations, their pros and cons, their usage statistics and
how they are used in other ecosystems.

The second part proposes merging the concepts of "categories" and "keywords" on
crates.io into a single concept, achieving the best of both worlds with a list
of recommended keywords.


# History

- June 2014:
  - [First commit to crates.io](https://github.com/rust-lang/crates.io/commit/54cfc8d)
- Oct. 2014: 
  - [Keywords are added to crates.io](https://github.com/rust-lang/crates.io/commit/8f03b72)
- Nov. 2016:
  - [Categories are added to crates.io](https://github.com/rust-lang/crates.io/pull/473)
  - [Discussion on the initial list of categories is started](https://github.com/rust-lang/crates.io/pull/488)
    - The initial list was adopted from https://github.com/rust-unofficial/awesome-rust
    - A couple of comments suggest using a list of "official" keywords instead
      of introducing a parallel concept of categories (https://github.com/rust-lang/crates.io/pull/488#issuecomment-265593195
      and https://github.com/rust-lang/crates.io/pull/488#issuecomment-265623340)


# Statistics

The following numbers have been extracted from a recent
[crates.io database dump](https://crates.io/data-access#database-dumps):

- 40,649 crates have `categories`
  - 3,464 of those crates have only `categories`, and no `keywords`
  - 3,622 of those crates have only a single category
- 61,216 crates have `keywords`
  - 24,031 of those crates have only `keywords`, and no `categories`
  - 4,694 of those crates have only a single keyword
- 37,185 crates have both `categories` and `keywords`
- 56,694 crates have neither `categories`, nor `keywords`


# Categories vs. Keywords

**Categories** describe a closed set of names to categorize packages. The set 
being "closed" means that users have to choose from a fixed list of available
categories, and can't easily add new ones.

**Keywords** (aka. tags) describe an open set of names to categorize packages.
The set being "open" means that users can use arbitrary keywords without having
to choose from a fixed list of available names.

**Comparison:**

- **Creation:** As described above, the keywords concept makes it
quite easy to create new names by simply using them in the `Cargo.toml` file.
With categories, a pull request to the crates.io repository has to be opened,
followed by a discussion among the crates.io team on whether there might be
enough justification for adding such a new category.

- **Typos:** The chance of misspelling categories is reduced since cargo and
crates.io can warn about misspelled category names. Keywords can be arbitrary,
so a typo-detection is not quite as easy.

  - It would be possible to compare keywords against a list of commonly used
  keywords though and warn the user about potential typos.

- **Duplicates:** Since the list of categories is fixed and new additions are
checked by the crates.io team, the chance for having categories with similar
purposes is small. With keywords, it is easy to have both `math` and
`mathematics` keywords available on crates.io. In that case it would be the
burden of the crate authors to use the most appropriate and/or commonly used
keywords.

  - This could be countered to some degree by having a list of common aliases
  on the server-side that are applied either at publish-time or when a user
  searches for a particular keyword. [Experience from libs.rs indicates that
  there is no trivial solution to this though](https://github.com/rust-lang/rfcs/pull/3488#issuecomment-1721208225) 

  - The concept of duplicates itself is also more ambiguous than you'd think.
  One good example is the endless decision on whether keywords should be
  combined or split up: for example, using "http" and "server" or "http-server"
  keywords. This problem isn't unique to keywords or categories, since there's
  always overlap between different groups and it's never clear where to draw the
  boundaries.

- **Clarity:** Categories, as implemented on crates.io, have a description
associated with them, so the chance of misunderstanding what kinds of crates
belong to them is reduced. Keywords by themselves can be unclear in some
cases. One example being the [`cli` keyword](https://github.com/rust-lang/crates.io/pull/488#issuecomment-265600770)
which includes libraries for building CLI applications, as well as CLI
applications themselves.

- **Endorsement:** Some users feel that having a category exist on crates.io is an
endorsement of that category by the Rust project. This makes the introduction
and deletion of categories hold more weight than they should, and it puts an
increased burden of discussion for all those involved. One example of this
being a [discussion about the "cryptocurrencies" category](https://github.com/rust-lang/crates.io/discussions/6762), and
whether it should be deleted or not. Keywords don't have this issue since they
aren't added with any involvement from the crates.io team, and thus don't have
the same authority. The crates.io team can still remove keywords that violate 
e.g. the [Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct),
but that's much easier to decide than having to take a stance on whether
keywords are endorsed or not.

- **Grassroots:** Some users feel that the crates.io team dictating the list of
available categories can be seen as ["top-down" decision-making](https://github.com/rust-lang/crates.io/pull/488#issuecomment-265593195),
while keywords are a more "bottom-up" approach, where usage patterns influence
what keywords are shown on the front page.

- **Choice:** Categories can be [easier to pick from since the list is relatively
short](https://github.com/rust-lang/crates.io/pull/488#issuecomment-265619779),
while with keywords the number of options is almost infinite. At the same
time, having a short list can make it harder for some crates to categorize
them because they don't fit in any of the existing categories.

- **Bike-shedding:** As the original implementation acknowledged, having a fixed
list of categories will turn into endless bike-shedding discussions on which
categories should exist or not, while that time could be spent on more useful
work instead.


# Combinations

**Categories && Keywords** describes the current implementation where
crates.io supports both concepts at the same time.

**Recommended Keywords** (aka. noteworthy keywords, blessed keywords, official
keywords, …) are a system where users can create new keywords on their own, but
the crates.io team can add descriptions and potentially other metadata to
so-called "recommended" keywords. Such a system was also suggested in the
original categories implementation pull request [here](https://github.com/rust-lang/crates.io/pull/488#issuecomment-265593195)
and [here](https://github.com/rust-lang/crates.io/pull/488#issuecomment-265623340).

**Comparison:**

- **Confusion:** Having both categories and keywords can be confusing to users
due to the [overlap](https://github.com/rust-lang/crates.io/pull/488#issuecomment-265593195)
of functionality in both concepts. Especially when searching for crates, it is
not clear whether it's better to look for a matching category or a matching
keyword. With just keywords it can potentially be confusing too if multiple
matching keywords exist, though that can be countered to some degree by
promoting one of them to a recommended keyword.

- **Creation:** One big advantage of the "recommended keywords" concept is that
keywords can be promoted to be "recommended" retroactively
[based on usage data](https://github.com/rust-lang/crates.io/pull/488#issuecomment-265597304).
Categories need to be created by the crates.io team before they can be used by
any crates, which leads to situations where such categories are not actually
being used and [only contain a single crate](https://crates.io/categories/science::neuroscience).

- **Duplicates:** With a list of recommended keywords the chance for duplicates
can be reduced, but not eliminated completely. Categories have the advantage
of not having that problem, but at the same time they also restrict crate
authors to only those categories that have been pre-selected by the crates.io
team.

- **Clarity:** In addition to having a list of recommending keywords to choose
from, providing descriptions alongside those keywords helps users decide which
crates should be labeled with those keywords too. The same applies to categories
as well.

- **Endorsement:** As discussed above, the introduction of a category is a very
heavy decision since it can be seen as an endorsement from the Rust project
itself. If keywords are promoted primarily based on usage data then this
reflects less on the project and more on its users, reducing this pressure on
the crates.io team.

- **Grassroots:** When keywords are promoted to being recommended based
primarily on usage data then there is less perception of "top-down
decision-making" compared to the fixed list of categories "dictated" by the
crates.io team.

- **Choice:** Choosing from the list of all available keywords can quickly lead
to choice overload. Having recommended keywords can counter this by giving
crate authors a short list of keywords to choose from, and only if they don't
find any fitting keywords they can drop down to the full list of keywords or
introduce a new one.

- **Bike-shedding:** As with previous points in this list, promoting keywords
based on usage data means that any bike-shedding discussions on what keywords to
use have already happened by users, and the crates.io team only has to partake
in the logistics of adding information about existing keywords.


# Flat vs. Nested

**Flat** means that there is no hierarchy of terms. A crate related to doing
HTTP requests could for example use the keywords `web`, `http`, and
`http-client`. 

**Nested** means that there is a hierarchy of terms. The same crate could be
tagged with `web/http-client`, where `web` is the parent category and
`http-client` the child.

**Comparison:**

- **Simplicity:** Having a flat namespace is generally simpler for crate authors
and users to understand.

- **Duplicates:** Nested hierarchies work well when using a closed set of terms.
When users can create new terms without moderation the chance of having
duplicate terms increases significantly though since both the parent and child
terms could now be duplicated. One hypothetical example of that being
`maths` and `science/maths` terms being used.

- **Refinement:** Nested hierarchies make it possible to refine the search space
from e.g. `science` to `science/biology`. In a flat namespace you could
potentially combine multiple keywords to achieve similar results. Displaying a
list of [related keywords](https://github.com/rust-lang/crates.io/pull/488#issuecomment-265600285)
to the one the user is currently looking at can help with this though.

- **Clarity:** In a flat namespace the chance of a single keyword being used for
multiple purposes is slightly increased. With nested hierarchies, the parent
term can often give a bit more context on how the keyword/category is supposed
to be used.

- **Bike-shedding:** While flat namespaces already have bike-shedding potential,
a nested namespace makes this worse by adding questions like: "should this
name be nested under another name?" or "what parent name should this be nested
under?"

- **Implementation:** Nested categories on crates.io are currently implemented
using the [`ltree` Postgres extension](https://www.postgresql.org/docs/current/ltree.html).
This could be implemented differently as well, but from a purely technical
view a flat list of terms is significantly easier to implement and maintain.
Admittedly, parts of this are crates.io implementation details though.


# Prior art

Looking at other package repositories:

- [PyPI](https://pypi.org) (Python) offers curated [classifiers](https://pypi.org/classifiers)
without metadata. They don't seem to have an official policy on adding new
classifiers, having instead taken the original list [from external sources](https://peps.python.org/pep-0301/#distutils-trove-classification).

- [Hackage](https://hackage.haskell.org) (Haskell) has both keywords and
uncurated categories without any metadata.

- [NPM](https://www.npmjs.com) (JavaScript) has keywords.

- [pub.dev](https://pub.dev) (Dart) has keywords under the name "topics".

- [Maven Central](https://mvnrepository.com/repos/central) (Java),
[NuGet](https://www.nuget.org) (.NET), [Packagist](https://packagist.org)
(PHP) have keywords under the name "tags".

- [Dub](https://code.dlang.org) (D), [Go](https://pkg.go.dev),
[RubyGems](https://rubygems.org) (Ruby) have no keywords or categories.


# Proposal / Motivation

This RFC proposes to migrate crates.io from using categories **and** keywords
to using only keywords, with a list of recommended keywords to address
the duplication and clarity concerns from above. Keywords will keep their
structure as a flat hierarchy, allowing crate owners to express nested
categories as keyword combinations instead.

The plan is for crates.io to not show any of the category user interface elements
anymore. Eventually the crates.io server will also start to ignore the `categories`
field for new uploads and remove the existing categories data from the database.

An automated migration from categories to keywords for existing crates is
currently not planned as the majority of crates that have been using categories
are also using keywords at the same time, while the reverse is not the case
(see "Statistics" section above).

The lists of pros and cons above already hint at a couple of reasons that
motivate this RFC:

The first reason being the confusion and complexity that is caused by having two
competing concepts implemented right next to each other. This makes it hard for
Rust users to figure out which concept to use when browsing and searching on
crates.io. It also means extra work for crate authors that currently have to
choose both keywords *and* categories for their crates.

As can be seen in the "Prior art" section above, most package registries stick
to either keywords or categories, and only the Haskell ecosystem is also
implementing both.

Another reason is the friction that is caused by the crates.io team having to
decide upfront on whether to introduce a new category or not. As written above,
this is seen by some people as endorsing certain categories of crates and thus
requires more work. If the "recommended keywords" concept is implemented
primarily based on usage data of existing keywords, this discussion can largely
be sidestepped, and it gives the Rust community the power to decide what
keywords should be recommended and presented on the crates.io frontpage.

Finally, having the community decide on the keywords recommendations through
their usage data reduces the amount of bike-shedding discussions that currently
exist when categories are decided upon with very little up-front usage data.


# Drawbacks

- As mentioned above, the chance for having duplicates is increased since not
everyone will use the list of recommended keywords. This can be countered to
some degree with aliases for recommended keywords, but can't be prevented
completely.

- The main drawback of changing how we categorize crates is the change itself.
Existing users and crate authors are used to having both concepts available,
and the existing tooling like IDE autocomplete of categories assumes the same
thing. It will take a bit of time and educating people to adjust the ecosystem
to the new concept.


# Implementation

The following implementation steps are planned for the crates.io side:

- [ ] Add a publicly visible list of recommended keywords based on existing
keyword usage on crates.io and the list of existing categories.
- [ ] Add a publish-time deprecation warning for crates that are published with
only categories but no keywords.
- [ ] Remove categories from the crates.io frontend:
  - Crates inside category pages
  - Categories list
  - Category slugs list
  - Crate details page sidebar
  - Support for `category:foo` in search bar
- [ ] Remove categories from the crates.io backend:
  - `/crates?category=foo` returns an empty list
  - `/crates/foo` returns an empty list of `categories`
  - `/category_slugs` returns an empty list
  - `/categories` returns an empty list
  - `/categories/foo` returns `404 Not Found`
  - `/crates/new` stops saving categories for new uploads
- [ ] Remove categories from that crates.io database (incl. the `ltree`
extension).
- [ ] Add a publish-time deprecation warning for all crates that are published
with categories.


# Requirements for recommended keywords

The following basic rules should be followed when promoting keywords to the list
of recommended keywords.

1. The keyword does not violate the [Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct)
of the Rust project. (Such keywords are not allowed anyway, but the crates.io
team is not currently monitoring new additions to the list of keywords and will
only act on this when users report violations)

2. The keyword is already used by substantial amount of crates from separate
authors. (Intentionally vague to account for the growth of the package registry)

3. The keyword serves a purpose that is not covered by any other recommended
keyword.


# Unresolved questions

- Should the `categories` field be deprecated in cargo too?
  - Are third-party registries using categories?
  - How much usage of cargo is with crates.io vs. third party registries?
  - Would users be confused if it was only deprecated on crates.io but not in cargo?

- Should crates.io automatically map submitted categories to keywords for new
publishes? Should crates.io do the same for existing published versions?

- Is "recommended keywords" a good name? One of the problems with categories is
this sense of endorsement, and "recommended" keywords can have a similar 
connotation, even though the idea is that these are recommended by the
community, not crates.io itself. This RFC intentionally avoids bikeshedding the
name because we could spend all day on that, but it's worth mentioning anyway.

- What will be the initial list of recommended keywords?
(out of scope for this RFC)


# Future possibilities

- Implementing aliases for recommended keywords requires a little bit more
design to answer questions like:
  - Should aliases be applied/normalized at publish time or at search time?
  - Should we warn the user or publisher that we applied an alias?
  - Does having aliases incentivize publishers to not care about the list of
  recommended keywords anymore?

  We propose to slightly defer the implementation of keyword aliases for now
  until some of these questions have clearer answers.

- https://www.npmjs.com has a set of handpicked keywords on the frontpage.
Should crates.io have something similar instead of (just) the list of the ten
most used keyword?

- [Stack Overflow](https://stackoverflow.com) allows users to look up questions
with specific tags. On these pages it shows you some other popular tags from 
the selected set of questions so that users are able to refine their search. 
We could build something similar for crates.io, where on the `#http` keyword
page you would see refinement options for e.g. `#server` and `#client`.
(see https://github.com/rust-lang/crates.io/pull/488#issuecomment-265600285)
