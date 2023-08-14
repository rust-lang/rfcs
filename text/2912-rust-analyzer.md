- Feature Name: (fill me in with a unique ident, `my_awesome_feature`)
- Start Date: 2020-04-20
- RFC PR: [rust-lang/rfcs#2912](https://github.com/rust-lang/rfcs/pull/2912)
- Rust Issue: [rust-analyzer/rust-analyzer#4224](https://github.com/rust-analyzer/rust-analyzer/issues/4224)

# Summary
[summary]: #summary

The RFC proposes a plan to adopt rust-analyzer as Rust's official LSP implementation. The transition to rust-analyzer will take place in a few stages:

* **Feedback** -- encourage people to use rust-analyzer and report problems
* **Deprecation period** -- announce that the RLS is deprecated and encourage people to migrate to rust-analyzer
* **Final transition** -- stop supporting the older RLS

As detailed below, one major concern with rust-analyzer as it stands today is that it shares very little code with rustc. To avoid creating an unsustainable maintenance burden, this RFC proposes extracting shared libraries that will be used by both rustc and rust-analyzer ("library-ification"), which should eventually lead to rustc and rust-analyzer being two front-ends over a shared codebase.

# Motivation
[motivation]: #motivation

## Current status: RLS and rust-analyzer

Currently, Rust users who wish to use an editor that supports Microsoft's Language Server Protocol (LSP) have two choices:

* Use the RLS, the official IDE project of the Rust language.
* Use rust-analyzer, a more experimental, unofficial project that has recently been gaining ground.

Ideally, we would like to concentrate our efforts behind a single implementation.

## Architectural divide: save-analysis vs on-demand queries

The key technical difference between these two projects is that the RLS is based around rustc's "save-analysis" data, which basically means that the compiler -- after compilation -- can dump all sorts of of information about the code that it compiles into files. These files can be loaded by the RLS and used to do things like display errors, handle jump-to-definition, and other sorts of things. This architecture has the advantage that it builds on rustc itself, so it is generally up-to-date and accurate. However, generating save-analysis files is slow, and the architecture is not considered suitable for handling things like completions, where latency is at a premium.

In contrast, rust-analyzer effectively reimplements the Rust compiler in a fully incremental, on-demand style. This is the same architecture that rustc has been slowly evolving towards. This architecture enables much faster response time and it can also (in principle) handle things like fully type-correct completions. However, because rust-analyzer is not complete, it is currently not able to offer several key features, such as reporting errors or doing precise "find all usages".

Even in its current, experimental form, many users derive value from rust-analyzer. Many users are using it as their day-to-day IDE. It is particularly useful for larger codebases, such as the compiler.

## Challenges to overcome

There are several things that we would like to improve about the current situation:

* We would like to concentrate our efforts behind one LSP server, not support both the RLS and rust-analyzer.
    * Further, the goal for some time has been to adopt a query-based architecture much like the one that rust-analyzer is using.
* We would like to (eventually) avoid having two implementations of the Rust compiler to support, one in rustc and one in rust-analyzer.
* We would like to "pay down" technical debt within the compiler itself and to make it approachable.
    * To that end, we've been pursuing the creation of independent libraries, like miri or chalk. Smaller libraries with stronger API boundaries are not only easier to reason about but also provide an easier way for people to get involved in compiler development.

However, in making the transition from the existing RLS setup to rust-analyzer, we have to be careful not to introduce user confusion. In particular, we wish to make the experience of "managing one's editor" smooth, both for:

* Existing RLS users (who need to transition from the RLS to rust-analyzer), and
* New Rust users (who need to find and install rust-analyzer for the first time).

## Separate goal: making the compiler more approachable via 'library-ification'

Independently from IDEs, The compiler team has been pursuing a process of "library-ification", meaning converting rustc from a monolithic codebase into one with well-defined libraries and reasonably stable API boundaries. You can find more details in the [design meeting from 2019-09-13][2019-09-13]. The goal is ultimately for both rustc and rust-analyzer to be shallow wrappers around the same core codebases, as well as to improve the accessibility of the rustc codebase by having well-defined modules that can be learned independently.

[2019-09-13]: https://rust-lang.github.io/compiler-team/minutes/design-meeting/2019-09-13-rust-analyzer-and-libraryification/

As of today, rust-analyzer and rustc share the same lexer, which was extracted from rustc as part of this process. Meanwhile, rust-analyzer relies on [chalk] for its trait solving, and efforts are underway to integrate chalk into rustc and thus have a shared trait solver. Similarly, we are working to [extract a common library for representing types][chalk-ty].

[chalk]: https://github.com/rust-lang/chalk
[chalk-ty]: https://rust-lang.github.io/compiler-team/minutes/design-meeting/2020-03-12-shared-library-for-types/

## Observation: the needs of batch compilation and the needs of an IDE are not always the same

One observation that we have seen over time is that batch compilation and IDE interaction have somewhat different needs. We would like to share as much code as possible, but we might like to specialize some aspects of it.

For example, the compiler currently interns all of its types and frees them all at once at the end of compilation. This is highly efficient but not necessarily appropriate for a long-lived process, as over time it can lead to very high memory usage. An IDE, in contrast, might prefer to use a different strategy such as ref-counting or even garbage collection.

Similarly, in rustc, we have been moving towards a model where the dependency graph between queries is streamed out to the disk as soon as it is generated, and never stored in memory. This is because the dependency information is only needed when you start the *next* compilation. But in an IDE, that dependency information is needed as soon as the next keypress, and hence it doesn't make sense to stream it to disk.

Library-ification can address these concerns by two distinct "host processes" that make use of shared libraries differently. In the case of types, for example, we can be generic over whether types are interned or stored in some other sort of pointer. Similarly, the query infrastructure might have two modes or implementation strategies. 

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The high-level plan is effectively to adopt rust-analyzer as the primary LSP implementation for the Rust project, and to aggressively pursue 'library-ification' as a means to eliminate code duplication. The ultimate vision is that the majority of the compiler logic should live in shared libraries which have two "front-ends", one from rustc and one from rust-analyzer.

## Adopting rust-analyzer as the primary LSP implementation

### Installing rust-analyzer today

Today, [to install rust-analyzer for VSCode][r-a-install], one simply installs the rust-analyzer plugin. The plugin will download the rust-analyzer LSP implementation and automatically keep it up to date (currently on a once-per-week release cadence). The plugin also adds the `rust-src` component to rustup.

[r-a-install]: https://rust-analyzer.github.io/manual.html#installation

The experience of installing rust-analyzer for other editors is more varied. Currently, the rust-analyzer project only directly supports VSCode, while other editor plugins are maintained independently. During this period where rust-analyzer is still under heavy development, this makes sense. As rust-analyzer matures, we may wish to re-evaluate and contribute directly to their development or maintain them within the rust-lang org.

### Timeline for transition

Transition will occur in three phases:

* **Feedback:** During this first phase, we will post a blog post encouraging RLS users to try out rust-analyzer and see whether it works for them. If we encounter unexpected, blocking issues, or cases where people feel rust-analyzer is a significant step backward in their user experience, we may try to fix those issues before fully replacing the existing RLS.
* **Deprecation period:** We announce that support for the RLS is deprecated. We begin putting in place the tooling to transition existing users away from the RLS.
* **Final transition:** We no longer support the RLS plugin in its older form and no longer distribute RLS over rustup.

### How will rust-analyzer binaries be distributed

Presently, rust-analyzer binaries are distributed on a weekly basis by the rust-analyzer project. The plugin detects when new releases are available and automatically upgrades. We expect to transition that binary distribution to use rustup. This change to use rustup should occur during the feedback period.

### Conformance to the LSP protocol

Before the deprecation period begins, rust-analyzer should fully conform to the LSP protocol.

Furthermore, rust-analyzer sometimes adds extensions to the core LSP
protocol, to enable features that the core LSP does not yet
support. Some examples include:

* running specific tests (https://github.com/microsoft/language-server-protocol/issues/944)
* inlay hints (https://github.com/microsoft/vscode-languageserver-node/pull/609)

In some cases, these extensions go on to become part of the standard
protocol, as happened with these two extensions:

* extend selection (https://github.com/microsoft/language-server-protocol/issues/613)
* syntax highlighting (https://github.com/microsoft/vscode-languageserver-node/issues/576)

rust-analyzer will document the status and stability of these
extensions. Further, disruptive or unstable extensions will be made
opt-in (via client settings) until they are suitable for wider
use. However, we do not consider it a "semver violation" to remove
support for extensions if they don't seem to be working out, as the
LSP protocol already permits a negotiation between client and server
with respect to which extensions are supported.

### What is the transition plan?

The precise transition plan is not part of this RFC. It will be determined and announced as we enter the deprecation period, based on the feedback we've gotten and how many users have manually transitioned away from the RLS. We will endeavor to keep the experience as smooth as possible, but it may require some manual steps.

### Branding: how to talk about rust-analyzer/RLS going forward?

* We propose to keep the "rust-analyzer" name, at least for the transition period.
* In keeping with the [proposed rust-lang github access policy](https://github.com/rust-lang/rfcs/pull/2872), the repositories from the [rust-analyzer github org](https://github.com/rust-analyzer) will be consolidated and then merged into the [rust-lang github org](https://github.com/rust-lang).
    * They will be maintained by the compiler team with a dedicated working group.
    * The infra team will work with rust-analyzer to integrate the binary release and upgrade process
* The [rust-analyzer.github.io](https://rust-analyzer.github.io/) website will redirect to `rust-lang.github.io/rust-analyzer`.

# Drawbacks
[drawbacks]: #drawbacks

The primary drawback to the plan is that, in the short term, rust-analyzer and rustc represent two distinct codebases performing essentially the same function. We do hope to rectify this by extracting shared libraries that both can use but this will take some time. In the meantime, we'll have to support them both. This could mean that there is more of a "lag" between rustc gaining support for some new syntax and that same support making its way into the IDE.

A secondary drawback is that rust-analyzer today sometimes uses approximate answers where the current RLS is able to offer precise results. This can occur, for example, with jump to definition. This situation will continue to be the case until we make progress on library-ification of parsing and name resolution. 

More generally, switching the official IDE from RLS to rust-analyzer will incur tooling churn on users, and would not be strictly better in the short term (although the expectation is that it will be significantly better on average).

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Reimplement rust-analyzer within rustc

The primary alternative considered was to halt work on rust-analyzer and instead attempt to port the lessons from its development to rustc. In effect, the idea would be to create a LSP server based on rustc itself.

The primary appeal of this plan is that there would always be a single codebase. Moreover, the fundamental architecture of rustc has been moving steadily towards the demand-driven, IDE-friendly design that rust-analyzer has also adopted (the two have indeed influenced one another), so this would be a natural extension of that work.

However, there are a number of practical concerns with taking that approach. One concern is that, for rust-analyzer's current users, it would represent a regression. rust-analyzer would no longer be available (or at least no longer updated) and it would take some time until rustc is at feature parity. Moreover, experience has shown that refactoring rustc can move relatively slowly, simply due to the age of the codebase, the amount of code involved, and the complicated non-standard build process.

Further, the "reimplement" approach would represent a constraint on the ordering in which we do our work. With the design proposed in this RFC, for example, rust-analyzer is able to make use of the chalk library already. This is only possible because rust-analyzer has a "stub" version of Rust's name resolution engine and type checker embedded in it -- this type checker is not perfect, but it's good enough to drive chalk and gain useful experience. This allows us to create an end-to-end IDE user experience sooner, in effect.

In contrast, if we were to try and rebuild rust-analyzer within rustc, even if we had rustc adopt chalk or some other IDE-friendly trait resolution algorithm, that would not be of use to IDE users until we had also upgraded the name resolution algorithm and type checker to be IDE friendly. In short, having a "prototype" version of these algorithms that lives in rust-analyzer is both a pro and a con: it means we have to maintain two versions, but it means users get benefits faster and developers can experiment more freely.

## Require feature parity between the existing RLS and rust-analyzer

One of the key points in this RFC is that feature parity with RLS is not strictly required. While rust-analyzer offers a number of things that the RLS does not support, there are three specific ways that it lags behind:

* It does not support reporting errors or lints without saving
* It does not support precise find-all-usages, goto-definition, or renames, in some cases falling back to approximations.
* It does not persist data to disk, which can lead to large startup times.

The reasons behind these limitations are that it will take some time to implement those features "the right way" (i.e., using the demand-driven approach that rust-analyzer is pioneering). Initially, we expected to require full feature parity, but we realized that this would lead to us creating "throwaway" code to temporarily patch over the limitation, and that this would in turn slow the progress towards our ultimate goals. Therefore, we decided not to require this, but instead to opt for a "feedback" period to assess the biggest pain points and see what we can do to relieve them.

# Prior art
[prior-art]: #prior-art

The current proposal is informed by experience with existing RLS and query-based compilation in rustc. Additionally, rust-analyzer heavily draws from lessons learned while developing IntelliJ Rust.

It's interesting that many compilers went through a phase with parallel implementations to get a great IDE support

* For C#, the [Roslyn](https://github.com/dotnet/roslyn) project was a from scratch implementation.
* [Dart for a long time had different front-ends for command line and interactive compilers](https://youtu.be/WjdrUphF5l4?t=2204)
* [Swift is transitioning to new syntax tree library by "reimplement separately, then swap" approach](https://medium.com/@kitasuke/deep-dive-into-integrating-libsyntax-into-the-compiler-pipeline-2d478c8600a1)

Notable exceptions:

* Kotlin, TypeScript -- these languages were implemented with IDEs in mind from the start
* OCaml with merlin/[ocaml-lsp] and C++ with clangd -- languages with header files and forward declarations make it easier to adapt traditional compiler architecture to certain IDE tasks like completion

[ocaml-lsp]: https://github.com/ocaml/ocaml-lsp

# Unresolved questions
[unresolved-questions]: #unresolved-questions

* How and when will we complete the transition from the existing RLS to rust-analyzer?
    * As stated above, this will be determined based on the feedback we receive during the Feedback phase.

# Future possibilities
[future-possibilities]: #future-possibilities
