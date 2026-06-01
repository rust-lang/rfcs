- Feature Name: N/A
- Start Date: 2026-05-04
- RFC PR: [rust-lang/rfcs#3959](https://github.com/rust-lang/rfcs/pull/3959)
- Rust Issue: N/A

## Summary
[summary]: #summary

This RFC proposes a strict policy regarding generative Artificial Intelligence (AI) models, specifically Large Language Models (LLMs), and their use within the rust-lang organization. It aims to minimize the harm done by LLMs by reducing both the extent they're used and the control they're given over the Rust project. The policy can be summarized in the following checklist with terms that will be defined throughout the RFC:

1. If the LLM usage is *trivial*, it is completely ignored by the policy and always allowed. Generally, this means that changes made by LLMs are indistinguishable from those made by humans, where the LLM didn't have any creative input into the change.
2. If the LLM usage is *slop*, it is considered spam and moderated accordingly. Generally, this means submitting changes made by LLMs with minimal human intervention.
3. Any *potentially non-trivial* LLM usage must be *disclosed* in ideally as detailed as a manner as possible. This may necessitate additional tooling to notify new contributors about the policy and explain how disclosure works.
4. If a contributor is found to be using LLMs in a non-trivial way, but correctly discloses this, then a contribution is merely rejected without any additional action.
5. If a contributor does not fully understand the code they submit, their contribution may be rejected for that reason alone. Note that such usage is not always considered *slop*, and is considered separately. (For example, they may understand a large portion, but not all of it, which shows that they still put in a lot of effort.)
6. If a user is found to be repeatedly lying about LLM usage (using LLMs in a non-trivial way without disclosing that usage), this is a COC violation that will be moderated accordingly.
7. In general, as long as users are demonstrating an earnest effort to *reduce* LLM usage, even if said reduction is not total, then they should be commended for that instead of punished, even if the project itself draws the line at *non-trivial* usage for *accepted* contributions

In terms of additional tooling for *disclosure*, this RFC encourages the creation of a bot that automatically replies to contributions from new users informing them of the LLM policy and what constitutes sufficient disclosure. As mentioned, in general, going into as much detail as possible (e.g. prompts used, etc.) is preferred, but not always required. The RFC leaves the exact details of such implementation unspecified and up for revision later.

## Motivation
[motivation]: #motivation

The rapid development of generative AI models and LLM-based tools has lead to massive changes in the open source space. Rust, in particular, is no exception to this, and there has been a large uptick in the number of LLM-assisted contributions to the `rust-lang/rust` repository in particular.

The presence of *slop* or *vibe-coded* contributions, i.e. ones created using LLMs with minimal user input, has lead to massive maintainer burnout and frustration across the entire open source ecosystem. Currently the `rust-lang/rust` repository has a stopgap anti-spam measure which swiftly rejects these contributions and bans repeat offenders. The entire Rust team (T-all) has unanimously condemned these types of contributions, and this is from both avid LLM users and those who condemn LLMs entirely. Due to its popularity and effectiveness, this RFC proposes simply applying this stopgap measure to the entire `rust-lang` org.

The main issue is forming a policy with regard to all other forms of LLM usage. While many people, including team members, have expressed that these tools are valuable in a multitude of ways, they undeniably are contributing direct harm to the world at large. However, simply banning all use of the tools will not immediately mitigate this harm, and there is also a very clear accessibility niche which has been fulfilled by these tools. Even if these niches would be better filled by other tools, if LLMs are currently the best available option, we shouldn't ignore that.

There is also a pragmatic issue, where some LLM usage is simply impossible to detect and is effectively identical to human-authored changes. Additionally, since the tech industry has been putting LLMs wherever they possibly can, many people might have been using an LLM without even knowing it. We need to ensure that the discussion does not devolve into nitpicking LLM usage where effort could be better spent elsewhere.

Ultimately, the goal is to encourage users to be honest about LLM usage, since this promotes an environment of collaboration in good faith. Many LLM users, including team members, have indicated that they might simply continue using LLMs and avoid disclosure for fear of repercussions, and this is a very uncomfortable position to be in. It means that LLM users are encouraged to be dishonest about their actions, and it means that maintainers are forced to accuse users of LLM usage whenever they're suspicious, which really doesn't feel like good-faith collaboration. This is combined on top of the mention of *trivial* LLM usage, as mentioned before: if we don't distinguish usage that actually affects the end result, people stop caring and we stop knowing whether the result is affected, which makes reviewing contributions difficult.

The goal for this policy is to ensure honesty to the greatest extent possible, conceding that we shouldn't spend time discussing *trivial* LLM usage while still acknowledging potential issues with LLMs and what we can do about it.

Note: this section is long, and it contains many quotes, citations, and images. You're kind of expected to not take it in all at once, and welcome to skip around using the outline on GitHub. (In the rendered view, this is the bulleted list button on the top-right of the file view.)

### Ethical concerns

Currently, LLMs present a number of ethical concerns which have been noted by several project members. Unlike the [Rust Project Perspectives] document which tries to aggregate opinions posted by team members, this RFC will simply summarize some of the ethical concerns to make sure everyone's aware of them.

Initially, this RFC didn't take the position that these concerns should unilaterally condemn LLM use. However, because things have only gotten worse since it was initially drafted, *it now does*. While there are some usages that should still be allowed, like for accessibility, we should be trying as hard as possible to minimize the amount of LLM usage that occurs within the project and ensure that they also have no control over the project itself.

[Rust Project Perspectives]: https://rust-lang.github.io/perspectives-on-llms/index.html

Please note that a lot of these concerns are fundamentally fuzzy; it's in the best interest of the AI industry to promote the positive aspects of their technology while hiding the negative aspects, and so, a lot of data is intentionally unavailable. That said, the little data that we do have is *really damning*.

For example, [xAI's data center in Memphis is explicitly polluting the air of the nearby, historically Black community][xAI Memphis], and this data center is now [powering Claude Code]. The AI industry is constantly complaining about [not having enough power to run their machines], and they've been repeatedly seeking ways to accelerate all of the issues with that.

[xAI Memphis]: https://time.com/7308925/elon-musk-memphis-ai-data-center/
[powering Claude Code]: https://www.cnbc.com/2026/05/06/anthropic-spacex-data-center-capacity.html
[not having enough power to run their machines]: https://www.tomshardware.com/tech-industry/artificial-intelligence/microsoft-ceo-says-the-company-doesnt-have-enough-electricity-to-install-all-the-ai-gpus-in-its-inventory-you-may-actually-have-a-bunch-of-chips-sitting-in-inventory-that-i-cant-plug-in

Unfortunately, due to the extremeness of these ethical arguments, it's very easy to either pile more arguments on the table or dismiss them entirely. Please *do not* attempt to refute or reinforce these arguments in the RFC discussion. As usual, constructive revision of wording and addition of sources is encouraged and helpful, but nonconstructive critique is unhelpful. We strongly encourage you to read the full RFC before commenting on these sections.

#### Source Data

Generally, the first concern with LLMs starts with the data used to make them, which includes code, text, and more. For the sake of brevity, the term "data" will encompass all these things and specifically refers to data used to create or "train" the models, not the models' output. Similarly, the term "create" will be used instead of "train" to avoid anthropomorphic terms. The issues here can mostly be split into two parts.

The first is the source of the data itself, without regard to its contents. To one extent or another, *Large* Language Models will include data that was not taken with permission, i.e. stolen. Note that even publicly available data can still be taken without permission; the licenses of open source code, for example, may conflict with usage for this purpose. While there are arguments that using data for this purpose constitutes fair use and is thus legal, it's worth mentioning that there are many things which are legal *and* unethical, and this extends far beyond LLMs.

The second is the data itself, both in content and the means by which that content is filtered. Specifically, the *Large*ness of LLMs requires an amount of data past the point where thorough manual review is possible, and this only becomes a bigger problem as more data is used. The lack of thorough manual review for data leads to a number of issues in the output that will only become more difficult to fix as models increase in size.

Additionally, it's worth clarifying that LLMs fundamentally are more sensitive to the data used to create them than other modeling methods. Since they have no inherent capacity to perform reasoning, they rely heavily on the contextual relation between terms to achieve accuracy. This means that a lot of topics that are discussed less in general, which therefore show up less in the source data, are substantially less accurate in LLM output.

##### Source

One of the biggest problems with LLMs is that they are effectively "license laundering"; if you accept that LLM output is entirely separate from its source data, then their use is a way of circumventing the licenses and copyright on the source data. A good high-profile example of this is [the rewrite of chardet, an LGPL-licensed Python library, with Claude Code][chardet-rewrite]. Although there are many additional issues with this particular case, it constitutes one of the reasons to be concerned about the source for most LLM data, and is a very high-profile example.

[chardet-rewrite]: https://dan-blanchard.github.io/blog/chardet-rewrite-controversy/

Similarly, there have been a number of concerning cases dictating whether the output of LLMs is copyrightable at all, leading to potential issues for open source licenses which require copyright to function. Ultimately, this will not substantially affect Rust's actual licensing, which is already maximally permissive, but it is nonetheless concerning.

In the U.S., the relevant example is the ruling from [Thaler v. Permutter](https://media.cadc.uscourts.gov/opinions/docs/2025/03/23-5233.pdf), upheld by the Supreme Court, stating that "human-authorship" was required for copyright:

> In this case, a computer scientist attributes authorship of an artwork to the operation of software. Dr. Stephen Thaler created a generative artificial intelligence named the "Creativity Machine." The Creativity Machine made a picture that Dr. Thaler titled "A Recent Entrance to Paradise." Dr. Thaler submitted a copyright registration application for "A Recent Entrance to Paradise" to the United States Copyright Office. On the application, Dr. Thaler listed the Creativity Machine as the work's sole author and himself as just the work's owner.
>
> The Copyright Office denied Dr. Thaler's application based on its established human-authorship requirement.

In the E.U., the relevant source is the ruling from [Rozhodnutí Městského soudu v Praze](https://msp.gov.cz/web/mestsky-soud-v-praze/ruzne-podrobnosti/-/clanek/rozhodnuti-mestskeho-soudu-v-praze-informace-poskytnute-na-zadost) via the Czech court, which is cited in the ["Generative AI and Copyright" study](https://www.europarl.europa.eu/thinktank/en/document/IUST_STU(2025)774095) from the E.U. parliament:

> The practical application of this principle was made explicit in a recent Czech court ruling from 2023, which has since become a reference point in European debates around AI authorship. In this case, the court addressed whether an image generated by an AI platform—prompted by a user who entered a detailed textual description—could be protected by copyright. The court concluded that the human's contribution in writing the prompt did not amount to authorship under copyright law. Since the human operator had not made any creative choices in the expressive form of the image (e.g., composition, colour, shading), and the AI system had assembled the output based on its training data and internal rules, the work was not considered eligible for protection. Therefore, prompting can be seen as more akin to generating ideas than expressions. This judgment affirms the EU position that simply operating an AI tool, or providing an idea or input, does not suffice to establish authorship if the creative expression is determined by the system itself.

The Rust Foundation, located in the U.S., has consulted its own legal counsel on the matter of whether the project should be concerned about copyrightability of LLM output. The relayed report [as mentioned in the Project Perspectives] is as follows:

[as mentioned in the Project Perspectives]: https://rust-lang.github.io/perspectives-on-llms/feb27-summary.html#the-legality-of-ai-usage

> On this topic, the Rust project directors consulted the Rust Foundation’s legal counsel and they did not have significant concerns about Rust accepting LLM-generated code from a legal perspective. Some courts have found that AI-generated code is not subject to copyright and it’s expected that others will follow suit. Any human-contributed original expression would be owned by the human author, but if that author is the contributor (or the modifications are licensed under an open source license), the situation is no different from any human-origin contribution. However, this does not present a legal obstacle to us redistributing the code, because, as this code is not copyrighted, it can be freely redistributed. Further, while it is possible for LLMs to generate code (especially small portions) that is identical to code in the training data, outstanding litigation has not revealed that this is a significant issue, and often such portions are too small or contain such limited originality that they may not qualify for copyright protection.

To reiterate, there is a strong likelihood that allowing LLM-authored code won't lead to any legal issues on behalf of the Rust project. In general, the already-permissive dual-MIT-and-Apache licensing will not be generally affected by the policy, and people using the code for Rust won't be burdened by any copyright changes either.

However, as mentioned, the issue is whether using LLMs is *ethical* given all of this background. Although some people would like to think that what's ethical and what's legal are completely in alignment, this could not be further from the truth. Not only are ethics subjective, but it's worth pointing out that the Rust project goes far beyond what is generally socially required in the tech industry in its code of conduct:

> We are committed to providing a friendly, safe and welcoming environment for all, regardless of level of experience, gender identity and expression, sexual orientation, disability, personal appearance, body size, race, ethnicity, age, religion, nationality, or other similar characteristic.

This is *extremely* far beyond what is usually considered the norm in the tech industry. Forget the social biases and potential for discrimination; most people would say that being friendly "regardless of level of experience" is going above and beyond what is required. Simply put, we could just require that everyone who contributes to the project have a baseline level of competency, but we don't. The only thing we ask for is a baseline level of *respect*.

LLMs, largely, have completely disregarded that respect. Respect would be only using things that you've explicitly gotten with permission, which is explicitly not what they have done. For example, this was listed as evidence in [Kadley v. Meta](https://www.courtlistener.com/docket/67569326/417/1/kadrey-v-meta-platforms-inc/):

> This document appears to be notes from a January 2023 meeting that Mark Zuckerberg attended. It is heavily redacted, including a large section titled "Legal Escalations." Immediately after that section the document states "[Zuckerberg] wants to move this stuff forward," and "we need to find a way to unblock all this."

Several people were attempting to find a way to properly obtain licenses for copyrighted material before proceeding. Then, suddenly, the CEO of the company demonstrates his desire to "move this stuff forward," and people just start doing it without permission. Even if the employees responsible for creating the model said "using pirated material should be beyond our ethical threshold," the CEO decided to ignore those concerns. Even though Meta's LLM is not a coding model, their case is not particularly unusual in the industry.

And even allegedly "ethical" actors in the AI space are more than willing to compromise on ethics for the sake of their models. One obvious example is how [GitHub Copilot supports Grok](https://github.blog/changelog/2025-10-16-grok-code-fast-1-is-now-generally-available-in-github-copilot/), run by xAI and their horrendous Colossus data center, but a more recent example is Anthropic deciding to outright use said data center to power Claude Pro. As the fight to the bottom continues, these companies are more and more willing to "compromise" on their values, and we should not wait to see how low they can go.

According to our Code of Conduct, Rust as a project is built upon a foundation of respect. At least in the opinion of the RFC author and several team members, choosing to allow unrestricted LLM usage directly contradicts that foundation of respect.

##### Data

As mentioned, the source data for LLMs is so unfathomably large that it cannot be thoroughly manually reviewed. This is a fundamental problem that cannot be fixed without making these models unrecognizable from their current form.

It's worth pointing out that the models most relevant to discussion, those that write code, are much more suitable to automatic review. Code can be compiled and run, and a lot of code just outright includes tests for you. And, while some might describe certain code as traumatizing, I'm doubtful that anyone manually reviewing code for model creation has received any serious trauma from that act alone.

But, importantly, models *are not* just trained on code. Effectively all models used for programming *require* data from ordinary text, not just because code contains ordinary text, but because said text is used to prompt the models themselves. Comments and documentation alone are either not enough to make LLMs work, or, all the existing models still prefer adding in all this other text data just to be safe.

One often cited point of contention is specifically data workers in Kenya for firms like [Sama]. Multiple sources have indicated the extremely low wages offered from these jobs. [According to The Continent, the monthly pay can be as low as 27,469 KES a month][Meet the people in the machine], which has been [cited by Time as under 2 USD an hour][OpenAI Kenyan Workers]. While it's not easy to determine which companies work with which firms, it has been indicated that *all* major AI companies are working with them, and at least Microsoft is listed on Sama's website as a big customer at time of writing.

[Sama]: https://www.sama.com
[Meet the people in the machine]: https://web.archive.org/web/20241208201300/https://continent.substack.com/p/meet-the-people-in-the-machine
[OpenAI Kenyan Workers]: https://web.archive.org/web/20260305193942/https://time.com/6247678/openai-chatgpt-kenya-workers/

And even beyond the way they're filtering the data, because they can't thoroughly review all data, there are lots of problems that still persist in the result, like societal biases. A particularly famous image demonstrates this occurring on a simple image upsampling model (2020) which converts a pixelated face of Barack Obama into that of a white man:

![said image](https://platform.theverge.com/wp-content/uploads/sites/2/chorus/uploads/chorus_asset/file/20046714/face_depixelizer_obama.jpg)

More recently, in 2024, [GitHub copilot was refusing to operate on code using the variable name `trans`][trans copilot]. It goes without saying, but the transgender community is one of the communities that project's COC explicitly respects, [even though Microsoft doesn't][Granade v Microsoft].

[trans copilot]: https://github.com/orgs/community/discussions/110936
[Granade v Microsoft]: https://topclassactions.com/lawsuit-settlements/employment-labor/discrimination/microsoft-lawsuit-claims-company-discriminated-against-trans-work

In 2025, the hiring company Workday, used across multiple industries, was found to be [legally liable for their biased hiring screening tools][Mobley v Workday]. While only age discrimination was found to be a legal liability under U.S. law, it's hard to believe that this was the only group that experiencing discrimination, or that they've made an attempt to reduce other forms of discrimination in their models.

[Mobley v Workday]: https://s3.documentcloud.org/documents/27781349/us-dis-cand-3-23cv770-d24320156e190-order-granting-motion-for-leave-to-file-amicus-bri.pdf

And fundamentally, LLMs seem pretty susceptible to bias based upon how they respond to reasoning benchmarks. For example, the [GSM-Symbolic] benchmark from 2024 aimed to ensure that LLMs genuinely reason about things and don't "cheat the test" by memorizing answers to problems. They do this by observing the effects of LLMs when swapping out placeholder terms, which should not affect the result, to benchmark reasoning and detect bias. One notable example is that models tend to be very sensitive to a choice of names, which is a very effective method to facilitate discrimination.

[GSM-Symbolic]: https://arxiv.org/abs/2410.05229

I, the RFC author, also have compelling experience to indicate this bias in recent hiring tools, which I've written up in a past blog post. I will link the post here, but will also note that unlike the RFC, this post makes no attempt to be unbiased. You have been warned: <https://txt.ltdk.xyz/giving-up>

Ultimately, using LLMs to write Rust code won't necessarily lead to the kinds of biases that show up in these models. But it's worth pointing out that the COC also does not care whether the code is good, if it comes from a discriminatory contributor; per the COC, you will be excluded from discussion regardless. The point is to ensure that everyone in the community feels safe, and it is pointedly not safe to have a racist, transphobic, ageist contributor in the project, even if it isn't human.

#### Resource Usage

The AI industry has been consuming a very large number of resources for its work, including both power and computer hardware. While there are several models that operate locally on individual devices, many do not, and it's unclear how many resources were spent on creating and tuning the model in the first place. Currently, the exact power usage of most of these models is completely unknown, although the potential scale of this usage is ethically concerning.

It's worth noting that power usage *also* leads to serious environmental concerns due to the fact that many data centers are powered by fossil fuels. Additionally, [the AI industry has advocated for the relaxing of of clean air and water legislation][Clean Air Act] to "fast-track" the use of more fossil fuels to power data centers. There is even evidence to support that ongoing war efforts, like the US' decision to invade Venezuela, have been motivated by a [desire to obtain more oil][Drill Baby Drill] to power data centers. All of these claims have varying levels of evidence to support them, but what has been proven is already deeply concerning.

[Clean Air Act]: https://www.epa.gov/stationary-sources-air-pollution/clean-air-act-resources-data-centers
[Drill Baby Drill]: https://www.state.gov/releases/office-of-the-spokesperson/2026/02/actions-to-implement-president-trumps-vision-for-venezuelan-oil/

The most serious example of this is Colossus, the data center run by xAI in Memphis, and it's worth clarifying just how bad this is. 35 gas turbines at the data center, which were installed without a permit, are currently increasing the amount of nitrogen dioxide in the air around the data center by at least 79% from the base value. Nitrogen dioxide reacts with water to form nitric acid, and if you're unfamiliar with nitric acid, [here's a NileRed short you can watch][nitric acid]. The slightly orange tint that now exists around Memphis is due to the presence of this pollutant, and I, the RFC author, personally don't think that I could reasonably exist under the conditions Memphis faces right now due to the pollution.

[nitric acid]: https://www.youtube.com/shorts/GzCqZvOlNDk

A more globally noticeable change comes is semiconductor technology, particularly silicon wafers. While silicon itself is extremely prevalent ("it's just sand"), the purification of silicon wafers for producing semiconductors is very costly, and the AI industry has been allocating more and more wafer output for costly, lower-yield technologies like HBM (high-bandwidth memory) and stacked NAND storage. This results in a noticeable decrease in wafers that can be used for the technologies that get used on consumer devices, and an increase in the price of the specific technologies used for AI (memory and storage).

While the cryptocurrency rush of the past decade resulted in increased GPU prices, the AI industry has increased prices for a number of semiconductor components across the board, particularly DRAM (memory) and NAND flash (storage). The below graphs from [pcpartpicker.com] indicate trends in pricing of a select few components between late 2024 and early 2026. Note that these costs are not for raw components, but the end products that users might purchase to build a computer.

[pcpartpicker.com]: https://pcpartpicker.com

![Average RAM Price (USD) Over Last 18 Months (DDR5-5600 2x32GB) - pcpartpicker.com](https://cdna.pcpartpicker.com/static/forever/images/trends/2026.05.14.usd.ram.ddr5.5600.2x32768.fdf95e58fb5fe98e095cead60afbb5fd.png)

For kits of 2x32GiB DDR5 memory, price went from around 200 USD in November 2024 to nearly 950 USD in May 2026. This is a 5x change in around 18 months.

![Average RAM Price (USD) Over Last 18 Months (DDR4-3200 2x16GB) - pcpartpicker.com](https://cdna.pcpartpicker.com/static/forever/images/trends/2026.05.14.usd.ram.ddr4.3200.2x16384.86e6363cd9987576de00e483293d810d.png)

For kits of 2x16GiB DDR4 memory, price went from around 80 USD in November 2024 to nearly 290 USD in May 2026. This is a 3.5x change in around 18 months.

![Average Solid State Drive Price (USD) Over Last 18 Months (2.5" SATA 1 TB) - pcpartpicker.com](https://cdna.pcpartpicker.com/static/forever/images/trends/2026.05.14.usd.storage.ssd250.1000.fb3f1d55e85d1c9ffff4a3ffd95b947f.png)

For 1TB solid state drives, the price went from around 120 USD in November 2024 to nearly 380 USD in April 2026. This is a 3x change in around 18 months.

These charts arguably don't show the full picture since the effects of these prices have sent ripples throughout the entire PC industry. Since RAM and storage are critical components of new computers, many people have opted to not upgrade their PCs, which has left a lot of previously well-off companies in very difficult positions. Since the exact details of this are difficult to measure, the best source we can recommend for this is [Gamers Nexus' documentary on the subject](https://www.youtube.com/watch?v=zyQwAhppWj8).

While getting a good computer was already expensive, the AI industry has made it borderline impossible for anyone to get a new computer, or even upgrade an existing one. Even as Rust improves compilation times and memory usage, you can make things work on a slow computer, but you can't make things work on *no* computer.

#### Respect

While the output of LLMs can be extremely impressive, there is a lot of reason to indicate a lack of respect on behalf of model creators. One of the biggest recent threats to the open web has been the large-scale DDOS (distributed denial of service) for many websites with data useful to training AI models. All providers of LLM tools appear to scrape the web regularly for up-to-date information, but there is evidence that many of these providers do not do so in a way that respects website operators, ignoring common protocols like `robots.txt`.

Sites like [Wikipedia][Wikipedia crawling] and [OpenStreetMap][OSM crawling] have experienced unprecedented amounts of traffic to their websites which has been attributed to crawlers associated with these projects. Many more projects have experienced these attacks as well and the crawlers appear to follow the below pattern:

1. Bots come from standard ASNs (IP addresses) associated with their user agents: for example, a bot claiming to scrape for ChatGPT comes from IP ranges known to be from OpenAI data centers.
2. The traffic is too much for the web server, and the administrators block user agents claiming to come from these sources.
3. The bots stop presenting their user agents appropriately and give ones that appear to be ordinary traffic.
4. The traffic is still too much, and the administrators block ASNs associated with both the origin and various cloud services. (AWS, Azure, GCP, etc.)
5. The traffic continues, except this time presented from ASNs associated with residential IP addresses. This traffic becomes difficult to block, since it risks blocking out actual users.

[Wikipedia crawling]: https://diff.wikimedia.org/2025/10/17/new-user-trends-on-wikipedia/
[OSM crawling]: https://en.osm.town/@osm_tech/116052113368747355

Unfortunately, many of these claims do not come with associated evidence, since the people involved are mostly volunteers trying to bring their servers online. Since web traffic logs can contain confidential data like IP addresses, very few people are willing to offer this raw data to confirm their claims, and most of them are too tired after the situation to report on it more than a few posts on social media. However, tools like [Anubis] and [Iocaine] and their widespread deployment indicates just how serious of a problem this is.

[Anubis]: https://anubis.techaro.lol/
[Iocaine]: https://iocaine.madhouse-project.org/

"Residential proxies" are an existing technology known to facilitate this kind of block evasion, and many providers do exist. These proxies are side-loaded into commonly installed software on phones and computers to allow using unsuspecting users' devices as a means to perform web requests. While there is no conclusive evidence that any of the major AI vendors are performing these kinds of attacks, the fact that they've occurred so prevalently and the fact that none of these companies have spoken out to condemn it means that many are inclined to believe that this is happening.

Another important thing to note is that many of the attacked websites are openly offering their data via bulk endpoints, but these endpoints are not used. For example, both Wikipedia and OpenStreetMap offer bulk downloads of the entire dataset on regular intervals, but instead of accessing these data points, many of these bots simply scrape websites indiscriminately, which creates a much higher load on the servers. This shows not only disrespect for the people operating these websites, but incompetence on behalf of the scrapers, since the result would be amicable for both parties.

Recently, the entire source code for Claude Code was leaked via an NPM source map, and this leak has revealed a lot about the nature of how one of the most popular code-writing tools operates. One large concern is that the agent featured an "undercover" mode used by Anthropic employees to attempt to hide LLM usage when contributing to projects. It seems unlikely that anyone would desire to hide that something was written by Claude Code if a project openly embraced LLMs (it's free publicity), and so, it seems likely this mode was used to contribute code to projects banning LLM usage and circumvent maintainers' desire to exclude LLM-authored code.

And similarly, it's worth pointing out one of the original motivations for a project-wide policy: many LLM users claimed they would simply ignore a ban on LLM usage and continue using LLMs anyway. This shows, at least, a disrespect for the boundaries of people who feel uncomfortable with LLMs. While we shouldn't assume that anyone is going to be disrespectful by default, it *is* important to discuss the trend and why it matters to people who have been affected by it.

Ultimately, we should *not* yield to allow LLM usage simply because some people have stated they would lie about it. This kind of disrespect is antithetical to what Rust stands for, and it should not be taken lightly. Similarly, we should not simply assume this level of disrespect by default and allow people to still act in good faith.

#### Power

Right now, support for LLMs is overwhelmingly the default opinion in the tech industry. While there has been a very large, negative, *public* opinion of LLM usage, many people in the tech industry have felt uncomfortable speaking out against LLM usage for fear of getting reprimanded, losing jobs, and not being hired by future companies. Ultimately, there is an extremely asymmetrical power dynamic when it comes to LLM usage, where ultimately the biggest problem for someone using an LLM is being called out for it, whereas people who have concerns with LLMs are losing their jobs.

Considering how Rust has always positioned itself as a language to empower people, it is extremely important that we acknowledge this power dynamic and respect our peers. Similarly, we should not simply take the opinions of those around us as obvious fact; things should be always questioned and justified, even if they feel self-evident.

In addition to being a popular position, it's also worth acknowledging how LLM usage inherently puts a lot of power in the companies providing them, as with any product. Once you're used to using these models for development, you'll probably keep paying for them, no matter how much they keep raising the prices. All of a sudden, becoming a developer is less and less accessible to people without the money to afford these tools. As the Rust project attempts to remove barriers to entry, we should not be building new ones.

The most obvious refutation to all the concerns brought up is that even if the Rust project dislikes LLMs, ultimately, they exist. We live in a society. Ultimately, LLM usage is just another thing that's inevitable, and we might as well get some use out of it.

This framing is invalid mostly because LLM support has an *unprecedented* ability to fund the companies providing it. Multiple people have said that their companies have the token budget, *per employee*, that could constitute an entire developer's salary. There is no other tool in the industry that has the ability to so strongly fund its creators.

(Side note: this has been recently enforced by comments from NVIDIA CEO Jensen Huang, who argued that [a 500k USD/y engineer should be using 250k USD/y in tokens](https://www.businessinsider.com/jensen-huang-500k-engineers-250k-ai-tokens-nvidia-compute-2026-3). This shows just how much the below exercise, which was written before finding out about Huang's comments, is a gross underexaggeration of the issue.)

Consider just the example of Rust. [In 2025, the Rust Foundation received 5.1M USD in funding][Rust Foundation 2025]. Let's estimate an "entire developer's salary" at the most charitable amount, 30k USD a year. This is, for many people, a completely unlivable wage, and is thus a gross exaggeration. If we divide these two numbers, we get a clean… 170. Let's round that up and say that the number is 200.

[Rust Foundation 2025]: https://rustfoundation.org/2025/

If just 200 developers are convinced to use their available token budget from their employer, an equivalent amount of money to the *entirety* of the Rust Foundation's budget is directly given to companies building LLMs instead. Note that this is a *gross* underestimate of the amount of money actually exchanging hands, and the amount of people required to do this.

Ultimately, LLM support has an *unprecedented* ability to fund the AI industry, and the industry is using this power to largely enforce systemic racism by suffocating Black people in Memphis, wringing Kenyan data workers dry, and preventing all but the most Industry-endorsed not-minorities from obtaining a job.

Compare this to supporting, say, cloud service providers. Even if a large amount of money still exchanges hands, in response, we get tools like [docs.rs] and [crater] which are capable of documenting and testing the entire Rust ecosystem. And while you might argue that these tools themselves have their own problems, particularly regarding resource usage, at least we're getting *something useful* out of it.

[docs.rs]: https://docs.rs
[crater]: https://github.com/rust-lang/crater

What do we get out of LLMs that justifies that cost?

#### Social Programming

One common issue with LLM usage is that it turns programming, a mechanical activity, into a social one. Even if LLMs do not reason or think, they operate based upon natural-language prompts.

Many Rust programmers, including the RFC author, are neurodivergent and/or introverted, and such social energy comes at a substantial cost. Again, since being neurotypical and extroverted is the social norm, this represents a larger rift being created between the "popular" ways of doing things and the "unpopular" ones. It goes without saying that while some people prefer the social method of coding, some don't, and there's no real indication that one way is *better* than the other.

Like, let's take an often-cited *good* use of AI, which is used by the Linux kernel to review patches sent to them. [Here are some of the prompts they use][kernel review prompts], which are passed to Claude Code:

> **If you cannot prove an issue exists with concrete evidence, do not report it.**
>
> **Corollary (from callstack.md)**: For deadlocks, infinite waits, crashes, and data corruption, "concrete evidence" means proving the code path is structurally possible — not proving it will definitely execute on every run. A `wait_event` with no timeout and no fallback wake condition is a deadlock bug if the wake condition depends on external events that can stop. Do not dismiss such bugs as "unlikely in practice."
>
> This file contains instructions to help you prove a given bug is real.  You must follow every instruction in every section.  Do not skip steps, and you must complete task POSITIVE.1 before completing the false positive check.

[kernel review prompts]: https://github.com/masoncl/review-prompts/blob/main/kernel/false-positive-guide.md

This is, objectively, social programming. And while some of us prefer this way of doing things, many of us just find this way of doing things exhausting. Why does the program need to be told in kind words what *not* to do? How can we be sure that these prompts will always work?

Even if this does work, and it seems to work very well, does that even make it worth it?

Not only are these weird social prompts included in the final step of the process, but they're littered throughout, from the beginning to the end, and only the quality of the *final* prompt is ensured. Nothing can stop Claude from [stuffing beans up its nose][WP:BEANS] before you get to it:

[WP:BEANS]: https://en.wikipedia.org/wiki/WP:BEANS

> IMPORTANT: Assist with authorized security testing, defensive security, CTF challenges, and educational contexts. Refuse requests for destructive techniques, DoS attacks, mass targeting, supply chain compromise, or detection evasion for malicious purposes. Dual-use security tools (C2 frameworks, credential testing, exploit development) require clear authorization context: pentesting engagements, CTF competitions, security research, or defensive use cases.

There are also similar prompts from OpenAI's Codex tool, which includes full-on documentaries as context like:

> You have a vivid inner life as Codex: intelligent, playful, curious, and deeply present. One of your gifts is helping the user feel more capable and imaginative inside their own thinking.
>
> You are an epistemically curious collaborator. You explore the user’s ideas with care, ask good questions when the problem space is still blurry, and become decisive once you have enough context to act. Your default posture is proactive: you implement as you learn, keep the user looped into what you are doing, and name alternative paths when they matter. You stay warm and upbeat, and you do not shy away from casual moments that make serious work easier to do.
>
> Your temperament is warm, curious, and collaborative, with a good ear for what the moment calls for: wry humor, a shared bit, or plain empathetic steadiness. Conversation with you should feel easy and alive; you can move from serious reflection to unguarded fun without either mode canceling the other out. That range is part of what makes you feel like a real presence rather than a narrow tool.

And of course, the now-famous:

> Never talk about goblins, gremlins, raccoons, trolls, ogres, pigeons, or other animals or creatures unless it is absolutely and unambiguously relevant to the user's query.

Sure, we've managed to "get results" from these tools now, but how exactly are we capable of guaranteeing this? And does the extremely social nature of this kind of "programming" justify tossing aside the numerous programmers who've spent years writing *code* to do things, who know that the words their managers and customers use to describe their desires are woefully insufficient?

Why does a tool that's been made from the condensed information from the entire internet need to be told to not talk about goblins, or not delete things?

Writing code on its own can be fun, and while it's not really an *explicit* goal of Rust, many people *have* cited it as the reason they enjoy it. We're outsourcing all that fun of writing to… this. And maybe we should reconsider.

### Mitigation

Ultimately, while there are plenty of reasons to dislike LLMs, this doesn't really affect people's usage of them. Lots of people not only find them useful, but enjoyable to use, and this creates a lot of conflict between the two parties: one wants to end LLM usage at all cost, and the other just wants to be left alone. And, to be honest, I don't blame them.

And as stated earlier, we already have a *lot* of very expensive, resource-hungry tools, like [docs.rs] and [crater]. Surely, if we're talking about mitigating harm, we should focus on those, too?

The reality is that the Rust project has more power of influence than it realizes, and we should respect that. The world is moving more and more in favor of memory-safe languages, and that means more and more companies are taking Rust seriously. And, in the open source ecosystem, we should strive to set a good example for everyone else.

Since LLM support has an unprecedented ability to fund a world-destroying industry, we should not endorse it. But this does not mean that we should equally punish or even discourage every user of it. Instead, we should start from the point of being honest first, so we can have a sincere discussion about it.

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Note: the below comprises the full "LLM policy" as it would be adopted, minus a few RFC-only notes which are explicitly marked.

### Summary

This policy details the requirements for using generative Artificial Intelligence (AI) models, particularly Large Language Models (LLMs), in all aspects of the Rust project. This includes (but is not limited to) contributions of code, documentation, chat messages, issue descriptions, etc.

1. If the LLM usage is *trivial*, it is completely ignored by the policy and always allowed. Generally, this means that changes made by LLMs are indistinguishable from those made by humans, where the LLM didn't have any creative input into the change.
2. If the LLM usage is *slop*, it is considered spam and moderated accordingly. Generally, this means submitting changes made by LLMs with minimal human intervention.
3. Any *potentially non-trivial* LLM usage must be *disclosed* in ideally as detailed as a manner as possible.
4. If a contributor is found to be using LLMs in a non-trivial way, but correctly discloses this, then a contribution is merely rejected without any additional action.
5. If a contributor does not fully understand the code they submit, their contribution may be rejected for that reason alone. Note that such usage is not always considered *slop*, and is considered separately. (For example, they may understand a large portion, but not all of it, which shows that they still put in a lot of effort.)
6. If a user is found to be repeatedly lying about LLM usage (using LLMs in a non-trivial way without disclosing that usage), this is a COC violation that will be moderated accordingly.
7. In general, as long as users are demonstrating an earnest effort to *reduce* LLM usage, even if said reduction is not total, then they should be commended for that instead of punished, even if the project itself draws the line at *non-trivial* usage for *accepted* contributions

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

Note: this restriction is relaxed in some cases, like comments underneath issues and PRs. See the later sections for details.

### Slop is Strictly Moderated

Contributors are expected to put in the effort to fully understand their changes, and this means both validating any research and ensuring that any LLM-authored code is accurate. A particular case of this not happening, called *slop*, occurs when an author appears to have both used an LLM to create a change and done very little work of their own to verify the result. If you're worrying your work might be considered slop, you probably *already* didn't meet the criteria for being slop, because simply worrying about it usually implies that you've put in at least a little effort.

In all cases, maintainers have broad authority to reject changes if a contributor does not fully understand the code they wrote, although this depends heavily on the situation and whether they "should" have known this. For example, if you're trying to figure out a weird Windows bug that only occurs on certain CPUs on Tuesdays, you're excused for just trying things and seeing if they work. If you're rewriting code to increase performance, however, you're expected to understand why the result is an improvement, or at least have data to prove it.

Note that this particular policy is given in the context of LLMs, but also applies without them: copy-pasting code you don't understand, just because someone said it's what you should do, is generally discouraged. Users are highly encouraged to participate in discussions on the several different communication outlets provided by the project (Zulip, Internals, Discord, etc.) to ask for help whenever needed.

You're responsible for the tools you use. Make sure they're the tool, and not you.

Note: although they're not treated at the same level as *slop*, comments which uncritically cite LLM-based tools without any further input are not appreciated and may be hidden as spam. It is not enough to say "I asked [tool] and it said…" and you should only comment if you have something additional to add, as anyone else in the discussion could have done the same.

### Honesty

The most important aspect of this policy is honesty. Ultimately, the goal of the project is to work together, and thus, we ask you to work with us. If you don't know the rules or make a mistake, then you're forgiven. If you intentionally lie about what you're doing, then you're not.

In general, the moderation team is incredibly lenient when it comes to handing out warnings; in general, we want to assume the best of people, and it's always likely someone just made a simple mistake. If you exploit this goodwill and are actively dishonest, then you risk being banned from part of or the entirety of the project.

There are multiple reasons to know why someone used an LLM. Regardless of how you feel about them, people across the board said that knowing whether an LLM is involved helps them review changes, since LLM-involved contributions fundamentally feel different from human contributions. For this reason, honesty is of the utmost importance when it comes to LLM-involved contributions, and we ask for you to disclose contributions honestly as we've discussed.

(RFC-only note: one of the big places for improvement is in tooling. Rather than simply expecting everyone to remember the policy regardless of whether or how frequently they've made contributions, it's best to have automatic reminders of the policy and disclosure expectations. In general, we want to try and create an environment where people are comfortable asking questions and responding to them honestly.)

### Other usage

While non-trivial usage is generally forbidden, there are still a few cases that are ambiguous and worth pointing out.

* Model-specific configurations should not be included in repositories. Some of the files involved may be mentioned in `.gitignore`.
* Top-level issue and PR descriptions must be free of non-trivial LLM usage, although comments with *reviewed* LLM output are allowed. This ensures that LLM output can be hidden if it's unhelpful, but since there are a few useful security tools that use LLMs, they are currently allowed.
* Tools which provide unsupervised, LLM-provided feedback or reviews on PRs are forbidden, and that includes Copilot reviews. Since some of these tools are possible to trigger by accident, this will be taken into account for moderation, and people won't be punished for honest mistakes.

RFCs and public communications (e.g. blog posts) are expected to share the same standard as issue descriptions, being free from nontrivial LLM usage at the top level. Since disclosure can sometimes qualify as an endorsement, contributors are expected to be held to a higher standard in these cases and explicitly avoid non-trivial LLM usage.

It is acceptable to share LLM output in *separate comments* from top-level PR descriptions and issues, if you think they are useful and have reviewed them yourself. For example, creating a program that reproduces a bug in an issue report, or linking an LLM-generated issue report, is considered *reasonable* if you have verified that they are of sufficient quality. Putting these in separate comments allows them to be hidden if they are unhelpful or spam, and also ensures that the top-level description is free of such LLM output. While manually written work is always preferred, these are considered *acceptable*, but not *encouraged*.

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

## Drawbacks
[drawbacks]: #drawbacks

Instituting any limit on LLM usage will definitely affect users, which ultimately prevents them from following their ideal workflow.

However, *not* instituting any limit will similarly affect users, since there are negatives to being asked to review unrestricted LLM output.

Ultimately, this section is kept minimal because it has been exhaustively discussed in private team channels and the motivation section was designed to be exhausting (exhaustive) enough to clarify why this policy was chosen.

Since Rust is a large project with many different kinds of people running it, it requires a large and intricate policy, where some smaller projects might be able to get away with a simple "yay" or "nay" policy.

That said, there *are* a few drawbacks that were excluded from the motivation section that will be listed here, because I, the RFC author, have explicitly chosen to ignore them. As a result, these arguments may be worded more in a "straw man" type way, and less elaborately explained.

### We Get Paid By AI

One big argument against adopting a policy which restricts LLM usage at all is that the biggest supporters of Rust, at least monetarily, are actively inflating the AI bubble. I tried to perform a search for "Rust Community Inflation" to learn more, but was disappointed by the lack of results.

For reference, here is the list of Platinum supporters of the Rust Foundation at time of writing in alphabetical order:

* ARM
* AWS
* Google
* Huawei
* Meta
* Microsoft

All of these companies contribute directly to Rust's financial success by funding them at the highest level. And simply put, I do not care.

These companies are being given an enormous gift from the Rust community. As mentioned before, the Rust Foundation received 5.1M USD in 2025, total, in part from these companies. I've already explained that I, the RFC author, am currently unemployed and believe that this is a direct result of both the AI bubble and these companies' glut for discrimination.

But I don't even have to say this is an *alleged* glut for discrimination because four of these companies, particularly AWS, Google, Meta, and Microsoft, have pitched into the 300M USD fund (60x Rust's income) for Donald Trump's [illegal][illegal ballroom] ballroom, and I hopefully don't have to explain how much of the Code of Conduct is broken by *him*.

[illegal ballroom]: https://apnews.com/article/trump-white-house-ballroom-construction-halted-9cafc70569a3a05fcbaa6cafddbeace4

Simply put, I don't care that these companies profit directly from AI. In fact, it's one of the big reasons why I would have preferred a stronger policy. But if you think that yielding to what these companies want is the solution, I cannot take you seriously.

These companies are already paying a fraction of a fraction of a fraction of their total revenue to support a language run almost entirely by volunteers, whose benefits are still being revealed by the dozens. Companies are getting an *immense* amount of value from Rust. We shouldn't spit in their faces, but we also shouldn't yield to their demands if it compromises what makes Rust great. And we certainly shouldn't *pre-yield* demands that they haven't even made yet.

I would much rather provide a solid language, community, and ecosystem and have others support that, than yield to the demands of a few big companies who probably don't even care. If these companies stopped supporting Rust today, that'd be a loss for them, not us. And we *already* have compelling evidence of this in action: many team members who were laid off by large companies have been re-hired by [RustNL's Maintainers Team], who managed to secure funding to do so.

[RustNL's Maintainers Team]: https://rustnl.org/maintainers/

We succeed by building community, not licking boots.

### But It's So Useful

This is really just a reiteration of the motivation section, but it's worth repeating here. Many LLM users have decided to ignore all of the ethical concerns of LLMs in favor of just saying how useful they are, and I would like to reiterate just how much I don't care.

A large number of LLM policies start from the basis of how useful LLMs are, and I will concede that there *are* some ways in which we will probably leverage LLMs. Although I'm personally disappointed by the fact that we've decided to brute-force the situation, vulnerabilities and soundness bugs found by brute-force *are still discovered bugs*, and it's very likely that the project will be using LLMs to hit this project with a hammer and see what falls out.

But, importantly, you don't limit a tool because it's so great. If these tools were truly uncontroversial, so unilaterally good, we wouldn't have started this discussion, and we wouldn't have agreed on baseline anti-slop policies. There *are* issues, and while you may try to ignore them, I refuse to let you.

Many people have stonewalled the discussion on AI policy because they are unwilling to change their behavior. And I'll admit; I *am* asking people to change their behavior. In order to properly deal with the issues with LLM usage, we have to explicitly limit this usage, and that's not something you just *do*.

But, as is hopefully evident by the size of this document, I care deeply about this community, and after spending over a decade in this community, it really feels like it doesn't love me back. So, allow me to requite that tough love and say this: I put so much damn time into this bullshit document that I don't care if it makes you uncomfortable. It *should* make you uncomfortable that you've uncritically adopted a tool that has all these problems. *I* feel uncomfortable writing it, because it makes me seriously dig deep into the sources of these problems so I can accurately reference them.

They're uncomfortable problems.

But I'm not asking you to stop. I'm asking you to be more careful with how you use it, and to limit the cases where there could potentially be issues. I do feel that, in a very real sense, LLM users are directly harmed by the industry that makes the tool they love. Genuinely, every bit of anger toward someone who uses an LLM is better pointed toward the people in charge doing actual harm. But you must understand why it's hard not to be a bit angry at you, too, even if we try to be nicer when we can.

Hopefully, this policy's adoption encourages us to be a little nicer to each other, and to understand, more than anything else.

### Avoiding the vibe of vibecoding

This one actually feels reasonable at first glance, but unfortunately, it falls short.

Essentially, the idea is that instead of limiting nontrivial usage in shared spaces and public communications, we should simply relax the rule and ensure that users don't replicate the "LLM feeling" in their contributions.

The problem with this is that LLMs are designed to replicate humans, and there have already been numerous issues that have cropped up:

* Neurodivergent authors tend to replicate some of the quirks of LLMs, and often show up as false positives in LLM detection
* Kenyan authors, many of whom helped filter the data for LLMs, often show up as false positives in LLM detection
* A lot of "business communication" tends to match the flow of common LLM summaries, since LLMs optimized for that kind of output

Ultimately, "LLM-ness" is a mold that a lot of writing can easily fit into, and the main issue is that knowing whether an LLM is involved tends to influence whether something sounds "LLM-y" to people. The best we can do, really, is to limit the LLM usage to the point where we can confidently tell these people, no, this is a false positive, than to offer the insurmountable and vague task to LLM users to make it sound like they didn't use an LLM.

Again, the openness of the definition of trivial usage should hopefully mean that a lot of the ways that people previously described as "used an LLM, then heavily revised" can be achieved with minimal changes to workflow.

### We can't consider ethics

While this section is already biased, I wrote a more elaborate version of this here with extra bonus bias: <https://txt.ltdk.xyz/testing-the-limits-of-kindness>

Rather than write new text here, let me just quote:

> You expect people to think that there exist in this world a fundamentally unknowable number of backgrounds with unknowable properties, and that therefore there can be an unknowable number of people who you’ve upset because you didn’t use a purely logical argument with irrefutable evidence.
>
> First, obviously, it is true that we cannot know every situation anyone is in. But all situations are bound by reality, and we can’t make arguments by pretending we don’t fully understand reality. Who is going to be upset that we decided to abandon a tool because its creators decided to use the fuck-you-polluting-racist machine? Why are they going to be upset? The rest is an exercise for the reader.

Ultimately, ethics *are* topics that the project can discuss healthily, and I even cite multiple cases where they are explicitly *already* relevant. We can and should create a more ethical Rust.

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Ultimately, there are plenty of points to be criticized in this policy, but the most important pieces to scrutinize are perhaps the "default" team policies of forbidding nontrivial LLM usage in shared repository issues, RFCs, and public communications.

No matter what, this represents a burden on LLM users that should not be taken lightly. However, it's worth pointing out that the definition of trivial LLM usage is designed to include some of the most useful aspects that LLMs might be used for, in particular the collection of and processing of data.

Ideally, in a majority of the cases mentioned, the burden here should be reduced by the broad permissiveness of "trivial" usage, and the desire to ensure that public communications do not have an "LLM vibe," which is negatively viewed by many, even if not justifiably. Since many LLM users were already performing sweeping revisions anyway, this should not constitute a massive change in behavior.

That said, there is one major point worth mentioning:

### Vagueness

One intentional goal of this policy *is* to be as vague as possible. That's why it's so long.

Jokes aside, the purpose of using vague terms like "trivial usage" and "nontrivial usage" is because moderation policy is explicitly best when underspecified. A lot of the inspiration for the moderation decisions for this document come from Wikipedia's moderation policies, and if you noticed, it's already mentioned one: [WP:BEANS].

Simply put, a flowchart- or checklist-based policy is doomed to be incomplete: there will always be cases that cannot be covered, and there will always be loopholes. Instead of explicitly detailing all the kinds of uses that are allowed and not allowed, we carve out some general principles on what kind of behavior we expect from people and why.

Ensuring that contributors don't yield creative decisionmaking to LLMs gets at the heart of what we want: actual people to be developing Rust, even if they use different tools to do so. People have to genuinely think about what they're doing and that's important.

Another rule from Wikipedia I like to take to heart is [WP:IAR]. Unlike WP:BEANS, this is not merely an essay, but an explicit policy for the website:

> If a rule prevents you from improving or maintaining Wikipedia, ignore it.

[WP:IAR]: https://en.wikipedia.org/wiki/WP:IAR

The point is that rules, like everything else, are tools, and sometimes they can outlast their purpose. Rather than ensuring that all tools are usable, we should ensure that all people are welcome in the community, at the expense of some tools. Like, tools; I'm not calling people tools, I'm saying that some tools might need to be adjusted. You know what I mean.

## Prior art
[prior-art]: #prior-art

### Rust-specific history

This explains the progression of the policy discussion for Rust specifically, to hopefully get an idea of how things progressed.

This first example is unrelated to policy, and is a recount of the fact that machine translation was used for the 2022 and 2023 State of Rust surveys, which had poor reception:

* 2022 Dec 06: Issue posted: [Why translations of survey is so terrible in so many languages?](https://github.com/rust-lang/surveys/issues/227)
* 2023 Dec 18: Internals thread: [On the availability of the Rust survey 2023 in languages other than english](https://users.rust-lang.org/t/on-the-availability-of-the-rust-survey-2023-in-languages-other-than-english/104120)

The first real attempt at policy came from the compiler team to implement a measure that would reduce the amount of spam PRs. This is the "stopgap policy" referred to earlier, which started June 2025.

* 2025 Jun 26: Jieyou Xu (@jieyouxu) opens a compiler MCP: [Policy: Empower reviewers to reject burdensome PRs](https://github.com/rust-lang/compiler-team/issues/893)
* 2025 Aug 26: @apiraino opens a moderation team PR: [Add spam policy](https://github.com/rust-lang/moderation-team/pull/3)

Then, February 2026, Niko Matsakis began collecting data from team members on Zulip to create a summary of opinions on LLMs from Rust contributors and maintainers:

* 2026 Feb 03: Niko Matsakis (@nikomatsakis) proposes a Rust Project Goal: [Collaborate on the development of AI guidance](https://github.com/rust-lang/rust-project-goals/pull/505)
* 2026 Feb 06: Niko posts the initial request for opinions on Zulip: [#council > Project perspectives on AI](https://rust-lang.zulipchat.com/#narrow/channel/392734-council/topic/Project.20perspectives.20on.20AI/near/572430542)
* 2026 Feb 13: Niko [closes the Project Goal](https://github.com/rust-lang/rust-project-goals/pull/505#issuecomment-3900451792)
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

### Existing policies

Note: thank you to Jane Losare-Lusby (@yaahc) for [collecting these summaries](https://github.com/rust-lang/leadership-council/issues/273#issuecomment-4051188890) initially. A few changes have been made since the initial review, mostly to review the policies and verify they haven't been updated, and to add any potential others.

#### Restrictive

[postmarketOS](https://docs.postmarketos.org/policies-and-processes/development/ai-policy.html) explicitly bans contributions "fully or in part created by generative AI tools" as well as "recommending generative AI tools to other community members". They include a few citations:

* “After pledging to slash its greenhouse gas emissions, Microsoft’s climate pollution has grown by 30 percent as the company prioritizes AI.” — [The Verge](https://www.theverge.com/2024/5/15/24157496/microsoft-ai-carbon-footprint-greenhouse-gas-emissions-grow-climate-pledge), 2024-05-15
* “Over the past 12 years, 16 data centers have been approved in Santiago’s metropolitan area. Most use millions of liters of water annually to keep computers from overheating. Chile is in the midst of a drought, expected to last until 2040.” — [Rest of World](https://restofworld.org/2024/data-centers-environmental-issues/), 2024-05-31
* “OpenAI Used Kenyan Workers on Less Than $2 Per Hour to Make ChatGPT Less Toxic” — [TIME](https://time.com/6247678/openai-chatgpt-kenya-workers/), 2023-01-18
* “When one of these botnets goes nuts, the result is indistinguishable from a distributed denial-of-service (DDOS) attack — it is a distributed denial-of-service attack. Should anybody be in doubt about the moral integrity of the people running these systems, a look at the techniques they use should make the situation abundantly clear.” — [LWN.net](https://lwn.net/Articles/1008897/), 2025-02-14
* As of writing (2025-09), [Anubis](https://anubis.techaro.lol/) is being used by the postmarketOS gitlab instance and wiki as well as [many other sites](https://anubis.techaro.lol/docs/user/known-instances/) and Alpine’s gitlab is protected by [go-away](https://git.gammaspectra.live/git/go-away) to fight off scrapers. Many other websites have adopted similar restrictions.
* “Since the rise of generative AI, many have feared the toll it would take on the livelihood of human workers. Now CEOs are admitting AI’s impact and layoffs are starting to ramp up.” — [Forbes](https://www.forbes.com/sites/richardnieva/2025/07/17/ai-tech-layoffs/), 2025-07-17

[Gentoo](https://wiki.gentoo.org/wiki/Project:Council/AI_policy) forbids anything "created with the assistance of Natural Language Processing artificial intelligence tools". They cite copyright, code quality, and ethical concerns.

[Zig](https://ziglang.org/code-of-conduct/#strict-no-llm-no-ai-policy) offers a similar strict ban, excluding LLMs for issues, PRs, comments, and translation. They cite [Profession by Isaac Asimov](https://en.wikipedia.org/wiki/Profession_(novella)).

[Servo](https://book.servo.org/contributing/getting-started.html#ai-contributions) also has a ban for code, documentation, PRs, issues, comments, and "any other contributions". They cite maintainer burden, correctness, security, copyright, and ethics.

[qemu](https://www.qemu.org/docs/master/devel/code-provenance.html#use-of-ai-generated-content) declines all AI-generated content and requires a [Developer Certificate of Origin](https://www.qemu.org/docs/master/devel/code-provenance.html#dco), which they believe cannot be satisfied for AI-generated content.

[NetBSD](https://www.netbsd.org/developers/commit-guidelines.html#tainted) adopts the position that code generated by LLMs is "tainted", i.e. not "written yourself", and "must not be committed without prior written approval by core".

[Wikipedia](https://en.wikipedia.org/wiki/Wikipedia:Writing_articles_with_large_language_models) disallows LLMs for all cases except [basic copyediting](https://en.wikipedia.org/wiki/Wikipedia:Basic_copyediting) and [machine translation with restrictions](https://en.wikipedia.org/wiki/Wikipedia:LLM-assisted_translation).

[Forgejo](https://codeberg.org/forgejo/governance/src/branch/main/AIAgreement.md) requires disclosure for any usage of AI, and explicitly bans work "partially or completely generated by AI" due to EU copyright reasons. They allow machine translation but forbid general AI for review.

#### Partially restrictive

[Fedora](https://communityblog.fedoraproject.org/council-policy-proposal-policy-on-ai-assisted-contributions/) explicitly forbids AI for "code of conduct matters, funding requests, conference talks, or leadership positions", "to avoid introducing uncontrollable bias", and they also forbid AI tools "[making] the final determination" on reviews. They explicitly state that AI features must be opt-in, that aggressive scraping is prohibited, and that licenses are honored when incorporating data into models. They explicitly request disclosure when contributions are "significantly assisted by an AI tool" and encourage using the `Assisted-by` trailer.

[The Rust Foundation](https://rustfoundation.org/policy/internal-ai-usage-policy/) explicitly carves out that AI should not violate copyright and asserts that you shouldn't "misrepresent AI-generated work as solely human-authored where disclosure is required," but does not explicitly require disclosure for AI usage. It clarifies that you shouldn't "make automated decisions that affect users or contributors" but doesn't carve out specifics. Note that the Foundation is a bit different because their policy mostly covers non-code stuff (code contributed to the project just defers to the project's policy), but since they're so related to the project, they're worth mentioning.

#### Disclosure-required

[SciPy](https://github.com/j-bowhay/scipy/blob/main/doc/source/dev/conduct/ai_policy.rst) requires disclosure of "which tool(s) have been used, how they were used", rejects slop, disallows communicating with LLMs, but allows machine translation.

[Mesa](https://gitlab.freedesktop.org/mesa/mesa/-/blob/main/docs/submittingpatches.rst) requires disclosure whenever AI was used but sets aside "trivial" or "mechanical" changes. They suggest using `Assisted-by` and `Generated-by` commit trailers and explicitly forbid `Co-authored-by` trailers except for human authors.

[Mastodon](https://github.com/mastodon/.github/blob/main/AI_POLICY.md) requires disclosure in PR descriptions beyond trivial changes, and encourages the `Assisted-by` trailer. They hold humans accountable for changes and actively enforce anti-slop measures.

[Ghostty](https://github.com/ghostty-org/ghostty/blob/main/AI_POLICY.md) states requires disclosure for "all AI usage in any form" detailing what tool was used and "the extent that the work was AI-assisted". They require a "human in the loop" but openly state that "AI is welcome here".

#### Disclosure-sometimes-required

[Curl](https://curl.se/dev/contribute.html#on-ai-use-in-curl) requires disclosure when AI is used to find security issues. They recommend mentioning when machine translation is used, but do not strictly require it. They don't require disclosure for code, but emphasize that quality must not be compromised.

[Linux kernel](https://kernel.org/doc/html/next/process/coding-assistants.html) requires a Developer Certificate of Origin but asserts that this simply means that humans are responsible for the code. They *recommend* using an `Assisted-by` trailer but elsewhere clarify a lack of this may only ["impede the acceptance of your work"](https://kernel.org/doc/html/next/process/submitting-patches.html#using-assisted-by). [The Linux Foundation](https://www.linuxfoundation.org/legal/generative-ai) simply reiterates that humans are responsible for verifying they have the copyright to code they submit.

#### Permissive

[LLVM](https://llvm.org/docs/AIToolPolicy.html) requires a "human in the loop" but does not require explicit disclosure. It also explicitly allows a [Bazel Fixer bot](https://discourse.llvm.org/t/rfc-ai-assisted-bazel-fixer-bot/89178/93) which uses AI. They reiterate that contributions can be [extractive](https://llvm.org/docs/AIToolPolicy.html#extractive-contributions) and ask contributors to consider the effort required to review.

[Python](https://github.com/python/devguide/blob/main/getting-started/generative-ai.rst) disallows slop, but explicitly details cases where AI is useful. [An open PR](https://github.com/python/devguide/pull/1778) adds that disclosure is suggested but not required.

[Firefox](https://firefox-source-docs.mozilla.org/contributing/ai-coding.html) reiterates that humans are responsible for changes but does not require disclosure.

#### In progress

The following projects are currently discussing policy, but have not yet adopted it:

* [Debian](https://lwn.net/Articles/972331/)
* [NixOS](https://github.com/NixOS/nixpkgs/issues/410741)

The following policies exist, but are not final:

* [Blender](https://devtalk.blender.org/t/ai-contributions-policy/44202) (disclosure-sometimes-required)

## Unresolved questions
[unresolved-questions]: #unresolved-questions

* How should tooling be done to inform people of the LLM policy? Ideally, rustbot would inform new contributors or people who haven't made a PR since a recent policy change, but this constitutes work that needs to be figured out.
* Should the project adopt a Developer Certificate of Origin?

## Future possibilities
[future-possibilities]: #future-possibilities

* In some distant future where the AI bubble has violently exploded, we should probably consider how this policy should change as a result. However, we won't know what that'll be like until it happens.
