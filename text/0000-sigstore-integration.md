- Feature Name: cargo_crates_sigstore_integration
- Start Date: 2023-03-27
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

Authors:

* Tim Pletcher, Hewlett Packard Enterprise
* Ulf Lilleengen, Red Hat / IBM

# Summary
[summary]: #summary

This document proposes the integration with Sigstore for cargo and crates.io to support signing and verifying crates.

Sigstore consists of a set of components that together provide a complete artifact signing and verification mechanism. Sigstore was created as a “common good” service much in the same way that Let’s Encrypt was and in fact shares some underlying service components with Let’s Encrypt, most notably the Transparency Log implementation in the Rekor service. Moreover, Sigstore addresses some of the inherent challenges in legacy signing mechanisms such as the necessity to have long term sensitive key management capabilities.

This document will address two primary use case scenarios:

1. The developer building Crates locally on their desktop
1. Crates produced through a formal build plant (GitHub Actions, CircleCI, etc. and commercial enterprise internal build plants)

These use cases will allow us to start to draw trust boundaries around artifact production processes and identify the metadata attributes that will need to be generated at artifact build-time which are subsequently consumed at run-time for effective policy and consumption controls to be possible for the ultimate consumers.

This proposal has been influenced by [this document](https://docs.google.com/document/d/1mXrVAkUA9dd4M7fa_AJC8mQ55YnYJ-DKsGq30lh0FvA/edit#heading=h.jyrb6etgzah), as well as the work at GH regarding the NPM ecosystem, the RFC for which can be [found here](https://github.com/npm/rfcs/blob/main/accepted/0049-link-packages-to-source-and-build.md). Further we have engaged both parties in discussions to further glean relevant information related to this effort and to achieve a certain level of commonality in the overall pattern of implementation in the language ecosystem. 

# Motivation
[motivation]: #motivation

The Solar Winds breach event was an inflection point for the topic of software supply chain security. The Solar Winds event exposed weaknesses in two specific areas, artifact provenance and build system integrity. That event resulted in a broad variety of reactions ranging from an aggressive push by the USG to a rapid acceleration of security/provenance related OSS and commercial projects oriented on addressing the significant shortcomings in the OSS software ecosystem.

One of the most notable projects to arrive early on the heels of the Solar Winds event was the Sigstore project (sigstore.dev).  The design of Sigstore both functionally and from a process perspective make it trivially easy to implement and execute the signing and verification activities in any workflow, whether on the developer desktop or in the context of a formal build plant. It is for these reasons that Sigstore has seen rapid adoption over the course of the last 2 years and has an active and thriving contributor base. One of the most compelling aspects of Sigstore however is its potential to provide a unified common toolchain for the establishment of cryptographic identity across heterogenous artifacts and eliminate the overhead induced by ecosystem and vendor specific signing and verification implementations. These attributes of Sigstore and its rapid, broad adoption are precisely what make it a compelling choice for adoption by the Rust community for Cargo and crates.io.

When considering artifact provenance, one of the long known weaknesses in the OSS world are the central official package management systems that serve as a nexus for sharing and consumption of artifacts in any given language ecosystem. Historically these systems have struggled with a variety of attack vectors and suffered most specifically from a lack of concrete cryptographic artifact identities and security focused policies and tooling to manage contribution and ongoing artifact maintenance. The challenges are further exacerbated by the state of build system fidelity broadly. Provenance generation at artifact creation time and subsequent verification is where Sigstore comes in. Its approach, leveraging OIDC based identities as a gate to the signing function as opposed to long-term sensitive keys, as well as a public root and widely available verification capabilities make it a developer friendly, potentially ubiquitous signing architecture.

It is in the context of failure after failure in the OSS ecosystem related to compromised packages and lack of reliable provenance, as well as the looming changes that will be driven by the recently announced White House cybersecurity strategy, that this RFC is presented. The Rust ecosystem can *and should* lead on these topics given its accelerating adoption in mission critical scenarios, expanding use in linux core, etc, etc. Not only can the community address basic artifact signature operations, but it can also “skate to where the puck is going to be” on the evolving build plant ecosystem. 

On the topic of build system integrity, historically, this operational domain was not delineated as a specific function from generic security systems operations in the context of say, NIST. However, with the new SSDF directive NIST has specifically engaged on this topic. from a standards perspective. It’s not an exaggeration to say that NIST artifacts can be a bit dense, but more interestingly, one of the other early ecosystem events post Solar Winds was the launch of the Supply Chain Levels for Software Artifacts or “SLSA”. [SLSA](https://slsa.dev) is a capabilities framework that specifically and exclusively addresses the topic of build system fidelity and fills a gap in the existing standards. It is relatively lean and approachable and provides organizations with a fast, easy way to assess current capabilities and plan for improvement. SLSA addresses both artifact provenance and build system integrity and also very clearly lays out relevant attack vectors as can be seen below):


![SLSA: Supply-chain-threats](https://user-images.githubusercontent.com/20165/174750708-2be483ac-7e41-4bc3-8ee9-440ef33d9423.svg)

In the context of the SLSA framework, adopting Sigstore as proposed in this RFC  will immediately address the B, C, F and H attacks identified above.

It’s worth noting that the team at GitHub working on the NPM implementation have essentially landed at the same to use case patterns. In our discussions with them as this document was prepared they articulated a focus on the build plant pattern as that pattern allows for the establishment of an attestation story around the systems where the artifact was produced. Conversely, artifacts produced on a developer desktop or system that is not formally controlled from a security perspective must necessarily be classified as technically “unsafe”.

In all of this the elephant in the room is identity. However, identity can also be addressed in the context of the two primary use case buckets. The easy one is build system identity, the hard one is individual contributor identity. This will be addressed in the [Identity section](#identities) below.

## Current state of Cargo/ crates.io tooling

At present, crates.io provides the following information about published crates:

* checksum of crate contents
* crate author/ownership via GitHub

Our current reality is that:

* The verification of the above information relies on a third party (crates.io) not being compromised.
* In the event that crates.io is compromised, crates pulled from crates.io cannot be verified.
* There is no way to establish the identity of the system that built the artifact.
* There is only the limited identity metadata associated with the GH ID that is the source of the artifact.

By executing on the adoption of Sigstore and structuring crates.io metadata around the ability to present information capable of driving policy based consumption (both on the desktop and in the build and runtime environments), the Rust community will take a large and important step in evolving the its tooling to meet the demands of a rapidly evolving software supply chain ecosystem and of the security and regulatory demands that are coming into play.

One of the first principles for considering the implementing this type of tooling in the community ecosystem is making sure that the developer experience remains a Tier 1 consideration. Historically, when security specific capabilities are brought into the mix the impact to productivity and developer experience has been of such a level that very often the security “stuff” gets ignored or just falls to the wayside (see PGP signing as an example). This is one area where Sigstore truly shines. From the outset Sigstore was designed to be a toolchain that would be a trivial impact to the day to day workflow of a developer as to be a non-issue. The cosign application is easy to use on the desktop (and fairly easy to drop into a build system)  and the integration with many widely used IdP implementations (GH, GCP, etc) make the workflow simple. There are no long-term keys to manage, nor any other infrastructure when you are utilizing the public service. Further the verification service (Rekor) is accessed through a well documented API making both desktop CLI and machine level access a straight-forward proposition.

While not a part of this RFC, end users can implement additional projects such as [in-toto](https://in-toto.io/) to build more advanced verification and attestation of software.

## Goals

This proposal will enhance cargo and crates.io through adoption of the Sigstore capabilities and workflows for supporting both use cases as outlined above. Specifically, 

* Adopt Sigstore infrastructure to allow for the signing of crates for use both above.
* Facilitate workflows in both Cargo and crates.io to allow for signature generation in the former case and verification in the latter case of crates and their dependencies by crate consumers (a note on dependencies: the depth level should be 1 for verification in this first phase)

This implementation will support all three Sigstore implementation models which will be effectively transparent to both crates.io maintainers and crates.io consumers. At a high level the three Sigstore implementation models are:

1. Use of the public Sigstore common good service, signing with the Sigstore public root.
1. Signature operations via a private fulcio instance with intermediate signer issued internally, but signature metadata pushed to public Rekor instance.
1. Full internal Sigstore infra used with internal consumption only software.

# Non Goals

* Implementing TUF - this proposal describes a sequenced implementation with TUF in a phase 2. 
* Signing crates.io index - this is covered by other proposals, but is discussed in the context of TUF later in this document.
* Extending or altering the Identity approach of Sigstore, i.e. OIDC based

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Crate signatures and verifications are a way to tie the authenticity and integrity of a crate based on public key cryptography. The implication is that, as an end user, you can be sure that the crates that you download from crates.io have not been tampered with, and that they are signed by the correct owner of the package.

The process of `signing` involves generating a signature for a crate, using a private key, and attaching that signature when publishing a crate. The process of `verification` involves downloading a crate (using cargo) and using a _trusted_ public key to verify the authenticity (the signer) and integrity (the contents) of a crate.

To simplify key management for users, cargo supports using [Sigstore](https://sigstore.dev) as a way to generate ephemeral keys based on an identity, and as an immutable log that can be monitored and audited in the event of a compromise. By default, a publicly available instance ensures that you do not need to manage this infrastructure yourself. If you are running your own crate registry, Sigstore instance and identify provider, you can configure cargo to use that.

Verification is accomplished through an API interaction with the relevant Rekor service. This occurs at artifact submission time to the relevant crates registry and is also how the consumer will verify the artifact whether manually or in a run-time scenario.

## Implementation sequencing

## Phase 1

* Establish the required metadata structures and modifications to crates.io to accommodate the Sigstore signing process.
* Modify cargo to add the Sigstore specific capabilities for signing and verification, and configuration options to require signed artifacts from crates.io during dependency pulls, etc.
* Modify crates.io to accept the additional metadata attributes from the signing operation.
* Provide inspection / verification capabilities in the crates.io UI. for Sigstore signed crates

## Phase 2

* TUF (The Update Framework) - Phase 2 TUF AuthZ Architecture
   * The TUF protocol addresses a set of defined attacks and threat models specific to software distribution systems. It has the capability of providing a strong AuthZ implementation for any OSS project with respect to control of the ability to generate an artifact. For example, it is used by the Sigstore project itself to instantiate and manage the Sigstore Public Root. See: [public Sigstore instance](https://blog.sigstore.dev/a-new-kind-of-trust-root-f11eeeed92ef). This article does a good job of showing the options.
* Make Sigstore signed crates mandatory.
* Start marking legacy / unmaintained unsigned crates as unsafe
* Add  metadata attributes to allow for identification of build source,  i.e. “certified” system such as GitHub actions. This could also take the form of using Sigstore’s organic countersigning capability.

## Configuration changes to cargo

By default, cargo will use the publicly available Sigstore instances together with GitHub as the identity provider. To override (if using a different registry than crates.io), the `$HOME/.cargo/sigstore.toml` file can be configured with the location of Sigstore and identity services:

```
fulcio = "https://fulcio.sigstore.dev"
rekor = "https://rekor.sigstore.dev"
issuer = "https://oauth2.sigstore.dev/auth"
connector = "https://github.com/login/oauth"

[registry]
require-signed-crates =”yes/no”
```

## Signing

Signing a crate happens when passing the `--sign` argument upon publishing a crate:

```
cargo publish --sign
```

Cargo will perform the necessary steps to sign your crate before publishing, and attach the relevant information to the crates.io request.

Upon receiving the new crate metadata, crates.io will verify that the signature belongs to the crate owner.

Signing an existing crate can be done using `cargo sign`:

```
cargo sign [<crate which you own>]
```

This will retrieve the crate from crates.io, and perform the same steps as for publish to generate the signature, but will attach the signature and certificate to the existing crate.


## Operational Changes to Crates.io / Verification

Crate signatures can be verified for any crate that is retrieved from `crates.io` that has signatures attached.

To explicitly verify signatures for a crate, the `verify-signature` subcommand can be used:

```
cargo verify-signature [<crate>] [--skip-dependencies]
```

The command will fail if the crate (and dependencies) do not have verifiable signatures.

Once the community has become familiar with signing and verification, signing and verification can be enabled by default for cargo commands.

## Offline Verification

Sigstore allows offline verification of crates by including additional metadata for the crate that can be stored just like other data for offline cargo usage.


# Reference-Level Explanation
[reference-level-explanation]: #reference-level-explanation

The crates.io and cargo types for new crates, will need to be modified to include the following metadata:

* Signature - created based on the generated .crate file on publish
* Identities - this must be one or more of permitted identities of the signer
* Certificate - the public certificate that can be used to verify the crate
* Bundle - allows for offline verification not connecting to the transparency log

In the event that the crate is not being signed, these fields may be optional/null.
## Identity, AuthN and AuthZ

[identities]: #identities

### Identity

In the Summary section above we described two use cases specific to this implementation. Those use cases of course have associated identity types:

* Use-case #1(Developer Desktop Builds) clearly is associated with an individual
* Use-case #2(Build Plant) is what is commonly referred to as an Non-Person Entity or NPE

Individual (Person) identities are a sensitive topic and there is no “good” (let alone great) story on this broadly in the OSS ecosystem. There are efforts in several regions globally beginning to take shape around a government issued digital identity however, and this has the potential to allow a base upon which to move forward. In the meantime we are left with email addresses and associated credentialing through IdP’s provided by large scale private entities such as GH, Gitlab, Google, MSFT, Apple, etc.

The public Sigstore service has out of the box integration with many of the above-mentioned IdP’s and so use of the public service is quite straightforward/simple for any individual developer. It is also worth noting that businesses that choose to implement an internal Sigstore signing service can very easily wire up the Fulcio component of Sigstore to the internal IdP for private signing / verification operations.

Implementation of Sigstore signing does need to consider (but will not directly address) regulatory requirements related to PII. For example, the EU GDPR requires that personally identifiable information (PII) must be removable (email addresses fall into this classification). However, the Rekor component of Sigstore is implemented on top of a Transparency Log which is by definition and design immutable. Effectively this means that for now, people with privacy concerns should not be using actual personal identities tied to their GH account and email. 

It is very important to acknowledge that while this situation with personal identity persists, crate artifacts produced on the desktop must always be considered as unsafe, even if signed. In the same way that one can use “burner’ GH and email accounts to protect their privacy, malicious actors can as well to introduce attacking software. There are other operational imperatives related to the safe consumption of any OSS, but that is beyond the scope of this document. 

However, in practice most OSS projects where there are more than a handful of contributors will very likely use CI/CD (Build) capabilities provided through an aaS model. In this scenario, identity in the context of the build activity now shifts to the NPE model. Here, providers capable of authenticating to the public Sigstore service via organic OIDC capabilities. Mainstream commercial build plants running aaS are de-facto subject to various standards and certifications (both operational and security), and so can provide certain levels of attestation metadata related to the production of the artifact. Further, they of course have no privacy concerns. It is for these reasons, and others that the team at GH implementing Sigstore tooling for the NPM ecosystem have chosen to focus on the aaS build scenarios as they talk with and evangelize that focus as a broader pattern. This is a pattern that this RFC agrees with and adopts.

## Authentication

Sigstore uses the OIDC pattern and ecosystem as the basis for authentication against the signing issuer (Fulcio). We assume that most are familiar with the OIDC pattern and tooling and so will not address those topics here, with the exception of the OIDC identity payload attributes. The certificates used for signing of course need to have an associated identity:

* In the case of an individual developer signing from his desktop, Sigstore will embed the email address of the signing individual in the metadata delivered to the Rekor service. 
* In the case of an NPE signer the metadata delivered to the Rekor service will have an Issuer attribute that looks something like this: "Issuer": "https://token.actions.githubusercontent.com".

## Authorization

With respect to AuthZ, while there has been discussion in the pre-rfc thread regarding TUF, this proposal asserts that two phases will need to be undertaken. Phase one will rely on the existing mechanisms implemented on top of git and provided by the most commonly used platforms (GitHub, GitLab, etc, etc) that assign OSS project participants to a group, i.e. “Maintainers” or “Members” in the case of GH. Only project maintainers are typically allowed to either trigger a build event manually or implement automation to do so based on a set of project specific evaluation criteria. We choose this approach to AuthZ as it will allow focus on the core Sigstore implementation for cargo and crates.io in the near term, and a deliberate focus on TUF in the next phase. This is important as TUF is tricky to implement well and represents a significant shift from the existing AuthZ approach that is heavily socialized/embedded broadly in the OSS ecosystem currently.

This RFC does not recommend a specific timeframe to enter into Phase 2, but assumes that after some suitable period with the basic signature mechanics in place, Phase 2 can be addressed and TUF implementation can be designed and added.

## Cargo publish flow

![publish flow](https://raw.githubusercontent.com/lulf/rfc-resources/main/sigstore/cargo_sigstore-publish.drawio.png)

The following changes must be made to cargo when publishing a crate:

1. Authenticate the publisher using OIDC to retrieve a token
1. Generate the ephemeral signing key and certificate request
1. Generate a certificate signing request and pass with a token to Sigstore Fulcio.
1. Sign the generated .crate file using the private key
1. Attach signature, identity and certificate to crates.io publish request
1. Publish an entry to the Rekor log with the .crate digest and certificate public key

Most of the above is already implemented in https://github.com/sigstore/sigstore-rs.

### crates.io flow

The crates.io sub-flow on publish is described below:

![crates.io flow](https://raw.githubusercontent.com/lulf/rfc-resources/main/sigstore/cargo_sigstore-crates.io.drawio.png)

The following components need changing for crates.io:

1. Additional columns must be added to the PostgreSQL database
1. HTTP API must handle a new version of NewCratePublish request with the signature data
1. The HTTP handler talks to Rekor and verifies that the signature is valid and that owner matches certificate

## Cargo verify flow

The cargo flow during verification is shown below:

![verify flow](https://raw.githubusercontent.com/lulf/rfc-resources/main/sigstore/cargo_sigstore-verify.drawio.png)

The following changes must be made to cargo when building/verifying a crate:

1. Retrieve the crate contents, signature and certificate.
1. Verify that the crate contents is signed by the signature.
1. Verify that the certificate and signature is present in the transparency log.

### Offline flow

The offline verification flow is shown below

![verify flow](https://raw.githubusercontent.com/lulf/rfc-resources/main/sigstore/cargo_sigstore-verify-offline.drawio.png)

## Note on cargo dependencies

The interaction with Sigstore services is implemented in the `sigstore-rs` crate, which has the following sub-dependencies:

* tough: used to fetch fulcio public key and rekor certificates from Sigstore's TUF repository. 
* openid-connect: this is used to generate keyless signatures via Rust
* oauth2: this is a transitive dependency of openid-connect

These crate use `reqwest` and some async Rust. To handle this, we can either:

* Add the necessary dependencies to cargo with the goal of replacing the curl with reqwest to avoid multiple HTTP clients in use
* Use the lower level APIs in sigstore-rs and implement the HTTP interaction using curl which is used by cargo today. 

# Drawbacks
[drawbacks]: #drawbacks

Cargo may be slower if signing and verification is enabled by default. To start, it should be made opt-in for a period so that maintainers and users can get to know the tool and we can learn more about the reliability.

Reliance on the public [Sigstore uptime](https://www.chainguard.dev/unchained/sigstore-is-generally-available) of 99.5% availability goal may affect users signing and verifying crates, as well as crates.io verifying signature identities upon publish.

Additional metadata will be stored for each crate, which will increase storage requirements for the index.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

There are other approaches to key management out there, such as GPG. However, these typically have a higher effort requiring keys to be managed without the downsides of longevity and compromises. Sigstore is meant to capture some of these common patterns of managing keys and storing information for auditing, which lowers the barrier for signing crates.

Sigstore is in the process of being supported by [NPM](https://thenewstack.io/npm-to-adopt-sigstore-for-software-supply-chain-security/), [Maven](https://blog.sonatype.com/maven-central-and-sigstore) and [Python](https://www.python.org/download/sigstore/), and it has a large community.

The impact of not doing this leaves crate owners vulnerable to a crates.io compromise. Organizations that need to have way to verify crate integrity will be left to build their own solution and private package registries.

One specific alternative is to adopt [TUF](https://theupdateframework.io/) without Sigstore, but managing a full TUF root can be a lot for an already busy crates.io team. RFCs such as [this](https://github.com/withoutboats/rfcs/pull/7) describes using TUF for signing the crates.io index, and later signing crates themselves. Integration with Sigstore does not necessarily rule out the use of TUF for managing a signed crates.io index later, and there is ongoing work on making TUF and Sigstore play nicely together.

## Alternatives for identity providers (IdP)

Several alternatives are discussed [here](https://docs.google.com/document/d/1mXrVAkUA9dd4M7fa_AJC8mQ55YnYJ-DKsGq30lh0FvA/edit#):

* Running an IdP on crates.io: This would involve that crates.io managed their own identity provider. Since crates.io already relies on GitHub for identity, this can be left as an option for other registries.
* *Proposed* External IDP such as GitHub: Use what's already there, and there is already association of crate owners with GitHub identities.

# Prior art
[prior-art]: #prior-art

The topic of signing crates have been brought up several times in the past:

* https://github.com/rust-lang/crates.io/issues/75 - this issue raises the initial concern
* https://github.com/sigstore/community/issues/25 - contains an attempt at sigstore support, but focusing on rustup because cargo maintainers being overloaded
* https://github.com/rust-lang/cargo/issues/4768 - an attempt to provide a signed crates.io index
* https://github.com/withoutboats/rfcs/pull/7 - a modification of the above issue using TUF to signing index commits

[NPM](https://thenewstack.io/npm-to-adopt-sigstore-for-software-supply-chain-security/), [Maven](https://blog.sonatype.com/maven-central-and-sigstore) and PyPI are all in the process of adopting Sigstore. It's not uncommon for organizations to use multiple programming languages, so integrating with Sigstore means that they can reuse the same infrastructure for verifying and auditing their software.

The Sigstore community has written a [document](https://docs.google.com/document/d/1mXrVAkUA9dd4M7fa_AJC8mQ55YnYJ-DKsGq30lh0FvA/edit#) with recommendations and alternative designs for projects integrating Sigstore with artifact repositories.

## Articles and papers

The [Sigstore Blog](https://blog.sigstore.dev/) contains a lot of articles related Sigstore. In particular, [this post](https://blog.sigstore.dev/signatus-ergo-securus-who-can-sign-what-with-tuf-and-sigstore-ea4d3d84b8b6) covers the problems faced by similar package systems like PyPI.

The GitHub tema has published a few [blog posts](https://github.blog/2022-10-25-why-were-excited-about-the-sigstore-general-availability/) on Sigstore. 

A [paper](https://dl.acm.org/doi/abs/10.1145/3548606.3560596) about Sigstore on ACM.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

* Opt-in or opt-out of signing and/or verification? Making it opt-out could increase the percentage of signed crates, but making it opt-in to start with seems like the safest choice.

# Future possibilities
[future-possibilities]: #future-possibilities

* Use the The Update Framework (TUF) to manage trust roots for their own Sigstore instance, which would make it independent of the public Sigstore instances. 
* Use the public Sigstore instance to manage root keys for TUF.
* Extending other cargo commands to make use of the information stored in the transparency log when listing dependencies and other places where it makes sense.
* Integrating with [in-toto.io](https://in-toto.io) to provide more advanced attestation of artifacts.
* Build source attestation: adding special modes for public CI systems such as GitHub actions where trusted builders can sign to attest that artifact is built from a particular source.

* Use the public Sigstore instance to manage root keys for TUF.
* Extending other cargo commands to make use of the information stored in the transparency log when listing dependencies and other places where it makes sense.
* Integrating with [in-toto.io](https://in-toto.io) to provide more advanced attestation of artifacts..

# Threat model

The threat model is based on the [NPM RFC](https://github.com/npm/rfcs/blob/main/accepted/0049-link-packages-to-source-and-build.md).

Our threat model focuses on a package hijacking attack where an attacker gains access to a crate maintainer's crates.io credentials and uploads a modified version of the crate under a version number that is likely to get pulled in by cargo. Developers that use this crate cannot know whether the crate is built from the original source repository or on the attacker's laptop.

This proposal does not mitigate against compromised crates.io accounts. The aim is to make it harder to execute these types of attacks by creating a public audit trail for where, how and who published a package. Over time the presence of this information can be enforced.

As such, what we care about in this threat model is whether this provenance information can be forged and whether verification can be bypassed.

The following table highlights the impact of different compromises:


| Compromised infrastructure | Forge build provenance | Bypass verification |
| -------------------------- | ---------------------- | ------------------- |
| crates.io                  | No                     | Yes                 |
| User account               | No                     | No                  |
| CI/CD                      | Yes                    | No                  |
| Sigstore                   | Yes                    | No                  |
| Network                    | No                     | Yes                 |

An attacker could forge what goes in the provenance information if the CI/CD provider or Sigstore was compromised. Mitigating these attacks is out of scope in this proposal.

An attacker could bypass verification if the crates.io registry, the user or the network was compromised by not providing provenance information or removing it in transit.

The long-term solution to bypassing verification is enforcing or requiring that all crates have provenance information set during install time using cargo. It might take years to get to the point where a large portion of crates have this information set so we'll need to find ways to get there gradually.

Maintainers and developers consuming crates could also opt-in to requiring all their dependencies to include provenance information once their dependencies include this.

# Glossary
[glossary]: #glossary

Overview of the tools, techniques and terms used throughout this RFC document (taken from [the NPM RFC](https://github.com/npm/rfcs/blob/main/accepted/0049-link-packages-to-source-and-build.md#glossary))

The definitions are not exhaustive and only cover how it's been used in this document and the npm context.

- **Attestation**: Verifiable metadata (signed statement) about one or more software artifacts. 
- **Build provenance**: Verifiable information about software artifacts describing where, when and how something was produced.
- **[Cosign](https://github.com/sigstore/cosign)**: CLI tool used to interact with Sigstore: Focused on signing container images.
- **[DSSE Signature Envelope](https://github.com/secure-systems-lab/dsse)**: Data structure for passing around interoperable signatures.
- **[Fulcio](https://docs.sigstore.dev/fulcio/overview/)**: Certificate Authority that can notarize log-ins (i.e. verify and attest to legitimacy) with OIDC identities (e.g. Google or GitHub accounts), returning a time-limited certificate that's used to prove that you had access to an identity (e.g. email) at the time of signing.
- **Keyless signing with disposable/ephemeral keys**: Signing technique where you never handle long-lived signing keys, instead short-lived keys are used that only live long enough for the signing to occur. Sometimes referred to as "keyless signing".
- **Offline verification**:  In this context, when a signature has been uploaded to Rekor a detached copy is returned that can be verified offline.
- **OpenID Connect ([OIDC](https://openid.net/specs/openid-connect-core-1_0.html)) identity or ID token**: Verifiable information that the user or identity has been authenticated by a OIDC provider. The ID token is a JSON Web Token (JWT).
- **OpenID Connect ([OIDC](https://openid.net/specs/openid-connect-core-1_0.html)) identity provider**:  Authenticates users and issues verifiable id tokens that includes identity information, e.g. email. Examples include Google or GitHub.
- **Package hijacking attack**: Where a malicious version of an existing open source package gets uploaded to the registry. The [eslint-scope](https://eslint.org/blog/2018/07/postmortem-for-malicious-package-publishes) attack was a notable example of this kind of attack, and it frequently occurs due to compromised npm credentials but can also happen due to compromised builds or CI/CD.
- **[Rekor](https://docs.sigstore.dev/rekor/overview)**: Public immutable tamper-resistant ledger of signed software artifacts. Verifies that the Fulcio certificate was valid at the time of signing.
- **Publish attestation**: Verifiable metadata stating that the npm registry has authorized and accepted a published package version.
- **[Sigstore](https://www.sigstore.dev/)**: Public good infrastructure and standard for signing and verifying artifacts using short-lived “disposable keys”.
- **[SLSA](https://slsa.dev/)**: Comprehensive checklists of best practices for securing the software supply chain. SLSA levels specify how secure the different components are.
- **Software supply chain**: The series of actions performed to create a software product. These steps usually begin with users committing to a version control system and end with the software product's installation on a client's system.
- **Trusted builder**: Immutable build system that can't be modified by the source repository where it's being executed.
