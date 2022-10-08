- Feature Name: `promote-aarch64-unknown-linux-gnu-to-tier-1`
- Start Date: 2020-07-17
- RFC PR: [rust-lang/rfcs#2959](https://github.com/rust-lang/rfcs/pull/2959)
- Rust Issue: [rust-lang/rust#78251](https://github.com/rust-lang/rust/issues/78251)

# Summary
[summary]: #summary

Promote the Arm aarch64-unknown-linux-gnu Rust target to Tier-1.

The next section provides a justification for the promotion.

**Please note that the following are required next steps that should ideally emerge from ensuing discussions:**

   * An approval from the Compiler Team that Tier-1 target requirements have been met.

   * An approval from the Infrastructure Team that the target in question may be integrated into CI.

   * An approval from the Release Team that supporting the target in question is viable in the long term.


# Motivation
[motivation]: #motivation

The Arm aarch64-unknown-linux-gnu target is [currently a Tier-2 Rust target](https://forge.rust-lang.org/release/platform-support.html#tier-2), in accordance with the target tier policy articulated [here](https://rust-lang.github.io/compiler-team/minutes/design-meeting/2019-09-20-target-tier-policy/).

In the last 2 quarters, very good progress has been made in understanding and filling the gaps that remain in the path to attaining Tier-1 status for this target.

As a direct result, those gaps have either already been filled or are very close to being filled.

As such, this RFC aims to:

- Evidence what has been done.

- On the basis of that evidence propose that the proceedings to promote the aarch64-unknown-linux-gnu target to the Tier-1 category may please be kickstarted.

- Culminate in the actual promotion of the aarch64-unknown-linux-gnu target to Tier-1, including any and all of the relevant processes and actions as appropriate.

Please note that the narrative here doesn't always match the RFC template so some liberties may have been taken in the expression.

Please also note, by way of wilful disclosure, that this RFC's author is an employee of Arm.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

1. **In essence, the target tier policy for a Tier-1 target aims to obtain the following technical and tangible assurances:**

   a. The Rust compiler and compiler tests must all build and pass reliably for the target in question.

   b. All necessary supporting infrastructure, including dedicated hardware, to build and run the Rust compiler and compiler tests reliably must be available openly.

   c. There must exist a robust and convenient CI integration for the target in question.

2. **In addition, the target tier policy for a Tier-1 target aims to obtain the following strategic assurances:**

   a. The long term viability of the existence of a target specific ecosystem should be clear.

   b. The long term viability of supporting the target should be clear.

   c. The target must have substantial and widespread interest within the Rust developer community.

   d. The target must serve the interests of multiple production users of Rust across multiple organizations or projects.

3. **Finally, the target tier policy for a Tier-1 target aims to obtain the following approvals:**

   a. An approval from the Compiler Team that Tier-1 target requirements have been met.

   b. An approval from the Infrastructure Team that the target in question may be integrated into CI.

   c. An approval from the Release Team that supporting the target in question is viable in the long term.

The following section details how points 1 and 2 of the above assurances have either already been met or are close to being met. 

As mentioned in the [summary](#Summary), items in 3 above are **required next steps.**

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

**1.a. The Rust compiler and compiler tests must all build and pass reliably for the target in question.**

 - As of today, ***all*** tests pass reliably.

 - In addition, as a result of inputs from the core team, engineers from Arm performed an audit of all tests that are currently marked **'only-x86_64'** and **'only-aarch64'** has been done. This was to ascertain whether past viewpoints and/or decisions that led to those markings are still valid. 
     - The audit report is available [here.](https://docs.google.com/spreadsheets/d/1B-Jg1Ml6nAF6Tf9wJGTgqkFUNeJEejC3aMikGl6vXlc/edit?usp=sharing)

     - Work is being planned under the guidance of core team members to upstream patches that came out of the audit as well as to address any open questions that came about.

**1.b. All necessary supporting infrastructure, including dedicated hardware, to build and run the Rust compiler and compiler tests reliably must be available openly.**

 - Two quarters ago, Arm donated a [Packet c2.large.arm system](https://www.packet.com/cloud/servers/c2-large-arm/) to the core team.

 - It is noteworthy that the core team have done a brilliant job in integrating this system into Rust's CI infrastructure while also circumventing myriad Github Actions security problems that popped up.

 - Over time, Arm intends to further donate newer and more capable hardware to this initiative.

**1.c. There must exist a robust and convenient CI integration for the target in question.**

 - The happy outcome of the core team's work with the donated system is that the system integrates largely seamlessly with existing Rust CI infrastructure. 

 - The integration has been verified to produce green runs once patches from the two outstanding PRs are in place.

**2.a. The long term viability of the existence of a target specific ecosystem should be clear.**

 - It is hard to concretely quantify this aspect.

 - That said, Arm AArch64 silicon is either already prevalent or is en-route to prevalance in a wide spectrum of application domains ranging from 'traditional' embedded systems at one end of the spectrum, on to mobile phones, clam-shell devices, desktops, vehicle autonomy controllers, datacenter servers etc all the way to high performance super-computers.

 - The evidence to that effect is too numerous to quote but generally easy to verify openly. 

 - It is fair to state that this is an ongoing reality which is unlikely to stop trending upwards and sidewards for the foreseeable future.

 - Software stacks built for those domains predominantly use an AArch64 Linux kernel build.

 - Rust presents an attractive value proposition across all such domains, irrespective of the underlying processor architecture.

 - **As such, the Rust aarch64-unknown-linux-gnu target's ecosystem presents very strong viability for the long term.**

**2.b. The long term viability of supporting the target should be clear.**

 - It is hard to concretely quantify this aspect.

 - It is worth calling out, in the same vein as the previous point, that given the increasing prevalance of AArch64 silicon deployments and given Rust's general value proposition, **supporting the Rust aarch64-unknown-linux-gnu target presents very strong viability for the long term.**

 - Note that the core team have created a ['marker team' for Arm](https://github.com/rust-lang/team/blob/master/teams/arm.toml) as well as the [t-compiler/arm Zulip stream](https://zulip-archive.rust-lang.org/242906tcompilerarm/index.html). These form important parts of a support story for aarch64-unknown-linux-gnu (amongst other Arm targets). Arm's Rust team is represented in both.

**2.c. The target must have substantial and widespread interest within the Rust developer community.**

 - It is hard to concretely quantify this aspect.

 - It is generally fair to state that **there is already substantial and widespread interest for the aarch64-unknown-linux-gnu target in the Rust developer community**.

 - It is also generally fair to state that there is a clear upward trend in the use of AArch64 systems as self hosted development environments. 

 - Most major operating system environments support hosted development on AArch64 based systems and this trend is increasing.

 - As a somewhat related note: Slow but steady progress is being made to support Windows AArch64 targets, initially for cross-platform development. This shall inevitably trend towards hosted development.

 - As such, **it is very likely that developer interest in Rust on aarch64-unknown-linux-gnu will continue to increase in the medium to long term.**

**2.d. The target must serve the interests of multiple production users of Rust across multiple organizations or projects.**

 - It is hard to concretely quantify this aspect.

 - Most major Arm software ecosystem partners are either already using Rust extensively, or are building up to extensive use. A few publicly known examples are Microsoft, Google and Amazon. There are many more.

 - Arm itself recognises Rust as an important component to consider in a broader horizontal safety and security foundation across multiple processor portfolios. 

 - Arm has dedicated a small team to help improve Rust for the aarch64-unknown-linux-gnu target. This team is included in the ['marker team' for Arm](https://github.com/rust-lang/team/blob/master/teams/arm.toml) as well as the [t-compiler/arm Zulip stream](https://zulip-archive.rust-lang.org/242906tcompilerarm/index.html) created by the core team.

 - **It is very likely that support for aarch64-unknown-linux-gnu in these organisations will trend upwards commensurate with the increasing prevalence of AArch64 silicon based systems.**

Points 3.a through 3.c from the [Guide-level explanation](#Guide-level-explanation) section above are addressed in the [Unresolved questions](#unresolved-questions) section below.

# Drawbacks
[drawbacks]: #drawbacks

**There is no drawback envisioned in promoting the Rust aarch64-unknown-linux-gnu to Tier-1.**

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Given the narrative above, it is the opinion of the author that it would now be tactically sound to promote aarch64-unknown-linux-gnu to Tier-1.

- Inclusion in the Tier-1 category is very likely to be a self sustaining action in that it will promote increased scrutiny with increasing quality as a return. With that return, interest in Rust will grow further both in the AArch64 context and even more generally.

- Anecdotally, not having the Tier-1 'badge' has been seen to become an obstacle to increasing mindshare in Rust for this target. Organisations tend to associate a Tier-1 categorisation with better quality, suitability for key projects, longevity etc. With a reasonably justified Tier-1 badge in place, the likelihood is that such organisations will tend to promote the use of Rust in production.

As such **there is no substantially robust reason to not proceed with promoting aarch64-unknown-linux-gnu to Tier-1.**

# Prior art
[prior-art]: #prior-art

- Existing Tier-1 targets represent prior-art.

- It is appropriate to call out that no non i686 or x86_64 based target has ever been promoted to Tier-1. The fact that those targets have intrinsically supported self hosted development has arguably been a primary reason for their maturity.

- The aarch64-unknown-linux-gnu target is therefore somewhat uncharted territory.

However, as emphasised in the narrative thus far, **the aarch64-unknown-linux-gnu target now exhibits the properties required by a Tier-1 target as per the target tier policy.**

# Unresolved questions
[unresolved-questions]: #unresolved-questions

No unresolved questions or issues remain.

# Future possibilities
[future-possibilities]: #future-possibilities

As the first non i686 and non x86_64 target to be considered for promotion to Tier-1, the aarch64-unknown-linux-gnu target will likely set a precedent for other AArch64 and non-AArch64 targets to follow in the future.
