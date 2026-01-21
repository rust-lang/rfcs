- Feature Name: N/A
- Start Date: 2025-01-14
- RFC PR: [rust-lang/rfcs#3764](https://github.com/rust-lang/rfcs/pull/3764)
- Rust Issue: N/A

## Summary

Propose a slate of 39 project goals for 2025H1, including 3 flagship goals:

* Continue making Rust easier to use for network systems by [**bringing the Async Rust experience closer to parity with sync Rust**](https://rust-lang.github.io/rust-project-goals/2025h1/./async.html). In 2025H1 we plan to:
    * tell a complete story for the use of async fn in traits, unblocking wide ecosystem adoption;
    * improve the ergonomics of `Pin`, which is frequently used in low-level async code; and
    * prepare to support asynchronous (and synchronous) generators in the language.
* Continue helping Rust support low-level projects by [**stabilizing compiler options and tooling used by the Rust-for-Linux project**](https://rust-lang.github.io/rust-project-goals/2025h1/./rfl.html). In 2025H1 we plan to:
    * implement [RFC #3716](https://github.com/rust-lang/rfcs/pull/3716) to allow stabilizing ABI-modifying compiler flags to control code generation, sanitizer integration, and so forth;
    * taking the first step towards stabilizing [`build-std`](https://rust-lang.github.io/rust-project-goals/2025h1/https://doc.rust-lang.org/cargo/reference/unstable.html#build-std) by [creating a stable way to rebuild core with specific compiler options](./build-std.html);
    * add rustdoc features to extract and customize rustdoc tests (`--extract-doctests`);
    * stabilize clippy configuration like `.clippy.toml` and `CLIPPY_CONF_DIR`;
    * stabilize compiler flags to extract dependency info (e.g., as via `-Zbinary-dep-depinfo=y`) and to configure no-std without requiring it in the source file (e.g., as via `-Zcrate-attr`);
* Address the biggest concerns raised by Rust maintainers, lack of face-to-face interaction, by [**organizing the Rust All-Hands 2025**](https://rust-lang.github.io/rust-project-goals/2025h1/./all-hands.html). In 2025H1 we plan to:
    * convene Rust maintainers to celebrate Rust's tenth birthday at [RustWeek 2025](https://2025.rustweek.org) (co-organized with [RustNL](https://2025.rustweek.org/about/));
    * author a first draft for a [Rust vision doc](https://rust-lang.github.io/rust-project-goals/2025h1/./rust-vision-doc.html) and gather feedback.


## Motivation

The 2025H1 goal slate consists of 39 project goals, of which we have selected 3 as **flagship goals**. Flagship goals represent the goals expected to have the broadest overall impact.

### How the goal process works

**Project goals** are proposed bottom-up by a **point of contact**, somebody who is willing to commit resources (time, money, leadership) to seeing the work get done. The point of contact identifies the problem they want to address and sketches the solution of how they want to do so. They also identify the support they will need from the Rust teams (typically things like review bandwidth or feedback on RFCs). Teams then read the goals and provide feedback. If the goal is approved, teams are committing to support the point of contact in their work.

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
* **Slow and steady wins the race.** We don't want to create stress via unrealistic, ambitious goals. We want to make steady progress each goal period on important problems.

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

### Flagship goals

The flagship goals proposed for this roadmap are as follows:

* Continue making Rust easier to use for network systems by [**bringing the Async Rust experience closer to parity with sync Rust**](https://rust-lang.github.io/rust-project-goals/2025h1/./async.html). In 2025H1 we plan to:
    * tell a complete story for the use of async fn in traits, unblocking wide ecosystem adoption;
    * improve the ergonomics of `Pin`, which is frequently used in low-level async code; and
    * prepare to support asynchronous (and synchronous) generators in the language.
* Continue helping Rust support low-level projects by [**stabilizing compiler options and tooling used by the Rust-for-Linux (RFL) project**](https://rust-lang.github.io/rust-project-goals/2025h1/./rfl.html). In 2025H1 we plan to:
    * implement [RFC #3716](https://github.com/rust-lang/rfcs/pull/3716) to allow stabilizing ABI-modifying compiler flags to control code generation, sanitizer integration, and so forth;
    * taking the first step towards stabilizing [`build-std`](https://doc.rust-lang.org/cargo/reference/unstable.html#build-std) by [creating a stable way to rebuild core with specific compiler options](https://rust-lang.github.io/rust-project-goals/2025h1/./build-std.html);
    * add rustdoc features to extract and customize rustdoc tests (`--extract-doctests`);
    * stabilize clippy configuration like `.clippy.toml` and `CLIPPY_CONF_DIR`;
    * stabilize compiler flags to extract dependency info (e.g., as via `-Zbinary-dep-depinfo=y`) and to configure no-std without requiring it in the source file (e.g., as via `-Zcrate-attr`);
* Address the biggest concerns raised by Rust maintainers, lack of face-to-face interaction, by [**organizing the Rust All-Hands 2025**](https://rust-lang.github.io/rust-project-goals/2025h1/./all-hands.html). In 2025H1 we plan to:
    * convene Rust maintainers to celebrate Rust's tenth birthday at [RustWeek 2025](https://2025.rustweek.org) (co-organized with [RustNL](https://2025.rustweek.org/about/));
    * author a first draft for a [Rust vision doc](https://rust-lang.github.io/rust-project-goals/2025h1/./rust-vision-doc.html) and gather feedback.

#### Why these particular flagship goals?

[**Async.**](https://rust-lang.github.io/rust-project-goals/2025h1/./async.html) Rust is a great fit for server development thanks to its ability to scale to very high load while retaining low memory usage and tight tail latency. 52% of the respondents in the [2023 Rust survey](https://blog.rust-lang.org/2024/02/19/2023-Rust-Annual-Survey-2023-results.html) indicated that they use Rust to build server-side or backend applications. In [2025H1 our plan](https://rust-lang.github.io/rust-project-goals/2025h1/./async.html) is to deliver (a) improved support for async-fn-in-traits, completely subsuming the functionality of the [`async-trait` crate](https://crates.io/crates/async-trait); (b) progress towards sync and async generators, simplifying the creation of iterators and async data streams; (c) and improve the ergonomics of `Pin`, making lower-level async coding more approachable. These items together start to unblock the creation of the next generation of async libraries in the wider ecosystem, as progress there has been blocked on a stable solution for async traits and streams.

[**Rust for Linux.**](https://rust-lang.github.io/rust-project-goals/2025h1/./rfl.html) The [experimental support for Rust development in the Linux kernel][RFL.com] is a watershed moment for Rust, demonstrating to the world that Rust is indeed a true alternative to C. Currently the Linux kernel support depends on a wide variety of unstable features in Rust; these same features block other embedded and low-level systems applications. We are working to stabilize all of these features so that RFL can be built on a stable toolchain. As we have successfully stabilized the majority of the language features used by RFL, we plan in 2025H1 to turn our focus to compiler flags and tooling options. We will (a) implement [RFC #3716](https://github.com/rust-lang/rfcs/pull/3716) which lays out a design for ABI-modifying flags; (b) take the first step towards stabilizing [`build-std`](https://doc.rust-lang.org/cargo/reference/unstable.html#build-std) by [creating a stable way to rebuild core with specific compiler options](https://rust-lang.github.io/rust-project-goals/2025h1/./build-std.html); (c) extending rustdoc, clippy, and the compiler with features that extract metadata for integration into other build systems (in this case, the kernel's build system).

[**Rust All Hands 2025.**](https://rust-lang.github.io/rust-project-goals/2025h1/./all-hands.html) May 15, 2025 marks the 10-year anniversary of Rust's 1.0 release; it also marks 10 years since the [creation of the Rust subteams](https://internals.rust-lang.org/t/announcing-the-subteams/2042). At the time [there were 6 Rust teams with 24 people in total](http://web.archive.org/web/20150517235608/http://www.rust-lang.org/team.html). There are now 57 teams with 166 people. In-person All Hands meetings are an effective way to help these maintainers get to know one another with high-bandwidth discussions. This year, the Rust project will be coming together for [RustWeek 2025](https://2025.rustweek.org), a joint event organized with [RustNL](https://2025.rustweek.org/about/). Participating project teams will use the time to share knowledge, make plans, or just get to know one another better. One particular goal for the All Hands is reviewing a draft of the [Rust Vision Doc](https://rust-lang.github.io/rust-project-goals/2025h1/./rust-vision-doc.html), a document that aims to take stock of where Rust is and lay out high-level goals for the next few years.

[RFL.com]: https://rust-for-linux.com/
[RFL#2]: https://github.com/Rust-for-Linux/linux/issues/2

### Project goals

The full slate of project goals are as follows. These goals all have identified owners who will drive the work forward as well as a viable work plan. The goals include asks from the listed Rust teams, which are cataloged in the [reference-level explanation](#reference-level-explanation) section below.

**Invited goals.** Some goals of the goals below are "invited goals", meaning that for that goal to happen we need someone to step up and serve as an owner. To find the invited goals, look for the ![Help wanted][] badge in the table below. Invited goals have reserved capacity for teams and a mentor, so if you are someone looking to help Rust progress, they are a great way to get involved.

| Goal                                                                                                        | Point of contact | Team                                                           |
| :--                                                                                                         | :--              | :--                                                            |
| ["Stabilizable" prototype for expanded const generics](https://rust-lang.github.io/rust-project-goals/2025h1/min_generic_const_arguments.html)                      | [Boxy][]         | [lang], [types]                                                |
| [Bring the Async Rust experience closer to parity with sync Rust](https://rust-lang.github.io/rust-project-goals/2025h1/async.html)                                 | [Tyler Mandry][]         | [compiler], [lang], [libs-api], [spec], [types]                |
| [Continue resolving `cargo-semver-checks` blockers for merging into cargo](https://rust-lang.github.io/rust-project-goals/2025h1/cargo-semver-checks.html)          | [Predrag Gruevski][]      | [cargo], [rustdoc]                                             |
| [Declarative (`macro_rules!`) macro improvements](https://rust-lang.github.io/rust-project-goals/2025h1/macro-improvements.html)                                    | [Josh Triplett][]    | [lang], [wg-macros]                                            |
| [Evaluate approaches for seamless interop between C++ and Rust](https://rust-lang.github.io/rust-project-goals/2025h1/seamless-rust-cpp.html)                       | [Tyler Mandry][]         | [compiler], [lang], [libs-api]                                 |
| [Experiment with ergonomic ref-counting](https://rust-lang.github.io/rust-project-goals/2025h1/ergonomic-rc.html)                                                   | [Santiago Pastorino][]      | [lang]                                                         |
| [Expose experimental LLVM features for GPU offloading](https://rust-lang.github.io/rust-project-goals/2025h1/GPU-Offload.html)                                      | [Manuel Drehwald][]          | [compiler], [lang]                                             |
| [Extend pubgrub to match cargo's dependency resolution](https://rust-lang.github.io/rust-project-goals/2025h1/pubgrub-in-cargo.html)                                | [Jacob Finkelman][]          | [cargo]                                                        |
| [Externally Implementable Items](https://rust-lang.github.io/rust-project-goals/2025h1/eii.html)                                                                    | [Mara Bos][]         | [compiler], [lang]                                             |
| [Finish the libtest json output experiment](https://rust-lang.github.io/rust-project-goals/2025h1/libtest-json.html)                                                | [Ed Page][]           | [cargo], [libs-api], [testing-devex]                           |
| [Implement Open API Namespace Support](https://rust-lang.github.io/rust-project-goals/2025h1/open-namespaces.html)                                                  | ![Help Wanted][] | [cargo], [compiler]                                            |
| [Implement restrictions, prepare for stabilization](https://rust-lang.github.io/rust-project-goals/2025h1/restrictions.html)                                        | [Jacob Pratt][]         | [compiler], [lang], [spec]                                     |
| [Improve state machine codegen](https://rust-lang.github.io/rust-project-goals/2025h1/improve-rustc-codegen.html)                                                   | [Folkert de Vries][]      | [compiler], [lang]                                             |
| [Instrument the Rust standard library with safety contracts](https://rust-lang.github.io/rust-project-goals/2025h1/std-contracts.html)                              | [Celina G. Val][]        | [compiler], [libs]                                             |
| [Making compiletest more maintainable: reworking directive handling](https://rust-lang.github.io/rust-project-goals/2025h1/compiletest-directive-rework.html)       | [Jieyou Xu][]        | [bootstrap], [compiler], [rustdoc]                             |
| [Metrics Initiative](https://rust-lang.github.io/rust-project-goals/2025h1/metrics-initiative.html)                                                                 | [Jane Lusby][]           | [compiler], [infra]                                            |
| [Model coherence in a-mir-formality](https://rust-lang.github.io/rust-project-goals/2025h1/formality.html)                                                          | [Niko Matsakis][]    | [types]                                                        |
| [Next-generation trait solver](https://rust-lang.github.io/rust-project-goals/2025h1/next-solver.html)                                                              | [lcnr][]            | [types]                                                        |
| [Nightly support for ergonomic SIMD multiversioning](https://rust-lang.github.io/rust-project-goals/2025h1/simd-multiversioning.html)                               | [Luca Versari][]        | [lang]                                                         |
| [Null and enum-discriminant runtime checks in debug builds](https://rust-lang.github.io/rust-project-goals/2025h1/null-enum-discriminant-debug-checks.html)         | [Bastian Kersting][]          | [compiler], [lang], [opsem]                                    |
| [Optimizing Clippy & linting](https://rust-lang.github.io/rust-project-goals/2025h1/optimize-clippy-linting-2.html)                                                 | [Alejandra González][]         | [clippy]                                                       |
| [Organize Rust All-Hands 2025](https://rust-lang.github.io/rust-project-goals/2025h1/all-hands.html)                                                                | [Mara Bos][]         | [leadership-council]                                           |
| [Prepare const traits for stabilization](https://rust-lang.github.io/rust-project-goals/2025h1/const-trait.html)                                                    | [Oliver Scherer][]         | [compiler], [lang], [types]                                    |
| [Promoting Parallel Front End](https://rust-lang.github.io/rust-project-goals/2025h1/parallel-front-end.html)                                                       | [Sparrow Li][]      | [compiler]                                                     |
| [Prototype a new set of Cargo "plumbing" commands](https://rust-lang.github.io/rust-project-goals/2025h1/cargo-plumbing.html)                                       | ![Help Wanted][] | [cargo]                                                        |
| [Publish first rust-lang-owned release of "FLS"](https://rust-lang.github.io/rust-project-goals/2025h1/spec-fls-publish.html)                                       | [Joel Marcey][]      | [bootstrap], [spec]                                            |
| [Publish first version of StableMIR on crates.io](https://rust-lang.github.io/rust-project-goals/2025h1/stable-mir.html)                                            | [Celina G. Val][]        | [compiler], [project-stable-mir]                               |
| [Research: How to achieve safety when linking separately compiled code](https://rust-lang.github.io/rust-project-goals/2025h1/safe-linking.html)                    | [Mara Bos][]         | [compiler], [lang]                                             |
| [Run the 2025H1 project goal program](https://rust-lang.github.io/rust-project-goals/2025h1/stabilize-project-goal-program.html)                                    | [Niko Matsakis][]    | [leadership-council]                                           |
| [Rust Vision Document](https://rust-lang.github.io/rust-project-goals/2025h1/rust-vision-doc.html)                                                                  | [Niko Matsakis][]    | [leadership-council]                                           |
| [SVE and SME on AArch64](https://rust-lang.github.io/rust-project-goals/2025h1/arm-sve-sme.html)                                                                    | [David Wood][]       | [compiler], [lang], [types]                                    |
| [Scalable Polonius support on nightly](https://rust-lang.github.io/rust-project-goals/2025h1/Polonius.html)                                                         | [Rémy Rakic][]             | [types]                                                        |
| [Secure quorum-based cryptographic verification and mirroring for crates.io](https://rust-lang.github.io/rust-project-goals/2025h1/verification-and-mirroring.html) | [walterhpearce][]   | [cargo], [crates-io], [infra], [leadership-council], [release] |
| [Stabilize public/private dependencies](https://rust-lang.github.io/rust-project-goals/2025h1/pub-priv.html)                                                        | ![Help Wanted][] | [cargo], [compiler]                                            |
| [Stabilize tooling needed by Rust for Linux](https://rust-lang.github.io/rust-project-goals/2025h1/rfl.html)                                                        | [Niko Matsakis][]    | [cargo], [clippy], [compiler], [rustdoc]                       |
| [Unsafe Fields](https://rust-lang.github.io/rust-project-goals/2025h1/unsafe-fields.html)                                                                           | [Jack Wrenn][]         | [compiler], [lang]                                             |
| [Use annotate-snippets for rustc diagnostic output](https://rust-lang.github.io/rust-project-goals/2025h1/annotate-snippets.html)                                   | [Scott Schafer][]        | [compiler]                                                     |
| [build-std](https://rust-lang.github.io/rust-project-goals/2025h1/build-std.html)                                                                                   | [David Wood][]       | [cargo]                                                        |
| [rustc-perf improvements](https://rust-lang.github.io/rust-project-goals/2025h1/perf-improvements.html)                                                             | [David Wood][]       | [compiler], [infra]                                            |


## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The following table highlights the asks from each affected team.
The rows are goals and columns are [asks being made of the team][valid_team_asks].
The contents of each cell may contain extra notes (or sometimes footnotes) with more details.
Teams often use these notes to indicate the person on the team signed up to do the work, for example.


### bootstrap team
| Goal                                                                                                  | [Good vibes][valid_team_asks] | [r?][valid_team_asks] |
| :--                                                                                                   | :-- | :-- |
| [Making compiletest more maintainable: reworking directive handling](https://rust-lang.github.io/rust-project-goals/2025h1/compiletest-directive-rework.html) | \*2 | \*3 |
| [Publish first rust-lang-owned release of "FLS"](https://rust-lang.github.io/rust-project-goals/2025h1/spec-fls-publish.html)                                 |     | \*1 |


\*1: For any tooling integration ([from here](https://rust-lang.github.io/rust-project-goals/2025h1/spec-fls-publish.html))


\*2: including consultations for desired test behaviors and testing infra consumers ([from here](https://rust-lang.github.io/rust-project-goals/2025h1/compiletest-directive-rework.html))


\*3: Probably mostly [bootstrap] or whoever is more interested in reviewing [`compiletest`] changes ([from here](https://rust-lang.github.io/rust-project-goals/2025h1/compiletest-directive-rework.html))

### cargo team
| Goal                                                                                               | [Good vibes][valid_team_asks] | [r?][valid_team_asks] | [Ded. r?][valid_team_asks] | [Design mtg.][valid_team_asks] |
| :--                                                                                                | :-- | :-- | :-- | :--     |
| [Continue resolving `cargo-semver-checks` blockers for merging into cargo](https://rust-lang.github.io/rust-project-goals/2025h1/cargo-semver-checks.html) | ✅   |     |     |         |
| [Crates.io mirroring](https://rust-lang.github.io/rust-project-goals/2025h1/verification-and-mirroring.html)                                               |     |     | \*3 | \*1 \*2 |
| [Extend pubgrub to match cargo's dependency resolution](https://rust-lang.github.io/rust-project-goals/2025h1/pubgrub-in-cargo.html)                       | ✅   |     |     |         |
| [Finish the libtest json output experiment](https://rust-lang.github.io/rust-project-goals/2025h1/libtest-json.html)                                       | ✅   |     |     |         |
| [Implement Open API Namespace Support](https://rust-lang.github.io/rust-project-goals/2025h1/open-namespaces.html)                                         | ✅   |     |     |         |
| [Prototype a new set of Cargo "plumbing" commands](https://rust-lang.github.io/rust-project-goals/2025h1/cargo-plumbing.html)                              | ✅   |     |     |         |
| [Rust-for-Linux](https://rust-lang.github.io/rust-project-goals/2025h1/rfl.html)                                                                           | ✅   |     |     |         |
| [Stabilize public/private dependencies](https://rust-lang.github.io/rust-project-goals/2025h1/pub-priv.html)                                               | ✅   |     |     |         |
| [build-std](https://rust-lang.github.io/rust-project-goals/2025h1/build-std.html)                                                                          | ✅   | ✅   |     |         |


\*1: 1 hour Overall Design and threat model ([from here](https://rust-lang.github.io/rust-project-goals/2025h1/verification-and-mirroring.html))


\*2: 1 hour General design/implementation for index verification ([from here](https://rust-lang.github.io/rust-project-goals/2025h1/verification-and-mirroring.html))


\*3: 1 hour Design for novel incremental download mechanism for bandwidth conservation ([from here](https://rust-lang.github.io/rust-project-goals/2025h1/verification-and-mirroring.html))

### clippy team
| Goal                                                        | [r?][valid_team_asks] | [Stabilize.][valid_team_asks] |
| :--                                                         | :-- | :-- |
| [Optimizing Clippy & linting](https://rust-lang.github.io/rust-project-goals/2025h1/optimize-clippy-linting-2.html) | ✅   |     |
| [Rust-for-Linux](https://rust-lang.github.io/rust-project-goals/2025h1/rfl.html)                                    |     |     |
| ↳ Clippy configuration                                      |     | ✅   |

### compiler team
| Goal                                                                                                  | [Good vibes][valid_team_asks] | [r?][valid_team_asks] | [Ded. r?][valid_team_asks] | [Design mtg.][valid_team_asks] | [RFC][valid_team_asks] | [Stabilize.][valid_team_asks] | [Policy][valid_team_asks] |
| :--                                                                                                   | :-- | :--       | :-- | :-- | :-- | :-- | :-- |
| [Async](https://rust-lang.github.io/rust-project-goals/2025h1/async.html)                                                                                     |     |           |     |     |     |     |     |
| ↳ Implementable trait aliases                                                                         |     | ✅         |     |     |     |     |     |
| ↳ Return type notation                                                                                |     | ✅         |     |     |     |     |     |
| [Evaluate approaches for seamless interop between C++ and Rust](https://rust-lang.github.io/rust-project-goals/2025h1/seamless-rust-cpp.html)                 | ✅   |           |     | \*4 |     |     |     |
| [Expose experimental LLVM features for GPU offloading](https://rust-lang.github.io/rust-project-goals/2025h1/GPU-Offload.html)                                |     | ✅         |     |     |     |     |     |
| [Externally Implementable Items](https://rust-lang.github.io/rust-project-goals/2025h1/eii.html)                                                              |     | ✅         |     |     |     |     |     |
| [Implement Open API Namespace Support](https://rust-lang.github.io/rust-project-goals/2025h1/open-namespaces.html)                                            | ✅   |           |     |     |     |     |     |
| [Implement restrictions, prepare for stabilization](https://rust-lang.github.io/rust-project-goals/2025h1/restrictions.html)                                  |     | ✅         |     |     |     |     |     |
| [Improve state machine codegen](https://rust-lang.github.io/rust-project-goals/2025h1/improve-rustc-codegen.html)                                             | ✅   | ✅         |     |     |     |     |     |
| [Instrument the Rust standard library with safety contracts](https://rust-lang.github.io/rust-project-goals/2025h1/std-contracts.html)                        |     |           |     |     |     |     |     |
| ↳ Experimental Contract attributes                                                                    |     | ✅         |     | ✅   |     |     |     |
| [Making compiletest more maintainable: reworking directive handling](https://rust-lang.github.io/rust-project-goals/2025h1/compiletest-directive-rework.html) | \*5 | \*6       |     |     |     |     |     |
| [Metrics Initiative](https://rust-lang.github.io/rust-project-goals/2025h1/metrics-initiative.html)                                                           | ✅   | ✅         |     |     |     |     |     |
| [Null and enum-discriminant runtime checks in debug builds](https://rust-lang.github.io/rust-project-goals/2025h1/null-enum-discriminant-debug-checks.html)   |     | [Ben Kimock][] |     |     |     |     |     |
| [Prepare const traits for stabilization](https://rust-lang.github.io/rust-project-goals/2025h1/const-trait.html)                                              |     | ✅         |     |     |     |     |     |
| [Promoting Parallel Front End](https://rust-lang.github.io/rust-project-goals/2025h1/parallel-front-end.html)                                                 | ✅   |           |     |     |     |     |     |
| [Publish first version of StableMIR on crates.io](https://rust-lang.github.io/rust-project-goals/2025h1/stable-mir.html)                                      | ✅   |           |     |     |     |     |     |
| [Research: How to achieve safety when linking separately compiled code](https://rust-lang.github.io/rust-project-goals/2025h1/safe-linking.html)              |     | ✅         |     |     |     |     |     |
| [Rust-for-Linux](https://rust-lang.github.io/rust-project-goals/2025h1/rfl.html)                                                                              | ✅   |           |     |     |     |     |     |
| ↳ ABI-modifying compiler flags                                                                        |     | ✅         |     |     | \*1 | \*2 |     |
| ↳ Extract dependency information, configure no-std externally                                         |     | ✅         |     |     |     | ✅   |     |
| [SVE and SME on AArch64](https://rust-lang.github.io/rust-project-goals/2025h1/arm-sve-sme.html)                                                              | ✅   |           |     |     |     |     |     |
| ↳ Extending type system to support scalable vectors                                                   |     | ✅         |     |     |     |     |     |
| ↳ Investigate SME support                                                                             | ✅   |           |     |     |     |     |     |
| ↳ Land nightly experiment for SVE types                                                               |     | ✅         |     |     |     |     |     |
| [Stabilize public/private dependencies](https://rust-lang.github.io/rust-project-goals/2025h1/pub-priv.html)                                                  | ✅   |           |     |     |     |     |     |
| [Unsafe Fields](https://rust-lang.github.io/rust-project-goals/2025h1/unsafe-fields.html)                                                                     |     | ✅         |     |     |     |     |     |
| [Use annotate-snippets for rustc diagnostic output](https://rust-lang.github.io/rust-project-goals/2025h1/annotate-snippets.html)                             |     |           |     |     |     |     |     |
| ↳ Production use of annotate-snippets                                                                 |     |           | \*3 |     |     |     |     |
| ↳ Standard reviews                                                                                    |     | ✅         |     |     |     |     |     |
| [rustc-perf improvements](https://rust-lang.github.io/rust-project-goals/2025h1/perf-improvements.html)                                                       |     |           |     |     |     |     | \*7 |


\*1: [RFC #3716](https://github.com/rust-lang/rfcs/pull/3716), currently in PFCP ([from here](https://rust-lang.github.io/rust-project-goals/2025h1/rfl.html))


\*2: For each of the relevant compiler flags ([from here](https://rust-lang.github.io/rust-project-goals/2025h1/rfl.html))


\*3: [Esteban Kuber][] will be the reviewer ([from here](https://rust-lang.github.io/rust-project-goals/2025h1/annotate-snippets.html))


\*4: 2-3 meetings expected; all involve lang ([from here](https://rust-lang.github.io/rust-project-goals/2025h1/seamless-rust-cpp.html))


\*5: including consultations for desired test behaviors and testing infra consumers ([from here](https://rust-lang.github.io/rust-project-goals/2025h1/compiletest-directive-rework.html))


\*6: Probably mostly [bootstrap] or whoever is more interested in reviewing [`compiletest`] changes ([from here](https://rust-lang.github.io/rust-project-goals/2025h1/compiletest-directive-rework.html))


\*7: Update performance regression policy ([from here](https://rust-lang.github.io/rust-project-goals/2025h1/perf-improvements.html))

### crates-io team
| Goal                                                 | [Design mtg.][valid_team_asks] |
| :--                                                  | :--     |
| [Crates.io mirroring](https://rust-lang.github.io/rust-project-goals/2025h1/verification-and-mirroring.html) | \*1 \*2 |


\*1: 1 hour Overall Design, threat model, and discussion of key management and quorums ([from here](https://rust-lang.github.io/rust-project-goals/2025h1/verification-and-mirroring.html))


\*2: 1 hour General design/implementation for automated index signing ([from here](https://rust-lang.github.io/rust-project-goals/2025h1/verification-and-mirroring.html))

### infra team
| Goal                                                 | [Good vibes][valid_team_asks] | [Deploy][valid_team_asks] | [r?][valid_team_asks] | [Design mtg.][valid_team_asks] |
| :--                                                  | :-- | :-- | :-- | :-- |
| [Crates.io mirroring](https://rust-lang.github.io/rust-project-goals/2025h1/verification-and-mirroring.html) |     |     |     | \*1 |
| [Metrics Initiative](https://rust-lang.github.io/rust-project-goals/2025h1/metrics-initiative.html)          | ✅   |     |     |     |
| [rustc-perf improvements](https://rust-lang.github.io/rust-project-goals/2025h1/perf-improvements.html)      | ✅   | \*2 | ✅   |     |


\*1: 3 hours of design and threat model discussion. Specific production infrastructure setup will come at a later time after the initial proof of concept. ([from here](https://rust-lang.github.io/rust-project-goals/2025h1/verification-and-mirroring.html))


\*2: rustc-perf improvements, testing infrastructure ([from here](https://rust-lang.github.io/rust-project-goals/2025h1/perf-improvements.html))

### lang team
| Goal                                                                                                | [Good vibes][valid_team_asks] | [Champion][valid_team_asks] | [Experiment][valid_team_asks] | [Design mtg.][valid_team_asks] | [RFC][valid_team_asks] | [Stabilize.][valid_team_asks] | [Policy][valid_team_asks] |
| :--                                                                                                 | :-- | :--           | :--           | :--                 | :--            | :-- | :-- |
| ["Stabilizable" prototype for expanded const generics](https://rust-lang.github.io/rust-project-goals/2025h1/min_generic_const_arguments.html)              | ✅   |               |               |                     |                |     |     |
| [Async](https://rust-lang.github.io/rust-project-goals/2025h1/async.html)                                                                                   |     |               |               |                     |                |     |     |
| ↳ Implementable trait aliases                                                                       |     | [Tyler Mandry][]      |               |                     | ✅              |     |     |
| ↳ Pin ergonomics                                                                                    |     | [Tyler Mandry][]      | ![Complete][] |                     |                |     |     |
| ↳ Return type notation                                                                              |     | [Niko Matsakis][] |               |                     | ![Complete][]  | ✅   |     |
| ↳ Trait for generators (sync)                                                                       |     | [Tyler Mandry][]      |               | 2 meetings expected | ✅              |     |     |
| ↳ Unsafe binders                                                                                    |     | \*2           |               |                     | Stretch goal   |     |     |
| ↳ `async fn` in `dyn Trait`                                                                         |     | [Niko Matsakis][] |               |                     |                |     |     |
| [Declarative (`macro_rules!`) macro improvements](https://rust-lang.github.io/rust-project-goals/2025h1/macro-improvements.html)                            |     |               |               |                     |                |     | \*4 |
| ↳ Design and iteration for macro fragment fields                                                    |     | [Josh Triplett][] |               | ✅                   | ✅              |     |     |
| ↳ Design for macro metavariable constructs                                                          | ✅   |               |               |                     |                |     |     |
| ↳ `macro_rules!` attributes                                                                         |     | [Josh Triplett][] |               |                     | ✅              |     |     |
| ↳ `macro_rules!` derives                                                                            |     | [Josh Triplett][] |               |                     | ✅              |     |     |
| [Evaluate approaches for seamless interop between C++ and Rust](https://rust-lang.github.io/rust-project-goals/2025h1/seamless-rust-cpp.html)               | ✅   | [Tyler Mandry][]      |               | \*3                 |                |     |     |
| [Experiment with ergonomic ref-counting](https://rust-lang.github.io/rust-project-goals/2025h1/ergonomic-rc.html)                                           |     | [Niko Matsakis][] |               |                     |                |     |     |
| [Expose experimental LLVM features for GPU offloading](https://rust-lang.github.io/rust-project-goals/2025h1/GPU-Offload.html)                              |     | [TC][]  | ![Complete][] |                     |                |     |     |
| [Externally Implementable Items](https://rust-lang.github.io/rust-project-goals/2025h1/eii.html)                                                            |     | [Niko Matsakis][] | ![Complete][] |                     |                |     |     |
| [Implement restrictions, prepare for stabilization](https://rust-lang.github.io/rust-project-goals/2025h1/restrictions.html)                                | ✅   |               |               |                     |                | ✅   |     |
| [Improve state machine codegen](https://rust-lang.github.io/rust-project-goals/2025h1/improve-rustc-codegen.html)                                           |     | [TC][]  | ✅             |                     |                |     |     |
| [Nightly support for ergonomic SIMD multiversioning](https://rust-lang.github.io/rust-project-goals/2025h1/simd-multiversioning.html)                       |     |               | ✅             | ✅                   | ✅              |     |     |
| [Null and enum-discriminant runtime checks in debug builds](https://rust-lang.github.io/rust-project-goals/2025h1/null-enum-discriminant-debug-checks.html) | ✅   |               |               |                     |                |     |     |
| [Prepare const traits for stabilization](https://rust-lang.github.io/rust-project-goals/2025h1/const-trait.html)                                            |     | [Niko Matsakis][] | ![Complete][] | \*1                 | (stretch goal) |     |     |
| [Research: How to achieve safety when linking separately compiled code](https://rust-lang.github.io/rust-project-goals/2025h1/safe-linking.html)            | ✅   | [Niko Matsakis][] | [Niko Matsakis][] |                     |                |     |     |
| [SVE and SME on AArch64](https://rust-lang.github.io/rust-project-goals/2025h1/arm-sve-sme.html)                                                            | ✅   |               |               |                     |                |     |     |
| ↳ Extending type system to support scalable vectors                                                 |     | [David Wood][]    |               |                     | ✅              |     |     |
| ↳ Investigate SME support                                                                           | ✅   |               |               |                     |                |     |     |
| [Unsafe Fields](https://rust-lang.github.io/rust-project-goals/2025h1/unsafe-fields.html)                                                                   | ✅   | [Scott McMurray][]     |               | ✅                   | ✅              |     |     |


\*1: first meeting scheduled for Jan; second meeting may be required ([from here](https://rust-lang.github.io/rust-project-goals/2025h1/const-trait.html))


\*2: [Niko Matsakis][] (stretch) ([from here](https://rust-lang.github.io/rust-project-goals/2025h1/async.html))


\*3: 2-3 meetings expected; all involve lang ([from here](https://rust-lang.github.io/rust-project-goals/2025h1/seamless-rust-cpp.html))


\*4: Discussed with [Eric Holk][] and [Vincenzo Palazzo][]; lang would decide whether to delegate specific matters to wg-macros ([from here](https://rust-lang.github.io/rust-project-goals/2025h1/macro-improvements.html))

### leadership-council team
| Goal                                                                     | [Alloc funds][valid_team_asks] | [Org][valid_team_asks] | [Policy][valid_team_asks] | [Misc][valid_team_asks] |
| :--                                                                      | :-- | :-- | :-- | :-- |
| [Crates.io mirroring](https://rust-lang.github.io/rust-project-goals/2025h1/verification-and-mirroring.html)                     |     |     | \*2 |     |
| [Organize Rust All-Hands 2025](https://rust-lang.github.io/rust-project-goals/2025h1/all-hands.html)                             | \*3 |     |     | \*4 |
| ↳ Team swag                                                              |     |     |     | \*5 |
| [Run the 2025H1 project goal program](https://rust-lang.github.io/rust-project-goals/2025h1/stabilize-project-goal-program.html) |     | \*6 |     |     |
| [Rust Vision Document](https://rust-lang.github.io/rust-project-goals/2025h1/rust-vision-doc.html)                               |     |     |     | \*1 |


\*1: Create supporting subteam + Zulip stream ([from here](https://rust-lang.github.io/rust-project-goals/2025h1/rust-vision-doc.html))


\*2: 1 hour synchronously discussing the threat models, policy, and quorum mechanism. Note: The ask from the Leadership Council is not a detailed exploration of *how* we address these threat models; rather, this will be a presentation of the threat models and a policy decision that the project cares about those threat models, along with the specific explanation of why a quorum is desirable to address those threat models. ([from here](https://rust-lang.github.io/rust-project-goals/2025h1/verification-and-mirroring.html))


\*3: ![Complete][] for event ([from here](https://rust-lang.github.io/rust-project-goals/2025h1/all-hands.html))


\*4: Prepare one or two plenary sessions ([from here](https://rust-lang.github.io/rust-project-goals/2025h1/all-hands.html))


\*5: Decide on team swag; suggestions very welcome! ([from here](https://rust-lang.github.io/rust-project-goals/2025h1/all-hands.html))


\*6: approve creation of new team ([from here](https://rust-lang.github.io/rust-project-goals/2025h1/stabilize-project-goal-program.html))

### libs team
| Goal                                                                           | [Good vibes][valid_team_asks] | [r?][valid_team_asks] |
| :--                                                                            | :-- | :-- |
| [Instrument the Rust standard library with safety contracts](https://rust-lang.github.io/rust-project-goals/2025h1/std-contracts.html) | ✅   |     |
| ↳ Standard Library Contracts                                                   |     | ✅   |

### libs-api team
| Goal                                                                                  | [Good vibes][valid_team_asks] | [Design mtg.][valid_team_asks] | [RFC][valid_team_asks] |
| :--                                                                                   | :-- | :-- | :-- |
| [Async](https://rust-lang.github.io/rust-project-goals/2025h1/async.html)                                                                     |     |     |     |
| ↳ Trait for generators (sync)                                                         |     |     | ✅   |
| [Evaluate approaches for seamless interop between C++ and Rust](https://rust-lang.github.io/rust-project-goals/2025h1/seamless-rust-cpp.html) | ✅   | \*1 |     |
| [Finish the libtest json output experiment](https://rust-lang.github.io/rust-project-goals/2025h1/libtest-json.html)                          | ✅   |     |     |


\*1: 2-3 meetings expected; all involve lang ([from here](https://rust-lang.github.io/rust-project-goals/2025h1/seamless-rust-cpp.html))

### opsem team
| Goal                                                                                                | [Good vibes][valid_team_asks] | [r?][valid_team_asks] |
| :--                                                                                                 | :-- | :--       |
| [Null and enum-discriminant runtime checks in debug builds](https://rust-lang.github.io/rust-project-goals/2025h1/null-enum-discriminant-debug-checks.html) | ✅   | [Ben Kimock][] |

### project-stable-mir team
| Goal                                                             | [r?][valid_team_asks] |
| :--                                                              | :-- |
| [Publish first version of StableMIR on crates.io](https://rust-lang.github.io/rust-project-goals/2025h1/stable-mir.html) | ✅   |

### release team
| Goal                                                 | [Good vibes][valid_team_asks] |
| :--                                                  | :-- |
| [Crates.io mirroring](https://rust-lang.github.io/rust-project-goals/2025h1/verification-and-mirroring.html) | \*1 |


\*1: Asynchronous discussion of the release team's role in the chain of trust, and preliminary approval of an experimental proof of concept. Approximately ~1 hour of total time across the 6-month period. ([from here](https://rust-lang.github.io/rust-project-goals/2025h1/verification-and-mirroring.html))

### rustdoc team
| Goal                                                                                                  | [Good vibes][valid_team_asks] | [r?][valid_team_asks] | [RFC][valid_team_asks] | [Stabilize.][valid_team_asks] |
| :--                                                                                                   | :-- | :-- | :-- | :-- |
| [Continue resolving `cargo-semver-checks` blockers for merging into cargo](https://rust-lang.github.io/rust-project-goals/2025h1/cargo-semver-checks.html)    | ✅   |     |     |     |
| [Making compiletest more maintainable: reworking directive handling](https://rust-lang.github.io/rust-project-goals/2025h1/compiletest-directive-rework.html) | \*1 |     |     |     |
| [Rust-for-Linux](https://rust-lang.github.io/rust-project-goals/2025h1/rfl.html)                                                                              | ✅   |     |     |     |
| ↳ Rustdoc features to extract doc tests                                                               |     | ✅   | ✅   | ✅   |


\*1: including consultations for desired test behaviors and testing infra consumers ([from here](https://rust-lang.github.io/rust-project-goals/2025h1/compiletest-directive-rework.html))

### spec team
| Goal                                                                  | [Good vibes][valid_team_asks] | [Spec text][valid_team_asks] |
| :--                                                                   | :-- | :--           |
| [Async](https://rust-lang.github.io/rust-project-goals/2025h1/async.html)                                                     |     |               |
| ↳ Return type notation                                                |     | [Niko Matsakis][] |
| [Implement restrictions, prepare for stabilization](https://rust-lang.github.io/rust-project-goals/2025h1/restrictions.html)  |     | [Joel Marcey][]   |
| [Publish first rust-lang-owned release of "FLS"](https://rust-lang.github.io/rust-project-goals/2025h1/spec-fls-publish.html) | ✅   |               |

### testing-devex team
| Goal                                                         | [Good vibes][valid_team_asks] |
| :--                                                          | :-- |
| [Finish the libtest json output experiment](https://rust-lang.github.io/rust-project-goals/2025h1/libtest-json.html) | ✅   |

### types team
| Goal                                                                                   | [Good vibes][valid_team_asks] | [r?][valid_team_asks] | [RFC][valid_team_asks] | [RFC rev.][valid_team_asks] | [Stabilize.][valid_team_asks] | [FCP][valid_team_asks] |
| :--                                                                                    | :-- | :--            | :--          | :-- | :-- | :-- |
| ["Stabilizable" prototype for expanded const generics](https://rust-lang.github.io/rust-project-goals/2025h1/min_generic_const_arguments.html) | ✅   |                |              |     |     |     |
| [Async](https://rust-lang.github.io/rust-project-goals/2025h1/async.html)                                                                      |     |                |              |     |     |     |
| ↳ Implementable trait aliases                                                          |     | ✅              | ✅            |     |     |     |
| ↳ Return type notation                                                                 |     | ✅              |              |     | ✅   |     |
| ↳ Unsafe binders                                                                       |     |                | Stretch goal |     |     |     |
| [Model coherence in a-mir-formality](https://rust-lang.github.io/rust-project-goals/2025h1/formality.html)                                     | ✅   |                |              |     |     |     |
| [Next-generation trait solver](https://rust-lang.github.io/rust-project-goals/2025h1/next-solver.html)                                         | ✅   | ✅              |              |     |     | \*3 |
| [Prepare const traits for stabilization](https://rust-lang.github.io/rust-project-goals/2025h1/const-trait.html)                               |     |                |              | \*1 |     |     |
| ↳ Formalize const-traits in a-mir-formality                                            | \*2 |                |              |     |     |     |
| [SVE and SME on AArch64](https://rust-lang.github.io/rust-project-goals/2025h1/arm-sve-sme.html)                                               | ✅   |                |              |     |     |     |
| ↳ Extending type system to support scalable vectors                                    |     |                | ✅            |     |     |     |
| ↳ Investigate SME support                                                              | ✅   |                |              |     |     |     |
| ↳ Land nightly experiment for SVE types                                                |     |                | ✅            |     |     |     |
| [Scalable Polonius support on nightly](https://rust-lang.github.io/rust-project-goals/2025h1/Polonius.html)                                    |     | [Matthew Jasper][] |              |     |     |     |


\*1: Types team needs to validate the approach ([from here](https://rust-lang.github.io/rust-project-goals/2025h1/const-trait.html))


\*2: During types team office hours, we'll share information about our progress. ([from here](https://rust-lang.github.io/rust-project-goals/2025h1/const-trait.html))


\*3: for necessary refactorings ([from here](https://rust-lang.github.io/rust-project-goals/2025h1/next-solver.html))

### wg-macros team
| Goal                                                                     | [Good vibes][valid_team_asks] | [Policy][valid_team_asks] |
| :--                                                                      | :-- | :-- |
| [Declarative (`macro_rules!`) macro improvements](https://rust-lang.github.io/rust-project-goals/2025h1/macro-improvements.html) |     | \*1 |
| ↳ Design for macro metavariable constructs                               | ✅   |     |


\*1: Discussed with [Eric Holk][] and [Vincenzo Palazzo][]; lang would decide whether to delegate specific matters to wg-macros ([from here](https://rust-lang.github.io/rust-project-goals/2025h1/macro-improvements.html))


## Definitions

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

## Frequently asked questions

### What goals were not accepted?

The following goals were proposed but ultimately not accepted, either for want of resources or consensus. In some cases narrower versions of these goals were prepared.

| Goal                                          | Point of contact | Progress            |
| :--                                           | :--          | :--                 |
| [Field Projections](https://rust-lang.github.io/rust-project-goals/2025h1/field-projections.html)     | [Benno Lossin][]     | (no tracking issue) |
| [Rust Specification Testing](https://rust-lang.github.io/rust-project-goals/2025h1/spec-testing.html) | [Connor Horman][] | (no tracking issue) |


### What do the column names like "Ded. r?" mean?

[valid_team_asks]: #what-do-the-column-names-like-ded-r-mean

Those column names refer to specific things that can be asked of teams:

| Ask                            | aka         | Description                                                                                                                                                                                    |
| :--                            | :--         | :--                                                                                                                                                                                            |
| "Allocate funds"               | Alloc funds | allocate funding                                                                                                                                                                               |
| "Discussion and moral support" | Good vibes  | approve of this direction and be prepared for light discussion on Zulip or elsewhere                                                                                                           |
| "Deploy to production"         | Deploy      | deploy code to production (e.g., on crates.io                                                                                                                                                  |
| "Standard reviews"             | r?          | review PRs (PRs are not expected to be unduly large or complicated)                                                                                                                            |
| "Dedicated reviewer"           | Ded. r?     | assign a specific person (or people) to review a series of PRs, appropriate for large or complex asks                                                                                          |
| "Lang-team champion"           | Champion    | member of lang team or advisors who will champion the design within team                                                                                                                       |
| "Lang-team experiment"         | Experiment  | begin a [lang-team experiment](https://lang-team.rust-lang.org/how_to/experiment.html) authorizing experimental impl of lang changes before an RFC is written; limited to trusted contributors |
| "Design meeting"               | Design mtg. | hold a synchronous meeting to review a proposal and provide feedback (no decision expected)                                                                                                    |
| "RFC decision"                 | RFC         | review an RFC and deciding whether to accept                                                                                                                                                   |
| "RFC secondary review"         | RFC rev.    | briefly review an RFC without need of a formal decision                                                                                                                                        |
| "Org decision"                 | Org         | reach a decision on an organizational or policy matter                                                                                                                                         |
| "MCP decision"                 | MCP         | accept a [Major Change Proposal](https://forge.rust-lang.org/compiler/mcp.html)                                                                                                                |
| "ACP decision"                 | ACP         | accept an [API Change Proposal](https://std-dev-guide.rust-lang.org/development/feature-lifecycle.html)                                                                                        |
| "Finalize specification text"  | Spec text   | assign a spec team liaison to finalize edits to Rust reference/specification                                                                                                                   |
| "Stabilization decision"       | Stabilize.  | reach a decision on a stabilization proposal                                                                                                                                                   |
| "Policy decision"              | Policy      | make a decision related to team policy                                                                                                                                                         |
| "FCP decision(s)"              | FCP         | make formal decision(s) that require 'checkboxes' and a FCP (Final Comment Period)                                                                                                             |
| "Blog post approval"           | Blog        | approve of posting about this on the main Rust blog                                                                                                                                            |
| "Miscellaneous"                | Misc        | do some one-off action as described in the notes                                                                                                                                               |


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
[codegen-c-maintainers]: https://www.rust-lang.org/governance/teams
[community]: https://www.rust-lang.org/governance/teams
[community-content]: https://github.com/rust-community/content-team
[community-events]: https://github.com/rust-community/events-team
[community-localization]: https://github.com/rust-lang/community-localization
[community-rustbridge]: https://github.com/rustbridge/team
[community-survey]: https://github.com/rust-lang/surveys
[compiler]: http://github.com/rust-lang/compiler-team
[compiler-fcp]: https://www.rust-lang.org/governance/teams
[council-librarians]: https://www.rust-lang.org/governance/teams
[crate-maintainers]: https://www.rust-lang.org/governance/teams
[crates-io]: https://github.com/rust-lang/crates.io
[crates-io-admins]: https://www.rust-lang.org/governance/teams
[crates-io-on-call]: https://www.rust-lang.org/governance/teams
[devtools]: https://github.com/rust-dev-tools/dev-tools-team
[docker]: https://www.rust-lang.org/governance/teams
[docs-rs]: https://github.com/rust-lang/docs.rs
[docs-rs-reviewers]: https://www.rust-lang.org/governance/teams
[edition]: http://github.com/rust-lang/edition-team
[emacs]: https://www.rust-lang.org/governance/teams
[emscripten]: https://www.rust-lang.org/governance/teams
[foundation-board-project-directors]: https://www.rust-lang.org/governance/teams
[foundation-email-redirects]: https://www.rust-lang.org/governance/teams
[fuchsia]: https://www.rust-lang.org/governance/teams
[goal-owners]: https://www.rust-lang.org/governance/teams
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
[ospp-contributors]: https://www.rust-lang.org/governance/teams
[project-async-crashdump-debugging]: https://github.com/rust-lang/async-crashdump-debugging-initiative
[project-const-generics]: https://github.com/rust-lang/project-const-generics
[project-const-traits]: https://www.rust-lang.org/governance/teams
[project-dyn-upcasting]: https://github.com/rust-lang/dyn-upcasting-coercion-initiative
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
[types]: https://github.com/rust-lang/types-team
[types-fcp]: https://www.rust-lang.org/governance/teams
[vim]: https://www.rust-lang.org/governance/teams
[wasi]: https://www.rust-lang.org/governance/teams
[wasm]: https://www.rust-lang.org/governance/teams
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
[wg-polonius]: https://rust-lang.github.io/compiler-team/working-groups/polonius/
[wg-polymorphization]: https://rust-lang.github.io/compiler-team/working-groups/polymorphization/
[wg-prioritization]: https://rust-lang.github.io/compiler-team/working-groups/prioritization/
[wg-rustc-dev-guide]: https://rust-lang.github.io/compiler-team/working-groups/rustc-dev-guide/
[wg-rustc-reading-club]: https://rust-lang.github.io/rustc-reading-club/
[wg-safe-transmute]: https://github.com/rust-lang/project-safe-transmute
[wg-secure-code]: https://github.com/rust-secure-code/wg
[wg-security-response]: https://github.com/rust-lang/wg-security-response
[wg-self-profile]: https://rust-lang.github.io/compiler-team/working-groups/self-profile/
[wg-triage]: https://www.rust-lang.org/governance/teams
[windows]: https://www.rust-lang.org/governance/teams


[Bastian Kersting]: https://github.com/1c3t3a
[Boxy]: https://github.com/BoxyUwU
[Scott Schafer]: https://github.com/Muscraft
[Sparrow Li]: https://github.com/SparrowLii
[Manuel Drehwald]: https://github.com/ZuseZ4
[Alejandra González]: https://github.com/blyxyas
[Celina G. Val]: https://github.com/celinval
[Connor Horman]: https://github.com/chorman0773
[David Wood]: https://github.com/davidtwco
[Jacob Finkelman]: https://github.com/eh2406
[Eric Holk]: https://github.com/eholk
[Ed Page]: https://github.com/epage
[Esteban Kuber]: https://github.com/estebank
[Folkert de Vries]: https://github.com/folkertdev
[Jacob Pratt]: https://github.com/jhpratt
[Jieyou Xu]: https://github.com/jieyouxu
[Joel Marcey]: https://github.com/joelmarcey
[Josh Triplett]: https://github.com/joshtriplett
[Jack Wrenn]: https://github.com/jswrenn
[lcnr]: https://github.com/lcnr
[Rémy Rakic]: https://github.com/lqd
[Mara Bos]: https://github.com/m-ou-se
[Matthew Jasper]: https://github.com/matthewjasper
[Niko Matsakis]: https://github.com/nikomatsakis
[Predrag Gruevski]: https://github.com/obi1kenobi
[Oliver Scherer]: https://github.com/oli-obk
[Ben Kimock]: https://github.com/saethlin
[Scott McMurray]: https://github.com/scottmcm
[Santiago Pastorino]: https://github.com/spastorino
[Tyler Mandry]: https://github.com/tmandry
[TC]: https://github.com/traviscross
[Luca Versari]: https://github.com/veluca93
[Vincenzo Palazzo]: https://github.com/vincenzopalazzo
[walterhpearce]: https://github.com/walterhpearce
[Benno Lossin]: https://github.com/y86-dev
[Jane Lusby]: https://github.com/yaahc


[Complete]: https://img.shields.io/badge/Complete-green
[Help wanted]: https://img.shields.io/badge/Help%20wanted-yellow
[Not funded]: https://img.shields.io/badge/Not%20yet%20funded-red
[TBD]: https://img.shields.io/badge/TBD-red
[Team]: https://img.shields.io/badge/Team%20ask-red

