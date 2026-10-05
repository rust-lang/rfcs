- Feature Name: `candidate_target_policy`
- Start Date: 2021-06-24
- RFC PR:

# Summary
[summary]: #summary

This RFC introduces a policy for targets not yet ready to merge into Rust, to
allow for initial coordination and consensus-seeking.

# Motivation
[motivation]: #motivation

In some cases, a candidate
[target](https://doc.rust-lang.org/nightly/rustc/target-tier-policy.html) for
Rust may not be ready for immediate inclusion in the Rust compiler. For
instance, a target may require a new code generation backend, or non-upstream
patches to an existing code generation backend, or a set of support libraries
that have not yet been finished. Or, making useful use of the target may
require a set of Rust language or library extensions that have not yet been
reviewed or accepted.

In such cases, the target may begin development outside the Rust source tree,
to allow for ongoing development and experimentation prior to eventual
upstreaming. In general, the Rust project does not take out-of-tree work as
setting precedent or compatibility requirements for subsequent upstreaming; any
target developed out-of-tree may require additional changes (including
incompatible changes) before being merged into the Rust source tree. However,
for a candidate target, the possibility of certain types of compatibility
changes would make development and upstreaming much harder; for instance,
adding support within a code generation backend such as LLVM requires choosing
a target name ("triple") and determining some of the ABI and calling
conventions of the target (such as type sizes and alignment).

Rust teams or team members sometimes have informal discussions with the
developers of various candidate targets, to coordinate and build consensus on
the requirements needed to later upstream the target, and to determine
properties of the target such as the ABI and target name. However, such
informal discussions do not typically get recorded in any official location,
and may not be coordinated among the broader Rust project.

This policy (the "Candidate Target Policy") allows for the formal evaluation of
a candidate future target for Rust, and the acceptance of such a target as a
"Candidate Target". It has a lower threshold than even the "Tier 3" target
requirements, while still serving to start a conversation and reach some
initial conclusions on compatibility.

When the Rust project accepts a Candidate Target under this policy, that
primarily indicates that if the Rust project subsequently incorporates that
target at Tier 3 or higher, the agreed-upon properties of the target (name,
ABI, etc) will remain substantially the same unless there's a specific strong
reason to do otherwise. The acceptance of a Candidate Target does not represent
any binding committment on the part of either Rust or the proposers of the
target.

Once this RFC is accepted, the policy sections of this RFC (together with
portions of the motivation section by way of introduction) should be posted as
an appendix alongside the [Target Tier
Policy](https://doc.rust-lang.org/nightly/rustc/target-tier-policy.html). This
RFC will not be the canonical home of the up-to-date Candidate Target Policy.

# Candidate Target Policy
[candidate-target-policy]: #candidate-target-policy

A proposal for a new Candidate Target should be made as a compiler-team Major
Change Proposal (MCP).

Some requirements in this policy additionally require evaluation by other Rust
teams, if the target requires or recommends changes in those areas. Such
evaluation may take place via the normal approval process of the appropriate
team, such as an MCP. The compiler-team MCP should track any additional
approvals required, and such approvals should go through the appropriate
processes for those teams prior to the acceptance of the Candidate Target
proposal.

A Candidate Target proposal should quote the corresponding requirements
verbatim as part of explaining how the target meets those requirements.

A new target may directly apply for Tier 3 status without first becoming a
Candidate Target. In that case, the implementation of the target serves as a
focal point for coordination and review of the properties of the target. A
proposal for a new Candidate Target serves to provide that focal point for a
target that will not be ready to apply and qualify for Tier 3 status in the
near future.

Evaluation of a new Candidate Target is based on these requirements as well as
on the judgment of the teams involved. Those teams may apply additional
requirements, including subjective requirements, such as to deal with issues
not foreseen by this policy. (Such requirements may subsequently motivate
additions to this policy.)

While these criteria attempt to document the policy, this policy still involves
human judgment. Targets must fulfill the spirit of the requirements as well, as
determined by the judgment of the approving teams. Reviewers and team members
evaluating targets and target-specific patches should always use their own best
judgment regarding the quality of work, and the suitability of a target for the
Rust project. Neither this policy nor any decisions made regarding targets
shall create any binding agreement or estoppel by any party.

Each accepted Candidate Target will be documented on an appropriate official
page, such as a sub-page of
<https://doc.rust-lang.org/nightly/rustc/platform-support.html>, together with
the target's properties (name, ABI, etc) determined by this process.

The acceptance of a Candidate Target is not a stability guarantee about the
future availability or acceptance of that target at any tier.

Acceptance of a Candidate Target does not represent any commitment to avoid
language, library, or compiler changes that may preclude the inclusion of the
target in the future. Proposers of such changes, and teams evaluating such
changes, may take their impact on the viability of Candidate Targets into
account, but may determine that the proposed changes outweigh the future value
of the target.

In this policy, the words "must" and "must not" specify absolute requirements
that a target must meet to qualify for a tier. The words "should" and "should
not" specify requirements that apply in almost all cases, but for which the
approving teams may grant an exception for good reason. The word "may"
indicates something entirely optional, and does not indicate guidance or
recommendations. This language is based on [IETF RFC
2119](https://tools.ietf.org/html/rfc2119).

## Requirements

A proposed target or target-specific patch that substantially changes code
shared with other targets (not just target-specific code) must be reviewed and
approved by the appropriate team for that shared code before acceptance.

- A Candidate Target must have a designated developer or developers to serve as
  the point of contact for any subsequent coordination needed on the properties
  of the target. (The mechanism to track and CC such developers may evolve over
  time.) Ideally, this should be one or more of the developers of the target
  upstream. This should not be a role address; please keep the point of contact
  up to date with the Rust project.
- The proposers and reviewers of the Candidate Target must have a good-faith
  belief that the target will be proposed for inclusion in Rust in the future,
  and that the target will be capable of meeting the requirements for at least
  tier 3 even if it does not currently do so. The Candidate Target process
  exists to coordinate potential future targets, not to maintain targets
  out-of-tree indefinitely.
  - In particular, the target must be capable of meeting the licensing and
    legal expectations of a tier 3 target; thus, we do not review Candidate
    Target requests for proprietary targets or targets with other onerous
    restrictions, as such targets will never be able to become part of the Rust
    project.
  - The planned future addition of the target to the Rust project must be
    intended to become supportable as a first-class toolchain for the target.
    The target within the Rust project should not be an outdated snapshot of
    work done primarily out-of-tree.
- There must be a concrete plan to develop the Candidate Target; we don't
  expend review bandwidth on a hypothetical target that nobody plans to work
  on.
- Candidate Targets must use naming consistent with any existing targets; for
  instance, a target for the same CPU or OS as an existing Rust target should
  use the same name for that CPU or OS. Targets should normally use the same
  names and naming conventions as used elsewhere in the broader ecosystem
  beyond Rust (such as in other toolchains), unless they have a very good
  reason to diverge. Changing the name of a target can be highly disruptive,
  especially once the target reaches a higher tier, and the Candidate Target
  process helps coordinate and establish consensus on the name of the target.
  - Target names should not introduce undue confusion or ambiguity unless
    absolutely necessary to maintain ecosystem compatibility. For example, if
    the name of the target makes people extremely likely to form incorrect
    beliefs about what it targets, the name should be changed or augmented to
    disambiguate it.
- If the target would require a new code generation backend, the compiler team
  must additionally review and approve the addition of that backend, prior to
  the acceptance of the Candidate Target.
- If the target requires new language additions, such as new target-specific
  attributes, those new additions must be approved in principle for further
  design and experimentation, as evaluated by the language team. For instance,
  this may happen via a language MCP. This requirement does not preclude the
  possibility of further design and iteration on these features before their
  eventual stabilization; rather, this serves to require coordination and
  communication regarding these features while the target remains out-of-tree.
- If the target proposes an ABI different from that of any existing Rust target
  (such as in the sizes or alignments of standard types), the proposal must be
  approved by the Rust language team, who will evaluate whether the acceptance
  of the target would meet the semantic requirements of the language, and avoid
  adverse effects on the Rust ecosystem.
- Neither this policy nor any decisions made regarding targets or Candidate
  Targets shall create any binding agreement or estoppel by any party. If any
  member of an approving Rust team serves as one of the maintainers or points
  of contact for a target, or has any legal or employment requirement (explicit
  or implicit) that might affect their decisions regarding a target, they must
  recuse themselves from any approval decisions regarding the target's tier
  status, though they may otherwise participate in discussions.
  - This requirement does not prevent part or all of this policy from being
    cited in an explicit contract or work agreement (e.g. to implement or
    maintain support for a target). This requirement exists to ensure that a
    developer or team responsible for reviewing and approving a target does not
    face any legal threats or obligations that would prevent them from freely
    exercising their judgment in such approval, even if such judgment involves
    subjective matters or goes beyond the letter of these requirements.

Once a target is approved as a Candidate Target, a sub-page of the Rust
platform support page will list the target, its name, and any relevant
properties about it.

A Candidate Target may be removed if:
- it stops meeting these requirements,
- the point of contact becomes unreachable,
- the Candidate Target shows no signs of development activity, or
- the approving teams no longer believe the target will be successfully
  added to Rust.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This policy was inspired by the Target Tier Policy, as well as by many informal
discussions with the developers of potential future Rust targets. This policy
serves to formalize that informal process, providing the target developers with
additional confidence in future consensus approval, and providing the Rust
developers with a more thorough and consistent review of the target's
properties.

We could, alternatively, continue with existing informal reviews and advice,
without any formal review process or tracking. However, as Rust scales, a more
formal process ensures consistent and reliable answers.

In addition, a more formal review process provides a clear artifact that the
developers of a target may reference, such as when proposing the addition of
changes to a backend or to another project that will serve as a dependency.
This helps break potential cycles, in which the Rust project doesn't want to
add a target without backend support, and a backend doesn't want to add a
target without confidence that it'll be used.

# Future possibilities
[future-possibilities]: #future-possibilities

We may expand these requirements further as we evaluate new Candidate Targets,
and as we discover new aspects of targets that prove useful to coordinate and
gain consensus on.

This policy may interact with or serve as input to future Rust trademark
policies.
