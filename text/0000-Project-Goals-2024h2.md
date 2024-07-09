- Feature Name: N/A
- Start Date: 2024-07-09
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: N/A

## Summary

This RFC presents the Rust project goal slate for 2024H2. The slate consists of 24 total project goals of which we have selected 3 as our "flagship goals":

* Release the Rust 2024 edition (owner: [Travis Cross][])
* Bringing the Async Rust experience closer to parity with sync Rust (owners: [Tyler Mandry][], [Niko Matsakis][])
* Resolving the biggest blockers to Linux building on stable Rust (owners: [Josh Triplett][], [Niko Matsakis][])

Flagship goals represent the goals expected to have the broadest overall impact.

**This RFC follows an [unusual ratification procedure](https://rust-lang.zulipchat.com/#narrow/stream/435869-project-goals-2024h2/topic/Procedural.20next.20steps.20and.20timeline). Team leads are asked to review the [list of asks for their team](#reference-level-explanation) and confirm that their team is aligned. Leads should feel free to consult with team members and to raise concerns on their behalf. Once all team leads have signed off, the RFC will enter FCP.**

## Motivation

This RFC marks the first goal slate proposed under the experimental new roadmap process described in [RFC #3614](https://github.com/rust-lang/rfcs/pull/3614). It consists of NN project goals, of which we have selected three as **flagship goals**. Flagship goals represent the goals expected to have the broadest overall impact. 

### How the goal process works

**Project goals** are proposed bottom-up by an **owner**, somebody who is willing to commit resources (time, money, leadership) to seeing the work get done. The owner identifies the problem they want to address and sketches the solution of how they want to do so. They also identify the support they will need from the Rust teams (typically things like review bandwidth or feedback on RFCs). Teams then read the goals and provide feedback. If the goal is approved, teams are committing to support the owner in their work. 

Project goals can vary in scope from an internal refactoring that affects only one team to a larger cross-cutting initiative. No matter its scope, accepting a goal should never be interpreted as a promise that the team will make any future decision (e.g., accepting an RFC that has yet to be written). Rather, it is a promise that the team are aligned on the contents of the goal thus far (including the design axioms and other notes) and will prioritize giving feedback and support as needed.

Of the proposed goals, a small subset are selected by the roadmap owner as **flagship goals**. Flagship goals are chosen for their high impact (many Rust users will be impacted) and their shovel-ready nature (the org is well-aligned around a concrete plan). Flagship goals are the ones that will feature most prominently in our public messaging and which should be prioritized by Rust teams where needed.

### Rust’s mission

Our goals are selected to further Rust's mission of **empowering everyone to build reliable and efficient software**. Rust targets programs that prioritize

* reliability and robustness;
* performance, memory usage, and resource consumption; and
* long-term maintenance and extensibility.

We consider "any two out of the three" to the right heuristic for projects where Rust is a strong contender or possibly the best option.

### Axioms for selecting goals

We believe that...

* **Rust must deliver on its promise of peak performance and high reliability.** Rust’s maximum advantage is in applications that require peak performance or low-level systems capabilities. We must continue to innovate and support those areas above all.
* **Rust's goals require high productivity and ergonomics.** Being attentive to ergonomics broadens Rust impact by making it more appealing for projects that value reliability and maintenance but which don't have strict performance requirements.
* **Slow and steady wins the race.** For this first round of goals, we want a small set that can be completed without undue stress. As the Rust open source org continues to grow, the set of goals can grow in size.

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

### Flagship goals

The flagship goals proposed for this roadmap are as follows:

* [**Release the Rust 2024 edition**](https://rust-lang.github.io/rust-project-goals/2024h2/./Rust-2024-Edition.html), which will contain
    * a change in how `impl Trait` capture bounds work ([RFC #3498](https://github.com/rust-lang/rfcs/pull/3498) and [RFC #3617](https://github.com/rust-lang/rfcs/pull/3617))
    * reserving the `gen` keyword to allow for generators ([RFC #3513](https://github.com/rust-lang/rfcs/pull/3513))
    * never type fallback ([#123748](https://github.com/rust-lang/rust/issue/123748))
    * and a [number of other potential changes](https://github.com/rust-lang/rust/issues?q=label%3AC-tracking-issue+label%3AA-edition-2024+label%3AS-tracking-ready-to-stabilize%2CS-tracking-needs-documentation+-label%3AS-tracking-impl-incomplete%2CS-tracking-design-concerns) that may be included if they make enough progress
* [**Bringing the Async Rust experience closer to parity with sync Rust**](https://rust-lang.github.io/rust-project-goals/2024h2/./async.html) via:
    * resolving the "send bound problem", thus enabling foundational, generic traits like Tower's [`Service`]() trait;
    * stabilizing async closures, thus enabling richer, combinator APIs like sync Rust's [`Iterator`](https://doc.rust-lang.org/std/iter/trait.Iterator.html);
    * reorganizing the async WG, so the project can benefit from a group of async rust experts with deep knowledge of the space that can align around a shared vision.
* [**Resolving the biggest blockers to Linux building on stable Rust**](https://rust-lang.github.io/rust-project-goals/2024h2/./rfl_stable.html) via:
    * stabilizing support for arbitrary `self` types and unsizeable smart pointers, thus permitting ergonomic support for [in-place linked lists](https://rust-for-linux.com/arc-in-the-linux-kernel) on stable;
    * stabilizing features for labeled goto in inline assembler and extended `offset_of!` support, needed for various bts of low-level coding;
    * adding Rust For Linux project on Rust CI, thus ensuring we don't accidentally cause regressions for this highly visible project (done!);
    * stabilizing support for pointers to statics in constants, permitting the construction of vtables for kernel modules;

[MCP 727]: https://github.com/rust-lang/compiler-team/issues/727

#### Why these particular flagship goals?

**2024 Edition.** 2024 will mark the 4th Rust edition, following on the 2015, 2018, and 2021 editions. Similar to the [2021 edition](https://rust-lang.github.io/rust-project-goals/2024h2/https://github.com/nikomatsakis/rfcs/blob/rfl-project-goal/text/3085-edition-2021.html), the 2024 edition is not a "major marketing push" but rather an opportunity to correct small ergonomic issues with Rust that will make it overall much easier to use. The changes planned for the 2024 edition will (1) support `-> impl Trait` and `async fn` in traits by aligning capture behavior; (2) permit (async) generators to be added in the future by reserving the `gen` keyword; and (3) alter fallback for the `!` type.

**Async.** In 2024 we plan to deliver several critical async Rust building block features, most notably support for *async closures* and *`Send` bounds*. This is part of a multi-year program aiming to raise the experience of authoring "async Rust" to the same level of quality as "sync Rust". Async Rust is a crucial growth area, with 52% of the respondents in the [2023 Rust survey](https://blog.rust-lang.org/2024/02/19/2023-Rust-Annual-Survey-2023-results.html) indicating that they use Rust to build server-side or backend applications. 

**Rust for Linux.** The [experimental support for Rust development in the Linux kernel][RFL.com] is a watershed moment for Rust, demonstrating to the world that Rust is indeed capable of targeting all manner of low-level systems applications. And yet today that support rests on a [number of unstable features][RFL#2], blocking the effort from ever going beyond experimental status. For 2024H2 we will work to close the largest gaps that block support.

[RFL.com]: https://rust-for-linux.com/
[RFL#2]: https://github.com/Rust-for-Linux/linux/issues/2


### Project goals

The slate of additional project goals are as follows. These goals all have identified owners who will drive the work forward as well as a viable work plan. The goals include asks from the listed Rust teams, which are cataloged in the [reference-level explanation](#reference-level-explanation) section below. Some goals are actively looking for volunteers; these goals are tagged with ![Help wanted][].

| Goal                                                                                                      | Owner                | Team                          |
| ---                                                                                                       | ---                  | ---                           |
| [Const traits](https://rust-lang.github.io/rust-project-goals/2024h2/const-traits.html)                                                                           | [@fee1-dead][]           | [Types], [Lang]               |
| [Patterns of empty types](https://rust-lang.github.io/rust-project-goals/2024h2/Patterns-of-empty-types.html)                                                     | [@Nadrieril][]           | [Lang]                        |
| [Stabilize cargo-script](https://rust-lang.github.io/rust-project-goals/2024h2/cargo-script.html)                                                                 | [Ed Page][]               | [Cargo], [Lang]               |
| [Next-generation trait solver](https://rust-lang.github.io/rust-project-goals/2024h2/next-solver.html)                                                            | [lcnr][]                | [Types]                       |
| [Assemble project goal slate](https://rust-lang.github.io/rust-project-goals/2024h2/Project-goal-slate.html)                                                      | [Niko Matsakis][]        | [LC]                          |
| [Optimizing Clippy & linting](https://rust-lang.github.io/rust-project-goals/2024h2/optimize-clippy.html)                                                         | [Alejandra González][]             | [Clippy]                      |
| [Make Rustdoc Search easier to learn](https://rust-lang.github.io/rust-project-goals/2024h2/rustdoc-search.html)                                                  | [Michael Howell][]           | [Rustdoc], [Rustdoc-Frontend] |
| ["Stabilizable" prototype for expanded const generics](https://rust-lang.github.io/rust-project-goals/2024h2/min_generic_const_arguments.html)                    | [Boxy][]             | [Types]                       |
| [Explore sandboxed build scripts](https://rust-lang.github.io/rust-project-goals/2024h2/sandboxed-build-script.html)                                              | [Weihang Lo][]           | [Cargo]                       |
| [Stabilize parallel front end](https://rust-lang.github.io/rust-project-goals/2024h2/parallel-front-end.html)                                                     | [Sparrow Li (LiYuan)][]          | [Compiler]                    |
| [Scalable Polonius support on nightly](https://rust-lang.github.io/rust-project-goals/2024h2/Polonius.html)                                                       | [Rémy Rakic][]                 | [Types]                       |
| [Extend pubgrub to match cargo's dependency resolution](https://rust-lang.github.io/rust-project-goals/2024h2/pubgrub-in-cargo.html)                              | [Jacob Finkelman][]              | [Cargo]                       |
| [Use annotate-snippets for rustc diagnostic output](https://rust-lang.github.io/rust-project-goals/2024h2/annotate-snippets.html)                                 | [Esteban Kuber][], [Scott Schafer][] | [Compiler]                    |
| [Stabilize doc_cfg](https://rust-lang.github.io/rust-project-goals/2024h2/doc_cfg.html)                                                                           | [Guillaume Gomez][]      | [Rustdoc]                     |
| [Implement "merged doctests" to save doctest time](https://rust-lang.github.io/rust-project-goals/2024h2/merged-doctests.html)                                    | [Guillaume Gomez][]      | [Rustdoc]                     |
| [Testing infra + contributors for a-mir-formality](https://rust-lang.github.io/rust-project-goals/2024h2/a-mir-formality.html)                                    | [Niko Matsakis][]        | [Types]                       |
| [Ergonomic ref-counting](https://rust-lang.github.io/rust-project-goals/2024h2/ergonomic-rc.html)                                                                 | [Jonathan Kelley][]          | [Lang], [Libs-API]            |
| [Associated type position impl trait (https://rust-lang.github.io/rust-project-goals/2024h2/ATPIT)](ATPIT.html)                                                   | [Oli Scherer][]             | [Types], [Lang]               |
| [Expose experimental LLVM features for automatic differentiation and GPU offloading](https://rust-lang.github.io/rust-project-goals/2024h2/Rust-for-SciComp.html) | [Manuel Drehwald][]              | [Lang], [Compiler]            |
| [Administrator-provided reasons for yanked crates](https://rust-lang.github.io/rust-project-goals/2024h2/yank-crates-with-a-reason.html)                          | [二手掉包工程师][]           | [crates.io], [Cargo]          |


### Orphaned goals ![Help wanted][]

Goals in this section are "pre-approved" by the team but lack an owner. These indicate a place where we are looking for someone to step up and help drive the goal the goal to completion. Every orphaned goal has someone who is willing and able to serve as mentor, but lacks the time or resources to truly *own* the goal. If you are interested in serving as the owner for one of these orphaned goals, reach out to the listed mentor to discuss. Orphaned goals may also be used as the basis of applying for grants from the Rust Foundation or elsewhere.

| Goal                                                                    | Owner            | Team                |
| ---                                                                     | ---              | ---                 |
| [Experiment with relaxing the Orphan Rule](https://rust-lang.github.io/rust-project-goals/2024h2/Relaxing-the-Orphan-Rule.html) | ![Help wanted][] | [Lang][], [Types][] |


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The following table highlights the asks from each affected team.
The "owner" in the column is the person expecting to do the design/implementation work that the team will be approving.


### Cargo team
| Goal                                                                                     | Owner      | Notes |
| ---                                                                                      | ---        | --- |
| *Approve RFC*                                                                            |            |  |
| ↳ [Yank crates with a reason](https://rust-lang.github.io/rust-project-goals/2024h2/yank-crates-with-a-reason.html#ownership-and-team-asks)      | [二手掉包工程师][] |  |
| *Discussion and moral support*                                                           |            |  |
| ↳ [Explore sandboxed build scripts](https://rust-lang.github.io/rust-project-goals/2024h2/sandboxed-build-script.html#ownership-and-team-asks)   | [Weihang Lo][] |  |
| *Stabilization decision*                                                                 |            |  |
| ↳ [Stabilize cargo-script](https://rust-lang.github.io/rust-project-goals/2024h2/cargo-script.html#ownership-and-team-asks)                      | [Ed Page][]     |  |
| ↳ [Yank crates with a reason](https://rust-lang.github.io/rust-project-goals/2024h2/yank-crates-with-a-reason.html#ownership-and-team-asks)      | [二手掉包工程师][] |  |
| *Standard reviews*                                                                       |            |  |
| ↳ [Explore sandboxed build scripts](https://rust-lang.github.io/rust-project-goals/2024h2/sandboxed-build-script.html#ownership-and-team-asks)   | [Weihang Lo][] |  |

### Clippy team
| Goal                                                                | Owner    | Notes |
| ---                                                                 | ---      | --- |
| *Standard reviews*                                                  |          |  |
| ↳ [Optimization work](https://rust-lang.github.io/rust-project-goals/2024h2/optimize-clippy.html#ownership-and-team-asks)   | [Alejandra González][] |  |

### Compiler team
| Goal                                                                                                                                  | Owner                | Notes     |
| ---                                                                                                                                   | ---                  | ---       |
| *Collaboration with GSoC proc-macro project*                                                                                          |                      |           |
| ↳ [Explore sandboxed build scripts](https://rust-lang.github.io/rust-project-goals/2024h2/sandboxed-build-script.html#ownership-and-team-asks)                                                | [Weihang Lo][]           |           |
| *Discussion and moral support*                                                                                                        |                      |           |
| ↳ [Stabilize parallel front end](https://rust-lang.github.io/rust-project-goals/2024h2/parallel-front-end.html#ownership-and-team-asks)                                                       | [Sparrow Li (LiYuan)][]          |           |
| *Policy decision*                                                                                                                     |                      |           |
| ↳ [~~RFL on Rust CI~~](https://rust-lang.github.io/rust-project-goals/2024h2/rfl_stable.html#ownership-and-team-asks)                                                                         | [Jakub Beránek][]              |           |
| *Standard reviews*                                                                                                                    |                      |           |
| ↳ [Patterns of empty types](https://rust-lang.github.io/rust-project-goals/2024h2/Patterns-of-empty-types.html#ownership-and-team-asks)                                                       | [@Nadrieril][]           |           |
| ↳ [Async drop experiments](https://rust-lang.github.io/rust-project-goals/2024h2/async.html#ownership-and-team-asks)                                                                          | [Vadim Petrochenkov][]        |           |
| ↳ [Arbitrary self types v2](https://rust-lang.github.io/rust-project-goals/2024h2/rfl_stable.html#ownership-and-team-asks)                                                                    | [Adrian Taylor][]           |           |
| ↳ [Use annotate-snippets for rustc diagnostic output](https://rust-lang.github.io/rust-project-goals/2024h2/annotate-snippets.html#ownership-and-team-asks)                                   | [Esteban Kuber][], [Scott Schafer][] |           |
| ↳ [Ergonomic ref-counting](https://rust-lang.github.io/rust-project-goals/2024h2/ergonomic-rc.html#ownership-and-team-asks)                                                                   | [Jonathan Kelley][]          |           |
| ↳ [Expose experimental LLVM features for automatic differentiation and GPU offloading](https://rust-lang.github.io/rust-project-goals/2024h2/Rust-for-SciComp.html#ownership-and-team-asks)   | [Manuel Drehwald][]              |           |
| *dedicated reviewer*                                                                                                                  |                      |           |
| ↳ [Production use of annotate-snippets](https://rust-lang.github.io/rust-project-goals/2024h2/annotate-snippets.html#ownership-and-team-asks)                                                 | [Esteban Kuber][], [Scott Schafer][] | [Esteban Kuber][] |

### Infra team
| Goal                                                                  | Owner      | Notes |
| ---                                                                   | ---        | --- |
| *Collecting popular queries for review*                               |            |  |
| ↳ [Feedback and testing](https://rust-lang.github.io/rust-project-goals/2024h2/rustdoc-search.html#ownership-and-team-asks)   | [Michael Howell][] |  |
| *Inside Rust blog post inviting feedback*                             |            |  |
| ↳ [Feedback and testing](https://rust-lang.github.io/rust-project-goals/2024h2/rustdoc-search.html#ownership-and-team-asks)   | [Michael Howell][] |  |

### Leadership Council
| Goal                                                                                                  | Owner                | Notes |
| ---                                                                                                   | ---                  | --- |
| *Inside Rust blog post inviting feedback*                                                             |                      |  |
| ↳ [Assemble project goal slate](https://rust-lang.github.io/rust-project-goals/2024h2/Project-goal-slate.html#ownership-and-team-asks)                        | [Niko Matsakis][]        |  |
| *RFC decision*                                                                                        |                      |  |
| ↳ [Assemble project goal slate](https://rust-lang.github.io/rust-project-goals/2024h2/Project-goal-slate.html#ownership-and-team-asks)                        | [Niko Matsakis][]        |  |
| *Top-level Rust blog post*                                                                            |                      |  |
| ↳ [Rust 2024 Edition](https://rust-lang.github.io/rust-project-goals/2024h2/Rust-2024-Edition.html#ownership-and-team-asks)                                   | [Travis Cross][]         |  |
| *Top-level Rust blog post inviting feedback*                                                          |                      |  |
| ↳ [Assemble project goal slate](https://rust-lang.github.io/rust-project-goals/2024h2/Project-goal-slate.html#ownership-and-team-asks)                        | [Niko Matsakis][]        |  |
| ↳ [Make Rustdoc Search easier to learn](https://rust-lang.github.io/rust-project-goals/2024h2/rustdoc-search.html#ownership-and-team-asks)                    | [Michael Howell][]           |  |
| ↳ [Use annotate-snippets for rustc diagnostic output](https://rust-lang.github.io/rust-project-goals/2024h2/annotate-snippets.html#ownership-and-team-asks)   | [Esteban Kuber][], [Scott Schafer][] |  |

### Lang team
| Goal                                                                                                                                  | Owner            | Notes              |
| ---                                                                                                                                   | ---              | ---                |
| *Design meeting*                                                                                                                      |                  |                    |
| ↳ [Patterns of empty types](https://rust-lang.github.io/rust-project-goals/2024h2/Patterns-of-empty-types.html#ownership-and-team-asks)                                                       | [@Nadrieril][]       |                    |
| ↳ ["Stabilizable" prototype for expanded const generics](https://rust-lang.github.io/rust-project-goals/2024h2/min_generic_const_arguments.html#ownership-and-team-asks)                      | [Boxy][]         | Up to 1, if needed |
| ↳ [Experiment with relaxing the Orphan Rule](https://rust-lang.github.io/rust-project-goals/2024h2/Relaxing-the-Orphan-Rule.html#ownership-and-team-asks)                                     | ![Help wanted][] | Up to 1, if needed |
| ↳ [Ergonomic ref-counting](https://rust-lang.github.io/rust-project-goals/2024h2/ergonomic-rc.html#ownership-and-team-asks)                                                                   | [Jonathan Kelley][]      |                    |
| *Discussion and moral support*                                                                                                        |                  |                    |
| ↳ [Const traits](https://rust-lang.github.io/rust-project-goals/2024h2/const-traits.html#ownership-and-team-asks)                                                                             | [@fee1-dead][]       |                    |
| *Lang-team experiment*                                                                                                                |                  |                    |
| ↳ [Expose experimental LLVM features for automatic differentiation and GPU offloading](https://rust-lang.github.io/rust-project-goals/2024h2/Rust-for-SciComp.html#ownership-and-team-asks)   | [Manuel Drehwald][]          | (approved)         |
| *Org decision*                                                                                                                        |                  |                    |
| ↳ [Async WG reorganization](https://rust-lang.github.io/rust-project-goals/2024h2/async.html#ownership-and-team-asks)                                                                         | [Niko Matsakis][]    |                    |
| *RFC decision*                                                                                                                        |                  |                    |
| ↳ [Patterns of empty types](https://rust-lang.github.io/rust-project-goals/2024h2/Patterns-of-empty-types.html#ownership-and-team-asks)                                                       | [@Nadrieril][]       |                    |
| ↳ ["Send bound" problem](https://rust-lang.github.io/rust-project-goals/2024h2/async.html#ownership-and-team-asks)                                                                            | [Niko Matsakis][]    | ![Complete][]      |
| ↳ [Async closures](https://rust-lang.github.io/rust-project-goals/2024h2/async.html#ownership-and-team-asks)                                                                                  | [Michael Goulet][] |                    |
| ↳ [Derive smart pointer](https://rust-lang.github.io/rust-project-goals/2024h2/rfl_stable.html#ownership-and-team-asks)                                                                       | [Alice Ryhl][]        |                    |
| ↳ [Ergonomic ref-counting](https://rust-lang.github.io/rust-project-goals/2024h2/ergonomic-rc.html#ownership-and-team-asks)                                                                   | [Jonathan Kelley][]      |                    |
| *Secondary RFC review*                                                                                                                |                  |                    |
| ↳ [Trait for async iteration](https://rust-lang.github.io/rust-project-goals/2024h2/async.html#ownership-and-team-asks)                                                                       | [Eric Holk][]           |                    |
| *Stabilization*                                                                                                                       |                  |                    |
| ↳ ["Send bound" problem](https://rust-lang.github.io/rust-project-goals/2024h2/async.html#ownership-and-team-asks)                                                                            | [Niko Matsakis][]    |                    |
| *Stabilization decision*                                                                                                              |                  |                    |
| ↳ [Patterns of empty types](https://rust-lang.github.io/rust-project-goals/2024h2/Patterns-of-empty-types.html#ownership-and-team-asks)                                                       | [@Nadrieril][]       |                    |
| ↳ [Stabilize cargo-script](https://rust-lang.github.io/rust-project-goals/2024h2/cargo-script.html#ownership-and-team-asks)                                                                   | [Ed Page][]           |                    |
| ↳ [Arbitrary self types v2](https://rust-lang.github.io/rust-project-goals/2024h2/rfl_stable.html#ownership-and-team-asks)                                                                    | [Adrian Taylor][]       |                    |
| ↳ [Derive smart pointer](https://rust-lang.github.io/rust-project-goals/2024h2/rfl_stable.html#ownership-and-team-asks)                                                                       | [Alice Ryhl][]        |                    |
| ↳ [`asm_goto`](https://rust-lang.github.io/rust-project-goals/2024h2/rfl_stable.html#ownership-and-team-asks)                                                                                 | [Gary Guo][]        |                    |
| ↳ [Pointers to static in constants](https://rust-lang.github.io/rust-project-goals/2024h2/rfl_stable.html#ownership-and-team-asks)                                                            | [Niko Matsakis][]    |                    |
| ↳ [Rust 2024 Edition](https://rust-lang.github.io/rust-project-goals/2024h2/Rust-2024-Edition.html#ownership-and-team-asks)                                                                   | [Travis Cross][]     |                    |
| ↳ [Associated type position impl trait (https://rust-lang.github.io/rust-project-goals/2024h2/ATPIT)](ATPIT.html#ownership-and-team-asks)                                                     | [Oli Scherer][]         |                    |

### Libs team
| Goal                                                            | Owner         | Notes |
| ---                                                             | ---           | --- |
| *Org decision*                                                  |               |  |
| ↳ [Async WG reorganization](https://rust-lang.github.io/rust-project-goals/2024h2/async.html#ownership-and-team-asks)   | [Niko Matsakis][] |  |

### Libs-API team
| Goal                                                                     | Owner             | Notes |
| ---                                                                      | ---               | --- |
| *RFC decision*                                                           |                   |  |
| ↳ [Trait for async iteration](https://rust-lang.github.io/rust-project-goals/2024h2/async.html#ownership-and-team-asks)          | [Eric Holk][]            |  |
| ↳ [Ergonomic ref-counting](https://rust-lang.github.io/rust-project-goals/2024h2/ergonomic-rc.html#ownership-and-team-asks)      | [Jonathan Kelley][]       |  |
| *Stabilization decision*                                                 |                   |  |
| ↳ [Extended `offset_of` syntax](https://rust-lang.github.io/rust-project-goals/2024h2/rfl_stable.html#ownership-and-team-asks)   | [wieDasDing][] |  |

### Rust-Analyzer team
| Goal                                                                                 | Owner | Notes |
| ---                                                                                  | ---   | --- |
| *Standard reviews*                                                                   |       |  |
| ↳ [Stabilize coherence coherence support](https://rust-lang.github.io/rust-project-goals/2024h2/next-solver.html#ownership-and-team-asks)    | [lcnr][] |  |
| ↳ [Support in rust-analyzer](https://rust-lang.github.io/rust-project-goals/2024h2/next-solver.html#ownership-and-team-asks)                 | [lcnr][] |  |

### Rustdoc team
| Goal                                                                                               | Owner           | Notes |
| ---                                                                                                | ---             | --- |
| *Discussion and moral support*                                                                     |                 |  |
| ↳ [Make Rustdoc Search easier to learn](https://rust-lang.github.io/rust-project-goals/2024h2/rustdoc-search.html#ownership-and-team-asks)                 | [Michael Howell][]      |  |
| *RFC decision*                                                                                     |                 |  |
| ↳ [Stabilize doc_cfg](https://rust-lang.github.io/rust-project-goals/2024h2/doc_cfg.html#ownership-and-team-asks)                                          | [Guillaume Gomez][] |  |
| *Standard reviews*                                                                                 |                 |  |
| ↳ [Stabilize doc_cfg](https://rust-lang.github.io/rust-project-goals/2024h2/doc_cfg.html#ownership-and-team-asks)                                          | [Guillaume Gomez][] |  |
| ↳ [Implement "merged doctests" to save doctest time](https://rust-lang.github.io/rust-project-goals/2024h2/merged-doctests.html#ownership-and-team-asks)   | [Guillaume Gomez][] |  |

### Rustdoc-Frontend team
| Goal                                                                                  | Owner      | Notes |
| ---                                                                                   | ---        | --- |
| *Design meeting*                                                                      |            |  |
| ↳ [Improve on any discovered weaknesses](https://rust-lang.github.io/rust-project-goals/2024h2/rustdoc-search.html#ownership-and-team-asks)   | [Michael Howell][] |  |
| *FCP review*                                                                          |            |  |
| ↳ [Improve on any discovered weaknesses](https://rust-lang.github.io/rust-project-goals/2024h2/rustdoc-search.html#ownership-and-team-asks)   | [Michael Howell][] |  |
| *Standard reviews*                                                                    |            |  |
| ↳ [Improve on any discovered weaknesses](https://rust-lang.github.io/rust-project-goals/2024h2/rustdoc-search.html#ownership-and-team-asks)   | [Michael Howell][] |  |

### Types team
| Goal                                                                                                | Owner            | Notes              |
| ---                                                                                                 | ---              | ---                |
| *Design meeting*                                                                                    |                  |                    |
| ↳ [Experiment with relaxing the Orphan Rule](https://rust-lang.github.io/rust-project-goals/2024h2/Relaxing-the-Orphan-Rule.html#ownership-and-team-asks)   | ![Help wanted][] | Up to 1, if needed |
| *Discussion and moral support*                                                                      |                  |                    |
| ↳ [Const traits](https://rust-lang.github.io/rust-project-goals/2024h2/const-traits.html#ownership-and-team-asks)                                           | [@fee1-dead][]       |                    |
| ↳ [Next-generation trait solver](https://rust-lang.github.io/rust-project-goals/2024h2/next-solver.html#ownership-and-team-asks)                            | [lcnr][]            |                    |
| *FCP decisions*                                                                                     |                  |                    |
| ↳ [Associated type position impl trait (https://rust-lang.github.io/rust-project-goals/2024h2/ATPIT)](ATPIT.html#ownership-and-team-asks)                   | [Oli Scherer][]         |                    |
| *Stabilization decision*                                                                            |                  |                    |
| ↳ [Stabilize coherence coherence support](https://rust-lang.github.io/rust-project-goals/2024h2/next-solver.html#ownership-and-team-asks)                   | [lcnr][]            |                    |
| ↳ [Rust 2024 Edition](https://rust-lang.github.io/rust-project-goals/2024h2/Rust-2024-Edition.html#ownership-and-team-asks)                                 | [Travis Cross][]     |                    |
| ↳ [Associated type position impl trait (https://rust-lang.github.io/rust-project-goals/2024h2/ATPIT)](ATPIT.html#ownership-and-team-asks)                   | [Oli Scherer][]         |                    |
| *Standard reviews*                                                                                  |                  |                    |
| ↳ [Stabilize coherence coherence support](https://rust-lang.github.io/rust-project-goals/2024h2/next-solver.html#ownership-and-team-asks)                   | [lcnr][]            |                    |
| ↳ [Support in rust-analyzer](https://rust-lang.github.io/rust-project-goals/2024h2/next-solver.html#ownership-and-team-asks)                                | [lcnr][]            |                    |
| ↳ [Scalable Polonius support on nightly](https://rust-lang.github.io/rust-project-goals/2024h2/Polonius.html#ownership-and-team-asks)                       | [Rémy Rakic][]             | [@matthewjasper][]     |
| ↳ [Testing infra + contributors for a-mir-formality](https://rust-lang.github.io/rust-project-goals/2024h2/a-mir-formality.html#ownership-and-team-asks)    | [Niko Matsakis][]    |                    |

### crates.io team
| Goal                                                                                  | Owner      | Notes |
| ---                                                                                   | ---        | --- |
| *Approve RFC*                                                                         |            |  |
| ↳ [Yank crates with a reason](https://rust-lang.github.io/rust-project-goals/2024h2/yank-crates-with-a-reason.html#ownership-and-team-asks)   | [二手掉包工程师][] |  |
| *Standard reviews*                                                                    |            |  |
| ↳ [Yank crates with a reason](https://rust-lang.github.io/rust-project-goals/2024h2/yank-crates-with-a-reason.html#ownership-and-team-asks)   | [二手掉包工程师][] |  |
| *Try it out in crates.io*                                                             |            |  |
| ↳ [Yank crates with a reason](https://rust-lang.github.io/rust-project-goals/2024h2/yank-crates-with-a-reason.html#ownership-and-team-asks)   | [二手掉包工程师][] |  |


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

<!-- Goals -->

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


[Boxy]: https://github.com/BoxyUwU
[Alice Ryhl]: https://github.com/Darksonn
[Guillaume Gomez]: https://github.com/GuillaumeGomez
[Jakub Beránek]: https://github.com/Kobzol
[Scott Schafer]: https://github.com/Muscraft
[@Nadrieril]: https://github.com/Nadrieril
[Sparrow Li (LiYuan)]: https://github.com/SparrowLii
[Manuel Drehwald]: https://github.com/ZuseZ4
[Adrian Taylor]: https://github.com/adetaylor
[Alejandra González]: https://github.com/blyxyas
[Michael Goulet]: https://github.com/compiler-errors
[wieDasDing]: https://github.com/dingxiangfei2009
[Jacob Finkelman]: https://github.com/eh2406
[Eric Holk]: https://github.com/eholk
[Ed Page]: https://github.com/epage
[Esteban Kuber]: https://github.com/estebank
[@fee1-dead]: https://github.com/fee1-dead
[二手掉包工程师]: https://github.com/hi-rustin
[Jonathan Kelley]: https://github.com/jkelleyrtp
[Josh Triplett]: https://github.com/joshtriplett
[lcnr]: https://github.com/lcnr
[Rémy Rakic]: https://github.com/lqd
[@matthewjasper]: https://github.com/matthewjasper
[Gary Guo]: https://github.com/nbdd0121
[Niko Matsakis]: https://github.com/nikomatsakis
[Michael Howell]: https://github.com/notriddle
[Oli Scherer]: https://github.com/oli-obk
[Vadim Petrochenkov]: https://github.com/petrochenkov
[Tyler Mandry]: https://github.com/tmandry
[Travis Cross]: https://github.com/traviscross
[Weihang Lo]: https://github.com/weihanglo


[Cargo]: https://www.rust-lang.org/governance/teams/dev-tools#team-cargo
[Clippy]: https://www.rust-lang.org/governance/teams/dev-tools#team-clippy
[Compiler]: https://www.rust-lang.org/governance/teams/compiler
[Complete]: https://img.shields.io/badge/Complete-green
[Help wanted]: https://img.shields.io/badge/Help%20wanted-yellow
[Infra]: https://www.rust-lang.org/governance/teams/infra
[LC]: https://www.rust-lang.org/governance/teams/leadership-council
[Lang]: https://www.rust-lang.org/governance/teams/lang
[Libs]: https://www.rust-lang.org/governance/teams/library
[Libs-API]: https://www.rust-lang.org/governance/teams/library#team-libs-api
[Not funded]: https://img.shields.io/badge/Not%20yet%20funded-red
[Rustdoc]: https://www.rust-lang.org/governance/teams/dev-tools#team-rustdoc
[Rustdoc-Frontend]: https://www.rust-lang.org/governance/teams/dev-tools#team-rustdoc-frontend
[TBD]: https://img.shields.io/badge/TBD-red
[Team]: https://img.shields.io/badge/Team%20ask-red
[Types]: https://www.rust-lang.org/governance/teams/compiler#team-types
[crates.io]: https://www.rust-lang.org/governance/teams/dev-tools#team-crates-io
