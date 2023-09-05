- Start Date: 2023-07-24
- RFC PR: [rust-lang/rfcs#3463](https://github.com/rust-lang/rfcs/pull/3463)

# Summary
[summary]: #summary

The Rust community has outgrown the current crates.io policies. This RFC proposes a new "Terms of Use" policy based on prior work by PyPI, npm and GitHub.

# Motivation
[motivation]: #motivation

> Why are we doing this? What use cases does it support? What is the expected outcome?

crates.io has a "[Package Policies](https://crates.io/policies)" page, which describes the current, organically grown policies. A lot of support requests or questionable uses of crates.io however need explicit decisions from the crates.io team, since many cases are currently not covered by these policies. Additionally, decisions made by the team may be seen as arbitrary without written guidelines.

The situation around name squatting has lately also reached unsustainable levels, and while [namespaces](https://github.com/rust-lang/rfcs/pull/3243) might help with some parts of the problem we would still need policies for name squatting namespaces then.

The main motivation for this RFC is to give the crates.io team a fixed set of rules to determine if a project is using crates.io in a reasonable way, or if the user should get a warning and the project potentially be removed. It is mostly codifying the existing practices of the team, except for being more strict regarding name squatting.

# Proposal
[guide-level-explanation]: #guide-level-explanation

The following is a proposed new "Terms of Use" policy for crates.io, replacing <https://crates.io/policies> and <https://crates.io/data-access>.

-------------------------------------------------------------------------------

# Terms of Use

**Short version:** _crates.io is a critical resource for the Rust ecosystem,
which hosts a variety of packages from a diverse group of users. That resource
is only effective when our users are able to work together as part of a
community in good faith. While using crates.io, you must comply with our
Acceptable Use Policies, which include some restrictions on content and conduct
on crates.io related to user safety, intellectual property, privacy,
authenticity, and other limitations. In short, be excellent to each other!_

We do not allow content or activity on crates.io that:

- violates the [Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct)
  of the Rust project
- is unlawful or promotes unlawful activities, incurring legal liability in the
  countries the Rust Foundation officially operates in
- is libelous, defamatory, or fraudulent
- amounts to phishing or attempted phishing
- infringes any proprietary right of any party, including patent, trademark,
  trade secret, copyright, right of publicity, or other right
- unlawfully shares unauthorized product licensing keys, software for
  generating unauthorized product licensing keys, or software for bypassing
  checks for product licensing keys, including extension of a free license
  beyond its trial period
- contains malicious code, such as computer viruses, computer worms, rootkits,
  back doors, or spyware, including content submitted for research purposes
  (tools designed and documented explicitly to assist in security research are
  acceptable, but exploits and malware that use the crates.io registry as a
  deployment or delivery vector are not)
- uses obfuscation to hide or mask functionality
- is discriminatory toward, harasses or abuses another individual or group
- threatens or incites violence toward any individual or group, especially on
  the basis of who they are
- is using crates.io as a platform for propagating abuse on other platforms
- violates the privacy of any third party, such as by posting another person's
  personal information without consent
- gratuitously depicts or glorifies violence, including violent images
- is sexually obscene or relates to sexual exploitation or abuse, including of
  minors (see "Sexually Obscene Content" section below)
- is off-topic, or interacts with platform features in a way that significantly
  or repeatedly disrupts the experience of other users
- exists only to reserve a name for a prolonged period of time (often called 
  "name squatting") without having any genuine functionality, purpose, or
  significant development activity on the corresponding repository
- is related to buying, selling, or otherwise trading of package names or any
  other names on crates.io for money or other compensation
- impersonates any person or entity, including through false association with
  crates.io, or by fraudulently misrepresenting your identity or site's purpose
- is related to inauthentic interactions, such as fake accounts and automated
  inauthentic activity
- is using our servers for any form of excessive automated bulk activity, to
  place undue burden on our servers through automated means, or to relay any
  form of unsolicited advertising or solicitation through our servers, such as
  get-rich-quick schemes
- is using our servers for other automated excessive bulk activity or
  coordinated inauthentic activity, such as
    - spamming
    - cryptocurrency mining
- is not functionally compatible with the cargo build tool (for example, a
  "package" cannot simply be a PNG or JPEG image, a movie file, or a text
  document uploaded directly to the registry)
- is abusing the package index for purposes it was not intended

You are responsible for using crates.io in compliance with all applicable laws,
regulations, and all of our policies. These policies may be updated from time to
time. We will interpret our policies and resolve disputes in favor of protecting
users as a whole. The crates.io team reserves the possibility to evaluate each
instance on a case-by-case basis.

For issues such as DMCA violations, or trademark and copyright infringements,
the crates.io team will respect the legal decisions of the
[Rust Foundation](https://rustfoundation.org/) as the official legal entity
providing the crates.io service.


## Package Ownership

crates.io has a first-come, first-serve policy on crate names. Upon publishing a
package, the publisher will be made owner of the package on crates.io.

If you want to take over a package, we require you to first try and contact the
current owner directly. If the current owner agrees, they can add you as an
owner of the crate, and you can then remove them, if necessary. If the current
owner is not reachable or has not published any contact information the
crates.io team may reach out to help mediate the process of the ownership
transfer.

Crate deletion by their owners is not possible to keep the registry as immutable
as possible. If you want to flag your crate as open for transferring ownership
to others, you can publish a new version with a message in the README or
description communicating to the crates.io support team that you consent to
transfer the crate to the first person who asks for it:

> I consent to the transfer of this crate to the first person who asks
> help@crates.io for it.

The crates.io team may delete crates from the registry that do not comply with
the policies on this document. In larger cases of squatting attacks this may
happen without prior notification to the author, but in most cases the team will
first give the author the chance to justify the purpose of the crate.


## Data Access

If you need access to a large subset of the crates.io database we recommend
first looking at the **crates.io [index repository](https://github.com/rust-lang/crates.io-index)**.
This repository is updated live whenever new versions are published and contains
all the information needed for cargo to run the dependency resolution algorithm.

In case the index dataset is insufficient for your purposes, we also publish a
**database dump** every 24 hours. This includes the majority of data from our
database except for sensitive private information. The latest database dump is
available at <https://static.crates.io/db-dump.tar.gz> and information on using
the content is contained in the tarball. Please note that while we aim to keep
the data structure somewhat stable, we can not give any stability guarantees on
the exact database table layouts.

If the index repository and the database dump are insufficient you may also use
the crates.io API directly, though it is at the discretion of the crates.io to
block any excessive usage. We require users of the crates.io API to limit
themselves to a maximum of 1 request per second.

We also require all API users to provide a user-agent header that allows us to
uniquely identify your application. This allows us to more accurately monitor
any impact your application may have on our service. Providing a user agent that
only identifies your HTTP client library (such as `request/0.9.1`) increases the
likelihood that we will block your traffic.

It is recommended, to include contact information in your user-agent header:

- Bad: `User-Agent: reqwest/0.9.1`
- Better: `User-Agent: my_bot`
- Best: `User-Agent: my_bot (my_bot.com/info)` or `User-Agent: my_bot (help@my_bot.com)`

This allows us to contact you if we would like a change in your application's
behavior without having to block your traffic.

We reserve the right to block traffic from any client that we determine to be in
violation of this policy or causing an impact on the integrity of our service.


## Security

Safety is one of the core principles of Rust, and to that end, we would like to
ensure that cargo and crates.io have secure implementations. To learn more about
disclosing security vulnerabilities for these tools, please reference the
[Rust Security policy](https://www.rust-lang.org/policies/security) for more
details.

Note that this policy only applies to official Rust projects like crates.io and
cargo, and not individual crates. The crates.io team and the Security Response
working group are not responsible for the disclosure of vulnerabilities to
specific crates, and if any issues are found, you should seek guidance from
the individual crate owners and their specific policies instead.

Thank you for taking the time to responsibly disclose any issues you find.


## Sexually Obscene Content

We do not tolerate content associated with sexual exploitation or abuse of
another individual, including where minors are concerned. We do not allow
sexually themed or suggestive content that serves little or no purpose other
than to solicit an erotic or shocking response, particularly where that content
is amplified by its placement in profiles or other social contexts.

This includes:

- Pornographic content
- Non-consensual intimate imagery
- Graphic depictions of sexual acts including photographs, video, animation,
  drawings, computer-generated images, or text-based content

We recognize that not all nudity or content related to sexuality is obscene.
We may allow visual and/or textual depictions in artistic, educational,
historical or journalistic contexts, or as it relates to victim advocacy. In
some cases a disclaimer can help communicate the context of the project.


## Violations and Enforcement

crates.io retains full discretion to take action in response to a violation of
these policies, including account suspension, account termination, or removal of
content.

We will however not be proactively monitoring the site for these kinds of
violations, but instead relying on the community to draw them to our attention.

While the majority of interactions between individuals in the Rust community
falls within our policies, violations of those policies do occur at times.
When they do, the crates.io team may need to take enforcement action to address
the violations. In all cases, content and account deletion is permanent and there
is no basis to reverse these moderation actions taken by the crates.io team.
Account suspension may be lifted at the team's discretion however, for example in
the case of someone's account being compromised.


## Credits & License

This policy is partially based on [PyPI’s Acceptable Use Policy](https://github.com/pypi/warehouse/blob/3c404ada9fed7a03bbf7c3c74e86c383f705d96a/policies/acceptable-use-policy.md)
and modified from its original form.

Licensed under the [Creative Commons Attribution 4.0 International
license](https://creativecommons.org/licenses/by/4.0/).

-------------------------------------------------------------------------------

# Prior art
[prior-art]: #prior-art

As the "Credits & License" says, the main inspiration for the proposed policy is the [Acceptable Use Policy](https://pypi.org/policy/acceptable-use-policy/) of the Python Package Index (PyPI). Their policy in turn is based on the [Acceptable Use Policies](https://docs.github.com/en/site-policy/acceptable-use-policies/) of GitHub. Both of these policies are licensed under the [Creative Commons Attribution 4.0 International license](https://creativecommons.org/licenses/by/4.0/), so we can happily reuse them.

[PEP 541](https://peps.python.org/pep-0541/) (Python Enhancement Proposal) was also mixed into the document above, specifically the [Invalid Projects](https://peps.python.org/pep-0541/#invalid-projects) section.

The third source of material are the "[Open-Source Terms](https://docs.npmjs.com/policies/open-source-terms)" from npm, from which a few more rules on "Acceptable Content" were imported.

RubyGems, Maven Central, Packagist (PHP) and Nuget (C#) were also investigated, but they did not appear to have written rules published in easy-to-find places.


# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Is the wording of the "name reservation" clause sufficient to discourage name squatting in the future?
- Are there any current legitimate uses of crates.io that would suddenly be forbidden by these new rules?
- Should the crates.io policies forbid embedding executable binaries in the crate files?

# Future possibilities
[future-possibilities]: #future-possibilities

- [PEP 541](https://peps.python.org/pep-0541/) also defines rules for abandoned projects and how people could continue maintenance for them. Introducing something like that would be a large deviation for crates.io though, and something that would need a dedicated RFC. Nevertheless, it is worth thinking about if the majority of the Rust community would prefer having such a ruleset.
