- Feature Name: `project-unwind-FFI`
- Start Date: 2019-10-27
- RFC PR: [rust-lang/rfcs#2797](https://github.com/rust-lang/rfcs/pull/2797)
- Rust Issue: N/A

# Summary
[summary]: #summary

* To create a "project group" with the purpose of designing subsequent RFCs to
  extend the language to support unwinding that crosses FFI boundaries
  * The "project group" term is newly introduced: it is a specific type of
    working group whose goal is to flesh out a particular proposal or complete
    a project.
  * This project group plans to recommend specifications of how "C unwind" will work on major
    platforms.
  * The primary goal is to enable Rust panics to propagate safely across
    foreign frames.
    * A future goal may be to enable foreign exceptions to propagate across Rust
      frames.
    * We do not plan to allow catching or throwing foreign exceptions from Rust
      code

# Motivation
[motivation]: #motivation

Unwinding through Rust's `extern "C"` ABI is [Undefined Behavior]. There is an
[existing plan][abort-unwind] to make the behavior of Rust's `panic`
well-defined by causing Rust functions defined with `extern "C"` to abort the
application whenever an uncaught `panic` would otherwise escape into the
caller. Unfortunately, previous attempts to stabilize this behavior have caused
existing, working projects to break.

The problem here is not that the existing projects break *per se*: they are
relying on [Undefined Behavior], so breakage is to be expected as a
possibility. The problem is that there is no alternative available to them that
would allow them to keep working (even if they are continuing to rely on
behavior that is not yet fully specified).

Previous attempts to provide a well-defined mechanism for unwinding across FFI
boundaries have failed to reach consensus. Notably, two proposed RFCs generated
over 400 comments between them before ultimately being closed:

* [rust-lang/rfcs#2699](https://github.com/rust-lang/rfcs/pull/2699)
* [rust-lang/rfcs#2753](https://github.com/rust-lang/rfcs/pull/2753)

GitHub comment threads become difficult to follow for discussions this lengthy,
and the disagreements in these threads have felt less productive than we
believe they could be if more structure were provided.

We would also like to demonstrate the Rust lang team's commitment to providing
such a mechanism without needing to agree in advance on what language changes
will be stabilized in order to do so.

# Prototyping 'shepherded' project groups
[prototyping-project-groups]: #prototyping-shepherded-project-groups

With this RFC, we formally announce the formation of a project-specific,
shepherded "project group" to adopt responsibility for driving progress on
specifying unwinding behavior at FFI boundaries.

## What is a "project group"?

The "project group" term has not previously been used: it is intended to
formalize a concept that has existed informally for some time, under a number
of names (including "working group").

A "project group" is a group of people working on a particular project at the
behest of an official Rust team. Project groups must have:

* A **charter** defining the project's scope
* A **liaison** with an official Rust team (who may or may not also be a shepherd)
* A small number of **shepherds**, who are responsible for summarizing
  conversations and keeping the lang team abreast of interesting developments.
* A GitHub repository hosted under the `rust-lang` organization containing the
  charter and instructions for how community members can monitor the group's
  progress and/or participate.

[This blog post][shepherds-3.0] explains in detail the role of the
shepherds.

## Project group roadmap and RFCs

The first step of the project group is to define a **roadmap** indicating the
planned sequence in which it will design and propose particular behaviors and
features.  Once the project group feels it has completed work on some item in
the roadmap, that item will be submitted as an RFC or FCP for review by the lang team and the community at large.

## Stabilizing unspecified "TBD" behavior
[stabilizing-tbd]: stabilizing-unspecified-tbd-behavior

We would like to be able to provide features in stable Rust where some
of the details are only partially specified. For example, we might add
a new ABI "C unwind" that can be used from stable Rust, while
explicitly leaving the behavior when a foreign exception unwinds
across such a boundary unspecified. In such cases, we would attempt to
provide some bounds on what might happen -- for example, we might
state that a Rust panic propagating across a "C unwind" boundary must
be preserved and handled as normal.

In some cases, we intend to mark some of this unspecified behavior as
"To Be Determined" (TBD). This classification is meant to convey that
the behavior is behavior we intend to specify as part of this group,
although we have not done so *yet*. This categorization is purely
intental to the working group, however; such behavior would remain
formally unspecified until an RFC or other binding decision is
reached.

## Details of the FFI-unwind project group

[Repository][ffi-unwind project]

Initial shepherds:

* [acfoltzer (Adam)](https://github.com/acfoltzer)
* [batmanaod (Kyle)](https://github.com/batmanaod)

Lang team liaisons:

* [nikmoatsakis (Niko)](https://github.com/nikmoatsakis)
* [joshtriplett (Josh)](https://github.com/joshtriplett)

### Charter
[charter]: #charter

The FFI-unwind project group has the following initial scope:

* to define the details of the "C unwind" ABI on major Tier 1 platforms
* in particular, to define with sufficient detail to enable the use cases
  described in the Motivation section of this RFC
  
Certain elements are considered out of scope, at least to start:

* We do not expect to add new mechanisms for interacting with or
  throwing foreign exceptions.
    * However, if we specify what happens when a foreign exception
      passes into Rust code, then we must also specify how that
      exception will interact with pre-existing mechanisms like
      destructors and `catch_unwind`. We just don't intend to create
      new mechanisms.


### Constraints and considerations

In its work, the project-group should consider various constraints and
considerations:

* The possibility that C++ may adopt new unwinding mechanisms in the future.
* The possibility that Rust may alter its unwinding mechanism in the future --
  in particular, the project group must not propose a design that would
  constrain Rust's unwinding implementation on any target.

### Participation in the project group

Like any Rust group, the FFI-unwind project group intends to operate
in a public and open fashion and welcomes participation. Visit the
[repository][ffi-unwind project] for more details.

# Drawbacks
[drawbacks]: #drawbacks

* The adoption of project groups for major language design efforts is a change
  in the status quo. We believe that this change will be an improvement over
  the current RFC-centric process, but we should be wary of unintended
  consequences of from such a change.
* [Stabilization of "TBD" features][stabilizing-tbd] may be surprising or
  confusing to users, and it will encourage reliance on (some) unspecified
  behavior.

# Prior art
[prior-art]: #prior-art

Although the term "project group" is new, some existing efforts, such as the
Unsafe Code Guidelines effort and the work around defining const evaluation,
were organized in a similar fashion.

In addition to the [blog post Niko Matsakis][shepherds-3.0] about
shepherding, James Munns wrote a [more formal shepherding
proposal][shepherding-3.1].

The [governance WG][governance-wg] and [lang-team meta working
group][lang-meta-wg] were both formed at least in part to improve the process
for large-scale design efforts. One existing proposal is for ["staged
RFCs"][staged-rfc]; this may be considered a precursor to the current
"shepherded project group" proposal.


# Unresolved questions and Future possibilities
[unresolved-questions]: #unresolved-questions

Since this RFC merely formalizes the creation of the project group, it
intentionally leaves all technical details within the project's scope
unresolved.

# Future possibilities
[future-possibilities]: #future-possibilities

The project group will start with a fairly [limited scope][charter], but if the
initial effort to design and stabilize a safe cross-language unwinding feature
on a limited set of platforms goes well, there are many related areas of
potential exploration. Three noteworthy examples are:

* Catching foreign unwinding (e.g. Rust catching C++ exceptions, or C++
  catching Rust `panic`s)
* Defining coercions among `fn`s using ABIs with different `unwind`
  behavior
* Monitoring progress, or even participating in discussion about, the [ISO C and
  C++ proposal][c-cpp-unified-proposal] for cross-language error handling

[Undefined Behavior]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
[abort-unwind]: https://github.com/rust-lang/rust/issues/52652
[ffi-unwind project]: https://github.com/rust-lang/project-ffi-unwind
[shepherds-3.0]: http://smallcultfollowing.com/babysteps/blog/2019/09/11/aic-shepherds-3-0/
[c-cpp-unified-proposal]: http://open-std.org/JTC1/SC22/WG21/docs/papers/2018/p1095r0.pdf
[shepherding-3.1]: https://jamesmunns.com/blog/shepherding-3-1/
[governance-wg]: https://github.com/rust-lang/wg-governance
[lang-meta-wg]: https://github.com/rust-lang/lang-team/tree/master/working-groups/meta
[staged-rfc]: http://smallcultfollowing.com/babysteps/blog/2018/06/20/proposal-for-a-staged-rfc-process/
