리더십 심의회: PR 설명 및 RFC 개요

이 RFC는 @jntrnr(중심), @joshtriplett(언어팀 리드), @khionu(중재), @Mark-Simulacrum(중심 프로젝트 디렉터, 공개 리드), @rylev(중심 프로젝트 디렉터), @technetos(중재) 및 @yaahc(협업 프로젝트 디렉터)가 공동으로 작성했습니다.

'리더십 대화'의 모든 일원과 모든 Rust Project의 이해당사자 여러분, 일차 통과 검토와 피드백을 주셔서 정말 감사드립니다.

이 RFC는 중심 팀의 후임인 리더십 심의회를 설립합니다. 이 심의회는 팀에게 여러 권한을 위임합니다.

> **참고**: 이 개요는 RFC에 대한 개요를 제공하지만 강제성을 띄지는 않습니다.

# 절차적 정보

## 논의

이 PR에 대한 논의를 위해 [전용 Zulip 스트림]을 활용하세요(https://rust-lang.zulipchat.com/#narrow/stream/369838-rfc-leadership-council-feedback).

## 번역

강제성을 띈 RFC 버전은 영문으로 되어 있습니다. Rust의 거버넌스 구조와 정책을 널리 이해시키기 위해, 당사는 제시된 거버넌스 구조와 정책을 다른 언어로 번역하는 과정을 시작했습니다. 특히, 당사는 설문 응답자가 영어로 된 소통이 도움이 된다고 나타낸 상위 언어에 대한 [Rust 설문 데이터](https://blog.rust-lang.org/2022/02/15/Rust-Survey-2021.html)에 따라 사용할 수 있는 즉시 다음 언어로 된 (강제성이 없는) 번역본을 게시할 것입니다.

- 중국어(간체)
- 중국어(번체)
- 일본어
- 한국어
- 러시아어

당사는 이러한 언어로 된 번역본을 사용할 수 있게 되는 즉시 링크를 연결할 것입니다. 그렇다고 해서 비영어권 언어로 된 댓글을 다룰 준비를 마친 것은 아니오니 이에 유의하십시오. 향후 번역에 대한 일체의 결정은 이 그룹이 아니라 심의회가 내릴 것입니다. 이러한 번역에 대한 피드백이 있는 경우, 저희에게 알려주시면 번역에 대해 향후 결정을 내릴 때 유용하게 사용하겠습니다.

## 보충 파일

이 RFC에는 보충 텍스트 파일을 포함되어 있습니다. [이곳](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council/)에서 하위 디렉터리에 유의하세요.

-----

# RFC 개요

## 동기

[[전문]](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#motivation)

Rust는 대부분의 결정을 적합한 팀에게 위임하는 구조입니다. 하지만 업무의 많은 부분이 확립한 팀의 범위에 속하지 않습니다.

과거에 중심팀에서 팀의 범위를 벗어나는 중요한 업무를 식별한 후 팀 내부에서 이를 수행하려는 시도를 한 일화가 있습니다. 하지만 같은 팀 안에서 두 가지 활동을 수행하는 것은 규모를 확장할 수 없었으며 결국 번아웃에 이르게 되었습니다.

이 RFC에 따라 설립된 리더십 심의회는 팀의 범위를 벗어나는 업무를 식별하고 이를 우선순위에 올리는 것에 집중하빈다. 심의회는 이러한 업무를 직접 수행하지 않고 먼저 위임할 것입니다. 심의회는 또한 팀 간의 조율, 로드맵 및 프로젝트의 장기적인 성공을 돕는 등, 팀 사이를 조정하고 조직하고 책임지는 역할을 합니다.

이 RFC는 또한 심의회 자체와 각 심의회 위원, 중재팀, 프로젝트팀 및 프로젝트 일원 사이의 감독 및 책임성 메커니즘을 확립합니다.

## 심의회의 임무, 기대 사항 및 제한

[[전문]](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#duties-expectations-and-constraints-on-the-council)

심의회는 명확한 주인의식의 부재로 인해 완수되지 않는 작업을 식별하여 우선순위로 정하고 추적합니다. 심의회는 이러한 (새롭거나 임시적일 수 있는) 작업을 팀에게 위임합니다. 일부 상황에서는 명확한 책임자가 없는 *급박한* 사안에 대해 결정을 내릴 수 있습니다.

심의회는 또한 프로젝트 전반에 걸친 변화를 팀, 구조 또는 과정과 조율을 돕고 고위 팀으로 하여금 책임을 지게 하며 Rush Project의 공식 입장을 확립합니다.

## 심의회 구조

[[전문]](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#structure-of-the-council)

심의회는 고위 팀과 그 하위 팀을 각각 대표하는 여러 팀 [대표](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#candidate-criteria)로 구성되어 있습니다.

각 [고위 팀](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#top-level-teams)은 선임 과정을 거쳐 대표를 한 명 위임합니다. 고위 팀이나 그 하위 그룹의 일원이라면 누구나 자격을 충족합니다.

Rust Project의 모든 팀은 궁극적으로 최소 한 개의 고위 팀에 속합니다. 현재 상위 팀이 없는 팀에 대하여, 이 RFC는 임시 소속으로서 ['런칭 패드'팀](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#the-launching-pad-top-level-team)을 확립합니다. 심의회에서 모든 팀을 대표할 수 있도록 하기 위함입니다.

대표들은 [제한 조건](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#term-limits)을 적용받습니다. [엔터티의 대표 수 제한](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#limits-on-representatives-from-a-single-companyentity)도 있습니다. 팀은 [부재 시에 대비해 대안을 마련해 제공](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#alternates-and-forgoing-representation)해야 합니다.

## 심의회의 의사 결정 과정

[[전문]](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#the-councils-decision-making-process)

심의회는 [운영 및 정책 결정](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#operational-vs-policy-decisions) 모두를 내립니다. 심의회는 기본적으로 모든 결정에 있어 대표자로 하여금 명시적인 승인이 아닌 반대 의사를 묻는 방식인 [동개 동의 의사 결정 과정](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#the-consent-decision-making-process)을 활용합니다. 최소 [의사 결정 승인 요건](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#approval-criteria)으로서 정족수가 필요하며, 대표자로 하여금 제안 사항을 고려할 충분한 시간을 허용합니다.

심의회는 공개 정책 과정을 활용해 [여러 분류의 결정에 대해 다양한 의사 결정 과정](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#modifying-and-tuning-the-decision-making-process)을 확립할 수 있습니다. 심의회의 [안건 및 백로그](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#agenda-and-backlog)는 프로젝트 일원이 문제를 제시하는 기본적인 인터페이스입니다. 모든 정책 결정에는 [평가 날짜](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#feedback-and-evaluation)가 포함되어야 합니다.

## 의사 결정의 책임성 및 감독

[[전문]](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#transparency-and-oversight-for-decision-making)

리더십 심의회가 다루는 여러 유형의 결정에는 여러 수준의 투명성 및 감독이 필요합니다.

일부 유형의 운영 관련 결정은 [심의회가 내부적으로](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#decisions-that-the-council-may-make-internally) 결정할 수 있으며, 그러한 결정에 대한 추후 피드백을 허용합니다. 개인과 다른 당사자에 대해 공개되지 않은 정보가 있으며 이러한 정보가 공개될 경우 해당 개인 또는 당사자(안전 등) 및 프로젝트(신뢰도 하락) 모두에게 부정적인 영향이 우려되는 경우, 일부 결정은 [비공개로 다루어야 합니다](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#decisions-that-the-council-must-necessarily-make-privately). [다른 모든 결정은 공개적으로 다루고](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#decisions-that-the-council-must-make-via-public-proposal) 사전에 해당 결정에 대한 피드백을 허용해야 합니다.

심의회 대표자는 본인이 [이해상충](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#conflicts-of-interest)이 있는 결정에 참여하거나 영향력을 행사해서는 안 됩니다. 심의회는 [고위 팀의 권한 확장을 승인해야 며, (중재 팀이 아닌) 해당 고위 팀의 권한을 조정할 수 있습니다](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#determining-and-changing-team-purviews).

## 감독과 책임성 메커니즘

즘[[전문]](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#mechanisms-for-oversight-and-accountability)

심의회는 [프로젝트의 이해당사자와 커뮤니티가 심의회에게 기대하는 바를 지속적으로 충족하고 있음을 공개적으로 다루어야 합니다](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#ensuring-the-council-is-accountable).

심의회 대표자는 대표자로서 맡은 책임을 얼마나 잘 이행하고 있는지 반영하기 위해 서로, 그리고 관련 고위 팀과 [정기적인 피드백에 참여해야 합니다](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#ensuring-council-representatives-are-accountable).

심의회는 또한 팀들이 [팀과 프로젝트와 관련하여 서로 신뢰를 유지할 수 있도록](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#ensuring-teams-are-accountable) 돕는 역할을 합니다.

## 중재, 반대 의견 및 충돌

[[전문]](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#moderation-disagreements-and-conflicts)

가능한 경우, 팀은 [필요에 따라 심의회의 지원을 받으며](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#disagreements-among-teams) 반대 의견을 직접 해결하려는 노력을 기울여야 합니다. 팀이나 프로젝트 일원이 포함된 충돌은 최대한 빨리 [중재팀에 보고해야 합니다](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#conflicts-involving-teams-or-project-members).

중재팀은 ['조건부 중재자'(https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#contingent-moderators)의 공개 내역을 유지해야 합니다. 조건부 중재자는 중재팀이 문서화된 정책과 절차를 따르고 있는지 확인하기 위해 [감사 과정](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#audits)에서 중재팀과 협업할 수 있습니다. 심의회 위원은 감사를 시작할 수 있으나 심의회는 비공개 중재 정보를 절대 볼 수 없습니다.

절대적인 최후의 수단으로서 심의회 또는 중재팀이 [두 팀을 동시에 해산하기로 결정할 수 있습니다](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#last-resort-accountability). 그 다음 팀에서는 새로운 대표자를 선임하고 조건부 중재자가 임시 중재팀이 되어 후임을 선정합니다.

[프로젝트 일원이 연관된 중재 건](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#moderation-actions-involving-project-members)에서는 어떤 당사자이든 감사를 요청할 수 있습니다. [심의회 대표자](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#conflicts-involving-council-representatives) 또는 [중재팀 일원](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#conflicts-involving-moderation-team-members)이 연관된 중재 건에는 추가적인 감독 및 책임성 조치가 있습니다.

## 이 RFC의 비준

[[전문]](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#ratification-of-this-rfc)

2021년 11월 이래로 모든 중심팀 일원, 모든 중재팀 일원, Rust Foundation 위원회의 모든 프로젝트 대표자 및 모든 '고위' 팀 선임은 사실상 프로젝트 리더십의 역할을 해왔습니다.
- 컴파일러
- Creates.io
- 개발 도구
- 인프라
- 언어
- 라이브러리
- 중재(이미 상기 포함)
- 공개

이 RFC는 표준 RFC 과정을 통해 비준을 받을 것이며 이를 승인하는 팀은 이러한 사실상의 리더십 그룹에 속한 모든 일원이 됩니다. 이 그룹은 프로젝트의 다른 일원을 대신하여 반대 의견을 표의할 수 있으며, 특히 팀 리더는 자신의 팀과 하위 팀의 피드백을 요청해야 합니다.

[렌더링됨](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md)
