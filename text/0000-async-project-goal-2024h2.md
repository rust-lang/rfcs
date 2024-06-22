- Feature Name: (fill me in with a unique ident, `my_awesome_feature`)
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)
- Goal owners: [tmandry][], [nikomatsakis][]
- Goal teams: [Lang], [Libs-API], [Libs]

# Summary
[summary]: #summary

This is a proposed flagship goal for 2024h2 covering **async Rust**. You can [read more about the project goal slate and its associated process here](https://rust-lang.github.io/rust-project-goals/2024h2/slate.html). This RFC is prepared using the project goal template, which differs from the typical RFC template.

The overall goal is **bringing the Async Rust experience closer to parity with sync Rust**. We have identified three high-priority goals that we believe would do the most to improve async over the long term:

* [resolving the "send bound problem"](#resolve-the-send-bound-problem), thus enabling foundational, generic traits like Tower's [`Service`]() trait;
* stabilizing async closures, thus enabling richer, combinator APIs like sync Rust's [`Iterator`](https://doc.rust-lang.org/std/iter/trait.Iterator.html);
* [reorganizing the async WG](#reorganize-the-async-wg), so the project can benefit from a group of async rust experts with deep knowledge of the space that can align around a shared vision;

We have also identified two "stretch goals":

* [stabilizing a trait in libstd for async iteration](#stabilize-trait-for-async-iteration), thus enabling the ecosystem to build atop a stable foundation;
* [completing the async drop experiments](#complete-async-drop-experiments) proposed in [MCP 727][], laying the groundwork for resolving the the next major gap in language feature support.

Approving this goal implies agreement from the [Lang][], [Libs][], and [Libs-API][] teams to the items marked as ![Team][] in the table of work items, along with potentially other design meetings as needed.

# Motivation

In 2024 we plan to deliver several critical async Rust building block features, most notably support for *async closures* and *`Send` bounds*. This is part of a multi-year program aiming to raise the experience of authoring "async Rust" to the same level of quality as "sync Rust". Async Rust is a crucial growth area, with 52% of the respondents in the [2023 Rust survey](https://blog.rust-lang.org/2024/02/19/2023-Rust-Annual-Survey-2023-results.html) indicating that they use Rust to build server-side or backend applications. 

## The status quo

### Async Rust performs great, but can be hard to use

Async Rust is the most common Rust application area according to our [2023 Rust survey](https://blog.rust-lang.org/2024/02/19/2023-Rust-Annual-Survey-2023-results.html). Rust is a great fit for networked systems, especially in the extremes:

* **Rust scales up**. Async Rust reduces cost for large dataplanes because a single server can serve high load without significantly increasing tail latency.
* **Rust scales down.** Async Rust can be run without requiring a garbage collector or [even an operating system][embassy], making it a great fit for embedded systems.
* **Rust is reliable.** Networked services run 24/7, so Rust's "if it compiles, it works" mantra means fewer unexpected failures and, in turn, fewer pages in the middle of the night.

Despite async Rust's popularity, using async I/O makes Rust significantly harder to use. As one Rust user memorably put it, "Async Rust is Rust on hard mode." Several years back the async working group collected a number of ["status quo" stories](https://rust-lang.github.io/wg-async/vision/submitted_stories/status_quo.html) as part of authoring an async vision doc. These stories reveal a number of characteristic challenges:

* Common language features do not support async, meaning that [users cannot write Rust code in the way they are accustomed to](https://rust-lang.github.io/wg-async/vision/submitted_stories/status_quo/barbara_plays_with_async.html?highlight=closure#the-story):
  * [x] ~~[traits](https://rust-lang.github.io/wg-async/vision/submitted_stories/status_quo/alan_needs_async_in_traits.html)~~ (they [do now][afitblog], though gaps remain)
  * [ ] closures
  * [ ] [drop](https://rust-lang.github.io/wg-async/vision/submitted_stories/status_quo/alan_finds_database_drops_hard.html)
  In many cases there are workarounds or crates that can close the gap, but users have to learn about and find those crates.
* Common async idioms have "sharp edges" that lead to unexpected failures, forcing users to manage [cancellation safety](https://rust-lang.github.io/wg-async/vision/submitted_stories/status_quo/barbara_gets_burned_by_select.html), subtle [deadlocks](https://rust-lang.github.io/wg-async/vision/submitted_stories/status_quo/aws_engineer/solving_a_deadlock.html) and other failure modes for [buffered streams](https://rust-lang.github.io/wg-async/vision/submitted_stories/status_quo/barbara_battles_buffered_streams.html). See also tmandry's blog post on [Making async Rust reliable](https://tmandry.gitlab.io/blog/posts/making-async-reliable/)).
* Using async today requires users to select a runtime which provides many of the core primitives. Selecting a runtime as a user [can be stressful](https://rust-lang.github.io/wg-async/vision/submitted_stories/status_quo/barbara_makes_their_first_steps_into_async.html#the-wrong-time-for-big-decisions), as the [decision once made is hard to reverse](https://rust-lang.github.io/wg-async/vision/submitted_stories/status_quo/barbara_wishes_for_easy_runtime_switch.html). Moreover, in an attempt to avoid "picking favories", the project has not endorsed a particular runtime, making it [harder to write new user documentation](https://rust-lang.github.io/wg-async/vision/submitted_stories/status_quo/niklaus_wants_to_share_knowledge.html). Libaries meanwhile [cannot easily be made interoperable across runtimes](https://rust-lang.github.io/wg-async/vision/submitted_stories/status_quo/barbara_writes_a_runtime_agnostic_lib.html) and so are often written against the API of a particular runtime; even when libraries can be retargeted, it is difficult to do things like run their test suites to test compatibility. [Mixing and matching libraries can cause surprising failures.](https://rust-lang.github.io/wg-async/vision/submitted_stories/status_quo/alan_started_trusting_the_rust_compiler_but_then_async.html)

[afitblog]: https://blog.rust-lang.org/2023/12/21/async-fn-rpit-in-traits.html
[embassy]: https://github.com/embassy-rs/embassy
[tokio]: https://tokio.rs/

### First focus: language parity, interop traits

Based on the above analysis, the Rust org has been focused on driving async/sync language parity, especially in those areas that block the development of a rich ecosystem. The biggest progress took place in [Dec 2023][afitblog], when async fn in traits and return position impl trait in trait were stabilized. Other work includes documenting async usability challenges in the original async vision doc, stabilizing helpers like [`std::future::poll_fn`](https://doc.rust-lang.org/std/future/fn.poll_fn.html), and polishing and improving async error messages.

### The need for an aligned, high judgment group of async experts

Progress on async-related issues within the Rust org has been slowed due to lack of coherence around a vision and clear steps. General purpose teams such as [Lang][] and [Libs-API][] have a hard time determining how to respond to, e.g., particular async stabilization requests, as they lack a means to judge whether any given decision is really the right step forward. Theoretically, the async working group could play this role, but it has not really been structured with this purpose in mind. For example, the [criteria for membership](https://rust-lang.github.io/wg-async/CHARTER.html#membership-requirements) is loose and the group would benefit from more representation from async ecosystem projects. This is an example of a larger piece of Rust "organizational debt", where the term "working group" has been used for many different purposes over the years.

## The next few steps

In the second half of 2024 we are planning on the following work items. The following three items are what we consider to be the highest priority, as they do the most to lay a foundation for future progress (and they themselves are listed in priority order):

* [resolve the "Send bound" problem](#stabilize-async-closures), which blocks the widespread usage of async functions in traits;
* [reorganize the async WG](#reorganize-the-async-wg), so that we can be better aligned and move more swiftly from here out;
* [stabilize async closures](#stabilize-async-closures), allowing for a much wider variety of async related APIs (async closures are implemented on nightly).

We have also identified two "stretch goals" that we believe could be completed:

* [stabilize trait for async iteration](#stabilize-trait-for-async-iteration)
* [complete async drop experiments](#complete-async-drop-experiments) (currently unfunded)

### Resolve the ["send bound"][sb] problem

Although async functions in traits were stabilized, there is currently no way to write a generic function that requires impls where the returned futures are `Send`. This blocks the use of async function in traits in some core ecosystem crates, such as [tower](https://crates.io/crates/tower), which want to work across all kinds of async executors. This problem is called the ["send bound"][sb] problem and there has been extensive discussion of the various ways to solve it. [RFC #3654][] has been opened proposing one solution and describing why that path is preferred. Our goal for the year is to adopt *some* solution on stable.

[RFC #3654]: https://github.com/rust-lang/rfcs/pull/3654

### Reorganize the Async WG

We plan to reorganize the async working group into a structure that will better serve the projects needs, especially when it comes to [aligning around a clear async vision](#the-need-for-an-aligned-high-judgment-group-of-async-experts). In so doing, we will help "launch" the async working group out from the [launchpad](https://forge.rust-lang.org/governance/council.html#the-launching-pad-top-level-team) umbrella team and into a more permanent structure.

Despite its limitations, the async working group serves several important functions for async Rust that need to continue:

* It provides a forum for discussion around async-related topics, including the `#async-wg` zulip stream as well as regular sync meetings. These forums don't necessarily get participation by the full set of voices that we would like, however.
* It owns async-related repositories, such as the sources for the [async Rust book](https://rust-lang.github.io/async-book/) (in dire need of improvement), [arewewebyet](https://www.arewewebyet.org/), the [futures-rs](https://rust-lang.github.io/futures-rs/) crate. Maintenance of these sites has varied though and often been done by a few individuals acting largely independently.
* It advises the more general teams (typically [Lang][] and [Libs-API][]) on async-related matters. The authoring of the (somewhat dated) [async vision doc](https://rust-lang.github.io/wg-async/vision/) took place under the auspices of the working group, for example. However, the group lacks decision making power and doesn't have a strong incentive to truly "coallesce" behind a shared vision, so it remains more a "set of individual voices" that can still leave the general purpose teams without clear guidance.

We plan to propose one or more permanent teams to meet these same set of needs. The expectation is that these will be subteams under the [Lang] and [Libs] top-level teams.

### Stabilize async closures

Building ergonomic APIs in async is often blocked by the lack of *async closures*. Async combinator-like APIs today typically make use of an ordinary Rust closure that returns a future, such as the `filter` API from [`StreamExt`](https://docs.rs/futures/latest/futures/prelude/stream/trait.StreamExt.html#method.filter):

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

![Stretch Goal](https://img.shields.io/badge/Stretch%20Goal-red)

There has been extensive discussion about the best form of the trait for async iteration (sometimes called `Stream`, sometimes `AsyncIter`, and now being called `AsyncGen`). We believe the design space has been sufficiently explored that it should be possible to author an RFC laying out the options and proposing a specific plan.

### Complete async drop experiments

![Not funded][]

Authors of async code frequently need to call async functions as part of resource cleanup. Because Rust today only supports synchronous destructors, this cleanup must take place using alternative mechanisms, forcing a divergence between sync Rust (which uses destructors to arrange cleanup) and async Rust. [MCP 727][] proposed a series of experiments aimed at supporting async drop in the compiler. We would like to continue and complete those experiments. These experiments are aimed at defining how support for async drop will be implemented in the compiler and some possible ways that we could modify the type system to support it (in particular, one key question is how to prevent types whose drop is async from being dropped in sync code).

## The "shiny future" we are working towards

Our eventual goal is to provide Rust users building on async with

* the same core language capabilities as sync Rust (async traits with dyn dispatch, async closures, async drop, etc);
* reliable and standardized abstractions for async control flow (streams of data, error recovery, concurrent execution), free of accidental complexity;
* an easy "getting started" experience that builds on a rich ecosystem;
* good performance by default, peak performance with tuning;
* the ability to easily adopt custom runtimes when needed for particular environments, language interop, or specific business needs.

# Design axiom

* **Uphold sync Rust's bar for reliability.** Sync Rust famously delivers on the general feeling of "if it compiles, it works" -- async Rust should do the same.
* **We lay the foundations for a thriving ecosystem.** The role of the Rust org is to develop the rudiments that support an interoperable and thriving async crates.io ecosystem.
* **When in doubt, zero-cost is our compass.** Many of Rust's biggest users are choosing it becase they know it can deliver the same performnace (or better) than C. If we adopt abstractions that add overhead, we are compromising that core strength. As we build out our designs, we ensure that they don't introduce an "abstraction tax" for using them.
* **From embedded to GUI to the cloud.** Async Rust covers a wide variety of use cases and we aim to make designs that can span those differing constraints with ease.
* **Consistent, incremental progress.** People are building async Rust systems *today* -- we need to ship incremental improvements while also steering towards the overall outcome we want.

# Ownership and other resources

Here is a detailed list of the work to be done and who is expected to do it. This table includes the work to be done by owners and the work to be done by Rust teams (subject to approval by the team in an RFC/FCP). The overall owners of the async effort (and authors of this goal document) are [tmandry][] and [nikomatsakis][]. We have identified owners for subitems below; these may change over time.

* The ![Funded][] badge indicates that the owner has committed and work will be funded by their employer or other sources.
* The ![Not founded][] badge indictes that there is a willing owner but they need funding to pursue the goal. Depending on the owner's individual circumstances, this could be support/authorizaiton from their employer, grants, or contracting.
* The ![Team][] badge indicates a requirement where Team support is needed.

| Subgoal                                  | Owner(s) or team(s)                     | Status              |
| ---------------------------------------- | --------------------------------------- | ------------------- |
| overall program management               | [tmandry][], [nikomatsakis][]           | ![Funded][]         |
| resolve the ["send bound"][sb] problem   |                                         |                     |
| ↳ ~~RTN implementation~~                 | ~~[compiler-errors][]~~                 | ![Complete][]       |
| ↳ ~~RTN RFC~~                            | [nikomatsakis][]                        | ![Complete][]       |
| ↳ approve RTN RFC or provide alternative | ![Team][] [Lang]                        | (in FCP)            |
| ↳ stabilization                          | [compiler-errors][]                     | ![Funded][]         |
| reorganize the async WG                  |                                         |                     |
| ↳ author proposal                        | [tmandry][], [nikomatsakis][]           | ![Funded][]         |
| ↳ approve changes to team structure      | ![Team][] [Libs], [Lang]                |                     |
| stabilize async closures                 |                                         |                     |
| ↳ ~~implementation~~                     | ~~[compiler-errors][]~~                 | ![Complete][]       |
| ↳ author RFC                             | [nikomatsakis][] or [compiler-errors][] | ![Funded][]         |
| ↳ approve RFC                            | ![Team][] [Lang]                        |                     |
| ↳ stabilization                          | [compiler-errors][]                     | ![Funded][]         |
| stabilize trait for async iteration      |                                         |                     |
| ↳ author RFC                             | [eholk][]                               | ![Funded][]         |
| ↳ approve RFC                            | ![Team][] [Libs-API]                    | ![Funded][]         |
| ↳ implementation                         | [eholk][]                               | ![Funded][]         |
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

Agreement from [Lang][], [Libs][] [Libs-API][] to prioritize the items marked ![Team][] in the table above.

The expectation is that

* async closures will occupy 2 design meetings from lang during H2
* async iteration will occupy 2 design meetings from lang during H2 and likely 1-2 from libs API
* misc matters will occupy 1 design meeting from lang during H2

for a total of 4-5 meetings from lang and 1-2 from libs API.

# Frequently asked questions

## Can we really do all of this in 6 months?

This is an ambitious agenda, no doubt. We believe it is possible if the teams are behind us, but things always take longer than you think. We have made sure to document the "priority order" of items for this reason. We intend to focus our attention first and foremost on the high priority items.

## Why focus on send bounds + async closures?

These are the two features that together block the authoring of traits for a number of common interop purposes. Send bounds are needed for generic traits like the `Service` trait. Async closures are needed for rich combinator APIs like iterators.

## Why not work on dyn dispatch for async fn in traits?

Async fn in traits do not currently support native dynamic dispatch. We have explored a [number of designs for making it work](https://smallcultfollowing.com/babysteps/blog/2021/09/30/dyn-async-traits-part-1/) but are not currently prioritizing that effort. It was determined that this idea is lower priority because it is possible to [workaround](https://smallcultfollowing.com/babysteps/blog/2021/10/15/dyn-async-traits-part-6/) the gap by having the  `#[trait_variant]` crate produce a dynamic dispatch wrapper type (e.g., `#[trait_variant(dyn = DynWidget)] trait Widget` would create a type `DynWidget<'_>` that acts like a `Box<dyn Widget>`). We do expect to support dyn async trait, hopefully in 2025.

## Why are we moving forward on a trait for async iteration?

There has been extensive discussion about the best design for the "Stream" or "async iter" trait and we judge that the design space is well understood. We would like to unblock generator syntax in 2025 which will require some form of trait.

The majority of the debate about the trait has been on the topic of whether to base the trait on a `poll_next` function, as we do today, or to try and make the trait use `async fn next`, making it more anaologous with the `Iterator` trait (and potentially even making it be two versions of a single trait). We will definitely explore forwards compatibility questions as part of this discussion. nikomatsakis for example still wants to explore maybe-async-like designs, especially for combinator APIs like `map`. However, we also refer to the design axiom that "when in doubt, zero-cost is our compass" -- we believe we should be able to stabilize a trait that does the low-level details right, and then design higher level APIs atop that.

## Why do you say that we lack a vision, don't we have an [async vision doc][avd]?

Yes, we do, and the [existing document][avd] has been very helpful in understanding the space. Moreover, that document was never RFC'd and we have found that it lacks a certain measure of "authority" as a result. We would like to drive stronger alignment on the path forward so that we can focus more on execution. But doing that is blocked on having a more effective async working group structure (hence the goal to [reorganize the async WG](#reorganize-the-async-wg)).

## What about "maybe async", effect systems, and keyword generics?

Keyword generics is an ambitious initiative to enable code that is "maybe async". It has generated significant controversy, with some people feeling it is necessary for Rust to scale and others judging it to be overly complex. One of the reasons to [reorganize the async WG](#reorganize-the-async-wg) is to help us come to a consensus around this point (though this topic is broader than async).

[tmandry]: https://github.com/tmandry
[nikomatsakis]: https://github.com/nikomatsakis
[compiler-errors]: https://github.com/compiler-errors
[eholk]: https://github.com/eholk
[petrochenkov]: https://github.com/petrochenkov
[Team]: https://img.shields.io/badge/Team%20ask-red
[MCP 727]: https://github.com/rust-lang/compiler-team/issues/727
[Lang]: https://www.rust-lang.org/governance/teams/lang
[Libs]: https://www.rust-lang.org/governance/teams/library
[Libs-API]: https://www.rust-lang.org/governance/teams/library#team-libs-api
[sb]: https://smallcultfollowing.com/babysteps/blog/2023/02/01/async-trait-send-bounds-part-1-intro/
