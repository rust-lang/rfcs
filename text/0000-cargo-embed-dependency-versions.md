- Feature Name: `cargo_embed_dependency_versions`
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Embed information equivalent to the contents of Cargo.lock into compiled binaries so it could be programmatically recovered later.

# Motivation
[motivation]: #motivation

Rust is very promising for security-critical applications due to its safety guarantees, but there currently are gaps in the ecosystem that prevent it. One of them is the lack of any infrastructure for security updates.

Linux distributions alert you if you're running a vulnerable software version and you can opt in to automatic security updates. Cargo not only has no automatic update infrastructure, it doesn't even know which libraries or library versions went into compiling a certain binary, so there's no way to check if your system is vulnerable or not.



The primary motivation is cross-referencing versions of the dependencies against [RustSec advisory database](https://github.com/RustSec/advisory-db). This also enables use cases such as making a fix in a library crate and then ensuring it's been rolled out to your entire fleet, or preventing binaries with unvetted dependencies from reaching production.

Why are we doing this? What use cases does it support? What is the expected outcome?

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Explain the proposal as if it was already included in the language and you were teaching it to another Rust programmer. That generally means:

- Introducing new named concepts.
- Explaining the feature largely in terms of examples.
- Explaining how Rust programmers should *think* about the feature, and how it should impact the way they use Rust. It should explain the impact as concretely as possible.
- If applicable, provide sample error messages, deprecation warnings, or migration guidance.
- If applicable, describe the differences between teaching this to existing Rust programmers and new Rust programmers.

For implementation-oriented RFCs (e.g. for compiler internals), this section should focus on how compiler contributors should think about the change, and give examples of its concrete impact. For policy RFCs, this section should provide an example-driven introduction to the policy, and explain its impact in concrete terms.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This is the technical portion of the RFC. Explain the design in sufficient detail that:

- Its interaction with other features is clear.
- It is reasonably clear how the feature would be implemented.
- Corner cases are dissected by example.

The section should return to the examples given in the previous section, and explain more fully how the detailed proposal makes those examples work.

# Drawbacks
[drawbacks]: #drawbacks

- Adds more platform-specific code to the build process which needs to be maintained.
- Slightly increases the size of the generated binaries. However, the increase is below 1%. A "Hello World" on x86 Linux compiles into a ~1Mb file in the best case (recent Rust without jemalloc, LTO enabled). Its Cargo.lock even with a couple of dependencies is less than 1Kb, that's under 1/1000 of the size of the binary. Since Cargo.lock grows linearly with the number of dependencies, it will keep being negligible.

Why should we *not* do this?

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Rationale:

- Version information is impossible to misplace. As long as you have the binary, you can recover the info about dependency versions. The importance of this cannot be overstated. This allows auditing e.g. a Docker container that you did not build yourself, or a server that somebody's built a year ago and left no audit trail.
- A malicious actor could lie about the version information. However, doing so requires modifying the binary - and if a malicious actor can do _that,_ you are already pwned. So this does not create any additional attack vectors - other than exploiting the tool that's recovering the version information, which can be easily sandboxed.
- Any software supply chain verification that might be deployed automatically applies to the version information. There is no need to separately authenticate it.
- This enables third parties such as cloud providers to scan your binaries for you. Google Cloud [already provides such a service](https://cloud.google.com/container-registry/docs/get-image-vulnerabilities), Amazon has [an open-source project you can deploy](https://aws.amazon.com/blogs/publicsector/detect-vulnerabilities-in-the-docker-images-in-your-applications/) while Azure [integrates several partner solutions](https://docs.microsoft.com/en-us/azure/security-center/security-center-vulnerability-assessment-recommendations).

Alternatives:

- Do nothing. Identifying vulnerable binaries will remain impossible.
- Track version information separately from the binaries, recording it when running `cargo install` and surfacing it through some other Cargo subcommand. When installing not though `cargo install`, rely on Linux package managers to track version information. Identifying vulnerable binaries will remain impossible on all other platforms, as well as on Linux for code compiled with `cargo build`. Verification by third parties remains impossible.

- Why is this design the best in the space of possible designs?
- What other designs have been considered and what is the rationale for not choosing them?
- What is the impact of not doing this?

# Prior art
[prior-art]: #prior-art

Discuss prior art, both the good and the bad, in relation to this proposal.
A few examples of what this can include are:

- For language, library, cargo, tools, and compiler proposals: Does this feature exist in other programming languages and what experience have their community had?
- For community proposals: Is this done by some other community and what were their experiences with it?
- For other teams: What lessons can we learn from what other communities have done here?
- Papers: Are there any published papers or great posts that discuss this? If you have some relevant papers to refer to, this can serve as a more detailed theoretical background.

This section is intended to encourage you as an author to think about the lessons from other languages, provide readers of your RFC with a fuller picture.
If there is no prior art, that is fine - your ideas are interesting to us whether they are brand new or if it is an adaptation from other languages.

Note that while precedent set by other languages is some motivation, it does not on its own motivate an RFC.
Please also take into consideration that rust sometimes intentionally diverges from common language features.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- The format of Cargo.lock is not stabilized and is evolving. Should we encode Cargo.lock as-is and require tooling to track the updates, or commit to a stable subset of Cargo.lock?
- Should this also apply to shared libraries?
- Should this information be removed when stripping the binary of debug symbols?
- Are there any cases where you would _not_ want to allow whoever is running the binary to check it for vulnerabilities? 

Out of scope for now:

- how to track and communicate versions of statically linked C libraries, such as OpenSSL?

# Future possibilities
[future-possibilities]: #future-possibilities

- Surface dependency information through an HTTP endpoint in a microservice environment. The [proof-of-concept](https://github.com/Shnatsel/rust-audit/issues/2) has a feature request for it. However, this does not require support from Cargo and can be implemented as a crate.
- Record and surface versions of C libraries statically linked into the Rust executable, e.g. OpenSSL. 

Think about what the natural extension and evolution of your proposal would
be and how it would affect the language and project as a whole in a holistic
way. Try to use this section as a tool to more fully consider all possible
interactions with the project and language in your proposal.
Also consider how the this all fits into the roadmap for the project
and of the relevant sub-team.

This is also a good place to "dump ideas", if they are out of scope for the
RFC you are writing but otherwise related.

If you have tried and cannot think of any future possibilities,
you may simply state that you cannot think of anything.

Note that having something written down in the future-possibilities section
is not a reason to accept the current or a future RFC; such notes should be
in the section on motivation or rationale in this or subsequent RFCs.
The section merely provides additional information.
