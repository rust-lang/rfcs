領導理事會。PR說明和RFC摘要

本 RFC 由 @jntrnr（核心成員）、@joshtriplett（語言團隊負責人）、@khionu（審核團隊）、@Mark-Simulacrum（核心專案主管，發佈負責人）、@rylev（核心專案主管）、@technetos（審核團隊）和 @yaahc（合作專案主管）共同撰寫。

非常感謝 「領導層交流」的所有成員和更大範圍的Rust專案的初步審查和回饋。

本 RFC 建立了一個繼承核心團隊的領導委員會。理事會將大部分權力下放給各團隊。

> **注意**。 此摘要對 RFC 進行了概述，但它不具有權威性。

# 程序性資訊

## 討論

關於對此 PR 的討論，請使用 [此 Zulip 回饋討論串]（https://rust-lang.zulipchat.com/#narrow/stream/369838-rfc-leadership-council-feedback)。

## 翻譯

本 RFC 的官方版本是英文版。然而，為了幫助人們廣泛理解 Rust 的管理架構和政策，我們已經開始將所計劃的管理架構和政策翻譯成其他語言。具體來說，根據 [Rust 調查數據]（https://blog.rust-lang.org/2022/02/15/Rust-Survey-2021.html）中認為非英語交流會有説明的被調查物件使用最多的語種，我們將完成並立即發佈以下語種的譯版（非官方性）：

- 中文（簡體）
- 中文（繁體）
- 日語
- 韓語
- 俄語

完成這些翻譯後，我們將在這裡發佈相關連結。請注意，這並不一定意味著我們會處理非英語評論。未來的任何翻譯計劃將由理事會決定，而非此小組。如果您對這些翻譯有建議或意見，歡迎您給我們任何回饋。 我們將在未來翻譯計劃方面參考您的回饋。

## 補充文檔

本 RFC 包括補充文本檔。請 [在此] 查看子目錄 （https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council/）。

-----

# RFC 摘要

## 動機

[[全文]] (https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#motivation)

Rust的管理架構是將多數決策給予適切的團隊處理。然而，過量的工作不屬於任何既定團隊的職權。

歷史上來說，核心團隊曾發現過不屬於團隊職權範圍的重要工作，但他們依然試圖親自完成它們。然而，將這兩種活動交給同一個團隊並無法使團隊擴展，反而會導致工作過度造成精疲力盡。  

本 RFC 建立的領導理事會將著重確定團隊職權之外的工作及其優先次序。理事會將對這些工作進行委託而非親自完成它們。理事會還能以跨團隊工作、規劃和項目的長期成功等為目標，成為團隊之間的協調、組織和問責機構。 

本 RFC 還建立了理事會全體、理事會成員個人、審核團隊、專案團隊和專案成員之間的監督和問責機制。

## 職責、期望和對理事會的限制

[[全文]] (https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#duties-expectations-and-constraints-on-the-council)

理事會將確定、優先處理和追蹤因歸屬不明而未完成的工作，並將這些工作委託給某團隊（新團隊或臨時團隊 ）。在某些時候，理事會可以在沒有明確責任方的情況下決定**緊急**的事項。

理事會還會協調因專案而導致的團隊、結構或流程的變化，確保高層負責，並設立 Rust 專案的官方職位。

## 理事會的結構

[[全文]](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#structure-of-the-council)

理事會由一組團隊 [理事會代表]（https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#candidate-criteria）組成， 他們分別代表一個一級團隊及其子團隊。

每個 [一級團隊]（https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#top-level-teams）通過其各自的選擇程序指定一名代表。一級團隊或其子團隊的任何成員都有資格爭取。

Rust專案中的所有團隊最終必須隸屬於至少一個一級團隊。對於目前沒有母隊的團隊，本RFC建立了[「啟動台 」團隊]（https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#the-launching-pad-top-level-team）作為其臨時母隊，以確保所有團隊都有理事會代表。

理事會代表有[任期限制]（https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#term-limits）。[一個團隊的理事會代表人數也有限制] (https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#limits-on-representatives-from-a-single-companyentity)。各團隊應[在理事會代表缺席時派出候補代表] (https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#alternates-and-forgoing-representation)。

## 理事會的決策過程

[[全文]]（https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#the-councils-decision-making-process）

理事會同時做出[業務和決策]（https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#operational-vs-policy-decisions)。預設情況下，理事會對所有決定都採用[共識決策流程]（https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#the-consent-decision-making-process)，各理事會代表將被要求提供反對意見而非明確同意。最低[決策批准標準]（https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#approval-criteria）要求，必須達到規定人數，且理事會代表們必須在規定的時間內瞭解提案。

透過公共政策流程，理事會可以[為不同類別的計劃制定決策流程] (https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#modifying-and-tuning-the-decision-making-process)。理事會的 [議程和未完成專案]（https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#agenda-and-backlog）是其處理專案成員所提出的問題的主要平台。所有政策決定都應該有[評估日期]（ https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#feedback-and-evaluation)。

## 決策的透明度與監督

[[全文]]（https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#transparency-and-oversight-for-decision-making）

領導理事會的不同類型決策需要不同程度的透明度和監督。

有些營運決策可以[由理事會內部作出]（https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#decisions-that-the-council-may-make-internally），並允許事後對其決策給出回饋。有些決策[必須私下作出] (https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#decisions-that-the-council-must-necessarily-make-privately)， 因為它們涉及到個人或其他實體的隱私細節。 公開這些細節會對這些個人或實體（如安全）和專案（降低信任度）產生負面影響。 [所有其他決策必須公開] (https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#decisions-that-the-council-must-make-via-public-proposal) 並允許對決策進行事前回饋。

理事會代表不得參與或影響與其本人有[利益衝突] （ https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#conflicts-of-interest） 的決定。 理事會必須批准[擴大一級團隊的職權，並可以調整一級團隊（除審核團隊外）的職權] (https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#determining-and-changing-team-purviews)。

## 監督和問責機制

[[全文]]（https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#mechanisms-for-oversight-and-accountability）

理事會必須[公開地確保始終達到更廣泛專案和社群對理事會的期望] (https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#ensuring-the-council-is-accountable)。

理事會代表應在各個代表之間以及與各自一級團隊之間的[進行定期回饋]（https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#ensuring-council-representatives-are-accountable），以反思他們身為代表的職責履行情況。

理事會也是一種[團隊共同對彼此和專案負責]（ https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#ensuring-teams-are-accountable）的方式。

## 審核、分歧和衝突

[[全文]] (https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#moderation-disagreements-and-conflicts)

團隊應儘可能嘗試獨自解決分歧，[必要時由理事會協助] (https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#disagreements-among-teams)。涉及團隊或專案成員的衝突[應儘快提交給審核團隊] (https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#conflicts-involving-teams-or-project-members)。 

審核團隊必須保留一份[「審核人代表團」] 的公開名單（https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#contingent-moderators）。審核人代表團可以在[審核過程]（https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#audits）中與審核團隊合作，以確保審核團隊遵循文件規定之政策及流程。理事會成員可以發起審核但無法看到私人審核資訊。

作為絕對的最後手段，理事會或審核團隊[可以選擇同時解散兩個團隊] (https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#last-resort-accountability)。然後，所有團隊將選擇新的理事會代表，而審核人代表團將成為臨時審核團隊並選擇繼任者。

在[涉及專案成員的審核案件]（ https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#moderation-actions-involving-project-members）中，任何一方都可以要求進行審核 。 涉及[理事會代表]（https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#conflicts-involving-council-representatives）或[審核團隊成員]（ https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#conflicts-involving-moderation-team-members）的審核案件有額外的監督和問責措施。

## 本RFC的批准

[[全文]] (https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#ratification-of-this-rfc)

自2021年11月以來，以下團隊為實際項目領導層：核心團隊的所有成員、審核團隊的所有成員、Rust基金會董事會的所有專案代表以及所有「一級」團隊的負責人：
- 編譯器
- Crates.io
- 開發工具
- 基礎架構
- 語言
- 函式庫
- 審核（已包含在上面）
- 發佈

此 RFC 將以標準 RFC 程序審批，由前述實質上的領導層成員來批准。這些成員還應代表專案中其他成員提出異議，更具體來說，團隊負責人應徵求他的團隊和子團隊的回饋。
[[好讀版]]（https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md）