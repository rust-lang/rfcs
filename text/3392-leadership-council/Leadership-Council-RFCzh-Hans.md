领导理事会。PR说明和RFC摘要

本RFC由@jntrnr（核心团队成员）、@joshtriplett（语言团队负责人）、@khionu（调解团队成员）、@Mark-Simulacrum（基金会核心项目主管，发布团队负责人）、@rylev（基金会核心项目主管）、@technetos（调解团队成员）和@yaahc（基金会合作项目主管）共同撰写。

非常感谢 "领导力交流"的所有成员和更大范围的Rust项目的所有成员的初步审查和反馈。

本RFC建立了一个取代核心团队的领导理事会。理事会将大部分权力下放给各团队。

**注意**。此摘要对RFC进行了概述，但它是非权威性的。

# 程序性信息

## 讨论

有关本PR的讨论，请使用[本专用Zulip流](https://rust-lang.zulipchat.com/#narrow/stream/369838-rfc-leadership-council-feedback)。

## 翻译

本RFC的权威版本是英文版。然而，为了帮助人们广泛理解Rust的管理结构和政策，我们已经开始将所提议的管理结构和政策翻译成其他语言。具体来说，根据[Rust调查数据](https://blog.rust-lang.org/2022/02/15/Rust-Survey-2021.html)中认为非英语交流会有帮助的被调查对象使用最多的语种，我们将在完成以下语种的（非权威性）译版后发布这些译版：

- 中文（简体）
- 中文（繁体）
- 日语
- 韩语
- 俄语

完成这些翻译后，我们将在这里发布相关链接。请注意，这并不一定意味着我们会处理非英语评论。未来的任何翻译计划将由理事会决定，而非此小组。如果您对这些翻译有建议或意见，请反馈给我们。我们将在未来翻译计划方面参考您的反馈。

## 补充文件

本RFC包括补充文本文件。请[在此](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council/)查看子目录。

-----

# RFC 摘要

## 出发点

[全文](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#motivation)

Rust的管理结构将大多数决定权交给了适当的团队。然而，有大量的工作是不属于任何既定团队的职权的。

历史上，核心团队既负责发现不属于其他团队职权范围内的那些重要工作，又负责努力自行完成这些工作。然而，将两部分都放置于此团队之内既没有很好的扩展性，又导致了团队成员的倦怠退出。

本RFC建立的领导理事会将着重确定团队职权之外的工作及其优先次序。理事会会对这些工作进行委托而非亲自完成它们。理事会还能够以跨团队工作、规划和项目的长期成功等为目标，成为团队之间的协调、组织和问责机构。

本RFC还建立了理事会全体、理事会成员个人、调解团队、项目团队和项目成员之间的监督和问责机制。

## 职责、期望和对理事会的限制

[全文](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#duties-expectations-and-constraints-on-the-council)

理事会将确定、优先处理和跟踪因为归属不明而未完成的工作，并将这些工作委托给某团队（新团队或临时团队）。在某些时候，理事会可以在没有明确责任方的情况下决定*紧急*的事项。

理事会还会协调因项目而导致的团队、结构或流程的变化，确保顶层团队负起责任，并展示Rust项目的官方态度。

## 理事会的结构

[[全文]](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#structure-of-the-council)

理事会由一组团队[代表](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#candidate-criteria)组成，他们各自代表某个顶层团队及其子团队。

每个[顶层团队](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#top-level-teams)通过其各自的选择程序指定一名代表。顶层团队或其子团队的任何成员都有资格。

Rust项目中的所有团队最终必须隶属于至少一个顶层团队。对于目前没有母队的团队，本RFC建立了[孵化器团队](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#the-launching-pad-top-level-team)作为其临时母队，来确保所有团队都有理事会代表。

代表有[任期](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#term-limits)。[每个团队的代表人数也有限制](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#limits-on-representatives-from-a-single-companyentity)。各团队应[在代表缺席时派出候补代表](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#alternates-and-forgoing-representation)。

## 理事会的决策过程

[[全文]](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#the-councils-decision-making-process)

理事会[既做事务性决策也做政策决策](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#operational-vs-policy-decisions)。默认情况下，理事会在做出所有决策时都采用[众人认同的决策程序](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#the-consent-decision-making-process)，询问各代表的反对票而无需各代表明确投出赞同票。最低[决策批准标准](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#approval-criteria)要求，必须达到规定人数，且必须达到规定的时间以便代表们能够了解提案。

利用公共政策程序，理事会可以[为不同类别的计划制定决策程序](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#modifying-and-tuning-the-decision-making-process)。理事会的[议程和未完成项目](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#agenda-and-backlog)是其处理项目成员所提出的问题的主要渠道。所有的政策决定都应该有[评估日期](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#feedback-and-evaluation)。

## 决策的透明度与监督

[[全文]](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#transparency-and-oversight-for-decision-making)

领导理事会的不同类型的决策需要不同程度的透明度和监督。

某些事务性决策可以[由理事会内部作出](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#decisions-that-the-council-may-make-internally)，并允许事后对决定决策反馈。有些决策[必须私下作出](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#decisions-that-the-council-must-necessarily-make-privately)，因为它们涉及到个人或其他实体的隐私细节。公开这些细节会对这些个人或实体产生负面影响（如安全）和对项目产生负面影响（降低信任度）。[所有其他决策必须公开](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#decisions-that-the-council-must-make-via-public-proposal)并允许对决策进行事前反馈。

理事会代表不得参与或影响与其本人有[利益冲突](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#conflicts-of-interest)的决定。理事会必须批准[顶层团队对职权的扩大，并可以调整（除调解团队外）顶层团队的职权](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#determining-and-changing-team-purviews)。

## 监督和问责机制

[[全文]](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#mechanisms-for-oversight-and-accountability)

理事会必须[公开确保始终达到更广泛项目和社区对理事会的期望](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#ensuring-the-council-is-accountable)。

理事会代表应在各个代表之间以及与各自所属顶层团队之间[进行定期反馈](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#ensuring-council-representatives-are-accountable)，来回顾他们身为代表的职责履行情况。

理事会也是一种[团队共同对彼此和项目负责](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#ensuring-teams-are-accountable)的方式。

## 调解、分歧和冲突

[[全文]](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#moderation-disagreements-and-conflicts)

团队应尽可能尝试独自解决分歧，[必要时由理事会协助](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#disagreements-among-teams)。涉及团队或项目成员的冲突[应尽快提交给调解团队](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#conflicts-involving-teams-or-project-members)。

调解团队必须保留一份[“临时调解人”](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#contingent-moderators)的公开名单。临时调解人可以在[审核过程](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#audits)中与调解团队合作，以确定调解团队是否遵循了文件规定的政策和程序。理事会成员可以发起审核，但理事会不会看到私人调解信息。

作为绝对的最后手段，理事会和调解团队均[可以选择同时解散两个团队](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#last-resort-accountability)。此时，团队选择新的代表，而临时调解人成为临时调解团队并选择继任者。

在[涉及项目成员的调解案件](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#moderation-actions-involving-project-members)中，任何一方都可以要求进行审核。涉及[理事会代表](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#conflicts-involving-council-representatives)或[调解团队成员](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#conflicts-involving-moderation-team-members)的调解案件有额外的监督和问责措施。

## 本RFC的批准

[[全文]](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#ratification-of-this-rfc)

自2021年11月以来，以下小组构成了项目事实上的领导层：核心团队的所有成员、调解团队的所有成员、Rust基金会董事会的所有项目代表以及所有“顶层”团队的负责人：
- 编译器
- Crates.io
- 开发工具
- 基础设施
- 语言
- 库
- 调解（已在前文包含）。
- 发布

本RFC将使用标准的RFC流程进行审批。审批的团队是实际领导小组的所有成员。此小组也应代表项目内其他成员将反对意见提出；特别是团队负责人应从各自的团队和子团队中对反馈意见进行搜集。

[呈现版](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md)