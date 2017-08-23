# Motivation

## Background

For the past several months, we have been investigating the module system, its
weaknesses, strengths, and areas of potential improvement. Two blog posts have
been produced, which lay out a part of the argument in favor of changing the
module system:

* [The Rust module system is too confusing][too-confusing] by withoutboats
* [Revisiting Rust's modules][revisiting] by aturon

Both of these posts contain proposals for how to revamp the module system,
neither of which are representations of what this RFC contains. However, they
provide valuable background on the conversation that has been occurring, since
January of this year, about how the Rust module system could be improved.

Fundamentally, we believe that the current module system is difficult to learn
how to use correctly, and contains unnecessary complexity which leaves new
users confused and advanced users annoyed. We have collected empirical data in
support of that belief, and also have formed a model of the underlying problems
with the system that we hope to resolve or mitigate with this proposal.

Its important to keep this in mind: our contention is not that the module
system is the most difficult part of the language, only that it is
**unnecessarily** difficult. Some aspects of Rust have high inherent novelty -
such as ownership & lifetimes - and cannot be just *made* easier (though the
syntax could always be more obvious, etc). We do not believe this is true of
our modules, which provide similar benefits to systems in many mainstream
languages which do not have such a problem.

## Evidence of a problem

### Survey data

In the survey data collected in 2017, ergonomics issues were one of the major
challenges for people using Rust today. While there were other features that
were raised more frequently than the module system (lifetimes for example), we
don't feel that modules ought to be considered an ergonomics problem at all.

Here are some quotes (these are not the only responses that mention the module
system):

> Also the module system is confusing (not that I say is wrong, just confusing
> until you are experienced in it).

> a colleague of mine that started rust got really confused over the module
> system

> You had to import everything in the main module, but you also had to in
> submodules, but if it was only imported in a submodule it wouldn't work. 

> I especially find the modules and crates design weird and verbose

> fix the module system

One user even states that the reason they stopped using Rust was that the
"module system is really unintuitive." Our opinion is that if any user has
decided to *stop using Rust* because they couldn't make modules work, the
module system deserves serious reconsideration.

### Found feedback

@aturon devoted some time to searching for feedback from Rust users on various
online forums about Rust (GitHub, the discourse forums, reddit, etc). He found
numerous examples of users expressing confusion or frustration about how the
module system works today. This data is collected in [this
gist][learning-modules].

## Underlying problems

### Lots of syntax, all of it frontloaded

The current Rust module system has four primary keywords: `extern crate`,
`mod`, `use`, and `pub`. All of these are likely to be encountered very early
by users - even if you're working on a small binary project, just to learn, you
are likely going to want to add a dependency and break your code into two
files. Doing those two things introduces the need to use  all four keywords
that are a part of the module system, and to understand what each of them does.

In our proposal, there are three primary keywords: `use`, `pub`, and `export`.
And in an early project, only `use` and `pub` are going to be necessary. Not
only that, the `export` keyword is just a visibility, like `pub`, and can be
viewed as an extension of the pre-existing knowledge about visibility learned
when your crate gains an external API.

We believe we will simplify the system by requiring users to learn and do less
stuff. 

We also believe reducing the syntax improves the ergonomics for advanced users
as well. Many of these declarations tend to feel like 'boilerplate' - you write
the only thing you could possibly write. You may forget to write a `mod`
statement, leading to a compiler error (worsening your edit-compile-debug
cycle). We, the RFC authors, frequently have this experience.

## Path confusion

A problem, recognized by many users, is a confusion that exists about how paths
work in Rust. Though this has often been glossed as confusion about the
difference between absolute and relative paths, we don't believe this is true.
Very many languages use absolute paths for their import statement, but relative
paths internally to a module, without demonstrating the same confusion. We
believe the confusion has two causes, both addressed by this RFC.

### Differences between crate root and other modules

In the crate root, absolute and relative paths are the same today. In other
languages this tends not to be the case. Instead, a common system is to place
the root of the current package under some namespace; this RFC proposes to do
just that, using the new `crate` namespace.

This will make absolute paths different from relative paths in every module,
including the crate root, so that users do not get mislead into believing that
`use` statements are relative, or that all paths are absolute.

### Multiple keywords import things

Another problem with the current system is that `extern crate` and `mod` bring
names into scope, but this is not immediately obvious. Users who have not
grasped this can be confused about how names come into scope, believing that
dependencies are inherently in scope in the root, for example (though it is
actually `extern crate` which brings them into scope). Some users have even
expressed confusion about how it seems to them that `extern crate` and `mod`
are "just" fancy import statements that they have to use in seemingly arbitrary
cases, not understanding that they are using them to construct the hierarchy
used by the `use` statements.

We solve this problem by removing both the `extern crate` and `mod` statements
from the language.

## Nonlocal reasoning

Another frustration of the current module system - which affects advanced users
at least as much as newer users - is the way that visibility is highly
nonlocal. When an item is marked `pub` today, its very unclear if it is
actually `pub`. We make this much clearer by the combination of two choices in
this RFC:

- All items that are `export` (equivalent of today's `pub`) must *actually* be
  exposed in the external API, or the user receives an error.
- All modules are crate visible by default, so every visibility less than
  `export` is inherently self-descriptive.

[too-confusing]: https://withoutboats.github.io/blog/rust/2017/01/04/the-rust-module-system-is-too-confusing.html
[revisiting]: https://aturon.github.io/blog/2017/07/26/revisiting-rusts-modules/
[learning-modules]: https://gist.github.com/aturon/2f10f19f084f39330cfe2ee028b2ea0c
