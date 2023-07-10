- Feature Name: `rust_spec`
- Start Date: 2022-12-06
- RFC PR: [rust-lang/rfcs#3355](https://github.com/rust-lang/rfcs/pull/3355)
- Rust Issue: [rust-lang/rust#113527](https://github.com/rust-lang/rust/issues/113527)

# Summary
[summary]: #summary

We should start working on a Rust specification.

# Goal of this RFC

The goal of this RFC is to reach consensus on:

- Whether we want a specification, and (if so),
- Some initial goals and non-goals, and
- How we want the work to be organised and coordinated.

This RFC _does not_ define the full scope of the specification
or discuss any details of how it would look like.
It only provides the minimal details necessary to be able to kick off the Rust specification work.

# Motivation

Why do we want a Rust specification at all?

There are many different kind of Rust users that would benefit from a Rust specification in their own way.
Things like the Rust Reference, the Unsafe Code Guidelines Project, the Rustonomicon, and so on,
all exist to fulfill certain needs of Rust users.
Unfortunately, their use is currently limited, because none of these are complete, entirely accurate, or normative.

Authors of unsafe code could benefit a lot from clear definitions of what is and isn't undefined behaviour.
Safety critical Rust software won't pass certification without a specification that clearly specifies how Rust code behaves.
Proposals and discussions about new Rust language features could be more efficient and precise
using accurately defined terms that everyone agrees on.
Questions about subtle interactions between features of the language could be answered
using precise information from a specification, instead a combination of guesses and several non-authoritative sources.

# Current state

Languages like C and C++ are standardized.
Rust is not. Standardization comes down to, basically:

1. Having an accurate specification (a document)
2. An (open) process for evolution of the language
3. Stability

Rust currently already has 2 and 3, but not 1.

For 1, we currently have:
the (incomplete) [Rust Reference](https://doc.rust-lang.org/stable/reference/),
the [Standard Library Reference Documentation](https://doc.rust-lang.org/stable/std/),
the [Rust Nomicon](https://doc.rust-lang.org/nightly/nomicon/),
the [Unsafe Code Guidelines Project](https://github.com/rust-lang/unsafe-code-guidelines/),
[Miri](https://github.com/rust-lang/miri/),
the collection of [accepted RFCs](https://rust-lang.github.io/rfcs/),
the [Ferrocene Language Specification](https://spec.ferrocene.dev/),
lots context and decisions spread over [tons of GitHub issues](https://github.com/rust-lang/rust/issues/),
[MiniRust](https://github.com/RalfJung/minirust),
the [source code](https://github.com/rust-lang/rust/),
and more.

These are currently all incomplete, and/or not a good source to rely on.

More background information is available in [this blog post](https://blog.m-ou.se/rust-standard/).

# Goals and non-goals

- The goal of the Rust specification work is the creation of a document, the Rust specification.

- The goal is _not_ to change how the language evolves;
  the relevant teams (Language, Libs-API, …) remain in charge of the evolution of their respective parts of Rust,
  and will continue to use processes as they see fit (e.g. RFCs).

- The specification will only be considered "official" once the relevant teams have approved of its contents.
  Changes to the official specification must be approved by the relevant team(s).

- The goal is to serve the needs of Rust users, such as
  authors of unsafe Rust code, those working on safety critical Rust software,
  language designers, maintainers of Rust tooling, and so on.

- It is _not_ a primary goal of the specification to aid in the development of alternative Rust implementations,
  although authors of alternative compilers might still find the specification to be useful.

  What this means is that, unlike the C or C++ standard,
  the Rust specification does not provide a set of requirements for a compiler to be able to call itself "a Rust™ compiler".
  Instead, it specifies the behaviour of the Rust compiler.
  (So, not "A Rust implementation should …", but instead "Rust will …".)

- The scope remains to be determined, but at least includes all topics currently included in 
  the (incomplete) [Rust Reference](https://doc.rust-lang.org/stable/reference/).

- The Rust specification is expected to replace the current Rust Reference.

- The scope of the specification can grow over time, depending on the needs of Rust users and
  time and motivation of those working on it.

  For example, it might grow over time to also specify details of Cargo,
  compiler flags, procedural macros, or other parts that we might initially consider out of scope.

- The specification is specific to the latest version of Rust.
  A copy of the specification is included with each Rust release,
  as we currently already do with much of our documentation.

  While the specification might include notes about the Rust version that a feature was introduced in
  for informative purposes (similar to standard library documentation),
  it does not attempt to accurately specify older, unsupported versions of Rust.

- The specification specifies all Rust _editions_, as supported by the latest version of the Rust compiler.

- Once the specification reaches an initial usable version,
  the relevant teams are expected to incorporate it in their process for language evolution.
  For example, the language team could require a new language feature to be included
  in the specification as a requirement for stabilization.

- The specification will be written in English and will be freely available under a permissive license
  that allows for translations and other derived works, just like all our existing documentation and code.

# Coordination and editing

Writing, editing, and in general coordinating all that's necessary for the creation of a Rust specification is a large amount for work.
While there are many volunteers willing to work on specific parts of it,
it's unlikely we'd end up with a complete, consistent, properly maintained specification if we rely entirely on volunteers.

So this RFC proposes that we ask the Rust Foundation to coordinate and take responsibility
for the parts of the work that would otherwise not get done.
The foundation should hire a technical editor
who will work with the Rust teams and contributors to create the Rust specification.
The editor will be responsible for maintaining the document and will coordinate with the relevant teams
(e.g. the language team, the operational semantics team, the compiler, the types team, the library API team, and so on)
to collect all relevant information and make sure that consensus is reached on everything that will end up in the official specification.

The relevant Rust teams keep authority on their respective parts of Rust.
The Rust Foundation supports and coordinates the work, but the Rust teams will remain in charge of what Rust is.

## Role of the Editor

The role of the editor is more than just a technical writer; the editor will be a leader in the specification development process.

The tasks of the editor (as [suggested by Joel](https://github.com/rust-lang/rfcs/pull/3355#issuecomment-1481813621)):

1. *Active coordination and management of the specification process*.
  Working with project members, an editor dedicated to the specification will
  work to ensure that there is continuous progress on the specification itself,
  through activities like coordinating meetings, suggesting relevant topics of
  discussion, managing the infrastructure around the creation of the
  specification.

2. *Collecting and aggregating information from spec-relevant Project teams*.
  Related to the coordination and management of the process, the editor will have
  an ear in all the relevant Project teams that have members working on the
  specification in order to understand their thoughts, ideas and requirements.
  The editor will aggregate this information to use during the specification
  process. The editor will work closely with Project teams such as the Language
  team, the Operational Semantics team, etc. to ensure, for example,
  specification proposals can be officially approved for inclusion into the
  specification. To be clear the editor is not necessarily a member of any
  particular team, but will work with those teams to ensure they are represented
  well and fairly in the specification.

3. *Technical writing*.
  The editor actually has to incorporate the concepts and write the words that
  will ultimately make up the specification. The reason that this is not
  necessarily the top priority is that without the coordination and information
  gathering, this cannot be done in any meaningful way. But, obviously, this is
  where the rubber meets the road and where the final output will be made. The
  editor, in conjunction with any potential required ancillary design or
  copyediting resources, will produce a developer and community friendly Rust
  language specification.

4. *Reporting progress*.
  Since not everyone in the Project will be involved in the specification process
  on a daily basis and with the expected interest within the Rust community, the
  editor will provide regular status updates on the progress of the
  specification. The vehicle by which this will be done is to be determined, but
  you can imagine public blog posts, a dedicated Zulip stream, etc.

5. *Propose technical clarifications and corrections to the specification*.
  As we work on the specification, there is a reasonable probability that we may
  find areas that are unclear, confusing and maybe even contradictory. While not
  a hard requirement and more of a nice-to-have, optimally the editor will be
  well-versed in programming languages and can offer potential clarifications and
  corrections for technical correctness and consistency purposes.

# Questions deliberately left open

This RFC deliberately leaves many questions open, to be answered later in the process.
For example:

- The starting point of the Rust specification.

  The Rust specification could take the [Ferrocene Specification](https://spec.ferrocene.dev/) or
  the [Rust Reference](https://doc.rust-lang.org/stable/reference/) as starting point,
  or start from scratch, as the editor sees fit.
  (The contents will still require approval from the Rust teams, regardless.)

- The shape, form, and structure of the document.

- The scope of the Rust specification.

  It should include at least all topics covered in
  [the Rust Reference](https://doc.rust-lang.org/stable/reference/),
  but the scope can grow depending on ongoing efforts in the Rust team and the needs of the Rust community.

- How certain topics will be specified.

  Certain parts of the specification might use a formal language
  for specifying behavior or syntax.
  For example, the grammar might be specified as EBNF,
  and parts of the borrow checker or memory model might be specified by
  a more formal definition that the document refers to.

- The depth of the specification for various topics.

  For example, it might specify only the existence of `#[allow(…)]` (etc.) without naming any lints,
  or at the other extreme it might fully specify the behaviour of every single lint.
  As another example, it could only specify the overall guarantees of the borrow checker
  (e.g. "it won't allow UB in safe code"), or it could precisely specify what currently is and isn't accepted by the borrow checker.
  The right level of detail for each topic should be discussed and agreed upon by the involved parties,
  and can change over time.

- Naming.

  The exact title of the document might carry significance depending on how it will be used.
  Before we officially publish a non-draft version of the specification, we
  should come to an agreement on whether to call it "The Rust Specification" or something else.
