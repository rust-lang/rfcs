- Feature Name: N/A
- Start Date: 2024-07-09
- RFC PR: [rust-lang/rfcs#3672](https://github.com/rust-lang/rfcs/pull/3672)
- Rust Issue: N/A

## Summary

This RFC presents the Rust project goal slate for 2024H2. The slate consists of 26 total project goals of which we have selected 3 as our "flagship goals":

* Release the Rust 2024 edition (owner: [TC][])
* Bring the Async Rust experience closer to parity with sync Rust (owners: [Tyler Mandry][], [Niko Matsakis][])
* Resolve the biggest blockers to Linux building on stable Rust (owners: [Josh Triplett][], [Niko Matsakis][])

Flagship goals represent the goals expected to have the broadest overall impact.

**This RFC follows an [unusual ratification procedure](https://rust-lang.zulipchat.com/#narrow/stream/435869-project-goals-2024h2/topic/Procedural.20next.20steps.20and.20timeline). Team leads are asked to review the [list of asks for their team](#reference-level-explanation) and confirm that their team is aligned. Leads should feel free to consult with team members and to raise concerns on their behalf. Once all team leads have signed off, the RFC will enter FCP.**

## Motivation

This RFC marks the first goal slate proposed under the experimental new roadmap process described in [RFC #3614](https://github.com/rust-lang/rfcs/pull/3614). It consists of 26 project goals, of which we have selected three as **flagship goals**. Flagship goals represent the goals expected to have the broadest overall impact. 

### How the goal process works

**Project goals** are proposed bottom-up by an **owner**, somebody who is willing to commit resources (time, money, leadership) to seeing the work get done. The owner identifies the problem they want to address and sketches the solution of how they want to do so. They also identify the support they will need from the Rust teams (typically things like review bandwidth or feedback on RFCs). Teams then read the goals and provide feedback. If the goal is approved, teams are committing to support the owner in their work. 

Project goals can vary in scope from an internal refactoring that affects only one team to a larger cross-cutting initiative. No matter its scope, accepting a goal should never be interpreted as a promise that the team will make any future decision (e.g., accepting an RFC that has yet to be written). Rather, it is a promise that the team are aligned on the contents of the goal thus far (including the design axioms and other notes) and will prioritize giving feedback and support as needed.

Of the proposed goals, a small subset are selected by the roadmap owner as **flagship goals**. Flagship goals are chosen for their high impact (many Rust users will be impacted) and their shovel-ready nature (the org is well-aligned around a concrete plan). Flagship goals are the ones that will feature most prominently in our public messaging and which should be prioritized by Rust teams where needed.

### Rust’s mission

Our goals are selected to further Rust's mission of **empowering everyone to build reliable and efficient software**. Rust targets programs that prioritize

* reliability and robustness;
* performance, memory usage, and resource consumption; and
* long-term maintenance and extensibility.

We consider "any two out of the three" as the right heuristic for projects where Rust is a strong contender or possibly the best option.

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
* [**Bring the Async Rust experience closer to parity with sync Rust**](https://rust-lang.github.io/rust-project-goals/2024h2/./async.html) via:
    * resolving the "send bound problem", thus enabling foundational, generic traits like Tower's [`Service`](https://docs.rs/tower-service/latest/tower_service/trait.Service.html) trait;
    * stabilizing async closures, thus enabling richer, combinator APIs like sync Rust's [`Iterator`](https://doc.rust-lang.org/std/iter/trait.Iterator.html);
    * reorganizing the async WG, so the project can benefit from a group of async rust experts with deep knowledge of the space that can align around a shared vision.
* [**Resolve the biggest blockers to Linux building on stable Rust**](https://rust-lang.github.io/rust-project-goals/2024h2/./rfl_stable.html) via:
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

| Goal                                                                                                      | Owner                | Team                           |
| ---                                                                                                       | ---                  | ---                            |
| ["Stabilizable" prototype for expanded const generics](https://rust-lang.github.io/rust-project-goals/2024h2/min_generic_const_arguments.html)                    | [Boxy][]             | [lang], [types]                |
| [Administrator-provided reasons for yanked crates](https://rust-lang.github.io/rust-project-goals/2024h2/yank-crates-with-a-reason.html)                          | [二手掉包工程师][]           | [cargo], [crates-io]           |
| [Assemble project goal slate](https://rust-lang.github.io/rust-project-goals/2024h2/Project-goal-slate.html)                                                      | [Niko Matsakis][]        | [leadership-council]           |
| [Associated type position impl trait](https://rust-lang.github.io/rust-project-goals/2024h2/ATPIT.html)                                                           | [Oliver Scherer][]             | [lang], [types]                |
| [Begin resolving `cargo-semver-checks` blockers for merging into cargo](https://rust-lang.github.io/rust-project-goals/2024h2/cargo-semver-checks.html)           | [Predrag Gruevski][]          | [cargo]                        |
| [Const traits](https://rust-lang.github.io/rust-project-goals/2024h2/const-traits.html)                                                                           | [Deadbeef][]           | [lang], [types]                |
| [Ergonomic ref-counting](https://rust-lang.github.io/rust-project-goals/2024h2/ergonomic-rc.html)                                                                 | [Jonathan Kelley][]          | [compiler], [lang], [libs-api] |
| [Explore sandboxed build scripts](https://rust-lang.github.io/rust-project-goals/2024h2/sandboxed-build-script.html)                                              | [Weihang Lo][]           | [cargo], [compiler]            |
| [Expose experimental LLVM features for automatic differentiation and GPU offloading](https://rust-lang.github.io/rust-project-goals/2024h2/Rust-for-SciComp.html) | [Manuel Drehwald][]              | [compiler], [lang]             |
| [Extend pubgrub to match cargo's dependency resolution](https://rust-lang.github.io/rust-project-goals/2024h2/pubgrub-in-cargo.html)                              | [Jacob Finkelman][]              | [cargo]                        |
| [Implement "merged doctests" to save doctest time](https://rust-lang.github.io/rust-project-goals/2024h2/merged-doctests.html)                                    | [Guillaume Gomez][]      | [rustdoc]                      |
| [Make Rustdoc Search easier to learn](https://rust-lang.github.io/rust-project-goals/2024h2/rustdoc-search.html)                                                  | [Michael Howell][]           | [rustdoc], [rustdoc-frontend]  |
| [Next-generation trait solver](https://rust-lang.github.io/rust-project-goals/2024h2/next-solver.html)                                                            | [lcnr][]                | [rust-analyzer], [types]       |
| [Optimizing Clippy & linting](https://rust-lang.github.io/rust-project-goals/2024h2/optimize-clippy.html)                                                         | [Alejandra González][]             | [clippy]                       |
| [Patterns of empty types](https://rust-lang.github.io/rust-project-goals/2024h2/Patterns-of-empty-types.html)                                                     | [@Nadrieril][]           | [compiler], [lang]             |
| [Scalable Polonius support on nightly](https://rust-lang.github.io/rust-project-goals/2024h2/Polonius.html)                                                       | [Rémy Rakic][]                 | [types]                        |
| [Stabilize cargo-script](https://rust-lang.github.io/rust-project-goals/2024h2/cargo-script.html)                                                                 | [Ed Page][]               | [cargo], [lang]                |
| [Stabilize doc_cfg](https://rust-lang.github.io/rust-project-goals/2024h2/doc_cfg.html)                                                                           | [Guillaume Gomez][]      | [rustdoc]                      |
| [Stabilize parallel front end](https://rust-lang.github.io/rust-project-goals/2024h2/parallel-front-end.html)                                                     | [Sparrow Li][]          | [compiler]                     |
| [Survey tools suitability for Std safety verification](https://rust-lang.github.io/rust-project-goals/2024h2/std-verification.html)                               | [Celina V.][]            | [libs]                         |
| [Testing infra + contributors for a-mir-formality](https://rust-lang.github.io/rust-project-goals/2024h2/a-mir-formality.html)                                    | [Niko Matsakis][]        | [types]                        |
| [Use annotate-snippets for rustc diagnostic output](https://rust-lang.github.io/rust-project-goals/2024h2/annotate-snippets.html)                                 | [Esteban Kuber][], [Scott Schafer][] | [compiler]                     |


### Orphaned goals ![Help wanted][]

Goals in this section are "pre-approved" by the team but lack an owner. These indicate a place where we are looking for someone to step up and help drive the goal the goal to completion. Every orphaned goal has someone who is willing and able to serve as mentor, but lacks the time or resources to truly *own* the goal. If you are interested in serving as the owner for one of these orphaned goals, reach out to the listed mentor to discuss. Orphaned goals may also be used as the basis of applying for grants from the Rust Foundation or elsewhere.

| Goal                                        | Owner            | Team    |
| ---                                         | ---              | ---     |
| [User-wide build cache](https://rust-lang.github.io/rust-project-goals/2024h2/user-wide-cache.html) | ![Help wanted][] | [cargo] |


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The following table highlights the asks from each affected team.
The "owner" in the column is the person expecting to do the design/implementation work that the team will be approving.


### cargo team
| Goal                                                                                                                        | Owner            | Notes |
| ---                                                                                                                         | ---              | --- |
| *Approve RFC*                                                                                                               |                  |  |
| ↳ [Yank crates with a reason](https://rust-lang.github.io/rust-project-goals/2024h2/yank-crates-with-a-reason.html#ownership-and-team-asks)                                         | [二手掉包工程师][]       |  |
| *Design meeting*                                                                                                            |                  |  |
| ↳ [User-wide caching](https://rust-lang.github.io/rust-project-goals/2024h2/user-wide-cache.html#ownership-and-team-asks)                                                           | ![Help wanted][] |  |
| *Discussion and moral support*                                                                                              |                  |  |
| ↳ [Explore sandboxed build scripts](https://rust-lang.github.io/rust-project-goals/2024h2/sandboxed-build-script.html#ownership-and-team-asks)                                      | [Weihang Lo][]       |  |
| ↳ [Extend pubgrub to match cargo's dependency resolution](https://rust-lang.github.io/rust-project-goals/2024h2/pubgrub-in-cargo.html#ownership-and-team-asks)                      | [Jacob Finkelman][]          |  |
| ↳ [Begin resolving `cargo-semver-checks` blockers for merging into cargo](https://rust-lang.github.io/rust-project-goals/2024h2/cargo-semver-checks.html#ownership-and-team-asks)   | [Predrag Gruevski][]      |  |
| *Stabilization decision*                                                                                                    |                  |  |
| ↳ [Stabilize cargo-script](https://rust-lang.github.io/rust-project-goals/2024h2/cargo-script.html#ownership-and-team-asks)                                                         | [Ed Page][]           |  |
| ↳ [Yank crates with a reason](https://rust-lang.github.io/rust-project-goals/2024h2/yank-crates-with-a-reason.html#ownership-and-team-asks)                                         | [二手掉包工程师][]       |  |
| *Standard reviews*                                                                                                          |                  |  |
| ↳ [User-wide caching](https://rust-lang.github.io/rust-project-goals/2024h2/user-wide-cache.html#ownership-and-team-asks)                                                           | ![Help wanted][] |  |
| ↳ [Explore sandboxed build scripts](https://rust-lang.github.io/rust-project-goals/2024h2/sandboxed-build-script.html#ownership-and-team-asks)                                      | [Weihang Lo][]       |  |

### clippy team
| Goal                                                                | Owner    | Notes |
| ---                                                                 | ---      | --- |
| *Standard reviews*                                                  |          |  |
| ↳ [Optimization work](https://rust-lang.github.io/rust-project-goals/2024h2/optimize-clippy.html#ownership-and-team-asks)   | [Alejandra González][] |  |

### compiler team
| Goal                                                                                                                                  | Owner                | Notes                          |
| ---                                                                                                                                   | ---                  | ---                            |
| *Collaboration with GSoC proc-macro project*                                                                                          |                      |                                |
| ↳ [Explore sandboxed build scripts](https://rust-lang.github.io/rust-project-goals/2024h2/sandboxed-build-script.html#ownership-and-team-asks)                                                | [Weihang Lo][]           |                                |
| *Discussion and moral support*                                                                                                        |                      |                                |
| ↳ [Stabilize parallel front end](https://rust-lang.github.io/rust-project-goals/2024h2/parallel-front-end.html#ownership-and-team-asks)                                                       | [Sparrow Li][]          |                                |
| *Policy decision*                                                                                                                     |                      |                                |
| ↳ [~~RFL on Rust CI~~](https://rust-lang.github.io/rust-project-goals/2024h2/rfl_stable.html#ownership-and-team-asks)                                                                         | [Jakub Beránek][]              |                                |
| *Standard reviews*                                                                                                                    |                      |                                |
| ↳ [Patterns of empty types](https://rust-lang.github.io/rust-project-goals/2024h2/Patterns-of-empty-types.html#ownership-and-team-asks)                                                       | [@Nadrieril][]           |                                |
| ↳ [Async drop experiments](https://rust-lang.github.io/rust-project-goals/2024h2/async.html#ownership-and-team-asks)                                                                          | [Vadim Petrochenkov][]        |                                |
| ↳ [Arbitrary self types v2](https://rust-lang.github.io/rust-project-goals/2024h2/rfl_stable.html#ownership-and-team-asks)                                                                    | [Adrian Taylor][]           |                                |
| ↳ [Use annotate-snippets for rustc diagnostic output](https://rust-lang.github.io/rust-project-goals/2024h2/annotate-snippets.html#ownership-and-team-asks)                                   | [Esteban Kuber][], [Scott Schafer][] |                                |
| ↳ [Ergonomic ref-counting](https://rust-lang.github.io/rust-project-goals/2024h2/ergonomic-rc.html#ownership-and-team-asks)                                                                   | [Jonathan Kelley][]          |                                |
| ↳ [Expose experimental LLVM features for automatic differentiation and GPU offloading](https://rust-lang.github.io/rust-project-goals/2024h2/Rust-for-SciComp.html#ownership-and-team-asks)   | [Manuel Drehwald][]              |                                |
| *dedicated reviewer*                                                                                                                  |                      |                                |
| ↳ [Production use of annotate-snippets](https://rust-lang.github.io/rust-project-goals/2024h2/annotate-snippets.html#ownership-and-team-asks)                                                 | [Esteban Kuber][], [Scott Schafer][] | [Esteban Kuber][] will be the reviewer |

### crates-io team
| Goal                                                                                  | Owner      | Notes |
| ---                                                                                   | ---        | --- |
| *Approve RFC*                                                                         |            |  |
| ↳ [Yank crates with a reason](https://rust-lang.github.io/rust-project-goals/2024h2/yank-crates-with-a-reason.html#ownership-and-team-asks)   | [二手掉包工程师][] |  |
| *Standard reviews*                                                                    |            |  |
| ↳ [Yank crates with a reason](https://rust-lang.github.io/rust-project-goals/2024h2/yank-crates-with-a-reason.html#ownership-and-team-asks)   | [二手掉包工程师][] |  |
| *Try it out in crates.io*                                                             |            |  |
| ↳ [Yank crates with a reason](https://rust-lang.github.io/rust-project-goals/2024h2/yank-crates-with-a-reason.html#ownership-and-team-asks)   | [二手掉包工程师][] |  |

### lang team
| Goal                                                                                                                                  | Owner            | Notes               |
| ---                                                                                                                                   | ---              | ---                 |
| *Design meeting*                                                                                                                      |                  |                     |
| ↳ [Async closures](https://rust-lang.github.io/rust-project-goals/2024h2/async.html#ownership-and-team-asks)                                                                                  | [Michael Goulet][] | 2 meetings expected |
| ↳ [Trait for async iteration](https://rust-lang.github.io/rust-project-goals/2024h2/async.html#ownership-and-team-asks)                                                                       | [Eric Holk][]           | 2 meetings expected |
| ↳ [Async drop experiments](https://rust-lang.github.io/rust-project-goals/2024h2/async.html#ownership-and-team-asks)                                                                          | [Vadim Petrochenkov][]    | 2 meetings expected |
| ↳ [Ergonomic ref-counting](https://rust-lang.github.io/rust-project-goals/2024h2/ergonomic-rc.html#ownership-and-team-asks)                                                                   | [Jonathan Kelley][]      | 2 meetings expected |
| *Discussion and moral support*                                                                                                        |                  |                     |
| ↳ [Const traits](https://rust-lang.github.io/rust-project-goals/2024h2/const-traits.html#ownership-and-team-asks)                                                                             | [Deadbeef][]       |                     |
| ↳ [Patterns of empty types](https://rust-lang.github.io/rust-project-goals/2024h2/Patterns-of-empty-types.html#ownership-and-team-asks)                                                       | [@Nadrieril][]       |                     |
| ↳ ["Stabilizable" prototype for expanded const generics](https://rust-lang.github.io/rust-project-goals/2024h2/min_generic_const_arguments.html#ownership-and-team-asks)                      | [Boxy][]         |                     |
| *Lang-team experiment*                                                                                                                |                  |                     |
| ↳ [Expose experimental LLVM features for automatic differentiation and GPU offloading](https://rust-lang.github.io/rust-project-goals/2024h2/Rust-for-SciComp.html#ownership-and-team-asks)   | [Manuel Drehwald][]          | (approved)          |
| *Org decision*                                                                                                                        |                  |                     |
| ↳ [Async WG reorganization](https://rust-lang.github.io/rust-project-goals/2024h2/async.html#ownership-and-team-asks)                                                                         | [Niko Matsakis][]    |                     |
| *RFC decision*                                                                                                                        |                  |                     |
| ↳ ["Send bound" problem](https://rust-lang.github.io/rust-project-goals/2024h2/async.html#ownership-and-team-asks)                                                                            | [Niko Matsakis][]    | ![Complete][]       |
| ↳ [Async closures](https://rust-lang.github.io/rust-project-goals/2024h2/async.html#ownership-and-team-asks)                                                                                  | [Michael Goulet][] |                     |
| ↳ [Trait for async iteration](https://rust-lang.github.io/rust-project-goals/2024h2/async.html#ownership-and-team-asks)                                                                       | [Eric Holk][]           |                     |
| ↳ [Derive smart pointer](https://rust-lang.github.io/rust-project-goals/2024h2/rfl_stable.html#ownership-and-team-asks)                                                                       | [Alice Ryhl][]        | ![Complete][]       |
| ↳ [Ergonomic ref-counting](https://rust-lang.github.io/rust-project-goals/2024h2/ergonomic-rc.html#ownership-and-team-asks)                                                                   | [Jonathan Kelley][]      |                     |
| *Stabilization decision*                                                                                                              |                  |                     |
| ↳ [Stabilize cargo-script](https://rust-lang.github.io/rust-project-goals/2024h2/cargo-script.html#ownership-and-team-asks)                                                                   | [Ed Page][]           |                     |
| ↳ ["Send bound" problem](https://rust-lang.github.io/rust-project-goals/2024h2/async.html#ownership-and-team-asks)                                                                            | [Niko Matsakis][]    |                     |
| ↳ [Async closures](https://rust-lang.github.io/rust-project-goals/2024h2/async.html#ownership-and-team-asks)                                                                                  | [Michael Goulet][] |                     |
| ↳ [Arbitrary self types v2](https://rust-lang.github.io/rust-project-goals/2024h2/rfl_stable.html#ownership-and-team-asks)                                                                    | [Adrian Taylor][]       |                     |
| ↳ [Derive smart pointer](https://rust-lang.github.io/rust-project-goals/2024h2/rfl_stable.html#ownership-and-team-asks)                                                                       | [Alice Ryhl][]        |                     |
| ↳ [`asm_goto`](https://rust-lang.github.io/rust-project-goals/2024h2/rfl_stable.html#ownership-and-team-asks)                                                                                 | [Gary Guo][]        |                     |
| ↳ [Pointers to static in constants](https://rust-lang.github.io/rust-project-goals/2024h2/rfl_stable.html#ownership-and-team-asks)                                                            | [Niko Matsakis][]    |                     |
| ↳ [Rust 2024 Edition](https://rust-lang.github.io/rust-project-goals/2024h2/Rust-2024-Edition.html#ownership-and-team-asks)                                                                   | [TC][]     |                     |
| ↳ [Associated type position impl trait](https://rust-lang.github.io/rust-project-goals/2024h2/ATPIT.html#ownership-and-team-asks)                                                             | [Oliver Scherer][]         |                     |

### leadership-council team
| Goal                                                                             | Owner         | Notes                       |
| ---                                                                              | ---           | ---                         |
| *RFC decision*                                                                   |               |                             |
| ↳ [Assemble project goal slate](https://rust-lang.github.io/rust-project-goals/2024h2/Project-goal-slate.html#ownership-and-team-asks)   | [Niko Matsakis][] | ![Complete][]               |
| ↳ [Rust 2024 Edition](https://rust-lang.github.io/rust-project-goals/2024h2/Rust-2024-Edition.html#ownership-and-team-asks)              | [TC][]  | ![Complete][] ([RFC #3501](https://github.com/rust-lang/rfcs/pull/3501)) |

### libs team
| Goal                                                                                                    | Owner         | Notes                                                                 |
| ---                                                                                                     | ---           | ---                                                                   |
| *Discussion and moral support*                                                                          |               |                                                                       |
| ↳ [Survey tools suitability for Std safety verification](https://rust-lang.github.io/rust-project-goals/2024h2/std-verification.html#ownership-and-team-asks)   | [Celina V.][]     |                                                                       |
| *Org decision*                                                                                          |               |                                                                       |
| ↳ [Async WG reorganization](https://rust-lang.github.io/rust-project-goals/2024h2/async.html#ownership-and-team-asks)                                           | [Niko Matsakis][] |                                                                       |
| *Standard review*                                                                                       |               |                                                                       |
| ↳ [Survey tools suitability for Std safety verification](https://rust-lang.github.io/rust-project-goals/2024h2/std-verification.html#ownership-and-team-asks)   | [Celina V.][]     | We would like to contribute upstream the contracts added to the fork. |

### libs-api team
| Goal                                                                     | Owner             | Notes |
| ---                                                                      | ---               | --- |
| *RFC decision*                                                           |                   |  |
| ↳ [Trait for async iteration](https://rust-lang.github.io/rust-project-goals/2024h2/async.html#ownership-and-team-asks)          | [Eric Holk][]            |  |
| ↳ [Ergonomic ref-counting](https://rust-lang.github.io/rust-project-goals/2024h2/ergonomic-rc.html#ownership-and-team-asks)      | [Jonathan Kelley][]       |  |
| *Stabilization decision*                                                 |                   |  |
| ↳ [Extended `offset_of` syntax](https://rust-lang.github.io/rust-project-goals/2024h2/rfl_stable.html#ownership-and-team-asks)   | [Ding Xiang Fei][] |  |

### rust-analyzer team
| Goal                                                                                          | Owner | Notes |
| ---                                                                                           | ---   | --- |
| *Standard reviews*                                                                            |       |  |
| ↳ [Stabilize next-generation solver in coherence](https://rust-lang.github.io/rust-project-goals/2024h2/next-solver.html#ownership-and-team-asks)     | [lcnr][] |  |
| ↳ [Support next-generation solver in rust-analyzer](https://rust-lang.github.io/rust-project-goals/2024h2/next-solver.html#ownership-and-team-asks)   | [lcnr][] |  |

### rustdoc team
| Goal                                                                                               | Owner           | Notes |
| ---                                                                                                | ---             | --- |
| *Discussion and moral support*                                                                     |                 |  |
| ↳ [Make Rustdoc Search easier to learn](https://rust-lang.github.io/rust-project-goals/2024h2/rustdoc-search.html#ownership-and-team-asks)                 | [Michael Howell][]      |  |
| *RFC decision*                                                                                     |                 |  |
| ↳ [Stabilize doc_cfg](https://rust-lang.github.io/rust-project-goals/2024h2/doc_cfg.html#ownership-and-team-asks)                                          | [Guillaume Gomez][] |  |
| *Standard reviews*                                                                                 |                 |  |
| ↳ [Stabilize doc_cfg](https://rust-lang.github.io/rust-project-goals/2024h2/doc_cfg.html#ownership-and-team-asks)                                          | [Guillaume Gomez][] |  |
| ↳ [Implement "merged doctests" to save doctest time](https://rust-lang.github.io/rust-project-goals/2024h2/merged-doctests.html#ownership-and-team-asks)   | [Guillaume Gomez][] |  |

### rustdoc-frontend team
| Goal                                                                                  | Owner      | Notes |
| ---                                                                                   | ---        | --- |
| *Design meeting*                                                                      |            |  |
| ↳ [Improve on any discovered weaknesses](https://rust-lang.github.io/rust-project-goals/2024h2/rustdoc-search.html#ownership-and-team-asks)   | [Michael Howell][] |  |
| *FCP review*                                                                          |            |  |
| ↳ [Improve on any discovered weaknesses](https://rust-lang.github.io/rust-project-goals/2024h2/rustdoc-search.html#ownership-and-team-asks)   | [Michael Howell][] |  |
| *Standard reviews*                                                                    |            |  |
| ↳ [Improve on any discovered weaknesses](https://rust-lang.github.io/rust-project-goals/2024h2/rustdoc-search.html#ownership-and-team-asks)   | [Michael Howell][] |  |

### types team
| Goal                                                                                                               | Owner         | Notes          |
| ---                                                                                                                | ---           | ---            |
| *Discussion and moral support*                                                                                     |               |                |
| ↳ [Const traits](https://rust-lang.github.io/rust-project-goals/2024h2/const-traits.html#ownership-and-team-asks)                                                          | [Deadbeef][]    |                |
| ↳ [Next-generation trait solver](https://rust-lang.github.io/rust-project-goals/2024h2/next-solver.html#ownership-and-team-asks)                                           | [lcnr][]         |                |
| ↳ ["Stabilizable" prototype for expanded const generics](https://rust-lang.github.io/rust-project-goals/2024h2/min_generic_const_arguments.html#ownership-and-team-asks)   | [Boxy][]      |                |
| *FCP decisions*                                                                                                    |               |                |
| ↳ [Associated type position impl trait](https://rust-lang.github.io/rust-project-goals/2024h2/ATPIT.html#ownership-and-team-asks)                                          | [Oliver Scherer][]      |                |
| *Stabilization decision*                                                                                           |               |                |
| ↳ [Stabilize next-generation solver in coherence](https://rust-lang.github.io/rust-project-goals/2024h2/next-solver.html#ownership-and-team-asks)                          | [lcnr][]         |                |
| ↳ [Rust 2024 Edition](https://rust-lang.github.io/rust-project-goals/2024h2/Rust-2024-Edition.html#ownership-and-team-asks)                                                | [TC][]  |                |
| ↳ [Associated type position impl trait](https://rust-lang.github.io/rust-project-goals/2024h2/ATPIT.html#ownership-and-team-asks)                                          | [Oliver Scherer][]      |                |
| *Standard reviews*                                                                                                 |               |                |
| ↳ [Stabilize next-generation solver in coherence](https://rust-lang.github.io/rust-project-goals/2024h2/next-solver.html#ownership-and-team-asks)                          | [lcnr][]         |                |
| ↳ [Support next-generation solver in rust-analyzer](https://rust-lang.github.io/rust-project-goals/2024h2/next-solver.html#ownership-and-team-asks)                        | [lcnr][]         |                |
| ↳ [Scalable Polonius support on nightly](https://rust-lang.github.io/rust-project-goals/2024h2/Polonius.html#ownership-and-team-asks)                                      | [Rémy Rakic][]          | [Matthew Jasper][] |
| ↳ [Testing infra + contributors for a-mir-formality](https://rust-lang.github.io/rust-project-goals/2024h2/a-mir-formality.html#ownership-and-team-asks)                   | [Niko Matsakis][] |                |


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


[all]: https://www.rust-lang.org/governance/teams
[alumni]: https://www.rust-lang.org/governance/teams
[android]: https://www.rust-lang.org/governance/teams
[apple]: https://www.rust-lang.org/governance/teams
[arewewebyet]: https://www.rust-lang.org/governance/teams
[arm]: https://www.rust-lang.org/governance/teams
[arm-maintainers]: https://www.rust-lang.org/governance/teams
[book]: https://www.rust-lang.org/governance/teams
[bootstrap]: https://github.com/rust-lang/rust
[cargo]: https://github.com/rust-lang/cargo
[clippy]: https://github.com/rust-lang/rust-clippy
[clippy-contributors]: https://github.com/rust-lang/rust-clippy
[cloud-compute]: https://www.rust-lang.org/governance/teams
[community]: https://www.rust-lang.org/governance/teams
[community-content]: https://github.com/rust-community/content-team
[community-events]: https://github.com/rust-community/events-team
[community-localization]: https://github.com/rust-lang/community-localization
[community-rustbridge]: https://github.com/rustbridge/team
[community-survey]: https://github.com/rust-lang/surveys
[compiler]: http://github.com/rust-lang/compiler-team
[compiler-contributors]: http://github.com/rust-lang/compiler-team
[core]: https://www.rust-lang.org/governance/teams
[council-librarians]: https://www.rust-lang.org/governance/teams
[crate-maintainers]: https://www.rust-lang.org/governance/teams
[crates-io]: https://github.com/rust-lang/crates.io
[crates-io-admins]: https://www.rust-lang.org/governance/teams
[crates-io-on-call]: https://www.rust-lang.org/governance/teams
[devtools]: https://github.com/rust-dev-tools/dev-tools-team
[docker]: https://www.rust-lang.org/governance/teams
[docs-rs]: https://github.com/rust-lang/docs.rs
[docs-rs-reviewers]: https://www.rust-lang.org/governance/teams
[emacs]: https://www.rust-lang.org/governance/teams
[foundation-email-redirects]: https://www.rust-lang.org/governance/teams
[fuchsia]: https://www.rust-lang.org/governance/teams
[gsoc-contributors]: https://www.rust-lang.org/governance/teams
[icebreakers-cleanup-crew]: https://www.rust-lang.org/governance/teams
[icebreakers-llvm]: https://www.rust-lang.org/governance/teams
[infra]: https://github.com/rust-lang/infra-team
[infra-admins]: https://www.rust-lang.org/governance/teams
[infra-bors]: https://www.rust-lang.org/governance/teams
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
[miri]: https://github.com/rust-lang/miri
[mods]: https://github.com/rust-lang/moderation-team
[mods-discord]: https://www.rust-lang.org/governance/teams
[mods-discourse]: https://www.rust-lang.org/governance/teams
[opsem]: https://github.com/rust-lang/opsem-team
[ospp]: https://www.rust-lang.org/governance/teams
[project-async-crashdump-debugging]: https://github.com/rust-lang/async-crashdump-debugging-initiative
[project-const-generics]: https://github.com/rust-lang/project-const-generics
[project-const-traits]: https://www.rust-lang.org/governance/teams
[project-dyn-upcasting]: https://github.com/rust-lang/dyn-upcasting-coercion-initiative
[project-edition-2024]: https://www.rust-lang.org/governance/teams
[project-error-handling]: https://www.rust-lang.org/governance/teams
[project-exploit-mitigations]: https://github.com/rust-lang/project-exploit-mitigations
[project-generic-associated-types]: https://github.com/rust-lang/generic-associated-types-initiative
[project-group-leads]: https://www.rust-lang.org/governance/teams
[project-impl-trait]: https://github.com/rust-lang/impl-trait-initiative
[project-keyword-generics]: https://github.com/rust-lang/keyword-generics-initiative
[project-negative-impls]: https://github.com/rust-lang/negative-impls-initiative
[project-portable-simd]: https://www.rust-lang.org/governance/teams
[project-stable-mir]: https://github.com/rust-lang/project-stable-mir
[project-trait-system-refactor]: https://github.com/rust-lang/types-team
[regex]: https://github.com/rust-lang/regex
[release]: https://github.com/rust-lang/release-team
[release-publishers]: https://www.rust-lang.org/governance/teams
[risc-v]: https://www.rust-lang.org/governance/teams
[rust-analyzer]: https://github.com/rust-lang/rust-analyzer
[rust-analyzer-contributors]: https://github.com/rust-lang/rust-analyzer
[rust-for-linux]: https://www.rust-lang.org/governance/teams
[rustconf-emails]: https://www.rust-lang.org/governance/teams
[rustdoc]: https://github.com/rust-lang/rust
[rustdoc-frontend]: https://www.rust-lang.org/governance/teams
[rustfmt]: https://github.com/rust-lang/rustfmt
[rustlings]: https://www.rust-lang.org/governance/teams
[rustup]: https://github.com/rust-lang/rustup
[social-media]: https://www.rust-lang.org/governance/teams
[spec]: https://github.com/rust-lang/spec
[spec-contributors]: https://github.com/rust-lang/spec
[style]: https://github.com/rust-lang/style-team
[team-repo-admins]: https://www.rust-lang.org/governance/teams
[testing-devex]: https://www.rust-lang.org/governance/teams
[triagebot]: https://github.com/rust-lang/triagebot
[twir]: https://www.rust-lang.org/governance/teams
[twir-reviewers]: https://www.rust-lang.org/governance/teams
[twitter]: https://www.rust-lang.org/governance/teams
[types]: https://github.com/rust-lang/types-team
[vim]: https://www.rust-lang.org/governance/teams
[web-presence]: https://www.rust-lang.org/governance/teams
[website]: https://www.rust-lang.org/governance/teams
[wg-allocators]: https://github.com/rust-lang/wg-allocators
[wg-async]: https://github.com/rust-lang/wg-async
[wg-binary-size]: https://github.com/rust-lang/wg-binary-size
[wg-bindgen]: https://github.com/rust-lang/rust-bindgen
[wg-cli]: https://www.rust-lang.org/governance/teams
[wg-compiler-performance]: https://github.com/rust-lang/rustc-perf
[wg-const-eval]: https://github.com/rust-lang/const-eval
[wg-debugging]: https://www.rust-lang.org/governance/teams
[wg-diagnostics]: https://rust-lang.github.io/compiler-team/working-groups/diagnostics/
[wg-embedded]: https://github.com/rust-embedded/wg
[wg-embedded-core]: https://www.rust-lang.org/governance/teams
[wg-embedded-cortex-a]: https://www.rust-lang.org/governance/teams
[wg-embedded-cortex-m]: https://www.rust-lang.org/governance/teams
[wg-embedded-cortex-r]: https://www.rust-lang.org/governance/teams
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
[wg-incr-comp]: https://www.rust-lang.org/governance/teams
[wg-inline-asm]: https://github.com/rust-lang/project-inline-asm
[wg-leads]: https://www.rust-lang.org/governance/teams
[wg-llvm]: https://rust-lang.github.io/compiler-team/working-groups/llvm/
[wg-macros]: https://github.com/rust-lang/wg-macros
[wg-mir-opt]: https://rust-lang.github.io/compiler-team/working-groups/mir-opt/
[wg-parallel-rustc]: https://rust-lang.github.io/compiler-team/working-groups/parallel-rustc/
[wg-pgo]: https://rust-lang.github.io/compiler-team/working-groups/pgo/
[wg-polonius]: https://rust-lang.github.io/compiler-team/working-groups/polonius/
[wg-polymorphization]: https://rust-lang.github.io/compiler-team/working-groups/polymorphization/
[wg-prioritization]: https://rust-lang.github.io/compiler-team/working-groups/prioritization/
[wg-rfc-2229]: https://rust-lang.github.io/compiler-team/working-groups/rfc-2229/
[wg-rust-by-example]: https://github.com/rust-lang/rust-by-example
[wg-rustc-dev-guide]: https://rust-lang.github.io/compiler-team/working-groups/rustc-dev-guide/
[wg-rustc-reading-club]: https://rust-lang.github.io/rustc-reading-club/
[wg-safe-transmute]: https://github.com/rust-lang/project-safe-transmute
[wg-secure-code]: https://github.com/rust-secure-code/wg
[wg-security-response]: https://github.com/rust-lang/wg-security-response
[wg-self-profile]: https://rust-lang.github.io/compiler-team/working-groups/self-profile/
[wg-triage]: https://www.rust-lang.org/governance/teams
[windows]: https://www.rust-lang.org/governance/teams


[Boxy]: https://github.com/BoxyUwU
[Alice Ryhl]: https://github.com/Darksonn
[Guillaume Gomez]: https://github.com/GuillaumeGomez
[Jakub Beránek]: https://github.com/Kobzol
[Scott Schafer]: https://github.com/Muscraft
[@Nadrieril]: https://github.com/Nadrieril
[Sparrow Li]: https://github.com/SparrowLii
[Manuel Drehwald]: https://github.com/ZuseZ4
[Adrian Taylor]: https://github.com/adetaylor
[Alejandra González]: https://github.com/blyxyas
[Celina V.]: https://github.com/celinval
[Michael Goulet]: https://github.com/compiler-errors
[Ding Xiang Fei]: https://github.com/dingxiangfei2009
[Jacob Finkelman]: https://github.com/eh2406
[Eric Holk]: https://github.com/eholk
[Ed Page]: https://github.com/epage
[Esteban Kuber]: https://github.com/estebank
[Deadbeef]: https://github.com/fee1-dead
[二手掉包工程师]: https://github.com/hi-rustin
[Jonathan Kelley]: https://github.com/jkelleyrtp
[Josh Triplett]: https://github.com/joshtriplett
[lcnr]: https://github.com/lcnr
[Rémy Rakic]: https://github.com/lqd
[Matthew Jasper]: https://github.com/matthewjasper
[Gary Guo]: https://github.com/nbdd0121
[Niko Matsakis]: https://github.com/nikomatsakis
[Michael Howell]: https://github.com/notriddle
[Predrag Gruevski]: https://github.com/obi1kenobi
[Oliver Scherer]: https://github.com/oli-obk
[Vadim Petrochenkov]: https://github.com/petrochenkov
[Tyler Mandry]: https://github.com/tmandry
[TC]: https://github.com/traviscross
[Weihang Lo]: https://github.com/weihanglo


[Complete]: https://img.shields.io/badge/Complete-green
[Help wanted]: https://img.shields.io/badge/Help%20wanted-yellow
[Not funded]: https://img.shields.io/badge/Not%20yet%20funded-red
[TBD]: https://img.shields.io/badge/TBD-red
[Team]: https://img.shields.io/badge/Team%20ask-red

