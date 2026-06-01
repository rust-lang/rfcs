- Feature Name: `advance_security_disclosure`
- Start Date: 2022-04-03
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC proposes a change to the project's security policy to expand the set
of organizations who are notified with the full details of Rust security
vulnerabilities ahead of public disclosure, and to pre-announce to the general
public upcoming security releases.

# Motivation
[motivation]: #motivation

The Security Response WG is responsible for handling incoming vulnerability reports
for all projects maintained by the Rust Team. Those reports are investigated
and fixed by a small set of trusted domain experts (invited to open reports on
an ad-hoc basis, even if they're not a member of any Rust team), and once the
fix is ready we coordinate with the reporter on a date for public disclosure.

To ensure all users have a toolchain update ready when the vulnerability is
disclosed, the current security policy mandates the early disclosure of
security vulnerabilites to some major Linux distributions 3 days before public
disclosure (through the private [distros@vs.openwall.org][distros] mailing
list). This allows those distributions to prepare package updates and release
the fix as soon as the vulnerability is announced. All other users will know
about the vulnerability only at the moment of public disclosure.

The WG thinks this model is not sufficient anymore:

* Nowadays there are more toolchain providers than the Rust project itself and
  the subset of Linux distributions who are part of that mailing list.
  Providing early disclosure just to [distros@vs.openwall.org][distros] results
  in some Rust users not having a fixed toolchain they can update to whenever a
  vulnerability is announced.

* Large production users have internal builds of the Rust toolchain, and they
  need to allocate personnel to update it and rebuild their Rust projects
  whenever the vulnerability is disclosed. Announcing vulnerabilities out of
  nowhere hinders their ability to plan that work.

This RFC proposes changes to the security policy to help with those cases.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The security policy will be extended to include two kinds of early
notifications sent before public disclosure of security vulnerabilities.

The Rust Security Response WG will aim to send them 5 business days ahead of
public disclosure, or 1 business day ahead of public disclosure in case of
critical, actively exploited vulnerabilities (subject to caveats discussed in
the reference-level explanation).

## Advance disclosures

To ensure all Rust users have access to a fixed toolchain, the Rust Security
Response WG will send the full vulnerability details ahead of time to
distributors of the Rust toolchain and maintainers of alternate Rust
implementations (when applicable), giving them all the information required to
prepare a toolchain update for their users or customers.

Organizations (either companies or established open source projects) eligible
to be notified will be able to apply for inclusion in the advance disclosure
list, as long as they meet the requirements outlines in the reference-level
section of this RFC. Organizations in the list are expected to keep all
information about those vulnerabilities confidential before public disclosure.

## Security releases pre-announcements

To ensure all Rust users are aware of upcoming security releases, before public
disclosure the Security Response WG will publish a pre-announcement of the
upcoming release on the [public security announces mailing list][announce-ml]
and the [Rust blog][blog], mentioning the planned date and time of the release,
the severity of the vulnerability and the affected part of the project.

This will allow all Rust users to schedule the time to update their toolchain.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Advance disclosures

### List of notified organizations

The Rust Security Response WG will maintain a list of organizations who are
eligible to receive advance disclosures. Organizations (either companies or
established open source projects) matching any of these criteria can apply:

* Organizations distributing the Rust toolchain (as provided by the Rust
  project) or a fork of the Rust toolchain explicitly meant to be used by their
  external customers or users. Organizations shipping the toolchain just to
  internal customers/users are not eligible, nor are organizations publicly
  shipping a toolchain primarily meant for internal use.

* Organizations developing an independent implementation of the Rust language,
  not originating from a fork of the toolchain provided by the Rust project.
  The implementation has to be mature, widely used and must follow all policies
  on alternate implementation set by the Rust project (if any are set).

Note that when evaluating the eligibility criteria, the spirit of the rules
will be considered: if the WG thinks an organization that would not be eligible
otherwise is trying to find a loophole to be able to apply, the WG will have
the authority to reject the application.

Organizations will be able to apply by opening an issue in the
[rust-lang/wg-security-response] repository, detailing:

* Which of their products or projects would make them eligible to be notified.

* One or more email addresses that should receive the advance disclosure.

* Two or more people that will serve as the organizational contact: those
  people will be contacted by the WG whenever the WG needs to discuss something
  with the organization.

* A pledge to maintain confidentiality, as detailed in the "Confidentiality"
  section of this RFC. The person submitting the application must be authorized
  to enter that pledge on behalf of their organization.

The application process will be public, and the decision will be made by the WG
by consensus. The resulting list of organizations receiving advance disclosure
will also be public.

In addition to individual organizations, industry-standard mailing lists and
disclosure venues that are frequented mostly by organizations that would
otherwise be eligible to be notified can be added to the list, if the WG
determines the venue is appropriate, and has sufficient security and
confidentiality practices.

Organizations won't need to pay any fee to receive early notifications, nor
will they have to be a sponsor to the Rust Foundation. There will be no formal
NDA in place, only the informal confidentiality pledge.

Once the application is accepted, the organizational contacts will be able to
request changes to the address receiving the notifications or the list of
contacts by opening another issue.

### Removal from the list

Organizations can be removed from the list in any of these cases:

* The reason why the organization joined the list is not applicable anymore,
  for example if they discontinued their distribution. Organizations can
  re-apply later if they become eligible again.

* The organization did not act on the early notifications it received for an
  extended period of time. The WG will work with the organization to see if
  there are valid reasons for not acting on the notifications before removing
  the organization. Organizations can re-apply later.

* The organization either accidentally violated confidentiality more than once
  without having taken appropriate steps to prevent it from happening again
  after the first violation, or intentionally/maliciously violated
  confidentiality one time. Organizations will only be able to re-apply with
  full consent from the whole Security Response WG and Rust Core Team.

The decision will be made by the WG, and can be appealed to the Rust Core Team.

### Logistics of the notification

Advance notifications will be sent to all organizations in the list, containing
the draft advisory and the relevant patches (if available). The timing of the
notification will depend on the impact of the vulnerability:

* For most vulnerabilities, five business days before public disclosure.
* For actively exploited critical vulnerabilities, one business day before
  public disclosure.
* For actively exploited critical vulnerabilities that do not require a
  toolchain update to be effectively mitigated, no advance notification will be
  sent.

Not all vulnerabilities will result in an advance notification being sent.
Namely, vulnerabilities in the project infrastructure, crates maintained by the
Rust project, or other projects that are not shipped as part of the toolchain
will not result in an advance notification, as there would be no update for the
organizations to prepare in advance.

If a specific vulnerability warrants it, the Security Response WG can also
decide to send advance notifications to other organizations who are not members
of the list, or to send the notification earlier than 5 days ahead of public
disclosure. This will be coordinated with and subject to approval of the person
who reported the vulnerability to the WG.

Depending on the terms of the embargo set by the person who reported the
vulnerability, in a small amount of cases the WG might not be able to send the
early notification without breaking the terms of the embargo. In those cases
the WG will still try to coordinate with the reporter to send a notification to
all the organizations in the list as close to our expected notification times
as possible.

### Confidentiality

All organizations who received an advance notification must treat all the
information they received and all fixes they developed as confidential until
the public disclosure by the WG. The information can only be shared inside the
organization on a need-to-know basis, and can never be shared with people
external to the organization without explicit permission from the WG.

Organizations must take appropriate steps to protect the confidentiality of
advance notifications and of all communications and materials associated with
them. If the organization already has established procedures for dealing with
confidential information received from partners, those procedures should at
least apply (in addition to any other procedure deemed necessary).

There will be no formal NDA to sign to start receiving early notifications, but
any violation of confidentiality will still have consequences (see "Removal
from the list").

## Public pre-announcement of security releases

If a security release is planned to address the vulnerability, at the same time
of advance disclosure the Security Response WG will post a pre-announcement of
the upcoming security release on the [public security announces mailing
list][announce-ml], the [Rust blog][blog] and any other venue the WG deems
appropriate. The announcement should mention:

* The planned date and time (with the time zone) for the release.
* The severity of the vulnerability, as determined by the WG.
* The affected part of the project (e.g., the compiler, the standard library...).

The Security Response WG can opt to mention less information than that, if it
deems sharing more would increase the likelihood of the vulnerability being
independently rediscovered too much.

# Drawbacks
[drawbacks]: #drawbacks

The Rust Security Response WG is currently well respected in the industry, and
we are trusted with reports about cross-cutting vulnerabilities affecting not
just Rust. This respect and trust is fragile, and any leak caused (indirectly)
by the WG could reduce the amount of vulnerabilities shared with us, worsening
the security of the ecosystem as a whole.

The risk of leaks will increase as we notify more organizations ahead of time.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The pillar that guided the drafting of this RFC is *"make sure every Rust
developer has access to an updated toolchain when the vulnerability is
disclosed"*. This gives all players in the ecosystem the same information
required to address and fix the vulnerabilities, and in the WG's opinion this
is the right balance between sharing with the least amount of people and having
the biggest impact.

The WG considered multiple alternatives, listed below.

## Keep the status quo

Do nothing, and continue with the current policy of only sharing
vulnerabilities ahead of time with [distros@vs.openwall.org][distros]. This
doesn't cover non-Linux distributions of Rust.

## Don't share with anyone

Restrict our policies even more and don't share vulnerabilities ahead of time
with anyone. This would put every Rust developer who doesn't use the toolchain
provided directly by the Rust project at risk whenever a vulnerability is
announced.

The use of toolchains not provided directly by the Rust project has use cases
that are critical for Rust's adoption in areas we care about. Some examples of
those could be:

* The use of Rust in place of other languages in operating systems,
  system-level tooling, and C libraries depends on the ability to build such
  code using toolchains provided by the same source (in order for
  distributions to remain self-contained).

* Some industry sectors (like qualified environments) require toolchains to be
  supported for 10 to 20 years, and it'd be unreasonable for the Rust project
  to provide such long term support on its own, leaving the task to third
  party toolchain vendors.

## Additionally notify big players in the ecosystem ahead of time

In addition to the proposed list of organizations, we could also notify big
players in the ecosystem to make sure products used by a lot of people are
fixed before the vulnerability is known to the general public, and the fallout
from the vulnerability is smaller.

In the WG's opinion, the (small) benefit this would provide to the wider
ecosystem is not worth it compared to the big increase of the risk of leakage
(as we'd need to notify a lot of companies under this program).

In addition, such a policy would require defining who a "big player" is, and
any such definition would either be too broad (disclosing in advance to a set
of organizations wide enough to make leaks inevitable) or become a kind of
preferential treatment for a subset of organizations.

# Prior art
[prior-art]: #prior-art

When drafting this RFC, the Security Response WG looked at other security
response programs in other open source projects. A summary of our findings is:

* Python: couldn't find any public mention of disclosing vulnerabilities early.

* LLVM: company and projects shipping projects depending on LLVM can join the
  Security Group and have access to all details from the start. In exchange
  they have to help with triaging and fixing the vulnerability upstream. Rust
  is one of such projects.

* Go: announces the vulnerability on [distros@vs.openwall.org][distros] and
  posts a pre-announcement 3 days before disclosing the vulnerability.

* Ruby: offer access to distributions, alternate implementations and companies
  offering Ruby in a Platform as a Service.

* Ruby on Rails (Web Framework): announces the vulnerability on
  [distros@vs.openwall.org][distros] 3 days before the disclosure, but doesn't
  notify any production user.

* Django (Web Framework): pre-announces the vulnerability a week before
  disclosure, and notifies distributions, a subset of maintainers and on a
  case-by-case basis a few large Django websites.

* Xen (Hypervisor): notifies distributions, public clouds, vendors of Xen-based
  systems, and large production users two weeks before public disclosure.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

*Nothing here so far.*

# Future possibilities
[future-possibilities]: #future-possibilities

*Nothing here so far.*

[distros]: https://oss-security.openwall.org/wiki/mailing-lists/distros
[announce-ml]: https://groups.google.com/g/rustlang-security-announcements
[rust-lang/wg-security-response]: https://github.com/rust-lang/wg-security-response
[blog]: https://blog.rust-lang.org
