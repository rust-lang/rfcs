- Feature Name: Extended Standard Library
- Start Date: 2025-05-08
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC describes an Extended Standard Library (ESL) for Rust. The ESL will consist of a set of crates providing commonly used functionality, subject to policies which promote excellent security, quality, and ease of use.

# Motivation
[motivation]: #motivation

Rust programs, on average, have many dependencies. This is largely because the Rust standard library is intentionally limited in scope. As a consequence, Rust programs are currently required to pull in many third-party dependencies to get functionality that is considered table-stakes by 2025 standards.

There are three issues this proposal aims to help address, all of which are related to the large number of independent entities providing crates that are widely depended on in Rust software:

### 1. Impractically-sized web of trust

Reliance on a high number of third-party dependencies creates a large web of trust for any non-trivial program. Given the size of this web of trust, it is practically impossible to be confident about the trustworthiness of the full dependency tree.

Being a _trustworthy_ dependency means it:

* Is developed and maintained by qualified, non-malicious authors
* Has a responsible vulnerability disclosure policy
* Has sufficient maintainer bandwidth to publish security updates in a reasonable timeframe
* Has reasonable code review and testing standards
* Uses secure authentication and release infrastructure
* Depends only on libraries that are themselves trustworthy
* Can be expected to maintain these properties well into the future

Establishing confidence in the above characteristics for any individual dependency requires significant effort; establishing confidence in these for all of a project’s dependencies is impractical. As a result, many projects currently rely on fuzzy heuristics to determine the trustworthiness of a dependency, or even choose to ignore the problem altogether and "hope for the best."

The status quo goes against Rust's goal of empowering everyone to build reliable software. Though it was never intended to be that way, as of today the Rust ecosystem nudges developers towards pulling in more dependencies than they can reasonably vet, which in the end hurts the reliability of the software.

### 2. Subpar developer experience

When figuring out which crate to use for relatively basic functionality, there are often many options to choose from with no clear indication of which one should be preferred and why. This makes the whole process difficult, time consuming, and generally unpleasant. The situation detracts significantly from the Rust developer experience.

### 3. Maintainer support

Supporting maintainers of the software we use is a good thing to do. It can also improve the quality and reliability of the software by allowing for additional investment in the software and the processes around it, reducing burnout, and minimizing the pressure for software to change hands to potentially less trustworthy maintainers.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Bringing Rust closer to its goal, empowering everyone to build reliable and efficient software, means developers need:

1. More confidence in the trustworthiness of the basic building blocks they rely on.
2. An easier time finding the right building blocks.

To achieve this, the Rust ecosystem needs a set of crates that includes the most commonly used functionality in Rust programs, referred to here as the ESL. It needs to be maintained by a trustworthy organization with a strong commitment to quality, security and support. As this set of crates matures, it should become possible to compose most Rust programs with a minimal number of dependencies outside of the ESL and the Rust standard library.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The ESL will consist of a set of crates providing commonly used functionality.

## Policies

The ESL will require a number of policies, the most essential of which will ensure timely security fixes, strike a thoughtful balance between API stability and evolution, provide long-term support for older compilers through a careful MSRV choice while avoiding stagnation, and maintain a high standard for quality. The ESL does not need to make the same policy choices as the Rust Standard Library.

A simplified list of example policies to use as potential starting point can be found in [Appendix A](#appendix-a). Policies may need to cover additional topics beyond the examples provided.

Policies should evolve as the ESL community learns about what works best.

## External Dependencies

In an ideal world the ESL would have no production (non-dev) dependencies outside of the ESL and the Rust standard library. However, this is likely going to be impractical because an existing set of widely used crates with dependencies on each other already exists. If one or more of these crates does not want to join the ESL, exceptions may need to be made allowing them to be external dependencies in order to avoid expending a huge amount of effort to create and maintain duplicates of their functionality for the ESL. The ESL community should work to minimize the number of external dependencies for the sake of offering the most consistent, secure, and reliable experience possible, but some will almost certainly be necessary.

## Selecting Functionality for Inclusion

The primary criteria for selecting which functionality to include are:

* Functionality that is widely used by the Rust ecosystem today, a pervasive dependency across many projects
* Not duplicative of something that already exists in the Rust standard library or the ESL

The following is an incomplete list of functionality that would likely be included based on these criteria, offered for the sake of roughly illustrating the intended scope of functionality:

### Tranche 1 - Base functionality, leaf nodes in many dependency trees
* ‘syn’ equivalent
* ‘bitflags’ equivalent
* ‘quote’ equivalent
* Platform bindings (libc)
* Encoding support (base64, hex, etc)
* ‘cfg-if’ equivalent
* Random number generation
* Serialization/deserialization
* Data structure serialization
* Date/time support
* Error handling support

### Tranche 2: Next step up in complexity/abstraction
* Regular expressions
* Compression (e.g. zlib, zstd)
* Command line argument parsing
* Tracing

### Tranche 3 - Common higher level application building blocks
* Default asynchronous runtime
* Cryptography
* Zeroize memory
* TLS
* HTTP

Ultimately the offered functionality would be selected by the ESL community.

The ESL can grow at a pace that is comfortable given available resources. This means that it might take a year or two to advance to the next tranche of functionality.

It should be OK to remove previously included functionality if doing so makes sense. This is part of evolving APIs, and should be regarded as potentially healthy as opposed to necessarily reflective of a failure.

## Adding Functionality

Once a decision is made to try to include functionality in the ESL, the ESL community should try to find a suitable existing crate with maintainers who are willing to join the ESL.

If a maintainer chooses to move their crate into the ESL, doing so does not require them to relinquish their role as a maintainer. They do need to commit to upholding the policies that the ESL community has decided on. In order to help with that and other maintenance tasks, maintainers will receive various forms of support as part of ESL membership (see below).

If the ESL community would like to add functionality but is unable to find a suitable crate with maintainers willing to join, the community will have to consider their options, which include creating a new crate or not adding the functionality. If the functionality is required in order to enable the addition of higher-order functionality, an exception could be made to allow for a dependency outside of the ESL.

## Maintainer Support

The ESL should endeavor to provide the following forms of support to maintainers:

* **Infrastructure Assistance**. Testing, security, release, and benchmarking infrastructure managed by an ESL infrastructure partner.
* **Funding**. The opportunity to receive grants to help cover the cost of their contributions.
* **Security Release Coverage**. If maintainers are unavailable to respond to a security issue, there is a team that can back them up.
* **Legal Assistance**. Legal assistance as necessary from an ESL legal partner.

## Crate Names

All crates in the ESL would ideally have names that:

1) Clearly describe the functionality they offer
2) Contain a reliable indicator of ESL membership

This would help Rust developers to identify functionality, understand where their dependencies are coming from, and make their Rust code easier to read.

However, the amount of effort involved in renaming existing popular crates and a lack of namespacing ([Rust RFC 3243](https://rust-lang.github.io/rfcs/3243-packages-as-optional-namespaces.html) remains unimplemented) makes this ideal difficult to achieve.

As such, pre-existing crates that join the ESL will keep their names unless there is a compelling reason to make a change.

If a crate is created or renamed for the ESL, a name clearly describing its functionality will be chosen.

Crate users will have to confirm ESL membership by checking the GitHub organization.

## Process for Creating the ESL

1. Set Up ESL Governance With Initial Crate Maintainers

A successful ESL depends on maintainers of existing crates wanting to move their crates into the ESL. This may not be the case for every piece of functionality, but it will need to be the case for much of it.

As such, the first step should be for the Rust Libraries team to find at least three maintainers of existing very commonly used crates providing "Tranche 1" functionality who are willing to be the first to join. It is recommended, if possible, that these initial crates only depend on the Rust standard library and/or each other, i.e., they represent some of the lowest level of functionality (above the Rust standard library) to start with.

One maintainer from each of the initial ESL crates, selected by the Rust Libraries team, will form an initial sub-team of the [Rust Libraries team](https://www.rust-lang.org/governance/teams/library), which will govern the ESL. Membership on this team may change over time, but at least 75% of its membership should be maintainers of ESL crates. The remaining membership may consist of ESL contractors/staff, or other individuals with relevant skills.

2. Create an ESL GitHub Organization

A GitHub organization should be created for the ESL. It should have its own GitHub organization to make it clear to Rust developers what is part of the ESL.

The ESL GitHub organization should host ESL crates as well as policy, governance, and ESL-specific testing and infrastructure repositories.

The ESL GitHub organization will be administered by the Rust Libraries ESL sub-team.

3. Create Initial Policies

The ESL community will decide on an initial set of policies, possibly similar in scope to the examples in the RFC.

ESL policies will be published in a policy repository in the ESL GitHub organization.

The ESL GitHub organization should be configured and administered by the Rust Libraries ESL sub-team in line with published ESL policies.

4. Create an Administrative and Infrastructure Support Relationship

The ESL community will engage the Rust Foundation to assist with administering finances, hiring any necessary contractors and/or staff, and operating supporting infrastructure. This may involve becoming an official Rust Foundation project.

Regardless of the details of the relationship to the Rust Foundation, the Rust Foundation will play a supporting role, much like it does for the Rust language, rather than making decisions for the ESL and its maintainers.

5. Secure Initial Funding Commitments

Work on the ESL will require resources to set up the necessary structure and assist maintainers with meeting ESL requirements and general maintenance. Additional resources should also be made available to the Rust Libraries team as necessary to assist with performance of their role.

The [Rust Foundation](https://rustfoundation.org/) and [ISRG](https://www.abetterinternet.org/) are willing to assist with raising the necessary funds.

6. Create the ESL

At this point the initial ESL crates will move into the ESL GitHub organization.

# Drawbacks
[drawbacks]: #drawbacks

1) This will require making some difficult decisions.

This project will require being able to make difficult decisions, such as selecting functionality to include and deciding on policies. ESL community and leadership should recognize this and work in such a way that difficult decisions can be made when necessary.

2) Managing community and maintainer relationships may be challenging.

Maintainers (and perhaps community members, generally), could be upset about the idea of “picking winners” from the ecosystem. If such a situation arises, the ESL community should strive to reach consensus among the people involved, while at the same time keeping analysis paralysis at bay. At the end of the day, making a decision is important even if it is not "perfect" by a given set of standards. In any case, the ESL providing specific functionality does not stop others from offering alternatives.

Tension might also arise if ESL policy changes, presumably because the change is believed to be good for the ecosystem, and some maintainers of crates in the ESL disagree with the change. This can be mitigated by making sure ESL maintainers have a say in ESL policies, and also by understanding that when people come together to do something like this, not everyone is going to like every decision that gets made.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

A primary goal of this proposal is to minimize the number of additional people and organizations that need to be trusted by Rust programs for widely used functionality.

The theoretically ideal minimum is zero additional trusted parties, which is the situation when dependencies come from the same organization as the compiler already being trusted (e.g. the Golang standard library). If the ideal of zero additional trusted parties cannot be achieved, the number should be no more than can be reasonably vetted by every programmer (e.g. the Boost C++ Libraries).

Minimizing the number of additional trusted parties for widely used dependencies would not entirely eliminate all supply chain security issues, as Rust programs will continue to use some third-party dependencies, but it would greatly reduce the attack surface.

"Batteries included" standard libraries for languages are a well-established way to achieve this goal. It is a safe and reliable solution so long as there are resources to pursue it. There are great examples to look to for ideas regarding architecture, implementation, and policies.

Languages that have continued to require large numbers of third-party dependency sources for most programs have had [outsized supply chain security problems](#appendix-c) despite, in some cases, massive investment. There is no reason to believe that the situation with Rust will be any different if the ecosystem does not make a concerted effort to reduce the average number of third-party dependencies in its programs.

The [Cargo Vet](https://mozilla.github.io/cargo-vet/) system was introduced to mitigate supply chain security issues in the Rust dependency ecosystem. While it's a helpful and laudable effort, it does not go far enough. Code audits vary in quality, even from programmers at large companies. They are not a substitute for trusting the entity that produced the code in the first place. Cargo Vet does not address questions about a project's security policies, processes, and resources. It also adds to the complexity of supply chain security for Rust programmers, which means that it's not widely used and contributes negatively to the ESL's secondary goal, improving the Rust development experience.

The [blessed.rs website](https://blessed.rs/crates) is a "hand-curated guide to the crates.io ecosystem, helping you choose which crates to use." It allows Rust developers to look up suggested crates based on function descriptions, significantly aiding with the issue of discovery and improving the Rust development experience. It does not, in its current form, contribute significantly to addressing the security concerns described in this RFC, but could in theory add more substantial and regular vetting for trustworthiness.

Implementing the already approved [namespaces for Rust crates RFC](https://rust-lang.github.io/rfcs/3243-packages-as-optional-namespaces.html) would be helpful in terms of both security and developer experience. It would contribute positively to addressing security concerns by allowing for simpler recognition of which crates have the same sources as well as providing the opportunity to programmatically restrict dependencies to a certain set of trusted sources. It would contribute to improving the Rust development experience by pushing the need for uniqueness in naming to the namespace, thus allowing for more functionally descriptive naming of crates.

# Prior art
[prior-art]: #prior-art

## Similar Efforts

Libraries similar to the vision for the ESL:

* [Go Standard Library](https://pkg.go.dev/std)
* [Swift Core Libraries](https://www.swift.org/documentation/core-libraries/)
* [Objective-C Foundation](https://developer.apple.com/documentation/foundation/)
* [C# .NET CLI Standard Libraries](https://en.wikipedia.org/wiki/Standard_Libraries_(CLI))
* [Python Standard Library](https://docs.python.org/3/library/index.html)

## Previous Relevant Proposals

[A number of proposals](#appendix-b) for effectively extending the Rust standard library have been made in the past, but none of them resulted in a change of the status quo. Generally, these prior proposals:

* Lacked a sense of urgency: they focused primarily on convenience and developer experience, which is a valid concern, yet not as urgent as improving Rust's supply chain security.
* Overlooked the issue of funding: an effort of this magnitude is unlikely to succeed without significant funding and credible fundraising capabilities.
* Failed to reach maturity: despite the effort and attention some proposals received, none managed to develop into a fully specified plan suitable for a formal discussion about potential adoption (i.e. none made it to the RFC stage).
* Were authored between 5 and 10 years ago. In the meantime, a lot has changed about the Rust ecosystem and community. For example, crates in the ecosystem are considerably more mature today in terms of functionality, APIs, and approaches. This, in turn, provides more clarity about what does and doesn't work. Also, we know more about how alternative mitigations suggested previously have played out.

This proposal aims to differ from previous proposals by addressing the above issues.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

The necessary amount of funding is yet to be determined. The Rust Foundation and ISRG will work on a plan with the ESL community as things move along.

# Future possibilities
[future-possibilities]: #future-possibilities

## Rust Project Maintained Crates and the ESL

The Rust Project maintains a collection of popular crates outside of the Rust standard library. Examples can be seen by looking at crates for which [rust-lang/libs is a maintainer](https://crates.io/teams/github:rust-lang:libs).

Ideally the ones providing functionality that the ESL should offer would, over time, move into the ESL (or the standard library).

## Migrating Functionality from ESL to the Standard Library

If the Rust Project decides they want to move something from the ESL into the Rust standard library, ESL developers should assist with the migration.

## Naming and Namespaces

Earlier it was stated that all crates in the ESL would ideally have names that:

1) Clearly describe the functionality they offer
2) Contain a reliable indicator of ESL membership

For practical reasons this may not be the case from the start of the ESL, but it's something to think about working towards in the future.

### Clearly Descriptive Crate Names

Because namespacing for crates is not implemented, crate names, rather than source entity names, must be unique (e.g. "CreativeName" vs. "CreativeOrg::DescriptiveName"). This encourages creative naming in order to achieve uniqueness. As a result, crates coming into ESL are likely to have names that are creative rather than clearly descriptive.

Clearly descriptive crate names would help Rust developers find what they need more easily. While crates can have descriptions associated with them, reading descriptions is slower than reading names.

Perhaps more importantly though, it would also help with the readability of Rust code since crate names are used in Rust code frequently. It's the difference between "creative::ClientConfig" and "tls::clientConfig". This can be worked around with "use [creative] as [descriptive]" but that is more work and not always done in clear ways.

### Reliably Indicating ESL Membership

Lack of namespacing also make it difficult to clearly and reliably indicate membership in the ESL, where reliably means that only crates in the ESL can indicate membership in the ESL.

As part of understanding and mitigating supply chain risks, Rust developers should be able to easily tell which crates come from the same source. Unfortunately, this is not easy today.

Adding support for namespacing crates is the best way to improve the situation. [Rust RFC 3243](https://rust-lang.github.io/rfcs/3243-packages-as-optional-namespaces.html) has been passed but is not yet implemented.

In the future, it would be good to implement RFC 3243 and get ESL crates into an ESL namespace (e.g. "esl::" or "ext_std::"). This would be an improvement over requiring GitHub organization checks, and it would allow ESL membership for dependencies to be reflected in Rust application code.

## Support for no_std

It should be possible, both technically and in terms of policy, for ESL crates to support no_std. This would not necessarily be feasible or make sense for all ESL crates, but it might be something to consider for some.

# Appendix A
[appendix-a]: #appendix-a

## Example Policies

A simplified list of example policies to use as potential starting point.

### Evolving APIs / Breaking Changes

ESL crates will be versioned with [Semantic Versioning](https://semver.org/).

Patch and minor updates may happen at any time.

Major updates will happen no more than once per year unless necessitated by a significant security concern.

### General Support Policy

Each major release of an ESL crate will receive backwards-compatible updates, at least addressing security and stability issues, for two years after the release of the subsequent major release. For example - if version 2.x.x is released on January 1, 2027, then version 1.x.x will receive updates until January 1, 2029.

The only exception to this policy is if resolving a security issue requires a new major release (i.e., cannot be fixed in a backwards-compatible manner). If this happens, all previous vulnerable major versions will cease to receive updates immediately.

### Security Updates

The ESL security policy should largely mirror the [Go Security Policy](https://go.dev/doc/security/policy).

Reports will be acknowledged within 7 days, and each vulnerability will be assigned a track - PUBLIC, PRIVATE, or URGENT. All security issues will be issued CVE numbers.

Security updates will be created and released as quickly as reasonably possible.

URGENT track issues are fixed in private and trigger an immediate dedicated security release, possibly with no pre-announcement.

### Code Review and Testing Standards

Rust ESL code will be managed in GitHub repositories. GitHub will be used for managing issues and pull requests.

The Rust ESL project will maintain a list of maintainers who are qualified to review PRs.

Any change to ESL code must be reviewed by a maintainer who is not the patch author. This will be enforced by GitHub settings. Pull requests will be required, no direct merges will be allowed.

Whenever possible, changes to functionality should come with tests. When it is reasonably possible to write a test, no changes to functionality will be merged until they are merged with tests.

Rust code in the ESL will follow the [Rust Style Guide](https://github.com/rust-lang/rust/tree/HEAD/src/doc/style-guide/src).

API styles for ESL crates should become consistent over time. At minimum, it should be a goal to converge on the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/about.html). This does not need to happen prior to a crate joining the ESL, but ESL APIs should evolve towards this ideal at an acceptable rate of change.

### Minimum Supported Rust Version (MSRV) Guarantees

A maximum MSRV will be selected for the entire ESL and updated as it makes sense to do so. The maximum MSRV will never be newer than two years old. Individual crates may have MSRVs older than the maximum.

### Platform Support

Platform support will be managed in the same way that [platform support is managed by the Rust language](https://doc.rust-lang.org/nightly/rustc/platform-support.html), with Tier 1 platform guaranteed to work, Tier 2 platforms guaranteed to build, and Tier 3 platforms with non-guaranteed support.

# Appendix B
[appendix-b]: #appendix-b

Previous relevant proposals:

* [stdx](https://github.com/brson/stdx), July 2015
* [The Rust Platform](https://aturon.github.io/blog/2016/07/27/rust-platform/), July 2016
* [Expansion of standard library](https://internals.rust-lang.org/t/expansion-of-standard-library/10475), June 2019

# Appendix C
[appendix-c]: #appendix-c

The following links provide information about supply chain attacks in other ecosystems:

* [Details about the event-stream incident](https://blog.npmjs.org/post/180565383195/details-about-the-event-stream-incident)
* [XZ Utils backdoor](https://en.wikipedia.org/wiki/XZ_Utils_backdoor)
* [No Unaccompanied Miners: Supply Chain Compromises Through Node.js Packages](https://cloud.google.com/blog/topics/threat-intelligence/supply-chain-node-js/)
* [[CVE-2019-15224] Version 1.6.13 published with malicious backdoor](https://github.com/rest-client/rest-client/issues/713)
* [Malicious remote code execution backdoor discovered in the popular bootstrap-sass Ruby gem](https://snyk.io/blog/malicious-remote-code-execution-backdoor-discovered-in-the-popular-bootstrap-sass-ruby-gem/)

All of these attacks were made possible because the compromised packages were not maintained in contexts that provided strong enough infrastructure, policy, and organizational resistance to such attacks.

Creating a package maintenance environment that provides strong enough resistance to such attacks is difficult, and if most applications depend on many different maintainers to do so independently the attack surface will huge.

One option, represented by this RFC, is to build a highly attack resistant context once and maintain many of an ecosystem's most common build blocks within it. There will still be surface area for attacks to occur, but it will be more limited and it will be easier for developers to avoid taking on that surface area if they wish to be careful about dependencies.
