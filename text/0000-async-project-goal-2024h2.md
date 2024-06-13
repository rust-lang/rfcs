- Feature Name: (fill me in with a unique ident, `my_awesome_feature`)
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)
- Goal owners: [tmandry][], [nikomatsakis][]
- Goal teams: [Lang], [Libs-API]

# Summary
[summary]: #summary

This is a proposed flagship goal for 2024h2 covering **async Rust**. You can [read more about the project goal slate and its associated process here](https://rust-lang.github.io/rust-project-goals/2024h2/slate.html). This RFC is prepared using the project goal template, which differs from the typical RFC template.

The overall goal is **bringing the Async Rust experience closer to parity with sync Rust** via the following steps:

* stabilizing async closures, thus enabling richer, combinator APIs like sync Rust's [`Iterator`](https://doc.rust-lang.org/std/iter/trait.Iterator.html);
* [resolving the "send bound problem"](#resolve-the-send-bound-problem), thus enabling foundational, generic traits like Tower's [`Service`]() trait;
* [stabilizing a trait in libstd for async iteration](#stabilize-trait-for-async-iteration), thus enabling the ecosystem to build atop a stable foundation;
* [authoring a draft RFC for async vision](#author-draft-rfc-for-async-vision), thus aligning the project around a coherent vision;
* [completing the async drop experiments](#complete-async-drop-experiments) proposed in [MCP 727][], laying the groundwork for resolving the the next major gap in language feature support.

Approving this goal implies agreement from the [Lang][] and [Libs-API][] team to the items marked as ![Team][] in the table of work items, along with potentially other design meetings as needed. The expectation is that 3-4 design meetings will be needed from lang over the course of H2 and 1-2 from libs API. Reviewing the async vision doc is expected to be the biggest requirement.

# Motivation

In 2024 we plan to deliver several critical async Rust building block features, most notably support for *async closures* and *`Send` bounds*. This is part of a multi-year program aiming to raise the experience of authoring "async Rust" to the same level of quality as "sync Rust". Async Rust is a crucial growth area, with 52% of the respondents in the [2023 Rust survey](https://blog.rust-lang.org/2024/02/19/2023-Rust-Annual-Survey-2023-results.html) indicating that they use Rust to build server-side or backend applications. 

## The status quo

### Async Rust performs great, but can be hard to use

Async Rust is the most common Rust application area according to our [2023 Rust survey](https://blog.rust-lang.org/2024/02/19/2023-Rust-Annual-Survey-2023-results.html). Rust is a great fit for networked systems, especially in the extremes:

* **Rust scales up**. Async Rust reduces cost for large dataplanes because a single server can serve high load without significantly increasing tail latency.
* **Rust scales down.** Async Rust can be run without requiring a garbage collector or [even an operating system][embassy], making it a great fit for embedded systems.
* **Rust is reliable.** Networked services run 24/7, so Rust's "if it compiles, it works" mantra means unexpected failures and, in turn, fewer pages in the middle of the night.

Despite async Rust's popularity, using async I/O makes Rust significantly harder to use. As one Rust user memorably put it, "Async Rust is Rust on hard mode." Several years back the async working group collected a number of ["status quo" stories](https://rust-lang.github.io/wg-async/vision/submitted_stories/status_quo.html) as part of authoring an async vision doc. These stories reveal a number of characteristic challenges:

* Common language features like ~~[traits](https://rust-lang.github.io/wg-async/vision/submitted_stories/status_quo/alan_needs_async_in_traits.html)~~ (they [do now][afitblog], though gaps remain), closures, and [drop](https://rust-lang.github.io/wg-async/vision/submitted_stories/status_quo/alan_finds_database_drops_hard.html) do not support async, meaning that [users cannot write Rust code in the way they are accustomed to](https://rust-lang.github.io/wg-async/vision/submitted_stories/status_quo/barbara_plays_with_async.html?highlight=closure#the-story). In many cases there are workarounds or crates that can close the gap, but users have to learn about and find those crates.
* Common async idioms have "sharp edges" that lead to unexpected failures, forcing users to manage [cancellation safety](https://rust-lang.github.io/wg-async/vision/submitted_stories/status_quo/barbara_gets_burned_by_select.html), subtle [deadlocks](https://rust-lang.github.io/wg-async/vision/submitted_stories/status_quo/aws_engineer/solving_a_deadlock.html) and other failure modes for [buffered streams](https://rust-lang.github.io/wg-async/vision/submitted_stories/status_quo/barbara_battles_buffered_streams.html). See also tmandry's blog post on [Making async Rust reliable](https://tmandry.gitlab.io/blog/posts/making-async-reliable/)).
* Using async today requires users to select a runtime which provides many of the core primitives. Selecting a runtime as a user [can be stressful](https://rust-lang.github.io/wg-async/vision/submitted_stories/status_quo/barbara_makes_their_first_steps_into_async.html#the-wrong-time-for-big-decisions), as the [decision once made is hard to reverse](https://rust-lang.github.io/wg-async/vision/submitted_stories/status_quo/barbara_wishes_for_easy_runtime_switch.html). Moreover, in an attempt to avoid "picking favories", the project has not endorsed a particular runtime, making it [harder to write new user documentation](https://rust-lang.github.io/wg-async/vision/submitted_stories/status_quo/niklaus_wants_to_share_knowledge.html). Libaries meanwhile [cannot easily be made interoperable across runtimes](https://rust-lang.github.io/wg-async/vision/submitted_stories/status_quo/barbara_writes_a_runtime_agnostic_lib.html) and so are often written against the API of a particular runtime; even when libraries can be retargeted, it is difficult to do things like run their test suites to test compatibility. [Mixing and matching libraries can cause surprising failures.](https://rust-lang.github.io/wg-async/vision/submitted_stories/status_quo/alan_started_trusting_the_rust_compiler_but_then_async.html)

[afitblog]: https://blog.rust-lang.org/2023/12/21/async-fn-rpit-in-traits.html
[embassy]: https://github.com/embassy-rs/embassy
[tokio]: https://tokio.rs/

### First focus: language parity, interop traits

Based on the above analysis, the Rust org has been focused on driving async/sync language parity, especially in those areas that block the development of a rich ecosystem. The biggest progress took place in [Dec 2023][afitblog], when async fn in traits and return position impl trait in trait were stabilizd. Other work includes authoring the async vision doc, stabilizing helpers like [`std::future::poll_fn`](https://doc.rust-lang.org/std/future/fn.poll_fn.html)), and polishing and improving async error messages.

### Lack of internal alignment within the Rust org about the direction for async

Progress on async-related issues within the Rust org has been slowed due to lack of coherence around a vision and clear steps. Discussion gets stuck not only on technical details but also on what problems to be resolving first. The lack of a centrally agreed upon vision has also made it hard for general purpose teams such as [Lang][] or [Libs-API][] to decide how to respond to requests to e.g. stabilize particular async-related constructs, as they lack a means to judge whether stabilizing any particular construct is really the right step forward and whether it meets its design needs.

## The next few steps

In the second half of 2024 we are planning on the following work items:

* [stabilize async closures](#stabilize-async-closures)
* [resolve the "Send bound" problem](#stabilize-async-closures)
* [stabilize trait for async iteration](#stabilize-trait-for-async-iteration)
* [author draft RFC for async vision](#author-draft-rfc-for-async-vision)
* [complete async drop experiments](#complete-async-drop-experiments) (currently unfunded)

### Stabilize async closures

Building ergonomic APIs in async is often blocked by the lack of *async closures*. Async combinator-like APIs today typically make use an ordinary Rust closure that returns a future, such as the `filter` API from [`StreamExt`](https://docs.rs/futures/latest/futures/prelude/stream/trait.StreamExt.html#method.filter):

```rust
fn filter<Fut, F>(self, f: F) -> Filter<Self, Fut, F>
where
    F: FnMut(&Self::Item) -> Fut,
    Fut: Future<Output = bool>,
    Self: Sized,
```

This approach however does not allow the closure to access variables captured by reference from its environment:

```rust
let mut accept_list = vec!["foo", "bar"]
stream
    .filter(|s| async { accept_list.contains(s) })
```

The reason is that data captured from the environment is stored in `self`. But the signature for sync closures does not permit the return value (`Self::Output`) to borrow from `self`:

```rust
trait FnMut<A>: FnOnce<A> {
    fn call_mut(&mut self, args: A) -> Self::Output;
}
```

To support natural async closures, a trait is needed where `call_mut` is an `async fn`, which would allow the returned future to borrow from `self` and hence modify the environment (e.g., `accept_list`, in our example above). Or, desugared, something that is equivalent to:

```rust
trait AsyncFnMut<A>: AsyncFnOnce<A> {
    fn call_mut<'s>(
        &'s mut self,
        args: A
    ) -> impl Future<Output = Self::Output> + use<'s, A>;
    //                                        ^^^^^^^^^^ note that this captures `'s`
    //
    // (This precise capturing syntax is unstable and covered by
    // rust-lang/rust#123432).
}
```

The goal for this year to be able to 

* support some "async equivalent" to `Fn`, `FnMut`, and `FnOnce` bounds
    * this should be usable in all the usual places
* support some way to author async closure expressions

These features should be sufficient to support methods like `filter` above.

The details (syntax, precise semantics) will be determined via experimentation and subject to RFC.

### Stabilize trait for async iteration

There has been extensive discussion about the best form of the trait for async iteration (sometimes called `Stream`, sometimes `AsyncIter`, and now being called `AsyncGen`). We believe the design space has been sufficiently explored that it should be possible to author an RFC laying out the options and proposing a specific plan.

### Resolve the ["send bound"][sb] problem

Although async functions in traits were stabilized, there is currently no way to write a generic function that requires only impls where the returned futures are `Send`. This blocks the use of async function in traits in some core ecosystem crates, such as [tower](https://crates.io/crates/tower), which want to work across all kinds of async executors. This problem is called the ["send bound"][sb] problem and there has been extensive discussion of the various ways to solve it. [RFC #3654][] has been opened proposing one solution and describing why that path is preferred. Our goal for the year is to adopt *some* solution on stable.

[RFC #3654]: https://github.com/rust-lang/rfcs/pull/3654

### Author draft RFC for async vision

We plan to revise the [Async Vision Doc][AVD] and restructure it as a draft RFC, most likely to be approved by the [Lang][] and [Libs-API][] teams (we do not necessarily expect that RFC to be accepted by end of year). Our observation is that the previous version of the async vision doc, which was never RFC'd, never attained the legitimacy of being the "plan of record". In addition, a number of things have changed in the intervening years (for example, async functions in traits are now stable) and we are in a position to identify clearer next steps. The 

[AVD]: https://rust-lang.github.io/wg-async/vision.html

This RFC will lay out a "plan of attack" for async, including both obvious good things (similar to [async closures][]) but also "known unknowns" and ways to resolve them. Areas the RFC is expected to cover are as follows:

[Making Async Rust Reliable]: https://tmandry.gitlab.io/blog/posts/making-async-reliable/

* Status quo, covering biggest challenges
    * Lack of strong learning material
    * Common idioms contain footguns that cause unexpected failures (see e.g., Tyler's blog post [Making Async Rust Reliable][])
    * Low-level performance hurdles, such as large future sizes and downsides of the poll model
    * Fragmentation between runtimes
* Design axioms to pursue for async (see e.g. axioms proposed)
* Goals, some variant of
    * Free of accidental complexity
    * Easy to get started
    * Easy to pick executor and integrate with other systems (e.g., mobile runtimes, company-specific threadpools, etc)
    * Moderately easy to adapt to "extreme" embedded environments
    * Good performance by default, peak performance with tuning
* Key unknowns in terms of how to achieve the above goals, for example 
    * how to replace footgun-prone APIs with more reliable alternatives:
        * buffered-streams, cancellation (esp. due to use of select)
        * patterns to express
            * merged streams -- processing one stream of data with occasional control events
            * task parallelism
        * cleanup and teardown
            * ordered destruction
    * how should async drop work (`?Leak` vs `?Drop` vs whatever):
        * how to prevent async drop from occuring in sync contexts?
    * what does runtime interface look like?
        * Can/should we be generic over runtime
* Strategy for how to get where we are going
    * What problems to attack first
    * How to reduce or find solutions to the above unknowns

### Complete async drop experiments

Authors of async code frequently need to call async functions as part of resource cleanup. Because Rust today only supports synchronous destructors, this cleanup must take place using alternative mechanisms, forcing a divergence between sync Rust (which uses destructors to arrange cleanup) and async Rust. [MCP 727][] proposed a series of experiments aimed at supporting async drop in the compiler. We would like to continue and complete those experiments. These experiments are aimed at defining how support for async drop will be implemented in the compiler and some possible ways that we could modify the type system to support it (in particular, one key question is how to prevent types that whose drop is async from being dropped in sync code).

## The "shiny future" we are working towards

Our eventual goal is to provide Rust users building on async with

* the same core language capabilities as sync Rust (async traits with dyn dispatch, async closures, async drop, etc);
* reliable and standardized abstractions for async control flow (streams of data, error recovery, concurrent execution);
* an easy "getting started" experience that builds on a rich ecosystem;
* the ability to easily adopt custom runtimes when needed for particular environments, language interop, or specific business needs.

# Design axiom

* **We lay the foundations for a thriving ecosystem.** The role of the Rust org is to deelop the rudiments that support an interoperable and thriving async crates.io ecosystem.
* **Uphold sync's Rust bar for reliability.** Sync Rust famously delivers on the general feeling of "if it compiles, in works" -- async Rust should do the same.
* **When in doubt, zero-cost is our compass.** Many of Rust's biggest users are choosing it becase they know it can deliver the same performnace (or better) than C. If we adopt abstractions that add overhead, we are compromising that core strength. As we build out our designs, we ensure that they don't introduce an "abstraction tax" for using them.
* **From embedded to GUI to the cloud.** Async Rust covers a wide variety of use cases and we aim to make designs that can span those differing constraints with ease.
* **Consistent, incremental progress.** People are building async Rust systems *today* -- we need to ship incremental improvements while also steering towards the overall outcome we want.

# Ownership and other resources

Here is a detailed list of the work to be done and who is expected to do it. This table includes the work to be done by owners and the work to be done by Rust teams (subject to approval by the team in an RFC/FCP). The overall owners of the async effort (and authors of this goal document) are [tmandry][] and [nikomatsakis][]. We have identified owners for subitems below; these may change over time.

* The ![Funded][] badge indicates that the owner has committed and work will be funded by their employer or other sources.
* The ![Team][] badge indicates a requirement where Team support is needed.

| Subgoal                                  | Owner(s) or team(s)                     | Status              |
| ---------------------------------------- | --------------------------------------- | ------------------- |
| overall program management               | [tmandry][], [nikomatsakis][]           | ![Funded][]         |
| stabilize async closures                 |                                         | ![Funded][]         |
| ↳ ~~implementation~~                     | ~~[compiler-errors][]~~                 | ![Complete][]       |
| ↳ author RFC                             | [nikomatsakis][] or [compiler-errors][] | ![Funded][]         |
| ↳ approve RFC                            | ![Team][] [Lang]                        |                     |
| ↳ stabilization                          | [compiler-errors][]                     | ![Funded][]         |
| resolve the ["send bound"][sb] problem   |                                         | ![Funded][]         |
| ↳ ~~RTN implementation~~                 | ~~[compiler-errors][]~~                 | ![Complete][]       |
| ↳ ~~RTN RFC~~                            | [nikomatsakis][]                        | ![Complete][]       |
| ↳ approve RTN RFC or provide alternative | ![Team][] [Lang]                        |                     |
| ↳ stabilization                          | [compiler-errors][]                     | ![Funded][]         |
| stabilize trait for async iteration      |                                         | ![Funded][]         |
| ↳ author RFC                             | [eholk][]                               | ![Funded][]         |
| ↳ approve RFC                            | ![Team][] [Libs-API]                    | ![Funded][]         |
| ↳ implementation                         | [eholk][]                               | ![Funded][]         |
| author draft RFC for async vision        |                                         | ![Funded][]         |
| ↳ author RFC                             | [tmandry][]                             | ![Funded][]         |
| ↳ approve RFC                            | ![Team][] [Lang], [Libs-API]            |                     |
| complete async drop experiments          |                                         |                     |
| ↳ ~~author MCP~~                         | ~~[petrochenkov][]~~                    | ![Complete][]       |
| ↳ ~~approve MCP~~                        | ~~[Compiler]~~                          | ![Complete][]       |
| ↳ implementation work                    | [petrochenkov][]                        | ![Not funded][] (*) |

(*) Implementation work on async drop experiments is currently unfunded. We are trying to figure out next steps.

[Funded]: https://img.shields.io/badge/Funded-yellow
[Not funded]: https://img.shields.io/badge/Not%20yet%20funded-red
[Approved]: https://img.shields.io/badge/Approved-green
[Not approved]: https://img.shields.io/badge/Not%20yet%20approved-red
[Complete]: https://img.shields.io/badge/Complete-green
[TBD]: https://img.shields.io/badge/TBD-red


## Support needed from the project

Agreement from [Lang] and [Libs-API] to the items marked ![Team][] in the table above. Potentially other design meetings as needed.

Expectation is that 3-4 design meetings will be needed from lang over the course of H2 and 1-2 from libs API. Reviewing the async vision doc is expected to be the biggest requirement.

# Outputs and milestones

Stabilized features for

* async closures
* a ["send bound"][sb] solution, most likely [RTN][]

# Frequently asked questions

## Why focus on send bounds + async closures?

These are the two features that together block the authoring of traits for a number of common interop purposes. Send bounds are needed for generic traits like the `Service` trait. Async closures are needed for rich combinator APIs like iterators.

## Why not work on dyn dispatch for async fn in traits?

Async fn in traits do not currently support native dynamic dispatch. We have explored a [number of designs for making it work](https://smallcultfollowing.com/babysteps/blog/2021/09/30/dyn-async-traits-part-1/) but are not currently prioritizing that effort. It was determined that this idea is lower priority because it is possible to [workaround](https://smallcultfollowing.com/babysteps/blog/2021/10/15/dyn-async-traits-part-6/) the gap by having the  `#[trait_variant]` crate produce a dynamic dispatch wrapper type (e.g., `#[trait_variant(dyn = DynWidget)] trait Widget` would create a type `DynWidget<'_>` that acts like a `Box<dyn Widget>`). We do expect to support dyn async trait, hopefully in 2025.

## Why are we moving forward on a trait for async iteration?

There has been extensive discussion about the best design for the "Stream" or "async iter" trait and we judge that the design space is well understood. We would like to unblock generator syntax in 2025 which will require some form of trait.

The majority of the debate about the trait has been on the topic of whether to base the trait on a `poll_next` function, as we do today, or to try and make the trait use `async fn next`, making it more anaologous with the `Iterator` trait (and potentially even making it be two versions of a single trait). We will definitely explore forwards compatibility questions as part of this discussion. nikomatsakis for example still wants to explore maybe-async-like designs, especially for combinator APIs like `map`. However, we also refer to the design axiom that that "when in doubt, zero-cost is our compass" -- we believe we should be able to stabilize a trait that does the low-level details right, and then design higher level APIs atop that.

## Why work on a revised async vision doc? Don't we already have one?

The existing doc was authored some time back and is in need of an update. Moreover, the original doc was never RFC'd and we have found that it lacks a certain measure of "authority" as a result. We would like to drive stronger alignment on the path forward so that we can focus more on execution.

## What about "maybe async", effect systems, and keyword generics?

Keyword generics is an ambitious initiative to enable code that is "maybe async". It has generated significant controversy, with some people feeling it is necessary for Rust to scale and others judging it to be overly complex. We anticipate having more debate on this topic as part of drafting the async vision doc.

[tmandry]: https://github.com/tmandry
[nikomatsakis]: https://github.com/nikomatsakis
[compiler-errors]: https://github.com/compiler-errors
[eholk]: https://github.com/eholk
[petrochenkov]: https://github.com/petrochenkov
[Team]: https://img.shields.io/badge/Team%20ask-red
[MCP 727]: https://github.com/rust-lang/compiler-team/issues/727
[Lang]: https://www.rust-lang.org/governance/teams/lang
[Libs-API]: https://www.rust-lang.org/governance/teams/library#team-libs-api
[sb]: https://smallcultfollowing.com/babysteps/blog/2023/02/01/async-trait-send-bounds-part-1-intro/
