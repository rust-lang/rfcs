リーダーシップ・カウンシル：PR説明とRFC要約

このRFCは@jntrnr (コア)、@joshtriplett (言語チームリード)、@khionu(モデレーション)、 @Mark-Simulacrum(コアプロジェクトディレクター、リリースリード)、@rylev(コアプロジェクトディレクター), @technetos (モデレーション)、@yaahc(コラボレーションプロジェクトディレクター)により共同で著作されました。

「リーダーシップチャット」のすべてのメンバーおよび初期のレビュー・フィードバックに関して Rust プロジェクトの多くの皆様に感謝します。

このRFCはリーダーシップ・カウンシルをコアチームの後継者として制定するものです。カウンシルはその権限の多くを諸チームに付与しています。

> **注意**:この要約はRFCの概要を提供していますが、正式なものではありません。

# 手続きに関する情報

## 議論

このPRの議論に関しては、[専用のZulipストリーム](https://rust-lang.zulipchat.com/#narrow/stream/369838-rfc-leadership-council-feedback)を使用してください。

## 翻訳

このRFCの正式版は英語版です。しかしながら、Rustのガバナンス体制とポリシーを幅広く理解してもらうために、提案されるガバナンス体制とポリシーをその他の言語に翻訳する過程を開始しました。特に、[Rust アンケートデータ](https://blog.rust-lang.org/2022/02/15/Rust-Survey-2021.html)に基づき、英語以外でのコミュニケーションができれば助かるという応答があった上位言語について、できあがり次第、以下の言語で翻訳版（正式版ではない）を掲載します。

- 中国語（簡体字）
- 中国語（繁体字）
- 日本語
- 韓国語
- ロシア語

できあがり次第翻訳版へのリンクを追加します。しかし、英語以外の言語で書かれたコメントに対応できるとは限らないことをご理解ください。この先、翻訳に関する決定はカウンシルに任されており、このグループに決定権はありません。これら翻訳版にフィードバックがあれば、翻訳に関して将来何かを決定する際に参考するための情報になるため、お知らせください。

## 補助的ファイル

このRFCには補助的なテキストファイルが含まれます。[こちら](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council/)のサブディレクトリをご参照ください。

---

＃RFC要約

## モチベーション

[[フルテキスト]](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#motivation)

Rustの体制では決定のほとんどが適切なチームに委任されます。しかしながら、既存チームの範囲外にある仕事が多くあります。

歴史的には、コアチームがチーム範囲外にある重要な仕事を認知し、また自分たちでその仕事をこなそうとしてきました。しかしながら、この両方の活動を同じチームで行おうとすることは、スケールせず、バーンアウトという結果になってしまいました。

このRFCで設立されるリーダーシップ・カウンシルでは、チームの範囲外にある仕事を認識し優先化することに焦点を当てます。カウンシルは基本的に、その仕事を自分たちでするのではなく、委任します。またカウンシルは、コーディネートする組織として、またチーム間でアカウンタビリティ、例えばチーム間協力、ロードマップ、プロジェクトの長期的成功などの説明責任のパートナーとして機能します。

このRFCはカウンシル全体と各メンバー、調整チーム、プロジェクトチーム、プロジェクトメンバーの間で、監視とアカウンタビリティの仕組みを確立します。

## カウンシルの責任、期待、制限

[[フルテキスト]](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#duties-expectations-and-constraints-on-the-council)

明確なオーナーシップが欠けているために、遂行されない仕事を認識し、優先度付け、追跡します。その仕事を諸チーム（新規・一時的なチームであることもある）に委任します。明確なオーナーのいない*緊急*事項は、カウンシルが決定する場合もあります。

またプロジェクト全体の変更をチームや体制、プロセスに振り分けて、トップレベルのチームが説明可能（アカウンタブル）であるように確認し、Rust プロジェクトの公的な地位を確立します。

## カウンシルの体制

[[フルテキスト]](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#structure-of-the-council)

カウンシルは一連のチーム[代表者](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#candidate-criteria)からなっており、各チームは、トップレベルのチーム一つと、その下に複数のサブチームをもち、それらを代表しています。

各[トップレベルチーム](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#top-level-teams)は、チームが選択する過程により、代表者を指名します。トップレベルチームのメンバーや、そのサブチームのメンバーは誰でも代表者になる資格があります。

Rustプロジェクトの全チームは、最終的に少なくとも一つのトップレベルのチームの下に置かれていなければなりません。親チームが現在ないチームに関しては、このRFCが[「ローンチパッド」チーム](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#the-launching-pad-top-level-team)を一時的なホームとして設置します。これによって全チームがカウンシル上で代表者がいるようになります。

代表者には[期間の限度](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#term-limits)があります。[1エンティティからの代表者の数には限度](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#limits-on-representatives-from-a-single-companyentity)があります。チームは[不在の場合は代替を提供](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#alternates-and-forgoing-representation)しなければなりません。

## カウンシルの決定過程

[[フルテキスト]](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#the-councils-decision-making-process)

カウンシルは[オペレーションの決定、ポリシーの決定](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#operational-vs-policy-decisions)の両方を行います。デフォルトで、カウンシルは、全ての意思決定に関し、代表者が明示的な承認ではなく異議を求められる[公の同意に基づく意思決定過程](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#the-consent-decision-making-process)を使用します。最小限の[意思決定承認基準](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#approval-criteria)は定足数が必要であり、代表者が提案を確認するための時間を必要とします。

公のポリシープロセスを使用することで、カウンシルは[決定事項の階級によって異なる意思決定プロセス](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#modifying-and-tuning-the-decision-making-process)を確立することができます。カウンシルの[アジェンダとバックログ](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#agenda-and-backlog)
はプロジェクトメンバーによって提起された問題のための第一のインターフェースです。全ポリシーの決断には[評価期間](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#feedback-and-evaluation)がなければなりません。

## 意思決定の透明性と監視

[[フルテキスト]](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#transparency-and-oversight-for-decision-making)

リーダーシップ・カウンシルで行われる様々な意思決定には色々なレベルの透明性と監視が必要になります。

オペレーション上の決定の中には[カウンシルが内部で]行うことが可能なものもあり(https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#decisions-that-the-council-may-make-internally)、後ほどその決定に関してフィードバックを求めることになります。決定の中には[プライベートで行われなければならない]ものがあり(https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#decisions-that-the-council-must-necessarily-make-privately)、その理由は個人やその他エンティティの私的な詳細に関わるものであり、その詳細を公にすることが個人やエンティティ（例: 安全性）またはプロジェクト に（例:信用を損なう）にネガティブな影響を与える可能性があるからです。[その他の全決定は公に行われなければならず](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#decisions-that-the-council-must-make-via-public-proposal)決定に関するフィードバックを前もって受けることができるようにします。

カウンシルの代表者は[利益相反]がある決定に参加したり影響を与えること(https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#conflicts-of-interest)をしてはなりません。カウンシルはトップレベルチームの範囲の拡大を承認しなければならず、またトップレベルチームの範囲を調整することができます(モデレーションチーム以外に)](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#determining-and-changing-team-purviews)。

## 監視とアカウンタビリティの仕組み

[[フルテキスト]](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#mechanisms-for-oversight-and-accountability)

カウンシルは、[プロジェクト全体とコミュティのカウンシルに対する期待が一貫して満たされるよう、公に確実にしなければ](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#ensuring-the-council-is-accountable)なりません。

カウンシルの代表者はお互いに、またトップレベルのチームと[定期的にフィードバックに参加し](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#ensuring-council-representatives-are-accountable)代表者としての責任をどの程度良く満たしているかについて見返す必要があります。

カウンシルはまた[チームがお互いに、またプロジェクトに対して説明責任を持つ](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#ensuring-teams-are-accountable)ための道具として役割を果たします。

## モデレーション、意見の相違と対立

[[フルテキスト]](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#moderation-disagreements-and-conflicts)

可能であれば、チームは自分たちで[必要ならばカウンシルの助けを借りて](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#disagreements-among-teams)意見の相違の解消を試みるべきです。チームやプロジェクトメンバーに関わる対立はできるだけ早く[モデレーションチームに相談](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#conflicts-involving-teams-or-project-members)すべきです。

モデレーションチームは["臨時モデレーター"](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#contingent-moderators)の公開リストを管理していなければなりません。臨時モデレーターは[監査プロセス](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#audits)でモデレーションチームがドキュメント化されたポリシーと手続きに沿っているか確認するためにモデレーションチームと協力して働くことができます。カウンシルメンバーは監査を開始することができますが、カウンシルがプライベートのモデレーション情報を見ることは決してありません。

絶対的な最終手段として、カウンシルまたはモデレーションチームが[両チームを同時に解消することを選択](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#last-resort-accountability)することもあります。チームはそれから新たな代表者を選択し、臨時モデレーターは一時的モデレーターになり、後継者を選択します。

[プロジェクトメンバーを含むモデレーションケース](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#moderation-actions-involving-project-members)の場合、当事者には監査人が必要になることがあります。[カウンシル代表者](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#conflicts-involving-council-representatives)を含むモデレーションケースや[モデレーション チームメンバー](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#conflicts-involving-moderation-team-members)の場合は付加的な監視やアカウンタビリティの方法があります。

## このRFCの批准

[[フルテキスト]](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#ratification-of-this-rfc)

2021年11月以来、コアチームの全メンバー、モデレーションチームの全メンバー、Rust Foundation理事会のプロジェクト代表者全員、「トップレベル」全チームのリードが事実上のプロジェクトリーダーシップとして行動しています:

- コンパイラー
- Crates.io
- 開発ツール
- インフラストラクチャ
- 言語
- ライブラリ
- モデレーション（既に上記に含まれる）
- リリース

このRFCは標準的なRFCプロセスを使用し、事実上のリーダーシップグループの全メンバーを承認チームとして、批准されます。このグループはプロジェクトの他のメンバーに代わって異議を申し立てるべきです。特に、チームリードはそのチームやサブチームからフィードバックを請うべきです。

[レンダリング済み](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md)