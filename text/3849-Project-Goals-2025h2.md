- Feature Name: N/A
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: [rust-lang/rfcs#3849](https://github.com/rust-lang/rfcs/issues/3849)
- Rust Issue: N/A

## Summary
[summary]: #summary

Propose a slate of 41 goals for 2025H2.

## Motivation

The 2025h2 goal slate consists of 41 project goals, of which we have selected a subset as **flagship goals**. Flagship goals represent the highest priority being done by the various Rust teams.


## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

### Rust's mission

Our goals are selected to further Rust's mission of making it dramatically more accessible to author and maintain *foundational software*—the software that underlies everything else. This includes the CLI tools and development infrastructure that developers rely on, the cloud platforms that run applications, the embedded systems in devices around us, and increasingly the kernels and operating systems that power it all.

Foundational software has particularly demanding requirements: reliability is paramount because when foundations fail, everything built on top fails too. Performance overhead must be minimized because it becomes a floor on what the layers above can achieve. Traditionally, meeting these requirements meant choosing between the power-but-danger of C/C++ or the safety-but-constraints of higher-level languages used in very specific ways.

Rust changes this balance by combining zero-cost abstractions with memory safety guarantees, often allowing you to write high-level code with low-level performance. While Rust's primary focus remains foundational software, we also recognize that supporting higher-level applications helps identify ergonomic improvements that benefit all users and enables developers to use Rust throughout their entire stack.

### Flagship goals

This period we have 12 flagship goals, broken out into four themes:

* [Beyond the `&`](#beyond-the-), making it possible to create user-defined smart pointers that are as ergonomic as Rust's built-in references `&`.
* [Unblocking dormant traits](#unblocking-dormant-traits), extending the core capabilities of Rust's trait system to unblock long-desired features for language interop, lending iteration, and more.
* [Flexible, fast(er) compilation](#flexible-faster-rust-compilation), making it faster to build Rust programs and improving support for specialized build scenarios like embedded usage and sanitizers.
* [Higher-level Rust](#higher-level-rust), making higher-level usage patterns in Rust easier.

#### "Beyond the `&`"

| Goal                                                                         | Point of contact | Team(s) and Champion(s)                      |
| :--                                                                          | :--          | :--                                          |
| [Reborrow traits](https://rust-lang.github.io/rust-project-goals/2025h2/autoreborrow-traits.html)                                    | [Aapo Alasuutari][]    | [compiler] ([Oliver Scherer][]), [lang] ([Tyler Mandry][])     |
| [Design a language feature to solve Field Projections](https://rust-lang.github.io/rust-project-goals/2025h2/field-projections.html) | [Benno Lossin][] | [lang] ([Tyler Mandry][])                            |
| [Continue Experimentation with Pin Ergonomics](https://rust-lang.github.io/rust-project-goals/2025h2/pin-ergonomics.html)            | [Frank King][]  | [compiler] ([Oliver Scherer][]), [lang] ([TC][]) |


One of Rust's core value propositions is that it's a "library-based language"—libraries can build abstractions that feel built-in to the language even when they're not. Smart pointer types like `Rc` and `Arc` are prime examples, implemented purely in the standard library yet feeling like native language features. However, Rust's built-in reference types (`&T` and `&mut T`) have special capabilities that user-defined smart pointers cannot replicate. This creates a "second-class citizen" problem where custom pointer types can't provide the same ergonomic experience as built-in references.

The "Beyond the `&`" initiative aims to share `&`'s special capabilities, allowing library authors to create smart pointers that are truly indistinguishable from built-in references in terms of syntax and ergonomics. This will enable more ergonomic smart pointers for use in cross-language interop (e.g., references to objects in other languages like C++ or Python) and for low-level projects like Rust for Linux which use smart pointers to express particular data structures.

#### "Unblocking dormant traits"

| Goal                                                    | Point of contact | Team(s) and Champion(s)                                        |
| :--                                                     | :--       | :--                                                            |
| [Evolving trait hierarchies](https://rust-lang.github.io/rust-project-goals/2025h2/evolving-traits.html)        | [Taylor Cramer][] | [compiler], [lang] ([Taylor Cramer][]), [libs-api], [types] ([Oliver Scherer][]) |
| [In-place initialization](https://rust-lang.github.io/rust-project-goals/2025h2/in-place-initialization.html)   | [Alice Ryhl][] | [lang] ([Taylor Cramer][])                                             |
| [Next-generation trait solver](https://rust-lang.github.io/rust-project-goals/2025h2/next-solver.html)          | [lcnr][]     | [types] ([lcnr][])                                                |
| [Stabilizable Polonius support on nightly](https://rust-lang.github.io/rust-project-goals/2025h2/polonius.html) | [Rémy Rakic][]      | [types] ([Jack Huey][])                                            |


Rust's trait system is one of its most powerful features, but it has a number of longstanding limitations that are preventing us from adopting new patterns. The goals in this category unblock a number of new capabilities:

* [Polonius](https://rust-lang.github.io/rust-project-goals/2025h2/./polonius.html) will enable new borrowing patterns, and in particular [unblock "lending iterators"](https://github.com/rust-lang/rust/issues/92985). Over the last few goal periods we have identified an "alpha" vesion of polonius that addresses the most important cases while being relatively simple and optimizable. Our goal for 2025H2 is to implement this algorithm in a form that is ready for stabilization in 2026.
* The [next gen trait solver](https://rust-lang.github.io/rust-project-goals/2025h2/./next-solver.html) is a refactored trait solver that unblocks better support for numerous language features (implied bounds, negative impls, the list goes on) in addition to closing a number of existing bugs and unsoundnesses. Over the last few goal periods, the trait solver went from early prototype to being production use in coherence. The goal for 2025H2 is to prepare it for stabilization.
* The work on [evolving trait hierarchies](https://rust-lang.github.io/rust-project-goals/2025h2/./evolving-traits.html) will make it possible to refactor some parts of an existing trait out into a new supertrait so they can be used on their own. This unblocks a number of features where the existing trait is insufficiently general, in particular stabilizing support for custom receiver types, a prior project goal that wound up blocking on this refactoring. This will also make it safer to provide stable traits in the standard library, while preserving the ability to evolve them in the future.
* The work to [expand Rust's `Sized` hierarchy](https://rust-lang.github.io/rust-project-goals/2025h2/./scalable-vectors.html) will permit us to express types that are neither `Sized` nor `?Sized`, such as extern types (which have no size) or ARM's Scalable Vector Extensions (which have a size that is known at runtime, but not compilation time). This goal builds on [RFC #3729](https://github.com/rust-lang/rfcs/pull/3729) and [RFC #3838](https://github.com/rust-lang/rfcs/pull/3838), authored in previous project goal periods.
* [In-place initialization](https://rust-lang.github.io/rust-project-goals/2025h2/./in-place-initialization.html) allows creating structs and values that are tied to a particular place in memory. While useful directly for projects doing advanced C interop, it also unblocks expanding `dyn Trait` to support for `async fn` and `-> impl Trait` methods, as compiling such methods requires the ability for the callee to return a future whose size is not known to the caller.

#### "Flexible, fast(er) compilation"

| Goal                                                                | Point of contact | Team(s) and Champion(s)                                      |
| :--                                                                 | :--         | :--                                                          |
| [build-std](https://rust-lang.github.io/rust-project-goals/2025h2/build-std.html)                                           | [David Wood][]  | [cargo] ([Eric Huss][]), [compiler] ([David Wood][]), [libs] ([Amanieu d'Antras][]) |
| [Promoting Parallel Front End](https://rust-lang.github.io/rust-project-goals/2025h2/parallel-front-end.html)               | [Sparrow Li][] | [compiler]                                                   |
| [Production-ready cranelift backend](https://rust-lang.github.io/rust-project-goals/2025h2/production-ready-cranelift.html) | [Folkert de Vries][] | [compiler], [wg-compiler-performance]                        |


The "Flexible, fast(er) compilation" initiative focuses on improving Rust's build system to better serve both specialized use cases and everyday development workflows:

* We are improving compilation performance through (1) [parallel compilation in the compiler front-end](https://rust-lang.github.io/rust-project-goals/2025h2/./parallel-front-end.html), which delivers 20-30% faster builds, and (2) [making the Cranelift backend production-ready for development use](https://rust-lang.github.io/rust-project-goals/2025h2/./production-ready-cranelift.html), offering roughly 20% faster code generation compared to LLVM for debug builds.
* We are working to [stabilize a core MVP of the `-Zbuild-std` feature](https://rust-lang.github.io/rust-project-goals/2025h2/./build-std.html), which allows developers to rebuild the standard library from source with custom compiler flags. This unblocks critical use cases for embedded developers and low-level projects like Rust for Linux, while also enabling improvements like using sanitizers with the standard library or building `std` with debug information.

#### "Higher-level Rust"

| Goal                                                                | Point of contact | Team(s) and Champion(s)                                                           |
| :--                                                                 | :--           | :--                                                                               |
| [Stabilize cargo-script](https://rust-lang.github.io/rust-project-goals/2025h2/cargo-script.html)                           | [Ed Page][]        | [cargo] ([Ed Page][]), [compiler], [lang] ([Josh Triplett][]), [lang-docs] ([Josh Triplett][]) |
| [Ergonomic ref-counting: RFC decision and preview](https://rust-lang.github.io/rust-project-goals/2025h2/ergonomic-rc.html) | [Niko Matsakis][] | [compiler] ([Santiago Pastorino][]), [lang] ([Niko Matsakis][])                                  |


People generally start using Rust for foundational use cases, where the requirements for performance or reliability make it an obvious choice. But once they get used to it, they often find themselves turning to Rust even for higher-level use cases, like scripting, web services, or even GUI applications. Rust is often "surprisingly tolerable" for these high-level use cases -- except for some specific pain points that, while they impact everyone using Rust, hit these use cases particularly hard. We plan two flagship goals this period in this area:

* We aim to stabilize [cargo script](https://rust-lang.github.io/rust-project-goals/2025h2/./cargo-script.html), a feature that allows single-file Rust programs that embed their dependencies, making it much easier to write small utilities, share code examples, and create reproducible bug reports without the overhead of full Cargo projects.
* We aim to finalize the design of [ergonomic ref-counting](https://rust-lang.github.io/rust-project-goals/2025h2/./ergonomic-rc.html) and to finalize the experimental impl feature so it is ready for beta testing. Ergonomic ref counting makes it less cumbersome to work with ref-counted types like `Rc` and `Arc`, particularly in closures.

### Project goals

The full slate of project goals are as follows. These goals all have identified points of contact who will drive the work forward as well as a viable work plan. The goals include asks from the listed Rust teams, which are cataloged in the [reference-level explanation](#reference-level-explanation) section below.

**Invited goals.** Some goals of the goals below are "invited goals", meaning that for that goal to happen we need someone to step up and serve as a point of contact. To find the invited goals, look for the ![Help wanted][] badge in the table below. Invited goals have reserved capacity for teams and a mentor, so if you are someone looking to help Rust progress, they are a great way to get involved.

| Goal                                                                                                       | Point of contact | Team(s) and Champion(s)                                                               |
| :--                                                                                                        | :--              | :--                                                                                   |
| [Develop the capabilities to keep the FLS up to date](https://rust-lang.github.io/rust-project-goals/2025h2/FLS-up-to-date-capabilities.html)                      | [Pete LeVasseur][]      | [bootstrap] ([Jakub Beránek][]), [lang] ([Niko Matsakis][]), [opsem], [spec] ([Pete LeVasseur][]), [types] |
| [Getting Rust for Linux into stable Rust: compiler features](https://rust-lang.github.io/rust-project-goals/2025h2/Rust-for-Linux-compiler.html)                   | [Tomas Sedovic][]    | [compiler] ([Wesley Wiser][])                                                             |
| [Getting Rust for Linux into stable Rust: language features](https://rust-lang.github.io/rust-project-goals/2025h2/Rust-for-Linux-language.html)                   | [Tomas Sedovic][]    | [lang] ([Josh Triplett][]), [lang-docs] ([TC][])                                    |
| [Borrow checking in a-mir-formality](https://rust-lang.github.io/rust-project-goals/2025h2/a-mir-formality.html)                                                   | [Niko Matsakis][]    | [types] ([Niko Matsakis][])                                                               |
| [Reborrow traits](https://rust-lang.github.io/rust-project-goals/2025h2/autoreborrow-traits.html)                                                                  | [Aapo Alasuutari][]        | [compiler] ([Oliver Scherer][]), [lang] ([Tyler Mandry][])                                              |
| [build-std](https://rust-lang.github.io/rust-project-goals/2025h2/build-std.html)                                                                                  | [David Wood][]       | [cargo] ([Eric Huss][]), [compiler] ([David Wood][]), [libs] ([Amanieu d'Antras][])                          |
| [Prototype Cargo build analysis](https://rust-lang.github.io/rust-project-goals/2025h2/cargo-build-analysis.html)                                                  | [Weihang Lo][]       | [cargo] ([Weihang Lo][])                                                                  |
| [Rework Cargo Build Dir Layout](https://rust-lang.github.io/rust-project-goals/2025h2/cargo-build-dir-layout.html)                                                 | [Ross Sullivan][]     | [cargo] ([Weihang Lo][])                                                                  |
| [Prototype a new set of Cargo "plumbing" commands](https://rust-lang.github.io/rust-project-goals/2025h2/cargo-plumbing.html)                                      | ![Help Wanted][] | [cargo]                                                                               |
| [Stabilize cargo-script](https://rust-lang.github.io/rust-project-goals/2025h2/cargo-script.html)                                                                  | [Ed Page][]           | [cargo] ([Ed Page][]), [compiler], [lang] ([Josh Triplett][]), [lang-docs] ([Josh Triplett][])     |
| [Continue resolving `cargo-semver-checks` blockers for merging into cargo](https://rust-lang.github.io/rust-project-goals/2025h2/cargo-semver-checks.html)         | [Predrag Gruevski][]      | [cargo] ([Ed Page][]), [rustdoc] ([Alona Enraght-Moony][])                                          |
| [Emit Retags in Codegen](https://rust-lang.github.io/rust-project-goals/2025h2/codegen_retags.html)                                                                | [Ian McCormack][]        | [compiler] ([Ralf Jung][]), [opsem] ([Ralf Jung][])                                           |
| [Comprehensive niche checks for Rust](https://rust-lang.github.io/rust-project-goals/2025h2/comprehensive-niche-checks.html)                                       | [Bastian Kersting][]          | [compiler] ([Ben Kimock][]), [opsem] ([Ben Kimock][])                                           |
| [Const Generics](https://rust-lang.github.io/rust-project-goals/2025h2/const-generics.html)                                                                        | [Boxy][]         | [lang] ([Niko Matsakis][])                                                                |
| [Ergonomic ref-counting: RFC decision and preview](https://rust-lang.github.io/rust-project-goals/2025h2/ergonomic-rc.html)                                        | [Niko Matsakis][]    | [compiler] ([Santiago Pastorino][]), [lang] ([Niko Matsakis][])                                      |
| [Evolving trait hierarchies](https://rust-lang.github.io/rust-project-goals/2025h2/evolving-traits.html)                                                           | [Taylor Cramer][]        | [compiler], [lang] ([Taylor Cramer][]), [libs-api], [types] ([Oliver Scherer][])                        |
| [Design a language feature to solve Field Projections](https://rust-lang.github.io/rust-project-goals/2025h2/field-projections.html)                               | [Benno Lossin][]     | [lang] ([Tyler Mandry][])                                                                     |
| [Finish the std::offload module](https://rust-lang.github.io/rust-project-goals/2025h2/finishing-gpu-offload.html)                                                 | [Manuel Drehwald][]          | [compiler] ([Manuel Drehwald][]), [lang] ([TC][])                                           |
| [Run more tests for GCC backend in the Rust's CI](https://rust-lang.github.io/rust-project-goals/2025h2/gcc-backend-tests.html)                                    | [Guillaume Gomez][]  | [compiler] ([Wesley Wiser][]), [infra] ([Marco Ieni][])                                       |
| [In-place initialization](https://rust-lang.github.io/rust-project-goals/2025h2/in-place-initialization.html)                                                      | [Alice Ryhl][]        | [lang] ([Taylor Cramer][])                                                                    |
| [C++/Rust Interop Problem Space Mapping](https://rust-lang.github.io/rust-project-goals/2025h2/interop-problem-map.html)                                           | [Jon Bauman][]         | [compiler] ([Oliver Scherer][]), [lang] ([Tyler Mandry][]), [libs] ([David Tolnay][]), [opsem]                  |
| [Finish the libtest json output experiment](https://rust-lang.github.io/rust-project-goals/2025h2/libtest-json.html)                                               | [Ed Page][]           | [cargo] ([Ed Page][]), [libs-api], [testing-devex]                                         |
| [MIR move elimination](https://rust-lang.github.io/rust-project-goals/2025h2/mir-move-elimination.html)                                                            | [Amanieu d'Antras][]         | [compiler], [lang] ([Amanieu d'Antras][]), [opsem], [wg-mir-opt]                                  |
| [Next-generation trait solver](https://rust-lang.github.io/rust-project-goals/2025h2/next-solver.html)                                                             | [lcnr][]            | [types] ([lcnr][])                                                                       |
| [Implement Open API Namespace Support](https://rust-lang.github.io/rust-project-goals/2025h2/open-namespaces.html)                                                 | ![Help Wanted][] | [cargo] ([Ed Page][]), [compiler] ([b-naber][]), [crates-io] ([Carol Nichols][])                 |
| [Promoting Parallel Front End](https://rust-lang.github.io/rust-project-goals/2025h2/parallel-front-end.html)                                                      | [Sparrow Li][]      | [compiler]                                                                            |
| [Continue Experimentation with Pin Ergonomics](https://rust-lang.github.io/rust-project-goals/2025h2/pin-ergonomics.html)                                          | [Frank King][]      | [compiler] ([Oliver Scherer][]), [lang] ([TC][])                                          |
| [Stabilizable Polonius support on nightly](https://rust-lang.github.io/rust-project-goals/2025h2/polonius.html)                                                    | [Rémy Rakic][]             | [types] ([Jack Huey][])                                                                   |
| [Production-ready cranelift backend](https://rust-lang.github.io/rust-project-goals/2025h2/production-ready-cranelift.html)                                        | [Folkert de Vries][]      | [compiler], [wg-compiler-performance]                                                 |
| [Stabilize public/private dependencies](https://rust-lang.github.io/rust-project-goals/2025h2/pub-priv.html)                                                       | ![Help Wanted][] | [cargo] ([Ed Page][]), [compiler]                                                          |
| [Expand the Rust Reference to specify more aspects of the Rust language](https://rust-lang.github.io/rust-project-goals/2025h2/reference-expansion.html)           | [Josh Triplett][]    | [lang-docs] ([Josh Triplett][]), [spec] ([Josh Triplett][])                                   |
| [reflection and comptime](https://rust-lang.github.io/rust-project-goals/2025h2/reflection-and-comptime.html)                                                      | [Oliver Scherer][]         | [compiler] ([Oliver Scherer][]), [lang] ([Scott McMurray][]), [libs] ([Josh Triplett][])                     |
| [Relink don't Rebuild](https://rust-lang.github.io/rust-project-goals/2025h2/relink-dont-rebuild.html)                                                             | [Jane Lusby][]           | [cargo], [compiler]                                                                   |
| [Rust Vision Document](https://rust-lang.github.io/rust-project-goals/2025h2/rust-vision-doc.html)                                                                 | [Niko Matsakis][]    | [leadership-council]                                                                  |
| [rustc-perf improvements](https://rust-lang.github.io/rust-project-goals/2025h2/rustc-perf-improvements.html)                                                      | [James][]    | [compiler], [infra]                                                                   |
| [Stabilize rustdoc `doc_cfg` feature](https://rust-lang.github.io/rust-project-goals/2025h2/rustdoc-doc-cfg.html)                                                  | [Guillaume Gomez][]  | [rustdoc] ([Guillaume Gomez][])                                                           |
| [Add a team charter for rustdoc team](https://rust-lang.github.io/rust-project-goals/2025h2/rustdoc-team-charter.html)                                             | [Guillaume Gomez][]  | [rustdoc] ([Guillaume Gomez][])                                                           |
| [SVE and SME on AArch64](https://rust-lang.github.io/rust-project-goals/2025h2/scalable-vectors.html)                                                              | [David Wood][]       | [compiler] ([David Wood][]), [lang] ([Niko Matsakis][]), [libs] ([Amanieu d'Antras][]), [types]           |
| [Rust Stabilization of MemorySanitizer and ThreadSanitizer Support](https://rust-lang.github.io/rust-project-goals/2025h2/stabilization-of-sanitizer-support.html) | [Jakob Koschel][]       | [bootstrap], [compiler], [infra], [project-exploit-mitigations]                       |
| [Type System Documentation](https://rust-lang.github.io/rust-project-goals/2025h2/typesystem-docs.html)                                                            | [Boxy][]         | [types] ([Boxy][])                                                                    |
| [Unsafe Fields](https://rust-lang.github.io/rust-project-goals/2025h2/unsafe-fields.html)                                                                          | [Jack Wrenn][]         | [compiler] ([Jack Wrenn][]), [lang] ([Scott McMurray][])                                             |


## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

### Goals broken out by champion

Who is championing which goals?

| Champion        | # | Goals                                                                                                                                                                                                                                                                                                                                                |
| :--             | :-- | :--                                                                                                                                                                                                                                                                                                                                                  |
| [Amanieu d'Antras][]        | 3 | ° [MIR move elimination](https://rust-lang.github.io/rust-project-goals/2025h2/mir-move-elimination.html)<br>° [SVE and SME on AArch64](https://rust-lang.github.io/rust-project-goals/2025h2/scalable-vectors.html)<br>° [build-std](https://rust-lang.github.io/rust-project-goals/2025h2/build-std.html)                                                                                                                                                                                                                  |
| [Guillaume Gomez][] | 2 | ° [Add a team charter for rustdoc team](https://rust-lang.github.io/rust-project-goals/2025h2/rustdoc-team-charter.html)<br>° [Stabilize rustdoc `doc_cfg` feature](https://rust-lang.github.io/rust-project-goals/2025h2/rustdoc-doc-cfg.html)                                                                                                                                                                                                                      |
| [Pete LeVasseur][]     | 1 | ° [Develop the capabilities to keep the FLS up to date](https://rust-lang.github.io/rust-project-goals/2025h2/FLS-up-to-date-capabilities.html)                                                                                                                                                                                                                                                              |
| [Ralf Jung][]       | 1 | ° [Emit Retags in Codegen](https://rust-lang.github.io/rust-project-goals/2025h2/codegen_retags.html)                                                                                                                                                                                                                                                                                                        |
| [Wesley Wiser][]    | 2 | ° [Getting Rust for Linux into stable Rust: compiler features](https://rust-lang.github.io/rust-project-goals/2025h2/Rust-for-Linux-compiler.html)<br>° [Run more tests for GCC backend in the Rust's CI](https://rust-lang.github.io/rust-project-goals/2025h2/gcc-backend-tests.html)                                                                                                                                                                              |
| [Manuel Drehwald][]         | 1 | ° [Finish the std::offload module](https://rust-lang.github.io/rust-project-goals/2025h2/finishing-gpu-offload.html)                                                                                                                                                                                                                                                                                         |
| [Alona Enraght-Moony][]  | 1 | ° [Continue resolving `cargo-semver-checks` blockers for merging into cargo](https://rust-lang.github.io/rust-project-goals/2025h2/cargo-semver-checks.html)                                                                                                                                                                                                                                                 |
| [b-naber][]        | 1 | ° [Implement Open API Namespace Support](https://rust-lang.github.io/rust-project-goals/2025h2/open-namespaces.html)                                                                                                                                                                                                                                                                                         |
| [Boxy][]        | 1 | ° [Type System Documentation](https://rust-lang.github.io/rust-project-goals/2025h2/typesystem-docs.html)                                                                                                                                                                                                                                                                                                    |
| [Carol Nichols][]  | 1 | ° [Implement Open API Namespace Support](https://rust-lang.github.io/rust-project-goals/2025h2/open-namespaces.html)                                                                                                                                                                                                                                                                                         |
| [Taylor Cramer][]       | 2 | ° [Evolving trait hierarchies](https://rust-lang.github.io/rust-project-goals/2025h2/evolving-traits.html)<br>° [In-place initialization](https://rust-lang.github.io/rust-project-goals/2025h2/in-place-initialization.html)                                                                                                                                                                                                                                        |
| [David Wood][]      | 2 | ° [SVE and SME on AArch64](https://rust-lang.github.io/rust-project-goals/2025h2/scalable-vectors.html)<br>° [build-std](https://rust-lang.github.io/rust-project-goals/2025h2/build-std.html)                                                                                                                                                                                                                                                                       |
| [David Tolnay][]        | 1 | ° [C++/Rust Interop Problem Space Mapping](https://rust-lang.github.io/rust-project-goals/2025h2/interop-problem-map.html)                                                                                                                                                                                                                                                                                   |
| [Eric Huss][]          | 1 | ° [build-std](https://rust-lang.github.io/rust-project-goals/2025h2/build-std.html)                                                                                                                                                                                                                                                                                                                          |
| [Ed Page][]          | 5 | ° [Continue resolving `cargo-semver-checks` blockers for merging into cargo](https://rust-lang.github.io/rust-project-goals/2025h2/cargo-semver-checks.html)<br>° [Finish the libtest json output experiment](https://rust-lang.github.io/rust-project-goals/2025h2/libtest-json.html)<br>° [Implement Open API Namespace Support](https://rust-lang.github.io/rust-project-goals/2025h2/open-namespaces.html)<br>° [Stabilize cargo-script](https://rust-lang.github.io/rust-project-goals/2025h2/cargo-script.html)<br>° [Stabilize public/private dependencies](https://rust-lang.github.io/rust-project-goals/2025h2/pub-priv.html)      |
| [Jack Huey][]       | 1 | ° [Stabilizable Polonius support on nightly](https://rust-lang.github.io/rust-project-goals/2025h2/polonius.html)                                                                                                                                                                                                                                                                                            |
| [Josh Triplett][]   | 4 | ° [Expand the Rust Reference to specify more aspects of the Rust language](https://rust-lang.github.io/rust-project-goals/2025h2/reference-expansion.html)<br>° [Getting Rust for Linux into stable Rust: language features](https://rust-lang.github.io/rust-project-goals/2025h2/Rust-for-Linux-language.html)<br>° [Stabilize cargo-script](https://rust-lang.github.io/rust-project-goals/2025h2/cargo-script.html)<br>° [reflection and comptime](https://rust-lang.github.io/rust-project-goals/2025h2/reflection-and-comptime.html)                                           |
| [Jack Wrenn][]        | 1 | ° [Unsafe Fields](https://rust-lang.github.io/rust-project-goals/2025h2/unsafe-fields.html)                                                                                                                                                                                                                                                                                                                  |
| [Jakub Beránek][]         | 1 | ° [Develop the capabilities to keep the FLS up to date](https://rust-lang.github.io/rust-project-goals/2025h2/FLS-up-to-date-capabilities.html)                                                                                                                                                                                                                                                              |
| [lcnr][]           | 1 | ° [Next-generation trait solver](https://rust-lang.github.io/rust-project-goals/2025h2/next-solver.html)                                                                                                                                                                                                                                                                                                     |
| [Marco Ieni][]      | 1 | ° [Run more tests for GCC backend in the Rust's CI](https://rust-lang.github.io/rust-project-goals/2025h2/gcc-backend-tests.html)                                                                                                                                                                                                                                                                            |
| [Niko Matsakis][]   | 5 | ° [Borrow checking in a-mir-formality](https://rust-lang.github.io/rust-project-goals/2025h2/a-mir-formality.html)<br>° [Const Generics](https://rust-lang.github.io/rust-project-goals/2025h2/const-generics.html)<br>° [Develop the capabilities to keep the FLS up to date](https://rust-lang.github.io/rust-project-goals/2025h2/FLS-up-to-date-capabilities.html)<br>° [Ergonomic ref-counting: RFC decision and preview](https://rust-lang.github.io/rust-project-goals/2025h2/ergonomic-rc.html)<br>° [SVE and SME on AArch64](https://rust-lang.github.io/rust-project-goals/2025h2/scalable-vectors.html)                           |
| [Oliver Scherer][]        | 5 | ° [C++/Rust Interop Problem Space Mapping](https://rust-lang.github.io/rust-project-goals/2025h2/interop-problem-map.html)<br>° [Continue Experimentation with Pin Ergonomics](https://rust-lang.github.io/rust-project-goals/2025h2/pin-ergonomics.html)<br>° [Evolving trait hierarchies](https://rust-lang.github.io/rust-project-goals/2025h2/evolving-traits.html)<br>° [Reborrow traits](https://rust-lang.github.io/rust-project-goals/2025h2/autoreborrow-traits.html)<br>° [reflection and comptime](https://rust-lang.github.io/rust-project-goals/2025h2/reflection-and-comptime.html)                                            |
| [Vadim Petrochenkov][]   | 1 | ° [Delegation](https://rust-lang.github.io/rust-project-goals/2025h2/delegation.html)                                                                                                                                                                                                                                                                                                                        |
| [Ben Kimock][]       | 1 | ° [Comprehensive niche checks for Rust](https://rust-lang.github.io/rust-project-goals/2025h2/comprehensive-niche-checks.html)                                                                                                                                                                                                                                                                               |
| [Scott McMurray][]       | 2 | ° [Unsafe Fields](https://rust-lang.github.io/rust-project-goals/2025h2/unsafe-fields.html)<br>° [reflection and comptime](https://rust-lang.github.io/rust-project-goals/2025h2/reflection-and-comptime.html)                                                                                                                                                                                                                                                       |
| [Santiago Pastorino][]     | 1 | ° [Ergonomic ref-counting: RFC decision and preview](https://rust-lang.github.io/rust-project-goals/2025h2/ergonomic-rc.html)                                                                                                                                                                                                                                                                                |
| [Tyler Mandry][]        | 4 | ° [C++/Rust Interop Problem Space Mapping](https://rust-lang.github.io/rust-project-goals/2025h2/interop-problem-map.html)<br>° [Design a language feature to solve Field Projections](https://rust-lang.github.io/rust-project-goals/2025h2/field-projections.html)<br>° [Emit Retags in Codegen](https://rust-lang.github.io/rust-project-goals/2025h2/codegen_retags.html)<br>° [Reborrow traits](https://rust-lang.github.io/rust-project-goals/2025h2/autoreborrow-traits.html)                                                                                                 |
| [TC][]    | 3 | ° [Continue Experimentation with Pin Ergonomics](https://rust-lang.github.io/rust-project-goals/2025h2/pin-ergonomics.html)<br>° [Finish the std::offload module](https://rust-lang.github.io/rust-project-goals/2025h2/finishing-gpu-offload.html)<br>° [Getting Rust for Linux into stable Rust: language features](https://rust-lang.github.io/rust-project-goals/2025h2/Rust-for-Linux-language.html)                                                                                                                    |
| [Weihang Lo][]      | 2 | ° [Prototype Cargo build analysis](https://rust-lang.github.io/rust-project-goals/2025h2/cargo-build-analysis.html)<br>° [Rework Cargo Build Dir Layout](https://rust-lang.github.io/rust-project-goals/2025h2/cargo-build-dir-layout.html)                                                                                                                                                                                                                          |


### Team asks

The following table highlights the asks from each affected team.
The "owner" in the column is the person expecting to do the design/implementation work that the team will be approving.


#### bootstrap team
| Goal                                                                                                       | [Ded. r?][valid_team_asks] |
| :--                                                                                                        | :-- |
| [Develop the capabilities to keep the FLS up to date](https://rust-lang.github.io/rust-project-goals/2025h2/FLS-up-to-date-capabilities.html)                      |     |
| [Rust Stabilization of MemorySanitizer and ThreadSanitizer Support](https://rust-lang.github.io/rust-project-goals/2025h2/stabilization-of-sanitizer-support.html) | ✅   |

#### cargo team
| Goal                                                                                               | [Design mtg.][valid_team_asks] | [RFC][valid_team_asks] |
| :--                                                                                                | :-- | :-- |
| [Continue resolving `cargo-semver-checks` blockers for merging into cargo](https://rust-lang.github.io/rust-project-goals/2025h2/cargo-semver-checks.html) |     |     |
| [Finish the libtest json output experiment](https://rust-lang.github.io/rust-project-goals/2025h2/libtest-json.html)                                       |     |     |
| [Implement Open API Namespace Support](https://rust-lang.github.io/rust-project-goals/2025h2/open-namespaces.html)                                         |     |     |
| [Prototype Cargo build analysis](https://rust-lang.github.io/rust-project-goals/2025h2/cargo-build-analysis.html)                                          |     |     |
| [Prototype a new set of Cargo "plumbing" commands](https://rust-lang.github.io/rust-project-goals/2025h2/cargo-plumbing.html)                              |     |     |
| [Relink don't Rebuild](https://rust-lang.github.io/rust-project-goals/2025h2/relink-dont-rebuild.html)                                                     |     |     |
| [Rework Cargo Build Dir Layout](https://rust-lang.github.io/rust-project-goals/2025h2/cargo-build-dir-layout.html)                                         |     |     |
| [Stabilize cargo-script](https://rust-lang.github.io/rust-project-goals/2025h2/cargo-script.html)                                                          |     |     |
| [Stabilize public/private dependencies](https://rust-lang.github.io/rust-project-goals/2025h2/pub-priv.html)                                               |     |     |
| [build-std](https://rust-lang.github.io/rust-project-goals/2025h2/build-std.html)                                                                          | \*1 | ✅   |


\*1: Review initial RFC draft ([from here](https://rust-lang.github.io/rust-project-goals/2025h2/build-std.html))

#### compiler team
| Goal                                                                                                       | [Ded. r?][valid_team_asks] | [Experiment][valid_team_asks] | [Design mtg.][valid_team_asks] | [RFC][valid_team_asks] | [MCP][valid_team_asks] | [Stabilize.][valid_team_asks] | [Policy][valid_team_asks] |
| :--                                                                                                        | :--       | :-- | :-- | :-- | :-- | :-- | :-- |
| [C++/Rust Interop Problem Space Mapping](https://rust-lang.github.io/rust-project-goals/2025h2/interop-problem-map.html)                                           |           |     |     |     |     |     |     |
| [Comprehensive niche checks for Rust](https://rust-lang.github.io/rust-project-goals/2025h2/comprehensive-niche-checks.html)                                       | [Ben Kimock][] |     |     |     | \*5 |     |     |
| [Continue Experimentation with Pin Ergonomics](https://rust-lang.github.io/rust-project-goals/2025h2/pin-ergonomics.html)                                          |           |     |     |     |     |     |     |
| [Emit Retags in Codegen](https://rust-lang.github.io/rust-project-goals/2025h2/codegen_retags.html)                                                                | \*1       |     | ✅   | ✅   |     |     |     |
| [Ergonomic ref-counting: RFC decision and preview](https://rust-lang.github.io/rust-project-goals/2025h2/ergonomic-rc.html)                                        |           |     |     |     |     |     |     |
| [Evolving trait hierarchies](https://rust-lang.github.io/rust-project-goals/2025h2/evolving-traits.html)                                                           |           |     |     |     |     |     |     |
| [Finish the std::offload module](https://rust-lang.github.io/rust-project-goals/2025h2/finishing-gpu-offload.html)                                                 |           |     |     |     |     |     |     |
| [Getting Rust for Linux into stable Rust: compiler features](https://rust-lang.github.io/rust-project-goals/2025h2/Rust-for-Linux-compiler.html)                   |           |     |     |     |     |     |     |
| ↳ Finish and stabilize a given `-Z...` flag                                                                |           |     |     |     |     | ✅   |     |
| [Implement Open API Namespace Support](https://rust-lang.github.io/rust-project-goals/2025h2/open-namespaces.html)                                                 |           |     |     |     |     |     |     |
| [MIR move elimination](https://rust-lang.github.io/rust-project-goals/2025h2/mir-move-elimination.html)                                                            |           |     |     | ✅   |     |     |     |
| [Production-ready cranelift backend](https://rust-lang.github.io/rust-project-goals/2025h2/production-ready-cranelift.html)                                        | \*2       |     |     |     |     |     |     |
| [Promoting Parallel Front End](https://rust-lang.github.io/rust-project-goals/2025h2/parallel-front-end.html)                                                      |           |     |     |     |     |     |     |
| [Reborrow traits](https://rust-lang.github.io/rust-project-goals/2025h2/autoreborrow-traits.html)                                                                  |           |     |     |     |     |     |     |
| [Relink don't Rebuild](https://rust-lang.github.io/rust-project-goals/2025h2/relink-dont-rebuild.html)                                                             |           |     | ✅   |     |     |     |     |
| [Run more tests for GCC backend in the Rust's CI](https://rust-lang.github.io/rust-project-goals/2025h2/gcc-backend-tests.html)                                    |           |     |     |     |     |     |     |
| [Rust Stabilization of MemorySanitizer and ThreadSanitizer Support](https://rust-lang.github.io/rust-project-goals/2025h2/stabilization-of-sanitizer-support.html) |           |     |     |     |     | ✅   |     |
| [SVE and SME on AArch64](https://rust-lang.github.io/rust-project-goals/2025h2/scalable-vectors.html)                                                              |           | \*6 |     |     |     |     |     |
| [Stabilize cargo-script](https://rust-lang.github.io/rust-project-goals/2025h2/cargo-script.html)                                                                  |           |     |     |     |     |     |     |
| ↳ Implement language feature `frontmatter`                                                                 |           |     |     |     |     |     |     |
| [Stabilize public/private dependencies](https://rust-lang.github.io/rust-project-goals/2025h2/pub-priv.html)                                                       |           |     |     |     |     |     |     |
| [Unsafe Fields](https://rust-lang.github.io/rust-project-goals/2025h2/unsafe-fields.html)                                                                          |           |     |     |     |     |     |     |
| [build-std](https://rust-lang.github.io/rust-project-goals/2025h2/build-std.html)                                                                                  |           |     | \*3 | ✅   |     |     |     |
| [reflection and comptime](https://rust-lang.github.io/rust-project-goals/2025h2/reflection-and-comptime.html)                                                      |           |     |     |     |     |     |     |
| ↳ Implement language feature                                                                               |           |     |     |     |     |     |     |
| [rustc-perf improvements](https://rust-lang.github.io/rust-project-goals/2025h2/rustc-perf-improvements.html)                                                      |           |     |     |     |     |     | \*4 |


\*1: Most of our changes are within `rustc_codegen_ssa`, but it would also be helpful to have feedback from someone familiar with how retags are handled within Miri's [`borrow_tracker`](https://doc.rust-lang.org/nightly/nightly-rustc/miri/borrow_tracker/index.html) module. ([from here](https://rust-lang.github.io/rust-project-goals/2025h2/codegen_retags.html))


\*2: Larger changes to `rustc_codegen_ssa`. While not strictly required, we think having a dedicated reviewer will speed up our progress. ([from here](https://rust-lang.github.io/rust-project-goals/2025h2/production-ready-cranelift.html))


\*3: Review initial RFC draft ([from here](https://rust-lang.github.io/rust-project-goals/2025h2/build-std.html))


\*4: Update performance regression policy ([from here](https://rust-lang.github.io/rust-project-goals/2025h2/rustc-perf-improvements.html))


\*5: Where to insert the check / checked load ([from here](https://rust-lang.github.io/rust-project-goals/2025h2/comprehensive-niche-checks.html))


\*6: Approve experiment of [rfcs#3838] ([from here](https://rust-lang.github.io/rust-project-goals/2025h2/scalable-vectors.html))

#### crates-io team
| Goal                                                       |
| :--                                                        |
| [Implement Open API Namespace Support](https://rust-lang.github.io/rust-project-goals/2025h2/open-namespaces.html) |

#### infra team
| Goal                                                                                                       | [Deploy][valid_team_asks] |
| :--                                                                                                        | :-- |
| [Run more tests for GCC backend in the Rust's CI](https://rust-lang.github.io/rust-project-goals/2025h2/gcc-backend-tests.html)                                    |     |
| [Rust Stabilization of MemorySanitizer and ThreadSanitizer Support](https://rust-lang.github.io/rust-project-goals/2025h2/stabilization-of-sanitizer-support.html) |     |
| [rustc-perf improvements](https://rust-lang.github.io/rust-project-goals/2025h2/rustc-perf-improvements.html)                                                      | \*1 |


\*1: rustc-perf improvements, testing infrastructure ([from here](https://rust-lang.github.io/rust-project-goals/2025h2/rustc-perf-improvements.html))

#### lang team
| Goal                                                                                     | [Experiment][valid_team_asks] | [Design mtg.][valid_team_asks] | [RFC][valid_team_asks] | [Stabilize.][valid_team_asks] |
| :--                                                                                      | :--           | :--                 | :--      | :-- |
| [C++/Rust Interop Problem Space Mapping](https://rust-lang.github.io/rust-project-goals/2025h2/interop-problem-map.html)                         |               |                     |          |     |
| [Const Generics](https://rust-lang.github.io/rust-project-goals/2025h2/const-generics.html)                                                      |               | \*3                 |          |     |
| [Continue Experimentation with Pin Ergonomics](https://rust-lang.github.io/rust-project-goals/2025h2/pin-ergonomics.html)                        |               | ✅                   |          |     |
| [Design a language feature to solve Field Projections](https://rust-lang.github.io/rust-project-goals/2025h2/field-projections.html)             | \*5           | \*4                 |          |     |
| [Develop the capabilities to keep the FLS up to date](https://rust-lang.github.io/rust-project-goals/2025h2/FLS-up-to-date-capabilities.html)    |               |                     |          |     |
| [Ergonomic ref-counting: RFC decision and preview](https://rust-lang.github.io/rust-project-goals/2025h2/ergonomic-rc.html)                      |               | \*7                 | \*8      |     |
| [Evolving trait hierarchies](https://rust-lang.github.io/rust-project-goals/2025h2/evolving-traits.html)                                         |               |                     |          | \*6 |
| [Finish the std::offload module](https://rust-lang.github.io/rust-project-goals/2025h2/finishing-gpu-offload.html)                               | ![Complete][] |                     |          |     |
| [Getting Rust for Linux into stable Rust: language features](https://rust-lang.github.io/rust-project-goals/2025h2/Rust-for-Linux-language.html) |               |                     |          |     |
| ↳ Finish and stabilize `arbitrary_self_types` and `derive_coerce_pointee`                |               |                     |          | ✅   |
| [In-place initialization](https://rust-lang.github.io/rust-project-goals/2025h2/in-place-initialization.html)                                    |               | Two design meetings |          |     |
| [MIR move elimination](https://rust-lang.github.io/rust-project-goals/2025h2/mir-move-elimination.html)                                          |               |                     | ✅        |     |
| [Reborrow traits](https://rust-lang.github.io/rust-project-goals/2025h2/autoreborrow-traits.html)                                                | \*2           |                     |          |     |
| [SVE and SME on AArch64](https://rust-lang.github.io/rust-project-goals/2025h2/scalable-vectors.html)                                            |               |                     | \*9 \*10 |     |
| [Stabilize cargo-script](https://rust-lang.github.io/rust-project-goals/2025h2/cargo-script.html)                                                |               |                     |          |     |
| ↳ Stabilize language feature `frontmatter`                                               |               |                     |          | ✅   |
| [Unsafe Fields](https://rust-lang.github.io/rust-project-goals/2025h2/unsafe-fields.html)                                                        |               | ✅                   | ✅        |     |
| [reflection and comptime](https://rust-lang.github.io/rust-project-goals/2025h2/reflection-and-comptime.html)                                    |               |                     |          |     |
| ↳ Design language feature to solve problem                                               | \*1           |                     |          |     |
| ↳ Implement language feature                                                             |               | ✅                   |          |     |


\*1: Needs libstd data structures (lang items) to make the specialization data available ([from here](https://rust-lang.github.io/rust-project-goals/2025h2/reflection-and-comptime.html))


\*2: allows coding pre-RFC; only for trusted contributors ([from here](https://rust-lang.github.io/rust-project-goals/2025h2/autoreborrow-traits.html))


\*3: topic: `adt_const_params` design ([from here](https://rust-lang.github.io/rust-project-goals/2025h2/const-generics.html))


\*4: Possibly more than one required as well as discussions on zulip. ([from here](https://rust-lang.github.io/rust-project-goals/2025h2/field-projections.html))


\*5: [Ding Xiang Fei][], [Benno Lossin][] ([from here](https://rust-lang.github.io/rust-project-goals/2025h2/field-projections.html))


\*6: Stabilizing `arbitrary_self_types`. Unblocked by new `Receiver` API. ([from here](https://rust-lang.github.io/rust-project-goals/2025h2/evolving-traits.html))


\*7: Two meetings to evaluate both approaches ([from here](https://rust-lang.github.io/rust-project-goals/2025h2/ergonomic-rc.html))


\*8: Choose between maximally additive vs seamlessly integrated ([from here](https://rust-lang.github.io/rust-project-goals/2025h2/ergonomic-rc.html))


\*9: Language team decide whether to accept [rfcs#3729] ([from here](https://rust-lang.github.io/rust-project-goals/2025h2/scalable-vectors.html))


\*10: Compiler/Library team decide whether to accept [rfcs#3838] ([from here](https://rust-lang.github.io/rust-project-goals/2025h2/scalable-vectors.html))

#### lang-docs team
| Goal                                                                                             | [Reference text][valid_team_asks] |
| :--                                                                                              | :--    |
| [Expand the Rust Reference to specify more aspects of the Rust language](https://rust-lang.github.io/rust-project-goals/2025h2/reference-expansion.html) |        |
| [Getting Rust for Linux into stable Rust: language features](https://rust-lang.github.io/rust-project-goals/2025h2/Rust-for-Linux-language.html)         |        |
| ↳ Finish and stabilize `arbitrary_self_types` and `derive_coerce_pointee`                        | ✅      |
| [Stabilize cargo-script](https://rust-lang.github.io/rust-project-goals/2025h2/cargo-script.html)                                                        |        |
| ↳ Stabilize language feature `frontmatter`                                                       | [Eric Huss][] |

#### leadership-council team
| Goal                                       |
| :--                                        |
| [Rust Vision Document](https://rust-lang.github.io/rust-project-goals/2025h2/rust-vision-doc.html) |

#### libs team
| Goal                                                             | [Experiment][valid_team_asks] | [Design mtg.][valid_team_asks] | [RFC][valid_team_asks] |
| :--                                                              | :-- | :-- | :-- |
| [C++/Rust Interop Problem Space Mapping](https://rust-lang.github.io/rust-project-goals/2025h2/interop-problem-map.html) |     |     |     |
| [SVE and SME on AArch64](https://rust-lang.github.io/rust-project-goals/2025h2/scalable-vectors.html)                    | \*3 |     |     |
| [build-std](https://rust-lang.github.io/rust-project-goals/2025h2/build-std.html)                                        |     | \*2 | ✅   |
| [reflection and comptime](https://rust-lang.github.io/rust-project-goals/2025h2/reflection-and-comptime.html)            |     |     |     |
| ↳ Design language feature to solve problem                       | \*1 |     |     |


\*1: Needs libstd data structures (lang items) to make the specialization data available ([from here](https://rust-lang.github.io/rust-project-goals/2025h2/reflection-and-comptime.html))


\*2: Review initial RFC draft ([from here](https://rust-lang.github.io/rust-project-goals/2025h2/build-std.html))


\*3: Approve experiment of [rfcs#3838] ([from here](https://rust-lang.github.io/rust-project-goals/2025h2/scalable-vectors.html))

#### libs-api team
| Goal                                                         | [Stabilize.][valid_team_asks] |
| :--                                                          | :-- |
| [Evolving trait hierarchies](https://rust-lang.github.io/rust-project-goals/2025h2/evolving-traits.html)             | \*1 |
| [Finish the libtest json output experiment](https://rust-lang.github.io/rust-project-goals/2025h2/libtest-json.html) |     |


\*1: Stabilizing `Receiver`. Unblocked by implementation. ([from here](https://rust-lang.github.io/rust-project-goals/2025h2/evolving-traits.html))

#### opsem team
| Goal                                                                                  | [Ded. r?][valid_team_asks] | [Design mtg.][valid_team_asks] | [RFC][valid_team_asks] |
| :--                                                                                   | :--       | :-- | :-- |
| [C++/Rust Interop Problem Space Mapping](https://rust-lang.github.io/rust-project-goals/2025h2/interop-problem-map.html)                      |           |     |     |
| [Comprehensive niche checks for Rust](https://rust-lang.github.io/rust-project-goals/2025h2/comprehensive-niche-checks.html)                  | [Ben Kimock][] |     |     |
| [Develop the capabilities to keep the FLS up to date](https://rust-lang.github.io/rust-project-goals/2025h2/FLS-up-to-date-capabilities.html) |           |     |     |
| [Emit Retags in Codegen](https://rust-lang.github.io/rust-project-goals/2025h2/codegen_retags.html)                                           | \*1       | ✅   | ✅   |
| [MIR move elimination](https://rust-lang.github.io/rust-project-goals/2025h2/mir-move-elimination.html)                                       |           | ✅   |     |


\*1: Most of our changes are within `rustc_codegen_ssa`, but it would also be helpful to have feedback from someone familiar with how retags are handled within Miri's [`borrow_tracker`](https://doc.rust-lang.org/nightly/nightly-rustc/miri/borrow_tracker/index.html) module. ([from here](https://rust-lang.github.io/rust-project-goals/2025h2/codegen_retags.html))

#### project-exploit-mitigations team
| Goal                                                                                                       | [Ded. r?][valid_team_asks] |
| :--                                                                                                        | :-- |
| [Rust Stabilization of MemorySanitizer and ThreadSanitizer Support](https://rust-lang.github.io/rust-project-goals/2025h2/stabilization-of-sanitizer-support.html) | ✅   |

#### rustdoc team
| Goal                                                                                               | [Org][valid_team_asks] |
| :--                                                                                                | :--                |
| [Add a team charter for rustdoc team](https://rust-lang.github.io/rust-project-goals/2025h2/rustdoc-team-charter.html)                                     | Write team charter |
| [Continue resolving `cargo-semver-checks` blockers for merging into cargo](https://rust-lang.github.io/rust-project-goals/2025h2/cargo-semver-checks.html) |                    |
| [Stabilize rustdoc `doc_cfg` feature](https://rust-lang.github.io/rust-project-goals/2025h2/rustdoc-doc-cfg.html)                                          |                    |

#### spec team
| Goal                                                                                             |
| :--                                                                                              |
| [Develop the capabilities to keep the FLS up to date](https://rust-lang.github.io/rust-project-goals/2025h2/FLS-up-to-date-capabilities.html)            |
| [Expand the Rust Reference to specify more aspects of the Rust language](https://rust-lang.github.io/rust-project-goals/2025h2/reference-expansion.html) |

#### testing-devex team
| Goal                                                         |
| :--                                                          |
| [Finish the libtest json output experiment](https://rust-lang.github.io/rust-project-goals/2025h2/libtest-json.html) |

#### types team
| Goal                                                                                  | [Ded. r?][valid_team_asks] | [FCP][valid_team_asks] |
| :--                                                                                   | :-- | :-- |
| [Borrow checking in a-mir-formality](https://rust-lang.github.io/rust-project-goals/2025h2/a-mir-formality.html)                              | \*2 |     |
| [Develop the capabilities to keep the FLS up to date](https://rust-lang.github.io/rust-project-goals/2025h2/FLS-up-to-date-capabilities.html) |     |     |
| [Evolving trait hierarchies](https://rust-lang.github.io/rust-project-goals/2025h2/evolving-traits.html)                                      |     |     |
| [Next-generation trait solver](https://rust-lang.github.io/rust-project-goals/2025h2/next-solver.html)                                        |     | \*1 |
| [SVE and SME on AArch64](https://rust-lang.github.io/rust-project-goals/2025h2/scalable-vectors.html)                                         | \*3 |     |
| [Stabilizable Polonius support on nightly](https://rust-lang.github.io/rust-project-goals/2025h2/polonius.html)                               |     |     |
| [Type System Documentation](https://rust-lang.github.io/rust-project-goals/2025h2/typesystem-docs.html)                                       |     |     |


\*1: for necessary refactorings ([from here](https://rust-lang.github.io/rust-project-goals/2025h2/next-solver.html))


\*2: Assign specific reviewers for Polonius Alpha model implementation ([Rémy Rakic][]) ([from here](https://rust-lang.github.io/rust-project-goals/2025h2/a-mir-formality.html))


\*3: Review Part II of Sized Hierarchy implementation ([from here](https://rust-lang.github.io/rust-project-goals/2025h2/scalable-vectors.html))

#### wg-compiler-performance team
| Goal                                                                | [Deploy][valid_team_asks] |
| :--                                                                 | :-- |
| [Production-ready cranelift backend](https://rust-lang.github.io/rust-project-goals/2025h2/production-ready-cranelift.html) | \*1 |


\*1: If possible, track and show `rustc_codegen_cranelift` performance. See note below for more details. ([from here](https://rust-lang.github.io/rust-project-goals/2025h2/production-ready-cranelift.html))

#### wg-mir-opt team
| Goal                                            | [Design mtg.][valid_team_asks] |
| :--                                             | :-- |
| [MIR move elimination](https://rust-lang.github.io/rust-project-goals/2025h2/mir-move-elimination.html) | ✅   |


### Definitions

Definitions for terms used above:

* *Author RFC* and *Implementation* means actually writing the code, document, whatever.
* *Design meeting* means holding a synchronous meeting to review a proposal and provide feedback (no decision expected).
* *RFC decisions* means reviewing an RFC and deciding whether to accept.
* *Org decisions* means reaching a decision on an organizational or policy matter.
* *Secondary review* of an RFC means that the team is "tangentially" involved in the RFC and should be expected to briefly review.
* *Stabilizations* means reviewing a stabilization and report and deciding whether to stabilize.
* *Standard reviews* refers to reviews for PRs against the repository; these PRs are not expected to be unduly large or complicated.
* Other kinds of decisions:
    * [Lang team experiments](https://lang-team.rust-lang.org/how_to/experiment.html) are used to add nightly features that do not yet have an RFC. They are limited to trusted contributors and are used to resolve design details such that an RFC can be written.
    * Compiler [Major Change Proposal (MCP)](https://forge.rust-lang.org/compiler/mcp.html) is used to propose a 'larger than average' change and get feedback from the compiler team.
    * Library [API Change Proposal (ACP)](https://std-dev-guide.rust-lang.org/development/feature-lifecycle.html) describes a change to the standard library.

## Frequently asked questions

### How are project goals proposed?

**Project goals** are proposed bottom-up by a **point of contact**, somebody who is willing to commit resources (time, money, leadership) to seeing the work get done. The point of contact identifies the problem they want to address and sketches the solution of how they want to do so. They also identify the support they will need from the Rust teams (typically things like review bandwidth or feedback on RFCs). Teams then read the goals and provide feedback. If the goal is approved, teams are committing to support the point of contact in their work.

### What goals were not accepted?

The following goals were not accepted as nobody stepped up to champion them. This should not be taken as a rejection of the underlying idea but likely indicates bandwidth constraints or concerns about scope.

| Goal                        | Point of contact | Team(s) and Champion(s)            |
| :--                         | :--           | :--                                |
| [Delegation](https://rust-lang.github.io/rust-project-goals/2025h2/delegation.html) | [Vadim Petrochenkov][] | [compiler] ([Vadim Petrochenkov][]), [lang] |


### Does accepting a goal mean that the work is going to happen for sure?

No. Accepting a goal is not a promise to accept an RFC, stabilize a feature, or take any other binding action. Rather, it means that the team wants the goal to make progress and is committing to commit time to complete the Team Asks described in the goal. To give some concrete examples, when the compiler team accepts a goal, they are committing to make sure reviews get done, but they are not committing to give an `r+` if the code doesn't pass muster. Similarly, the lang team is agreeing to discuss an RFC and provide actionable feedback, but not necessarily to accept it.

### What is a "team champion"? What do they do?

Team champions are people who have volunteered to track progress on the goal and to serve as a liaison between the goal owner(s) and the team. They are committing to support the owner to avoid the goal getting stuck in some kind of procedural limbo. For example, the goal champion might make sure the goal gets discussed in a meeting, or help to find a reviewer for a PR that is stuck in the queue. (In cases where the goal owner is also on the team, they can serve as their own champion.)

## What do the column names like "Ded. r?" mean?

[valid_team_asks]: #what-do-the-column-names-like-ded-r-mean

Those column names refer to specific things that can be asked of teams:

| Ask                            | aka            | Description                                                                                                                                                                                    |
| :--                            | :--            | :--                                                                                                                                                                                            |
| "Allocate funds"               | Alloc funds    | allocate funding                                                                                                                                                                               |
| "Discussion and moral support" | Good vibes     | approve of this direction and be prepared for light discussion on Zulip or elsewhere                                                                                                           |
| "Deploy to production"         | Deploy         | deploy code to production (e.g., on crates.io                                                                                                                                                  |
| "Standard reviews"             | r?             | review PRs (PRs are not expected to be unduly large or complicated)                                                                                                                            |
| "Dedicated reviewer"           | Ded. r?        | assign a specific person (or people) to review a series of PRs, appropriate for large or complex asks                                                                                          |
| "Lang-team experiment"         | Experiment     | begin a [lang-team experiment](https://lang-team.rust-lang.org/how_to/experiment.html) authorizing experimental impl of lang changes before an RFC is written; limited to trusted contributors |
| "Design meeting"               | Design mtg.    | hold a synchronous meeting to review a proposal and provide feedback (no decision expected)                                                                                                    |
| "RFC decision"                 | RFC            | review an RFC and deciding whether to accept                                                                                                                                                   |
| "RFC secondary review"         | RFC rev.       | briefly review an RFC without need of a formal decision                                                                                                                                        |
| "Org decision"                 | Org            | reach a decision on an organizational or policy matter                                                                                                                                         |
| "MCP decision"                 | MCP            | accept a [Major Change Proposal](https://forge.rust-lang.org/compiler/mcp.html)                                                                                                                |
| "ACP decision"                 | ACP            | accept an [API Change Proposal](https://std-dev-guide.rust-lang.org/development/feature-lifecycle.html)                                                                                        |
| "Review/revise Reference PR"   | Reference text | assign a lang-docs team liaison to finalize edits to Rust Reference                                                                                                                            |
| "Stabilization decision"       | Stabilize.     | reach a decision on a stabilization proposal                                                                                                                                                   |
| "Policy decision"              | Policy         | make a decision related to team policy                                                                                                                                                         |
| "FCP decision(s)"              | FCP            | make formal decision(s) that require 'checkboxes' and a FCP (Final Comment Period)                                                                                                             |
| "Blog post approval"           | Blog           | approve of posting about this on the main Rust blog                                                                                                                                            |
| "Miscellaneous"                | Misc           | do some one-off action as described in the notes                                                                                                                                               |


### Do goals have to have champions to be accepted?

Yes -- to be accepted, a goal needs some champions. They don't necessarily have to have a champion for *every team*, particularly not those with minor asks, but they do need to have enough champions that it seems the goal owner will be adequately supported. Those champions also need to not be too overloaded.

### How will we avoid taking on too many goals?

That's a tough one. Part of the reason to have champions is to help us filter out goals -- if one champion has too many goals, or nobody is willing to champion the goal, that's a bad sign.

[AGS]: ./Project-goal-slate.md
[AMF]: ./a-mir-formality.md
[Async]: ./async.md
[ATPIT]: ./ATPIT.md
[CS]: ./cargo-script.md
[CT]: ./const-traits.md
[ERC]: ./ergonomic-rc.md
[MGCA]: ./min_generic_const_arguments.md
[NBNLB]: ./Polonius.md
[NGS]: ./next-solver.md
[PET]: ./Patterns-of-empty-types.md
[PGC]: ./pubgrub-in-cargo.md
[RFL]: ./rfl_stable.md
[SBS]: ./sandboxed-build-script.md
[YKR]: ./yank-crates-with-a-reason.md
[SC]: ./Rust-for-SciComp.md
[OC]: ./optimize-clippy.md

<!-- Github usernames -->

[all]: https://www.rust-lang.org/governance/teams
[alumni]: https://www.rust-lang.org/governance/teams
[android]: https://www.rust-lang.org/governance/teams
[apple]: https://www.rust-lang.org/governance/teams
[arewewebyet]: https://www.rust-lang.org/governance/teams
[arm]: https://www.rust-lang.org/governance/teams
[arm-maintainers]: https://www.rust-lang.org/governance/teams
[book]: https://github.com/rust-lang/book
[bootstrap]: https://github.com/rust-lang/rust
[cargo]: https://github.com/rust-lang/cargo
[clippy]: https://github.com/rust-lang/rust-clippy
[clippy-contributors]: https://github.com/rust-lang/rust-clippy
[cloud-compute]: https://www.rust-lang.org/governance/teams
[codegen-c-maintainers]: https://github.com/rust-lang/rustc_codegen_c
[community]: https://github.com/rust-community/team
[community-content]: https://github.com/rust-community/content-team
[community-events]: https://github.com/rust-community/events-team
[community-localization]: https://github.com/rust-lang/community-localization
[community-rustbridge]: https://github.com/rustbridge/team
[community-survey]: https://github.com/rust-lang/surveys
[compiler]: http://github.com/rust-lang/compiler-team
[compiler-fcp]: http://github.com/rust-lang/compiler-team
[compiler-ops]: https://www.rust-lang.org/governance/teams
[content]: https://github.com/rust-lang/content-team
[cookbook]: https://github.com/rust-lang-nursery/rust-cookbook/
[council-librarians]: https://www.rust-lang.org/governance/teams
[crate-maintainers]: https://www.rust-lang.org/governance/teams
[crates-io]: https://github.com/rust-lang/crates.io
[crates-io-admins]: https://www.rust-lang.org/governance/teams
[crates-io-infra-admins]: https://www.rust-lang.org/governance/teams
[crates-io-on-call]: https://www.rust-lang.org/governance/teams
[devtools]: https://github.com/rust-dev-tools/dev-tools-team
[docker]: https://github.com/rust-lang/docker-rust/
[docs-rs]: https://github.com/rust-lang/docs.rs
[docs-rs-reviewers]: https://github.com/rust-lang/docs.rs
[edition]: http://github.com/rust-lang/edition-team
[emacs]: https://www.rust-lang.org/governance/teams
[emscripten]: https://www.rust-lang.org/governance/teams
[expect-test]: https://www.rust-lang.org/governance/teams
[foundation-board-project-directors]: https://www.rust-lang.org/governance/teams
[foundation-email-redirects]: https://www.rust-lang.org/governance/teams
[fuchsia]: https://www.rust-lang.org/governance/teams
[goal-owners]: https://www.rust-lang.org/governance/teams
[goals]: https://github.com/rust-lang/rust-project-goals
[gsoc-contributors]: https://www.rust-lang.org/governance/teams
[hiring]: https://www.rust-lang.org/governance/teams
[infra]: https://github.com/rust-lang/infra-team
[infra-admins]: https://www.rust-lang.org/governance/teams
[infra-bors]: https://github.com/rust-lang/bors
[inside-rust-reviewers]: https://www.rust-lang.org/governance/teams
[lang]: http://github.com/rust-lang/lang-team
[lang-advisors]: https://www.rust-lang.org/governance/teams
[lang-docs]: https://www.rust-lang.org/governance/teams
[lang-ops]: https://www.rust-lang.org/governance/teams
[launching-pad]: https://www.rust-lang.org/governance/teams
[leadership-council]: https://github.com/rust-lang/leadership-council
[leads]: https://www.rust-lang.org/governance/teams
[libs]: https://github.com/rust-lang/libs-team
[libs-api]: https://www.rust-lang.org/governance/teams
[libs-contributors]: https://www.rust-lang.org/governance/teams
[loongarch]: https://www.rust-lang.org/governance/teams
[mentors]: https://www.rust-lang.org/governance/teams
[mentorship]: https://www.rust-lang.org/governance/teams
[miri]: https://github.com/rust-lang/miri
[mods]: https://github.com/rust-lang/moderation-team
[mods-discourse]: https://www.rust-lang.org/governance/teams
[mods-venue]: https://www.rust-lang.org/governance/teams
[opsem]: https://github.com/rust-lang/opsem-team
[ospp]: https://www.rust-lang.org/governance/teams
[ospp-contributors]: https://www.rust-lang.org/governance/teams
[project-async-crashdump-debugging]: https://github.com/rust-lang/async-crashdump-debugging-initiative
[project-const-generics]: https://github.com/rust-lang/project-const-generics
[project-const-traits]: https://github.com/rust-lang/project-const-traits
[project-dyn-upcasting]: https://github.com/rust-lang/dyn-upcasting-coercion-initiative
[project-exploit-mitigations]: https://github.com/rust-lang/project-exploit-mitigations
[project-generic-associated-types]: https://github.com/rust-lang/generic-associated-types-initiative
[project-goal-reference-expansion]: https://www.rust-lang.org/governance/teams
[project-group-leads]: https://www.rust-lang.org/governance/teams
[project-impl-trait]: https://github.com/rust-lang/impl-trait-initiative
[project-keyword-generics]: https://github.com/rust-lang/keyword-generics-initiative
[project-negative-impls]: https://github.com/rust-lang/negative-impls-initiative
[project-portable-simd]: https://www.rust-lang.org/governance/teams
[project-stable-mir]: https://github.com/rust-lang/project-stable-mir
[project-trait-system-refactor]: https://github.com/rust-lang/types-team
[project-vision-doc-2025]: https://github.com/rust-lang/vision-doc-2025
[regex]: https://github.com/rust-lang/regex
[release]: https://github.com/rust-lang/release-team
[release-publishers]: https://github.com/rust-lang/release-team
[relnotes-interest-group]: https://www.rust-lang.org/governance/teams
[risc-v]: https://www.rust-lang.org/governance/teams
[rust-analyzer]: https://github.com/rust-lang/rust-analyzer
[rust-analyzer-contributors]: https://github.com/rust-lang/rust-analyzer
[rust-by-example]: https://github.com/rust-lang/rust-by-example
[rust-for-linux]: https://www.rust-lang.org/governance/teams
[rustconf-emails]: https://www.rust-lang.org/governance/teams
[rustdoc]: https://github.com/rust-lang/rust
[rustdoc-frontend]: https://www.rust-lang.org/governance/teams
[rustfmt]: https://github.com/rust-lang/rustfmt
[rustlings]: https://github.com/rust-lang/rustlings/
[rustup]: https://github.com/rust-lang/rustup
[social-media]: https://www.rust-lang.org/governance/teams
[spec]: https://github.com/rust-lang/spec
[spec-contributors]: https://github.com/rust-lang/spec
[style]: https://github.com/rust-lang/style-team
[team-repo-admins]: https://www.rust-lang.org/governance/teams
[testing-devex]: https://www.rust-lang.org/governance/teams
[triagebot]: https://github.com/rust-lang/triagebot
[twir]: https://github.com/rust-lang/this-week-in-rust
[twir-reviewers]: https://github.com/rust-lang/this-week-in-rust
[types]: https://github.com/rust-lang/types-team
[types-fcp]: https://github.com/rust-lang/types-team
[vim]: https://www.rust-lang.org/governance/teams
[wasi]: https://www.rust-lang.org/governance/teams
[wasm]: https://www.rust-lang.org/governance/teams
[web-presence]: https://www.rust-lang.org/governance/teams
[website]: https://github.com/rust-lang/www.rust-lang.org/
[wg-allocators]: https://github.com/rust-lang/wg-allocators
[wg-async]: https://github.com/rust-lang/wg-async
[wg-bindgen]: https://github.com/rust-lang/rust-bindgen
[wg-cli]: https://www.rust-lang.org/governance/teams
[wg-compiler-performance]: https://github.com/rust-lang/rustc-perf
[wg-const-eval]: https://github.com/rust-lang/const-eval
[wg-diagnostics]: https://forge.rust-lang.org/compiler/working-areas.html
[wg-embedded]: https://github.com/rust-embedded/wg
[wg-embedded-arm]: https://www.rust-lang.org/governance/teams
[wg-embedded-core]: https://www.rust-lang.org/governance/teams
[wg-embedded-hal]: https://www.rust-lang.org/governance/teams
[wg-embedded-infra]: https://www.rust-lang.org/governance/teams
[wg-embedded-libs]: https://www.rust-lang.org/governance/teams
[wg-embedded-linux]: https://www.rust-lang.org/governance/teams
[wg-embedded-msp430]: https://www.rust-lang.org/governance/teams
[wg-embedded-resources]: https://www.rust-lang.org/governance/teams
[wg-embedded-riscv]: https://www.rust-lang.org/governance/teams
[wg-embedded-tools]: https://www.rust-lang.org/governance/teams
[wg-embedded-triage]: https://www.rust-lang.org/governance/teams
[wg-ffi-unwind]: https://github.com/rust-lang/project-ffi-unwind
[wg-gamedev]: https://github.com/rust-gamedev
[wg-gcc-backend]: https://github.com/rust-lang/rustc_codegen_gcc
[wg-inline-asm]: https://github.com/rust-lang/project-inline-asm
[wg-leads]: https://www.rust-lang.org/governance/teams
[wg-llvm]: https://forge.rust-lang.org/compiler/working-areas.html
[wg-macros]: https://github.com/rust-lang/wg-macros
[wg-mir-opt]: https://forge.rust-lang.org/compiler/working-areas.html
[wg-parallel-rustc]: https://forge.rust-lang.org/compiler/working-areas.html
[wg-polonius]: https://forge.rust-lang.org/compiler/working-areas.html
[wg-rustc-dev-guide]: https://forge.rust-lang.org/compiler/working-areas.html
[wg-safe-transmute]: https://github.com/rust-lang/project-safe-transmute
[wg-secure-code]: https://github.com/rust-secure-code/wg
[wg-security-response]: https://github.com/rust-lang/wg-security-response
[wg-triage]: https://www.rust-lang.org/governance/teams
[windows]: https://www.rust-lang.org/governance/teams


[Bastian Kersting]: https://github.com/1c3t3a
[Amanieu d'Antras]: https://github.com/Amanieu
[Benno Lossin]: https://github.com/BennoLossin
[Boxy]: https://github.com/BoxyUwU
[Alice Ryhl]: https://github.com/Darksonn
[Guillaume Gomez]: https://github.com/GuillaumeGomez
[James]: https://github.com/Jamesbarford
[Pete LeVasseur]: https://github.com/PLeVasseur
[Ralf Jung]: https://github.com/RalfJung
[Sparrow Li]: https://github.com/SparrowLii
[Wesley Wiser]: https://github.com/WesleyWiser
[Manuel Drehwald]: https://github.com/ZuseZ4
[Aapo Alasuutari]: https://github.com/aapoalas
[Alona Enraght-Moony]: https://github.com/adotinthevoid
[b-naber]: https://github.com/b-naber
[Jon Bauman]: https://github.com/baumanj
[Boxy]: https://github.com/boxyuwu
[Carol Nichols]: https://github.com/carols10cents
[Taylor Cramer]: https://github.com/cramertj
[David Wood]: https://github.com/davidtwco
[Ding Xiang Fei]: https://github.com/dingxiangfei2009
[David Tolnay]: https://github.com/dtolnay
[Eric Huss]: https://github.com/ehuss
[Ed Page]: https://github.com/epage
[Folkert de Vries]: https://github.com/folkertdev
[Frank King]: https://github.com/frank-king
[Ian McCormack]: https://github.com/icmccorm
[Jack Huey]: https://github.com/jackh726
[Jakob Koschel]: https://github.com/jakos-sec
[Josh Triplett]: https://github.com/joshtriplett
[Jack Wrenn]: https://github.com/jswrenn
[Jakub Beránek]: https://github.com/kobzol
[lcnr]: https://github.com/lcnr
[Rémy Rakic]: https://github.com/lqd
[Marco Ieni]: https://github.com/marcoieni
[Niko Matsakis]: https://github.com/nikomatsakis
[Predrag Gruevski]: https://github.com/obi1kenobi
[Oliver Scherer]: https://github.com/oli-obk
[Vadim Petrochenkov]: https://github.com/petrochenkov
[Ross Sullivan]: https://github.com/ranger-ross
[Ben Kimock]: https://github.com/saethlin
[Scott McMurray]: https://github.com/scottmcm
[Santiago Pastorino]: https://github.com/spastorino
[Tyler Mandry]: https://github.com/tmandry
[Tomas Sedovic]: https://github.com/tomassedovic
[TC]: https://github.com/traviscross
[Weihang Lo]: https://github.com/weihanglo
[Jane Lusby]: https://github.com/yaahc


[Complete]: https://img.shields.io/badge/Complete-green
[Help wanted]: https://img.shields.io/badge/Help%20wanted-yellow
[Not funded]: https://img.shields.io/badge/Not%20yet%20funded-red
[TBD]: https://img.shields.io/badge/TBD-red
[Team]: https://img.shields.io/badge/Team%20ask-red

