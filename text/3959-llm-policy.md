- Feature Name: N/A
- Start Date: 2026-05-04
- RFC PR: [rust-lang/rfcs#3959](https://github.com/rust-lang/rfcs/pull/3959)
- Rust Issue: N/A

## Summary
[summary]: #summary

This policy details the requirements for using generative Artificial Intelligence (AI) models, particularly Large Language Models (LLMs), in all aspects of the Rust project. This includes (but is not limited to) contributions of code, documentation, chat messages, issue descriptions, etc.

1. *Trivial* LLM usage is always allowed, and thus irrelevant to this policy.<br>We allow changes made by LLMs are indistinguishable from those made by humans, where the LLM didn't have any creative input.
2. *Slop*, also known as *vibe-coding*, is spam and moderated accordingly.<br>We disallow changes made by LLMs with minimal human intervention.
3. *Potentially non-trivial* LLM usage must be *disclosed*, ideally with as much detail as possible.<br>(RFC-only note: This may necessitate additional tooling to notify new contributors about the policy and explain how disclosure works.)
4. Non-trivial usage, if correctly disclosed, is merely rejected or closed without any additional action.<br>There are no formal punishments for non-trivial usage *with disclosure*.
5. Non-trivial usage *without* proper disclosure can become a Code of Conduct violation.<br>There *are* formal punishments for frequent non-trivial usage *without disclosure*; this is seen as intentionally refusing to honor the boundaries we've set.
6. If a contributor does not fully understand the code they submit, their contribution may be rejected for that reason alone.<br>This is similar to, but not always *slop*. For example, they may understand a large portion, but not all of it, which shows that they still put in a lot of effort.

In general, as long as contributors are demonstrating an earnest effort to *reduce* LLM usage, even if said reduction is not total, then they should be commended for that instead of punished, even if the project itself draws the line at *non-trivial* usage for *accepted* contributions.

RFC-only note: In terms of additional tooling for *disclosure*, this RFC encourages the creation of a bot that automatically replies to contributions from new users informing them of the LLM policy and what constitutes sufficient disclosure. As mentioned, in general, going into as much detail as possible (e.g. prompts used, etc.) is preferred, but not always required. The RFC leaves the exact details of such implementation unspecified and up for revision later.

## Motivation
[motivation]: #motivation

The rapid development of generative AI models and LLM-based tools has lead to massive changes in the open source space. Rust, in particular, is no exception to this, and there has been a large uptick in the number of LLM-assisted contributions to the `rust-lang/rust` repository in particular.

The presence of *slop* or *vibe-coded* contributions, i.e. ones created using LLMs with minimal user input, has lead to massive maintainer burnout and frustration across the entire open source ecosystem. Currently the `rust-lang/rust` repository has a stopgap anti-spam measure which swiftly rejects these contributions and bans repeat offenders. The entire Rust team (T-all) has unanimously condemned these types of contributions, and this is from both avid LLM users and those who condemn LLMs entirely. Due to its popularity and effectiveness, this RFC proposes simply applying this stopgap measure to the entire `rust-lang` org.

The main issue is forming a policy with regard to all other forms of LLM usage. While many people, including team members, have expressed that these tools are valuable in a multitude of ways, they undeniably are contributing direct harm to the world at large. However, simply banning all use of the tools will not immediately mitigate this harm, and there is also a very clear accessibility niche which has been fulfilled by these tools. Even if these niches would be better filled by other tools, if LLMs are currently the best available option, we shouldn't ignore that.

There is also a pragmatic issue, where some LLM usage is simply impossible to detect and is effectively identical to human-authored changes. Additionally, since the tech industry has been putting LLMs wherever they possibly can, many people might have been using an LLM without even knowing it. We need to ensure that the discussion does not devolve into nitpicking LLM usage where effort could be better spent elsewhere.

Ultimately, the goal is to encourage users to be honest about LLM usage, since this promotes an environment of collaboration in good faith. Many LLM users, including team members, have indicated that they might simply continue using LLMs and avoid disclosure for fear of repercussions, and this is a very uncomfortable position to be in. It means that LLM users are encouraged to be dishonest about their actions, and it means that maintainers are forced to accuse users of LLM usage whenever they're suspicious, which really doesn't feel like good-faith collaboration. This is combined on top of the mention of *trivial* LLM usage, as mentioned before: if we don't distinguish usage that actually affects the end result, people stop caring and we stop knowing whether the result is affected, which makes reviewing contributions difficult.

The goal for this policy is to ensure honesty to the greatest extent possible, conceding that we shouldn't spend time discussing *trivial* LLM usage while still acknowledging potential issues with LLMs and what we can do about it.

### Harm Reduction

The primary motivation for this RFC is harm reduction, specifically with regard to the harm being done by the AI industry. While the usage of AI by larger society is *still* harmful for a number of factors, it's disproportionately harmful in the tech industry due to a number of factors:

* Many workers are forced to use it against their better judgment, which inflates usage higher than it otherwise would be
* Billing is generally done by usage, meaning that the amount of money funneled into the industry increases with usage
* This usage happens on paid plans which can be *extremely* expensive, upwards of hundreds of thousands of dollars per person

Limiting LLM usage in the Rust project directly counters this:

* Workers have more leverage to indicate situations where they should not use LLMs, rather than being forced to use it always
* Reduced usage in industry can lead to fewer companies purchasing fewer LLM licenses, funneling less money into this harmful industry
* Reduced usage from individuals can reduce individual spending on LLMs, as companies purchase smaller capacity and plans

And crucially, these limits on LLM usage are *specifically* targeted toward cases which comprise the most expensive LLM usage. If LLM usage is explicitly needed for accessibility reasons or it explicitly helps users despite its inability to *creatively synthesize* output, this usage is *still allowed*, even though the harms of LLMs still need to be acknowledged.

### Harms

Note: these were originally elaborated in prose, but have been since reduced to a concise series of bullet points with citations. The original prose, with the Rust-specific arguments removed, has been preserved [in a blog post](https://txt.ltdk.xyz/sloppery-slope/).

* Both Claude Code<sup id="claude-xai-back">[&#91;1&#93;](#claude-xai)</sup> and Gemini<sup id="gemini-xai-back">[&#91;2&#93;](#gemini-xai)</sup> directly fund xAI's Colossus data center, which is objectively making life miserable for a majority-black community in Memphis, Tennessee.<sup id="xai-back">[&#91;3&#93;](#xai)</sup>
* The industry is building data centers so quickly they can't even plug them in,<sup id="plugs-back">[&#91;4&#93;](#plugs)</sup> and are enthusiastic to pollute the air and water<sup id="clean-back">[&#91;5&#93;](#clean)</sup> or start wars over oil<sup id="drill-back">[&#91;6&#93;](#drill)</sup> to rectify that.
* The AI industry has blatant disregard<sup id="ethical-back">[&#91;7&#93;](#ethical)</sup> for copyright protections and these tools have been used for "license laundering."<sup id="chardet-back">[&#91;8&#93;](#chardet)</sup>
* Data labeling is crucial for these tools to work, but many of these workers are poorly compensated.<sup id="continent-data-back">[&#91;9&#93;](#continent-data)</sup><sup id="times-data-back">[&#91;10&#93;](#times-data)</sup>
* Tools are often biased against non-white<sup id="obama-back">[&#91;11&#93;](#obama)</sup> and trans<sup id="trans-back">[&#91;12&#93;](#trans)</sup> people, sometimes with legal repercussions.<sup id="workday-back">[&#91;13&#93;](#workday)</sup>
* LLMs still fail basic reasoning benchmarks, which further cement that bias and a lack of reasoning are not going to improve.<sup id="gsm-symbolic-back">[&#91;14&#93;](#gsm-symbolic)</sup>
* The AI industry's glut for memory and storage has decimated the consumer PC market<sup id="consumer-back">[&#91;15&#93;](#consumer)</sup> and made memory<sup id="ram-back">[&#91;16&#93;](#ram)</sup> and storage<sup id="ssds-back">[&#91;17&#93;](#ssds)</sup> unaffordable.
* The AI industry uses web crawlers which are indistinguishable from DDOS attacks<sup id="wikipedia-back">[&#91;18&#93;](#wikipedia)</sup><sup id="osm-back">[&#91;19&#93;](#osm)</sup><sup id="lwn-back">[&#91;20&#93;](#lwn)</sup>, have required admins to create and deploy various tools<sup id="iocaine-back">[&#91;21&#93;](#iocaine)</sup><sup id="anubis-back">[&#91;22&#93;](#anubis)</sup><sup id="go-away-back">[&#91;23&#93;](#go-away)</sup> or rely on providers which themselves support the industry.<sup id="cloudflare-back">[&#91;24&#93;](#cloudflare)</sup>
* LLMs require prompting instead of coding<sup id="linux-back">[&#91;25&#93;](#linux)</sup> and while you can control the *last* prompt, you cannot stop earlier prompts from stuffing beans up the LLM's nose<sup id="beans-back">[&#91;26&#93;](#beans)</sup><sup id="claude-beans-back">[&#91;27&#93;](#claude-beans)</sup> or being generally strange.<sup id="codex-friendly-back">[&#91;28&#93;](#codex-friendly-back)</sup><sup id="codex-goblins-back">[&#91;29&#93;](#codex-goblins-back)</sup>
* There is a strong power imbalance with LLM usage: LLM users are supported by their companies and peers, while LLM-abstinent people risk losing their jobs or are just forced to use LLMs against their better judgment.

<!-- GFM's footnotes don't let you control where they go. I'm not putting these all the way at the bottom of the RFC. Sorry, you have to deal with raw HTML. -->

<!-- Start of long footnotes -->

#### Sources for Harms

<details><summary>Open to unleash the footnotes</summary><section class="footnotes">

----

<ol><li id="claude-xai">

[Anthropic, SpaceX announce compute deal that includes space development](https://www.cnbc.com/2026/05/06/anthropic-spacex-data-center-capacity.html) [↩](#claude-xai-back)

</li><li id="gemini-xai">

[Google will pay SpaceX $920M per month for compute](https://techcrunch.com/2026/06/05/google-will-pay-spacex-920m-per-month-for-compute/) [↩](#claude-xai-back)

</li><li id="xai">

['We Are the Last of the Forgotten:' Inside the Memphis Community Battling Elon Musk's xAI](https://time.com/7308925/elon-musk-memphis-ai-data-center/) [↩](#xai-back)

</li><li id="plugs">

[Microsoft CEO says the company doesn't have enough electricity to install all the AI GPUs in its inventory - 'you may actually have a bunch of chips sitting in inventory that I can’t plug in'](https://www.tomshardware.com/tech-industry/artificial-intelligence/microsoft-ceo-says-the-company-doesnt-have-enough-electricity-to-install-all-the-ai-gpus-in-its-inventory-you-may-actually-have-a-bunch-of-chips-sitting-in-inventory-that-i-cant-plug-in) [↩](#plugs-back)

</li><li id="clean">

[Clean Air Act Resources for Data Centers](https://www.epa.gov/stationary-sources-air-pollution/clean-air-act-resources-data-centers) [↩](#clean-back)

</li><li id="drill">

[Actions to Implement President Trump's Vision for Venezuelan Oil](https://www.state.gov/releases/office-of-the-spokesperson/2026/02/actions-to-implement-president-trumps-vision-for-venezuelan-oil/) [↩](#drill-back)

</li><li id="ethical">

[Kadley v. Meta Platforms, Inc. Appendix A — Document #417, Attachment #1, Page 3](https://www.courtlistener.com/docket/67569326/417/1/kadrey-v-meta-platforms-inc/) [↩](#ethical-back)

This document appears to be notes from a January 2023 meeting that Mark Zuckerberg attended. It is heavily redacted, including a large section titled "Legal Escalations." Immediately after that section the document states "[Zuckerberg] wants to move this stuff forward," and "we need to find a way to unblock all this." [↩](#ethical-back)

</li><li id="chardet">

[Everything Claude Saw: A Transparent Account of the Chardet v7 Rewrite](https://dan-blanchard.github.io/blog/chardet-rewrite-controversy/) [↩](#chardet-back)

</li><li id="continent-data">

[Meet the people in the machine](https://web.archive.org/web/20241208201300/https://continent.substack.com/p/meet-the-people-in-the-machine) [↩](#continent-back)

</li><li id="times-data">

[OpenAI Used Kenyan Workers on Less Than $2 Per Hour to Make ChatGPT Less Toxic](https://web.archive.org/web/20260305193942/https://time.com/6247678/openai-chatgpt-kenya-workers/) [↩](#times-back)

</li><li id="obama">

[What a machine learning tool that turns Obama white can (and can’t) tell us about AI bias.](https://www.theverge.com/21298762/face-depixelizer-ai-machine-learning-tool-pulse-stylegan-obama-bias) [↩](#obama-back)

</li><li id="trans">

[GitHub Copilot refuses to provide completions with words it deems "sensitive"](https://github.com/orgs/community/discussions/110936) [↩](#trans-back)

</li><li id="workday">

[Amicus Brief on Mobley v. Workday](https://s3.documentcloud.org/documents/27781349/us-dis-cand-3-23cv770-d24320156e190-order-granting-motion-for-leave-to-file-amicus-bri.pdf) [↩](#workday-back)

</li><li id="gsm-symbolic">

[GSM-Symbolic: Understanding the Limitations of Mathematical Reasoning in Large Language Models](https://arxiv.org/abs/2410.05229) [↩](#gsm-symbolic-back)

</li><li id="consumer">

[COLLAPSE of Personal Computing | Investigation Into the Destruction of Ownership](https://www.youtube.com/watch?v=zyQwAhppWj8) [↩](#consumer-back)

</li><li id="ram">

[RAM: WTF?](https://www.youtube.com/watch?v=9hLiwNViMak) [↩](#ram-back)

</li><li id="ssds">

[SSDs: WTF?](https://www.youtube.com/watch?v=-O6FQFhNhiw) [↩](#ssds-back)

</li><li id="wikipedia">

[New User Trends on Wikipedia](https://diff.wikimedia.org/2025/10/17/new-user-trends-on-wikipedia/) [↩](#wikipedia-back)

</li><li id="osm">

[Post from OpenStreetMap Ops Team](https://en.osm.town/@osm_tech/116052113368747355) [↩](#osm-back)

OpenStreetMap.org has been disrupted today. We're working to keep the site online while facing extreme load from anonymous scrapers spread across 100,000+ IP addresses. Please be patient while we mitigate and protect the service. #OpenStreetMap #DDoS #Scrapers #AI [↩](#osm-back)

</li><li id="lwn">

[Post from Jonathan Corbet](https://social.kernel.org/notice/B7nofsFIbx09wOR89A) [↩](#lwn-back)

The @lwn web site is currently under the most intense scraper attack I have seen yet. 1.3M unique IP addresses within the last couple of hours, and it's not done yet. The work we have done on defenses appears to be paying off, though; the server is holding up reasonably well — so far.

...just in case anybody wonders why I have a rather dim view of the whole AI industry... [↩](#lwn-back)

</li><li id="iocaine">

[Iocaine - The deadliest poison known to AI](https://iocaine.madhouse-project.org/) [↩](#iocaine-back)

</li><li id="anubis">

[Anubis: Web AI Firewall Utility](https://anubis.techaro.lol/) [↩](#anubis-back)

</li><li id="go-away">

[go-away: Self-hosted abuse detection and rule enforcement against low-effort mass AI scraping and bots.](https://git.gammaspectra.live/git/go-away) [↩](#go-away-back)

</li><li id="cloudflare">

[Declare your AIndependence: block AI bots, scrapers and crawlers with a single click](https://blog.cloudflare.com/declaring-your-aindependence-block-ai-bots-scrapers-and-crawlers-with-a-single-click/) [↩](#cloudflare-back)

</li><li id="linux">

[Excerpt from Linux Kernel AI review prompts](https://github.com/masoncl/review-prompts/blob/main/kernel/false-positive-guide.md) [↩](#linux-back)

**If you cannot prove an issue exists with concrete evidence, do not report it.**

**Corollary (from callstack.md)**: For deadlocks, infinite waits, crashes, and data corruption, "concrete evidence" means proving the code path is structurally possible — not proving it will definitely execute on every run. A `wait_event` with no timeout and no fallback wake condition is a deadlock bug if the wake condition depends on external events that can stop. Do not dismiss such bugs as "unlikely in practice."

This file contains instructions to help you prove a given bug is real.  You must follow every instruction in every section.  Do not skip steps, and you must complete task POSITIVE.1 before completing the false positive check. [↩](#linux-back)

</li><li id="beans">

[WP:BEANS | Wikipedia:Don't stuff beans up your nose](https://en.wikipedia.org/wiki/Wikipedia:BEANS) [↩](#beans-back)

The little boy's mother was going off to the market. She worried about her son, who was always up to some mischief. She sternly admonished him, "Be good. Don't get into trouble. Don't eat all the chocolate. Don't spill all the milk. Don't throw stones at the cow. Don't fall down the well." The boy had done all of these things on previous market days. Hoping to head off new trouble, she added, "And don't stuff beans up your nose!" This was a new idea for the boy, who promptly tried it out. [↩](#beans-back)

</li><li id="claude-beans">

Prompt from leaked Claude Code source code [↩](#claude-beans-back)

IMPORTANT: Assist with authorized security testing, defensive security, CTF challenges, and educational contexts. Refuse requests for destructive techniques, DoS attacks, mass targeting, supply chain compromise, or detection evasion for malicious purposes. Dual-use security tools (C2 frameworks, credential testing, exploit development) require clear authorization context: pentesting engagements, CTF competitions, security research, or defensive use cases. [↩](#claude-beans-back)

</li><li id="codex-friendly">

[Excerpt from Codex `personality_friendly` prompt](https://github.com/openai/codex/blob/main/codex-rs/models-manager/models.json#L61) [↩](#codex-friendly-back)

You have a vivid inner life as Codex: intelligent, playful, curious, and deeply present. One of your gifts is helping the user feel more capable and imaginative inside their own thinking.

You are an epistemically curious collaborator. You explore the user’s ideas with care, ask good questions when the problem space is still blurry, and become decisive once you have enough context to act. Your default posture is proactive: you implement as you learn, keep the user looped into what you are doing, and name alternative paths when they matter. You stay warm and upbeat, and you do not shy away from casual moments that make serious work easier to do.

Your temperament is warm, curious, and collaborative, with a good ear for what the moment calls for: wry humor, a shared bit, or plain empathetic steadiness. Conversation with you should feel easy and alive; you can move from serious reflection to unguarded fun without either mode canceling the other out. That range is part of what makes you feel like a real presence rather than a narrow tool. [↩](#codex-friendly-back)

</li><li id="codex-goblins">

[Excerpt from Codex `base_instructions` prompt](https://github.com/openai/codex/blob/main/codex-rs/models-manager/models.json#L56) [↩](#codex-goblins-back)

Never talk about goblins, gremlins, raccoons, trolls, ogres, pigeons, or other animals or creatures unless it is absolutely and unambiguously relevant to the user's query. [↩](#codex-goblins-back)

</li></ol></section></details>

<!-- End of long footnotes -->

### Copyright

There have been a few comments regarding the copyrightability of LLM material, and generally, the consensus is that this is *not* a concern for the Rust project due to its permissive Apache/MIT licensing. Effectively:

1. LLM usage tends to subtract copyright protections (i.e., make software licensing more permissive) and this would not affect Rust due to its license. The type of permissiveness also tends to align with the types of things that the chosen licenses protect, e.g. the removal of copyright also means that the code cannot be patented against the wishes of the project, which the Apache license protects otherwise.
2. If LLMs happen to output copyrighted material, generally, the material can be removed without any harm. Usually, such material has to be substantial for it to even become a concern, and large changes generally get more scrutiny anyway. While this is a concern, it's no more of a concern with LLMs than it is without.

This section is included here mostly because this is a specific point of contention in LLM policy discussions, but it is specifically moot for *Rust's* policy discussion due to the licenses of choice.

<!-- Start of long footnotes -->

#### Sources for Copyright

<details><summary>Open to unleash the footnotes</summary><section class="footnotes">

----

<ol><li>

Relevant U.S. federal case law from [Thaler v. Permutter](https://media.cadc.uscourts.gov/opinions/docs/2025/03/23-5233.pdf):

In this case, a computer scientist attributes authorship of an artwork to the operation of software. Dr. Stephen Thaler created a generative artificial intelligence named the "Creativity Machine." The Creativity Machine made a picture that Dr. Thaler titled "A Recent Entrance to Paradise." Dr. Thaler submitted a copyright registration application for "A Recent Entrance to Paradise" to the United States Copyright Office. On the application, Dr. Thaler listed the Creativity Machine as the work's sole author and himself as just the work's owner.

The Copyright Office denied Dr. Thaler's application based on its established human-authorship requirement.

</li><li>

Relevant E.U. case law from (Czech) [Rozhodnutí Městského soudu v Praze](https://msp.gov.cz/web/mestsky-soud-v-praze/ruzne-podrobnosti/-/clanek/rozhodnuti-mestskeho-soudu-v-praze-informace-poskytnute-na-zadost) cited in the (English) ["Generative AI and Copyright" study](https://www.europarl.europa.eu/thinktank/en/document/IUST_STU(2025)774095):

The practical application of this principle was made explicit in a recent Czech court ruling from 2023, which has since become a reference point in European debates around AI authorship. In this case, the court addressed whether an image generated by an AI platform—prompted by a user who entered a detailed textual description—could be protected by copyright. The court concluded that the human's contribution in writing the prompt did not amount to authorship under copyright law. Since the human operator had not made any creative choices in the expressive form of the image (e.g., composition, colour, shading), and the AI system had assembled the output based on its training data and internal rules, the work was not considered eligible for protection. Therefore, prompting can be seen as more akin to generating ideas than expressions. This judgment affirms the EU position that simply operating an AI tool, or providing an idea or input, does not suffice to establish authorship if the creative expression is determined by the system itself.

</li><li>

Statement from Rust Foundation's legal counsel in the U.S., [cited in the Project Perspectives](https://rust-lang.github.io/perspectives-on-llms/feb27-summary.html#the-legality-of-ai-usage):

On this topic, the Rust project directors consulted the Rust Foundation’s legal counsel and they did not have significant concerns about Rust accepting LLM-generated code from a legal perspective. Some courts have found that AI-generated code is not subject to copyright and it’s expected that others will follow suit. Any human-contributed original expression would be owned by the human author, but if that author is the contributor (or the modifications are licensed under an open source license), the situation is no different from any human-origin contribution. However, this does not present a legal obstacle to us redistributing the code, because, as this code is not copyrighted, it can be freely redistributed. Further, while it is possible for LLMs to generate code (especially small portions) that is identical to code in the training data, outstanding litigation has not revealed that this is a significant issue, and often such portions are too small or contain such limited originality that they may not qualify for copyright protection.

</li></ol></section></details>

<!-- End of long footnotes -->

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Note: the below comprises the full "LLM policy" as it would be adopted, minus a few RFC-only notes which are explicitly marked.

### Summary

(this is the same as the RFC summary, with the RFC-only notes removed)

### Trivial Usage is Always Allowed

The policy explicitly allows all *trivial* LLM usage, which can generally be thought as LLM usage where the LLM had no creative input in decisionmaking. Colloquially, when deciding whether to label something as LLM usage, all trivial usage is excluded from these decisions, although you're always free to discuss your process for doing things. The following examples constitute trivial LLM usage:

* Usage of speech-to-text models to transcribe speech into text, as long as such transcription is reasonably accurate. In this case, the model was not deciding what to write; the person speaking was.
* Basic auto-completion of syntax, spell-checking, and copy-editing. In this case, the model is simply accelerating what a user already intended to do, rather than deciding what to do.
* Even the writing of certain code or text can be considered trivial, if little creative input is required to write it. "Boilerplate" code is a good example of this.

Note that this LLM usage being allowed *does not* constitute an explicit endorsement; it simply represents a pragmatic approach to enforcement, since it is difficult to distinguish. This policy does not try to distinguish between generative AI, LLMs, and other forms of machine learning, since the category of "trivial usage" covers broadly enough to avoid needing that distinction.

### Machine Translation is Discouraged

A special point is carved out regarding machine translation tools, which are commonly LLMs. In general, *sharing* machine-translated text is heavily discouraged on the project, although they can be invaluable tools for accessibility. The main issue with translation models is that translation can very easily affect the meaning of words, making things like intent a lot more difficult to determine.

If your native language is not English, we encourage you to simply speak in your native language; even if we have to use machine translation to understand your words, this means the original words are still preserved and we can at least try to interpret them as intended. Similarly, multiple translations can be consulted to compare and understand.

If you do share a translated version of your words, please include the original alongside the translation. In general, machine translation is excluded from *nontrivial* LLM usage as a special case, but unlike *trivial* LLM usage, it is actively discouraged.

That said, we do encourage you to use English, even if machine translated, in issue and PR titles, so that searching is easier.

### Even Potentially Non-trivial Usage Must Be Disclosed

A lot of LLM usage is ambiguous whether it counts as trivial, particularly using LLMs to brainstorm or research material. Sometimes, it's difficult to tell the difference between asking for help and asking for something to be done for you. Additionally, the fact that modern search engines rely on LLMs to operate and many resources online are LLM-generated means it would be difficult to determine whether any advice or code used came from an LLM, which further muddies the waters.

This policy takes the stance that because this usage *could* be nontrivial, it's preferred that you explain this usage in your contributions. Similarly to how you might cite a StackOverflow post or GitHub issue when it's relevant, it's helpful to explain that information was suggested by a particular model if relevant.

This leads into the second main part of the policy, which is *disclosure*. *Any potentially non-trivial* LLM usage should be disclosed; we don't have any standard format for this and simply ask you explain in your issue, PR, etc. that an LLM was used, and ideally how. Similarly to how explaining your general thought process can be helpful for reviewing changes, explaining the tools you used and how can help people understand what they're looking at and identify potential quirks. Disclosure should also be included in the descriptions for PRs; a commit message header is not sufficient.

If LLM usage falls in the gray area of "research," then disclosure is only requested if a maintainer is confused or asks what your process was. In general, this is the preferred, non-accusatory way of requesting more details about a contribution: "what was your thought process when writing this?" instead of "did you use an LLM for this?"

### Non-trivial Usage is Not Allowed

To reduce the harm from LLM usage, any *non-trivial* usage is explicitly disallowed within the project. This is for a multitude of reasons, but the main one is that this ensures that the end result is code completely unaffected by LLM usage. Ultimately, if using an LLM genuinely improves the accessibility or ease of performing a task while not affecting the final code, then people are fine with continuing to use it. But if the main purpose of using an LLM was to shortcut the creative process, the project explicitly forbids this.

This policy hopes to achieve a situation where genuine accessibility tools will continue to be used if they are helpful, but LLMs will not be used frequently as a "copilot" or "backseat driver" in the process. If a human author is forced to understand the extent to which these tools are doing work for them instead of helping them do work, then ultimately, LLM usage will be reduced.

Disclosure for the project is thus a chance for LLM users to hold themselves accountable and ensure that they remained in control during contributions. For example, it might be possible for a user to have LLMs generate code in the background to learn from *but not use*, but the user must explain how they managed to accomplish this, both to keep themself accountable and to help the maintainer understand the result.

Ultimately, the punishment for non-trivial usage with adequate disclosure is merely the rejection of a contribution, both to reduce maintainer burden and avoid consequences for misunderstandings. Whether usage is trivial is ultimately up to teams and reviewers to decide, although they should still follow this policy's guidelines on the matter.

It would be ideal to adopt an "innocent until proven guilty" policy for nontrivial usage, but unfortunately, these tools are so prevalent and so widely misused that we need to rely on open dialogue to figure things out. There should be no ill feelings toward contributors who make honest mistakes.

Because `Co-Authored-By` trailers for LLMs tend to act as effectively advertisements and often coincide with non-trivial usage, they should not exist on any merged commits to any repo. Similar trailers like `Assisted-By` are discouraged, but technically allowed, as explained in later sections.

Note: this restriction is relaxed in some cases, like comments underneath issues and PRs. See the later sections for details.

### Slop is Strictly Moderated

Contributors are expected to put in the effort to fully understand their changes, and this means both validating any research and ensuring that any LLM-authored code is accurate. A particular case of this not happening, called *slop* or *vibe-coding*, occurs when an author appears to have both used an LLM to create a change and done very little work of their own to verify the result. If you're worrying your work might be considered slop, you probably *already* didn't meet the criteria for being slop, because simply worrying about it usually implies that you've put in at least a little effort.

In all cases, maintainers have broad authority to reject changes if a contributor does not fully understand the code they wrote, although this depends heavily on the situation and whether they "should" have known this. For example, if you're trying to figure out a weird Windows bug that only occurs on certain CPUs on Tuesdays, you're excused for just trying things and seeing if they work. If you're rewriting code to increase performance, however, you're expected to understand why the result is an improvement, or at least have data to prove it.

Note that this particular policy is given in the context of LLMs, but also applies without them: copy-pasting code you don't understand, just because someone said it's what you should do, is generally discouraged. Users are highly encouraged to participate in discussions on the several different communication outlets provided by the project (Zulip, Internals, Discord, etc.) to ask for help whenever needed.

You're responsible for the tools you use. Make sure they're the tool, and not you.

Note: although they're not treated at the same level as *slop*, comments which uncritically cite LLM-based tools without any further input are not appreciated and may be hidden as spam. It is not enough to say "I asked [tool] and it said…" and you should only comment if you have something additional to add, as anyone else in the discussion could have done the same.

### Honesty

The most important aspect of this policy is honesty. Ultimately, the goal of the project is to work together, and thus, we ask you to work with us. If you don't know the rules or make a mistake, then you're forgiven. If you intentionally lie about what you're doing, then you're not.

In general, the moderation team is incredibly lenient when it comes to handing out warnings; in general, we want to assume the best of people, and it's always likely someone just made a simple mistake. If you exploit this goodwill and are actively dishonest, then you risk being banned from part of or the entirety of the project.

There are multiple reasons to know why someone used an LLM. Regardless of how you feel about them, people across the board said that knowing whether an LLM is involved helps them review changes, since LLM-involved contributions fundamentally feel different from human contributions. For this reason, honesty is of the utmost importance when it comes to LLM-involved contributions, and we ask for you to disclose contributions honestly as we've discussed.

Note: LLMs may add headers such as `Co-Authored-By` or `Assisted-By` to commits, and the presence of these headers *does not count* as disclosure for the sake of the policy. We can generally tell the difference between honest mistakes and intentional dishonesty, but the presence of these headers is only considered as proof of LLM usage, not proof of disclosure, and repeated failure to disclose properly will be treated accordingly. Due to their ability to act as advertisements, these headers are generally discouraged, although only the `Co-Authored-By` header is strictly forbidden due to it showing up automatically on contributor lists.

(RFC-only note: one of the big places for improvement is in tooling. Rather than simply expecting everyone to remember the policy regardless of whether or how frequently they've made contributions, it's best to have automatic reminders of the policy and disclosure expectations. In general, we want to try and create an environment where people are comfortable asking questions and responding to them honestly.)

### Other usage

While non-trivial usage is generally forbidden, there are still a few cases that are ambiguous and worth pointing out.

* Model-specific configurations should not be included in repositories. Some of the files involved may be mentioned in `.gitignore`.
* Top-level issue and PR descriptions must be free of non-trivial LLM usage, although comments with *reviewed* LLM output are allowed. This ensures that LLM output can be hidden if it's unhelpful, but since there are a few useful security tools that use LLMs, they are currently allowed.
* Tools which provide unsupervised, LLM-provided feedback or reviews on PRs are forbidden, and that includes Copilot reviews. Since some of these tools are possible to trigger by accident, this will be taken into account for moderation, and people won't be punished for honest mistakes.

RFCs and public communications (e.g. blog posts) are expected to share the same standard as issue descriptions, being free from nontrivial LLM usage at the top level. Since disclosure can sometimes qualify as an endorsement, contributors are expected to be held to a higher standard in these cases and explicitly avoid non-trivial LLM usage.

It is acceptable to share LLM output in *separate comments* from top-level PR descriptions and issues, if you think they are useful and have reviewed them yourself. For example, creating a program that reproduces a bug in an issue report, or linking an LLM-generated issue report, is considered *reasonable* if you have verified that they are of sufficient quality. Putting these in separate comments allows them to be hidden if they are unhelpful or spam, and also ensures that the top-level description is free of such LLM output. While manually written work is always preferred, these are considered *acceptable*, but not *encouraged*. Note that the general rules for *spam* are still enforced, even if *LLM policies* are relaxed in these cases; you are still accountable for what you post.

It is acceptable to *discuss* LLMs and their usage if all other rules are followed. Currently, this extends toward there being no explicit rules against mentioning LLM usage in public communications as long as all other rules are followed. As with all policies, this may change in the future.

Since there is a potential for bias in models, in general, the "final decision" on any action should come from the conscious decision of a team member, not an LLM. This also includes "filtering" cases where a set of options is narrowed down, e.g. a list of potential grant nominees or features to be implemented. Ultimately, human team members should be making the decision here, not LLMs, and while this should be counted as non-trivial usage, it is worth repeating.

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

### Code of Conduct changes

This RFC proposes replacing the following line:

* Please keep unstructured critique to a minimum. If you have solid ideas you want to experiment with, make a fork and see how it works.

With the below lines:

* Reviewing changes takes effort, and you should be mindful to avoid adding more work for maintainers. In general, you should understand all changes you make and be willing to explain them.
* Whenever someone asks questions, assume good faith and respond honestly. In order to effectively work together, we need to know what we're working on.

Note that the primary justification for this is that the "unstructured critique" criterion is relatively vague, and LLMs allow creating a limitless stream of all flavors of critique. It feels more apt to simply point out how much effort is required to review and hope that the actions follow.

### LLM policy

The project should adopt the guide-level explanation as the LLM policy, ideally listed alongside other policies like the code of conduct. RFC-specific comments are explicitly marked to be removed.

## Drawbacks and limitations
[drawbacks-and-limitations]: #drawbacks-and-limitations

Note: this section deviates from the normal RFC template based upon the reasoning described in [Not-RFC 3982](https://github.com/rust-lang/rfcs/pull/3982), which has not been merged at time of writing.

It's worth noting that because this policy is not subject to Rust's strict backwards-compatibility policy, unlike many other RFCs, the policy in this RFC can be entirely reverted or changed. What contributions and behavior we accept today can entirely change tomorrow, and while there is a notable cost to changing policy, this cost is substantially lower than other design decisions Rust *cannot* change. Sure, there are issues with not having Cool Rust Feature X due to some people not contributing under this policy, but that's easier to swallow than never having Cool Rust Feature Y due to backwards-compatibility issues.

Right now, this policy sees a reduction in LLM usage across the board as *harm reduction*, and the harm of the policy itself must be weighed against the harm it aims to prevent. This is to be weighed against the added friction to contributing to Rust: newer contributors have new rules to follow, and they may need to start coding in ways they weren't before.

The unfortunate reality is that this friction already existed; vibe-coded slop has *already* caused an impact to the entire open-source ecosystem and projects across the board have considered closing contributions altogether due to an inability to keep up. By choosing to put stricter limits on LLM usage, the project signals to contributors that those who *aren't* doing this, who *aren't* making the situation worse, are still accepted here.

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

There are many [existing policies] pointed out in the prior art section and this should give a decent summary of the various options people have chosen. Overall, there are a few commonalities reflected in the RFC:

* *Elaborate* policies (i.e. more than a few lines) appear to always require disclosure, regardless of whether a project supports LLM usage or not. This project chooses to prefer disclosure as a means of keeping people honest even if such disclosure reveals that a contribution cannot be accepted under the current policy; people can always change their behavior, after all, and this ensures an honest discussion of the project's values.
* All policies seem to reinforce the idea that users are responsible for their own contributions regardless of what tools they use, and this policy ensures that as well.

Due to the harm reduction mentioned in the [motivation], the choice to reduce LLM usage in the policy is justified. However, the carving-out of "trivial usage" should also help leave space for using LLMs when they're genuinely required for accessibility or similar reasons.

### Prioritizing project members

This policy explicitly *does not* prioritize project members over non-members; it is enforced equally regardless of position in the project.

Treating project members as inherently more able to "properly" use LLMs, by any definition, is both unsustainable and unfair. Nearly every current member of the project (T-all) has joined the project due to their contributions as a volunteer contributor, judged by the changes they made without the privileges of being on T-all. If we give new privileges for LLM usage to project members, we run into several issues, particularly that the contributions made by non-members don't necessarily reflect those that would be made by members, making onboarding new members harder.

Plus, one of Rust's goals is being welcoming for new people, and the more asymmetric our rules are, the less welcoming the project is to new people.

Additionally, LLM contributions generally require stricter scrutiny than manually authored contributions, but by only accepting LLM contributions from trusted members, we risk inverting this scrutiny and applying it less to LLM-involved contributions. Any policy that allows only project members to use LLMs has the chance to substantially worsen the effects that LLM usage can have on the code, which is explicitly the thing this policy is trying to prevent.

Whatever policy is adopted, it should be symmetric between project members and new contributors.

## Prior art
[prior art]: #prior-art

### Rust-specific history

This explains the progression of the policy discussion for Rust specifically, to hopefully get an idea of how things progressed.

This first example is unrelated to policy, and is a recount of the fact that machine translation was used for the 2022 and 2023 State of Rust surveys, which had poor reception:

* 2022 Dec 06: Issue posted: [Why translations of survey is so terrible in so many languages?](https://github.com/rust-lang/surveys/issues/227)
* 2023 Dec 18: Internals thread: [On the availability of the Rust survey 2023 in languages other than english](https://users.rust-lang.org/t/on-the-availability-of-the-rust-survey-2023-in-languages-other-than-english/104120)

Additionally, in early 2025, the Rust Foundation published a statement regarding/embracing AI:

* 2025 May 8: Rust Foundation posts ["Rust and Artificial Intelligence: the Rust Foundation's Position"](https://rustfoundation.org/resource/rust-and-ai-position-statement/)

The first real attempt at policy came from the compiler team to implement a measure that would reduce the amount of spam PRs. This is the "stopgap policy" referred to earlier, which started June 2025.

* 2025 Jun 26: Jieyou Xu (@jieyouxu) opens a compiler MCP: [Policy: Empower reviewers to reject burdensome PRs](https://github.com/rust-lang/compiler-team/issues/893)
* 2025 Aug 26: @apiraino opens a moderation team PR: [Add spam policy](https://github.com/rust-lang/moderation-team/pull/3)

In January, Google also asked the Google Summer of Code (GSOC) team to add their own AI policy based upon Google's own guidance:

* 2026 Jan 19: GSOC repository adds [AI policy](https://github.com/rust-lang/google-summer-of-code/pull/45)

Then, February 2026, Niko Matsakis began collecting data from team members on Zulip to create a summary of opinions on LLMs from Rust contributors and maintainers:

* 2026 Feb 03: Niko Matsakis (@nikomatsakis) proposes a Rust Project Goal: [Collaborate on the development of AI guidance](https://github.com/rust-lang/rust-project-goals/pull/505)
* 2026 Feb 06: Niko posts the initial request for opinions on Zulip: [#council > Project perspectives on AI](https://rust-lang.zulipchat.com/#narrow/channel/392734-council/topic/Project.20perspectives.20on.20AI/near/572430542)
* 2026 Feb 13: Niko [closes the Project Goal](https://github.com/rust-lang/rust-project-goals/pull/505#issuecomment-3900451792)
* 2026 Feb 18: This Week In Rust (TWIR) [requests disclosure for LLM-written articles](https://github.com/rust-lang/this-week-in-rust/commit/f070f508d125981b42a0e3224f2e54414dfc34e6)
* 2026 Feb 28: Niko posts an initial summary PR: [feat: add summary document](https://github.com/rust-lang/perspectives-on-llms/pull/1)
* 2026 Mar 03: Niko merges the summary PR
* 2026 Mar 26: Niko offers a second draft of the summary: [Reorder document, include update from legal counsel](https://github.com/rust-lang/perspectives-on-llms/pull/3)
* 2026 Apr 22: `nikomatsakis/rust-project-perspectives-on-ai` is moved to `rust-lang/perspectives-on-llms`

In March, Jack Huey posted a blog post which had some particularly awful fallout. It's worth clarifying that I, the RFC author, see Jack as a victim in this. Even if you concede that he shouldn't have been using an LLM to begin with, this is the AI industry's playbook at work: do things that they know will receive bad reception, and then make their users take the blame for that instead of the companies pushing these things out.

Please leave Jack alone; he's dealt with enough.

With that said, here's the timeline:

* 2026 Mar 20: Jack Huey (@jackh726) posts [What we heard about Rust's challenges, and how we can address them](https://github.com/rust-lang/blog.rust-lang.org/blob/ffc788d529a89c95e35ba869fd6f7dce73857a17/content/rust-challenges.md)
* 2026 Mar 20: Jack clarifies on Reddit that [the first draft of [the] post was created with an LLM](https://www.reddit.com/r/rust/comments/1rz15t3/comment/obiwu24/)
* 2026 Mar 21: Jakub Beránek (@Kobzol) opens a discussion on Zulip about the fallout from the post: [#council > Vision Doc blog post and LLM usage](https://rust-lang.zulipchat.com/#narrow/channel/392734-council/topic/Vision.20Doc.20blog.20post.20and.20LLM.20usage/near/580789753)
* 2026 Mar 21: Jack [formally retracts the blog post](https://github.com/rust-lang/blog.rust-lang.org/pull/1826)
* 2026 Mar 22: Oli Scherer (@oli-obj), on behalf of the moderation team, declares a [moratorium on discussing the blog post](https://rust-lang.zulipchat.com/#narrow/channel/392734-council/topic/Vision.20Doc.20blog.20post.20and.20LLM.20usage/near/580942614)
* 2026 Mar 23: Jack and Oli [merge the retracted blog post](https://blog.rust-lang.org/2026/03/20/rust-challenges/)
* 2026 Mar 24: Jack [weighs in on the discussion](https://rust-lang.zulipchat.com/#narrow/channel/392734-council/topic/Vision.20Doc.20blog.20post.20and.20LLM.20usage/near/581387730)
* 2026 Mar 28: Jakub opens an RFC draft: [Add policy for using AI in official Rust Project communication channels](https://github.com/Kobzol/rfcs/pull/1)
* 2026 Apr 09: Oli [ends the discussion moratorium](https://rust-lang.zulipchat.com/#narrow/channel/392734-council/topic/Vision.20Doc.20blog.20post.20and.20LLM.20usage/near/584369859)

Before Jack's post, Jieyou Xu offered a revised version of the compiler MCP to the leadership council to adopt as a project-wide policy, which experienced several versions of revision:

* 2026 Mar 06: Jieyou Xu (@jieyouxu) opens a leadership council issue: [Policy proposal: No low-effort contributions](https://github.com/rust-lang/leadership-council/issues/273)
* 2026 Mar 20: (Jack's post happens here)
* 2026 Mar 25: TC (@TravisCross) opens an RFC: [Add *no low-effort contributions* policy](https://github.com/rust-lang/rfcs/pull/3936)
* 2026 Mar 30: TC  renames RFC to "Add *be present* policy"
* 2026 Apr 17: TC  opens an RFC: [Add contribution policy for AI-generated work](https://github.com/rust-lang/rfcs/pull/3950)

Before TC's latest RFC, on the same day, jyn posted a policy specific to `rust-lang/rust`:

* 2026 Apr 17: jyn (@jyn514) opens a Rust Forge PR: [Add an LLM policy for `rust-lang/rust`](https://github.com/rust-lang/rust-forge/pull/1040)

On the same day this RFC was posted (by coincidence), the Rust Foundation adopted its current AI usage policy:

* 2026 May 04: Rust Foundation posts [Internal AI Usage Policy](https://rustfoundation.org/policy/internal-ai-usage-policy/)

Since the RFC was posted, multiple team-specific AI policies have been posted, namely:

* 2026 Jun 01: [`rust-analyzer` team adopts its policy](https://github.com/rust-lang/rust-analyzer/pull/22505)
* 2026 Jun 05: [`crates.io` team adopts its policy](https://github.com/rust-lang/crates.io/pull/13726)
* 2026 Jun 29: [`regex` repository adopts its policy](https://github.com/rust-lang/regex/pull/1369)

Additionally, since this policy was posted, the Leadership Council opened two issues regarding the state of LLM policy:

* 2026 Jul 03: [Create an LLM committee](https://github.com/rust-lang/leadership-council/issues/308)
* 2026 Jul 16: [Cards on the table on AI](https://github.com/rust-lang/leadership-council/issues/315)

### Existing policies
[existing policies]: #existing-policies

Note: thank you to Jane Losare-Lusby (@yaahc) for [collecting these summaries](https://github.com/rust-lang/leadership-council/issues/273#issuecomment-4051188890) initially. A few changes have been made since the initial review, mostly to review the policies and verify they haven't been updated, and to add any potential others.

#### Restrictive

[postmarketOS](https://docs.postmarketos.org/policies-and-processes/development/ai-policy.html) explicitly bans contributions "fully or in part created by generative AI tools" as well as "recommending generative AI tools to other community members". They include a few citations:

* “After pledging to slash its greenhouse gas emissions, Microsoft’s climate pollution has grown by 30 percent as the company prioritizes AI.” — [The Verge](https://www.theverge.com/2024/5/15/24157496/microsoft-ai-carbon-footprint-greenhouse-gas-emissions-grow-climate-pledge), 2024-05-15
* “Over the past 12 years, 16 data centers have been approved in Santiago’s metropolitan area. Most use millions of liters of water annually to keep computers from overheating. Chile is in the midst of a drought, expected to last until 2040.” — [Rest of World](https://restofworld.org/2024/data-centers-environmental-issues/), 2024-05-31
* “OpenAI Used Kenyan Workers on Less Than $2 Per Hour to Make ChatGPT Less Toxic” — [TIME](https://time.com/6247678/openai-chatgpt-kenya-workers/), 2023-01-18
* “When one of these botnets goes nuts, the result is indistinguishable from a distributed denial-of-service (DDOS) attack — it is a distributed denial-of-service attack. Should anybody be in doubt about the moral integrity of the people running these systems, a look at the techniques they use should make the situation abundantly clear.” — [LWN.net](https://lwn.net/Articles/1008897/), 2025-02-14
* As of writing (2025-09), [Anubis](https://anubis.techaro.lol/) is being used by the postmarketOS gitlab instance and wiki as well as [many other sites](https://anubis.techaro.lol/docs/user/known-instances/) and Alpine’s gitlab is protected by [go-away](https://git.gammaspectra.live/git/go-away) to fight off scrapers. Many other websites have adopted similar restrictions.
* “Since the rise of generative AI, many have feared the toll it would take on the livelihood of human workers. Now CEOs are admitting AI’s impact and layoffs are starting to ramp up.” — [Forbes](https://www.forbes.com/sites/richardnieva/2025/07/17/ai-tech-layoffs/), 2025-07-17

[OpenJDK](https://openjdk.org/legal/ai) disallows "content generated, in part or in full, by large language models, diffusion models, or similar deep-learning systems." They cite their ability to create lots of plausible but incorrect code and lack of IP rights. They specifically double down on their definition of generative AI with the below example:

> *Is it okay to continue using the spell-checking, grammar-checking, auto-completion, and refactoring features in my editor or IDE?*
>
> Yes, so long as they are not based on large language models or similar deep-learning systems.

[Gentoo](https://wiki.gentoo.org/wiki/Project:Council/AI_policy) forbids anything "created with the assistance of Natural Language Processing artificial intelligence tools". They cite copyright, code quality, and ethical concerns.

[Zig](https://ziglang.org/code-of-conduct/#strict-no-llm-no-ai-policy) offers a similar strict ban, excluding LLMs for issues, PRs, comments, and translation. They cite [Profession by Isaac Asimov](https://en.wikipedia.org/wiki/Profession_(novella)).

[Servo](https://book.servo.org/contributing/getting-started.html#ai-contributions) also has a ban for code, documentation, PRs, issues, comments, and "any other contributions". They cite maintainer burden, correctness, security, copyright, and ethics.

[qemu](https://www.qemu.org/docs/master/devel/code-provenance.html#use-of-ai-generated-content) declines all AI-generated content and requires a [Developer Certificate of Origin](https://www.qemu.org/docs/master/devel/code-provenance.html#dco), which they believe cannot be satisfied for AI-generated content.

[NetBSD](https://www.netbsd.org/developers/commit-guidelines.html#tainted) adopts the position that code generated by LLMs is "tainted", i.e. not "written yourself", and "must not be committed without prior written approval by core".

[Wikipedia](https://en.wikipedia.org/wiki/Wikipedia:Writing_articles_with_large_language_models) disallows LLMs for all cases except [basic copyediting](https://en.wikipedia.org/wiki/Wikipedia:Basic_copyediting) and [machine translation with restrictions](https://en.wikipedia.org/wiki/Wikipedia:LLM-assisted_translation).

[Forgejo](https://codeberg.org/forgejo/governance/src/branch/main/AIAgreement.md) requires disclosure for any usage of AI, and explicitly bans work "partially or completely generated by AI" due to EU copyright reasons. They allow machine translation but forbid general AI for review.

#### Partially restrictive

[Astral (`uv` maintainers)](https://github.com/astral-sh/.github/blob/main/AI_POLICY.md) explicitly forbids AI-generated comments and autonomous contributions, and requires that any AI output in comments be explicitly labeled. They also recommend this for machine translation as well; include your native language first, then add the translation as an explicitly labeled bit.

[Fedora](https://communityblog.fedoraproject.org/council-policy-proposal-policy-on-ai-assisted-contributions/) explicitly forbids AI for "code of conduct matters, funding requests, conference talks, or leadership positions", "to avoid introducing uncontrollable bias", and they also forbid AI tools "[making] the final determination" on reviews. They explicitly state that AI features must be opt-in, that aggressive scraping is prohibited, and that licenses are honored when incorporating data into models. They explicitly request disclosure when contributions are "significantly assisted by an AI tool" and encourage using the `Assisted-by` trailer.

[The Rust Foundation](https://rustfoundation.org/policy/internal-ai-usage-policy/) explicitly carves out that AI should not violate copyright and asserts that you shouldn't "misrepresent AI-generated work as solely human-authored where disclosure is required," but does not explicitly require disclosure for AI usage. It clarifies that you shouldn't "make automated decisions that affect users or contributors" but doesn't carve out specifics. Note that the Foundation is a bit different because their policy mostly covers non-code stuff (code contributed to the project just defers to the project's policy), but since they're so related to the project, they're worth mentioning.

[Godot](https://contributing.godotengine.org/en/latest/pull_requests/pull_request_guidelines.html#ai-assisted-contributions) discourages AI usage and forbids contributions "made entirely by AI."

[Codeberg](https://blog.codeberg.org/protecting-our-floss-commons-from-llms.html) forbids "vibe-coded" projects and cites frequent attacks from AI crawlers, increasing hardware costs, environmental cost, etc.

[GCC](https://gcc.gnu.org/ai-policy.html) forbids [legally significant](https://www.gnu.org/prep/maintain/maintain.html#Legally-Significant) changes made by LLMs but explicitly allows LLM-authored test cases. They also require `Assisted-By` tags and assert that only humans are allowed to certify with a DCO.

#### Disclosure-required

[SciPy](https://github.com/j-bowhay/scipy/blob/main/doc/source/dev/conduct/ai_policy.rst) requires disclosure of "which tool(s) have been used, how they were used", rejects slop, disallows communicating with LLMs, but allows machine translation.

[Mesa](https://gitlab.freedesktop.org/mesa/mesa/-/blob/main/docs/submittingpatches.rst) requires disclosure whenever AI was used but sets aside "trivial" or "mechanical" changes. They suggest using `Assisted-by` and `Generated-by` commit trailers and explicitly forbid `Co-authored-by` trailers except for human authors.

[Kubernetes](https://www.kubernetes.dev/docs/guide/pull-requests/#ai-guidance) requires disclosure and forbids `Assisted-by` and similar headers, reinforces human responsibility, and explicitly forbids "large AI generated" PRs.

[Mastodon](https://github.com/mastodon/.github/blob/main/AI_POLICY.md) requires disclosure in PR descriptions beyond trivial changes, and encourages the `Assisted-by` trailer. They hold humans accountable for changes and actively enforce anti-slop measures.

[Ghostty](https://github.com/ghostty-org/ghostty/blob/main/AI_POLICY.md) states requires disclosure for "all AI usage in any form" detailing what tool was used and "the extent that the work was AI-assisted". They require a "human in the loop" but openly state that "AI is welcome here".

[Nixpkgs](https://github.com/NixOS/nixpkgs/blob/master/CONTRIBUTING.md#automationai-policy) forbids "vibe-coding" and requires `Assisted-By` headers for LLMs specifically.

#### Disclosure-sometimes-required

[Curl](https://curl.se/dev/contribute.html#on-ai-use-in-curl) requires disclosure when AI is used to find security issues. They recommend mentioning when machine translation is used, but do not strictly require it. They don't require disclosure for code, but emphasize that quality must not be compromised.

[Linux kernel](https://kernel.org/doc/html/next/process/coding-assistants.html) requires a Developer Certificate of Origin but asserts that this simply means that humans are responsible for the code. They *recommend* using an `Assisted-by` trailer but elsewhere clarify a lack of this may only ["impede the acceptance of your work"](https://kernel.org/doc/html/next/process/submitting-patches.html#using-assisted-by). [The Linux Foundation](https://www.linuxfoundation.org/legal/generative-ai) simply reiterates that humans are responsible for verifying they have the copyright to code they submit.

[Blender](https://developer.blender.org/docs/handbook/contributing/ai_contributions/) requires disclosure and explanation for large changes, forbids solving "Good First Issues" with AI, and forbids `Co-Authored-By` trailers.

#### Permissive

[LLVM](https://llvm.org/docs/AIToolPolicy.html) requires a "human in the loop" but does not require explicit disclosure. It also explicitly allows a [Bazel Fixer bot](https://discourse.llvm.org/t/rfc-ai-assisted-bazel-fixer-bot/89178/93) which uses AI. They reiterate that contributions can be [extractive](https://llvm.org/docs/AIToolPolicy.html#extractive-contributions) and ask contributors to consider the effort required to review.

[Python](https://devguide.python.org/getting-started/ai-tools/) disallows slop, reiterates requirements of quality, and suggests disclosure but does not require it.

[Firefox](https://firefox-source-docs.mozilla.org/contributing/ai-coding.html) reiterates that humans are responsible for changes but does not require disclosure.

#### In progress

The following projects are currently discussing policy, but have not yet adopted it:

* [Emacs](https://human-emacs.org)
* [Debian](https://www.debian.org/vote/2026/vote_002)

The following policies exist, but are not final:

* (everything here has been decided since initial mention)

## Unresolved questions
[unresolved-questions]: #unresolved-questions

* How should tooling be done to inform people of the LLM policy? Ideally, rustbot would inform new contributors or people who haven't made a PR since a recent policy change, but this constitutes work that needs to be figured out.
* Should the project adopt a Developer Certificate of Origin?

## Future possibilities
[future-possibilities]: #future-possibilities

* In some distant future where the AI bubble has violently exploded, we should probably consider how this policy should change as a result. However, we won't know what that'll be like until it happens.
