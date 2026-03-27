**Руководящий совет – описание для PR и обзор RFC**

Соавторами данного RFC являются @jntrnr (Основная команда), @joshtriplett (Руководитель языковой команды), @khionu (Модерация), @Mark-Simulacrum (Директор основного проекта, Руководитель релизной команды), @rylev (Директор основного проекта), @technetos (Модерация) и @yaahc (Директор совместных проектов).

Благодарим всех участников "чата руководства" и Проект Rust в целом за многочисленные предварительные рецензии и обратную связь.

В данном RFC устанавливается роль Руководящего совета в качестве преемника основной команды. Совет делегирует основную часть своих полномочий другим командам.

> **Примечание**: В настоящем обзоре предоставляется краткое описание RFC, но оно не имеет официальной силы.

# Процедурная информация

## Обсуждения

Для обсуждения данного PR просим воспользоваться [специальным Zulip-стримом](https://rust-lang.zulipchat.com/#narrow/stream/369838-rfc-leadership-council-feedback).

## Переводы

Официальной версией данного RFC является версия на английском языке. Тем не менее, в целях широкого распространения информации об управленческой структуре и политиках Rust мы начали процесс перевода описания предлагаемой управленческой структуры и политик на другие языки. В частности, на основании [данных опроса Rust](https://blog.rust-lang.org/2022/02/15/Rust-Survey-2021.html) относительно наиболее популярных языков, указанных респондентами опроса в качестве предпочтительного в будущем средства коммуникации в дополнение к английскому, мы предложим (не имеющие официальной силы) переводы на следующие языки, как только они будут готовы:

- китайский (упрощенный)
- китайский (традиционный)
- японский
- корейский
- русский

Мы разместим здесь ссылки на эти переводы, как только они будут готовы. Обращаем ваше внимание на то, что это не обязательно означает, что мы будем готовы реагировать на комментарии на других языках, кроме английского. Любые потенциальные решения относительно перевода будут зависеть не от этой группы, а от Совета. Если вы захотите оставить отзыв о переводах, пожалуйста, поделитесь им с нами, чтобы мы могли учесть его при принятии дальнейших решений, связанных с переводами.

## Дополнительные файлы

Настоящий RFC включает в себя дополнительные текстовые файлы. См. подкаталог [здесь](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council/).

-----

# Обзор RFC

## Мотивация

[[полный текст]](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#motivation)

Структура Rust позволяет делегировать большинство решений соответствующим командам. Тем не менее, существует большой объем работ, который не входит в компетенцию ни одной из созданных команд.

Исторически сложилось, что основная команда и определяла важные работы, которые не входили в компетенцию отдельных команд, и пыталась выполнять эти работы своими силами. Однако совмещение этих двух задач в рамках одной команды не привело к масштабированию, но повлекло за собой выгорание.

Руководящий совет, создаваемый настоящим RFC, занимается выявлением и приоритизацией работ за пределами компетенции отдельных команд. Совет преимущественно не выполняет эту работу своими силами, а делегирует ее другим. Совет также может выступать в качестве координирующего, организующего и отчетного органа, работающего с командами, направляющего совместные усилия нескольких команд, координирующего планы действий и содействующего общему успеху Проекта.

В рамках данного RFC также устанавливаются механизмы надзора и подотчетности между Советом в целом, отдельными членами Совета, командой модерации, командами Проекта и членами Проекта.

## Обязанности, ожидания и ограничения, применимые к Совету

[[полный текст]](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#duties-expectations-and-constraints-on-the-council)

Совет выявляет и определяет приоритеты, а также отслеживает выполнение работы, которая остается не сделанной в силу отсутствия четких сфер ответственности. Он делегирует эту работу командам (причем они могут быть новыми или временными). В некоторых случаях он вправе разрешать *срочные* вопросы, не имеющие четкой сферы ответственности.

Совет также координирует общепроектные изменения в командах, структурах или процессах, обеспечивает подотчетность команд верхнего уровня и устанавливает официальные позиции Проекта Rust.

## Структура Совета

[[полный текст]](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#structure-of-the-council)

В состав Совета входят [представители команд](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#candidate-criteria), каждый из которых представляет одну из команд верхнего уровня и ее под-команд.

Каждая [команда верхнего уровня](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#top-level-teams) назначает своего представителя, пользуясь любой процедурой по своему выбору. Представителем может стать любой из членов команды верхнего уровня или любой из ее под-команд.

Все команды в рамках Проекта Rust должны в конечном итоге быть подотчетны как минимум одной из команд верхнего уровня. Для команд, у которых в настоящее время нет вышестоящей команды, настоящий RFC создает [команду "запуска"](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#the-launching-pad-top-level-team) в качестве временной аффилиации. Таким образом, все команды получают представительство в Совете.

Срок службы представителей [ограничен](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#term-limits). Существуют [ограничения на число представителей одной структуры](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#limits-on-representatives-from-a-single-companyentity). Команды должны [назначать заместителей на случай отсутствия представителя](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#alternates-and-forgoing-representation).

## Процедура принятия решений в Совете

[[полный текст]](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#the-councils-decision-making-process)

Совет принимает как [оперативные, так и относящиеся к политике решения](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#operational-vs-policy-decisions). По умолчанию Совет использует [процедуру принятия решений, основанную на общественном согласии](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#the-consent-decision-making-process) для принятия всех решений, относительно которых представителям предлагается высказать свои возражения, а не прямое одобрение. Минимальные [критерии принятия решений](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#approval-criteria) включают в себя наличие кворума и обеспечение представителям достаточного времени для ознакомления с предложением.

Используя публичную процедуру формирования политики, Совет может устанавливать [различные процедуры принятия решений для определенных категорий решений](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#modifying-and-tuning-the-decision-making-process). [Повестка и архив](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#agenda-and-backlog) Совета являются его основным интерфейсом для вопросов, поднимаемых участниками Проекта. Всем решениям относительно политики должны присваиваться [даты оценки](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#feedback-and-evaluation).

## Прозрачность и надзор за принятием решений

[[полный текст]](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#transparency-and-oversight-for-decision-making)

Различные виды решений, принимаемых Руководящим советом, нуждаются в различных уровнях прозрачности и надзора.

Некоторые виды операционных решений могут приниматься [внутри Совета](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#decisions-that-the-council-may-make-internally) с сохранением возможности получения в будущем обратной связи. Некоторые решения [должны приниматься в частном порядке](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#decisions-that-the-council-must-necessarily-make-privately), поскольку они связаны с конфиденциальными данными физических или иных лиц, обнародование которых имело бы негативные последствия для данных физических или иных лиц. [Все остальные решения должны приниматься публично](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#decisions-that-the-council-must-make-via-public-proposal), при условии предварительного получения обратной связи.

Представитель Совета не вправе принимать участие в принятии решения или оказывать влияние на принятие решения при наличии [конфликта интересов](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#conflicts-of-interest). Совет обязан утверждать [расширение сфер компетенции команды верхнего уровня и может корректировать сферы компетенции команд верхнего уровня (кроме команды модерации)](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#determining-and-changing-team-purviews).

## Механизмы надзора и отчетности

[[полный текст]](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#mechanisms-for-oversight-and-accountability)

Совет обязан [публично обеспечивать постоянное соответствие более общим требованиям Проекта и сообщества, применимым к деятельности Совета](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#ensuring-the-council-is-accountable).

Представители Совета [должны регулярно предоставлять обратную связь](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#ensuring-council-representatives-are-accountable) друг другу и своим соответствующим командам верхнего уровня относительно исполнения своих обязанностей в качестве представителей.

Совет также выступает средством [взаимного привлечения команд к ответственности по отношению друг к другу и к Проекту](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#ensuring-teams-are-accountable).

## Модерация, разногласия и конфликты

[[полный текст]](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#moderation-disagreements-and-conflicts)

По возможности команды должны пытаться разрешать разногласия собственными силами, [при необходимости прибегая к помощи Совета](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#disagreements-among-teams). Конфликты с участием команд или участников Проекта [доводятся до сведения команды модерации](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#conflicts-involving-teams-or-project-members) в кратчайшие сроки.

Команда модерации ведет публичный список ["контингента модераторов"](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#contingent-moderators). Контингент модераторов может работать совместно с командой модерации в рамках [процесса аудита](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#audits), чтобы определить, следовала ли команда модерации задокументированным политикам и процедурам. Члены Совета вправе инициировать аудиторские проверки, но конфиденциальные данные по модерации никогда не доводятся до сведения Совета.

В качестве самого крайнего средства либо Совет, либо команда модерации [может принять решение об одновременном роспуске обеих команд](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#last-resort-accountability). После этого команды выбирают новых представителей, и контингент модераторов становится временной командой модерации и выбирают кандидатов себе на смену.

[В случаях модерации с участием членов Проекта](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#moderation-actions-involving-project-members) любая сторона может запросить аудит. В случаях модерации с участием [представителей Совета](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#conflicts-involving-council-representatives) или [членов команды модераторов](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#conflicts-involving-moderation-team-members) предусмотрены дополнительные меры надзора и ответственности.

## Ратификация данного RFC

[[полный текст]](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md#ratification-of-this-rfc)

С ноября 2021 г. в качестве фактических лидеров Проекта выступает следующая группа: все члены основной команды, все члены команды модерации, все представители Проекта, входящие в состав правления Rust Foundation, а также руководители всех команд "верхнего уровня":

- компилятор
- Crates.io
- инструменты разработки
- инфраструктура
- язык
- библиотека
- модерация (уже включена выше) - релиз

Данный RFC подлежит ратификации с использованием стандартной процедуры RFC, причем утверждающая группа состоит из всех членов данной фактической руководящей группы. Эта группа также должна выдвигать возражения от имени других участников Проекта; в частности, руководители команд должны запрашивать обратную связь от своих команд и под-команд.

[Визуализация](https://github.com/rust-lang/rfc-leadership-council/blob/main/text/3392-leadership-council.md)
