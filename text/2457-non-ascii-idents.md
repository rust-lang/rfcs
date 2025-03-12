- Feature Name: `non_ascii_idents`
- Start Date: 2018-06-03
- RFC PR: [rust-lang/rfcs#2457](https://github.com/rust-lang/rfcs/pull/2457)
- Rust Issue: [rust-lang/rust#55467](https://github.com/rust-lang/rust/issues/55467)

# Summary
[summary]: #summary

Allow non-ASCII letters (such as accented characters, Cyrillic, Greek, Kanji, etc.) in Rust identifiers.

# Motivation
[motivation]: #motivation

Writing code using domain-specific terminology simplifies implementation and discussion as opposed to translating words from the project requirements. When the code is only intended for a limited audience such as with in-house projects or in teaching it can be beneficial to write code in the group's language as it boosts communication and helps people not fluent in English to participate and write Rust code themselves.

The rationale from [PEP 3131] nicely explains it:

> ~~Python~~ *Rust* code is written by many people in the world who are not familiar with the English language, or even well-acquainted with the Latin writing system. Such developers often desire to define classes and functions with names in their native languages, rather than having to come up with an (often incorrect) English translation of the concept they want to name. By using identifiers in their native language, code clarity and maintainability of the code among speakers of that language improves.
> 
> For some languages, common transliteration systems exist (in particular, for the Latin-based writing systems). For other languages, users have larger difficulties to use Latin to write their native words.

Additionally some math oriented projects may want to use identifiers closely resembling mathematical writing.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Identifiers include variable names, function and trait names and module names. They start with a letter or an underscore and may be followed by more letters, digits and some connecting punctuation.

Examples of valid identifiers are:

* ASCII letters and digits: `image_width`, `line2`, `Photo`, `el_tren`, `_unused`
* words containing accented characters: `garçon`, `hühnervögel`
* identifiers in other scripts: `Москва`, `東京`, ...

Examples of invalid identifiers are:

* Keywords: `impl`, `fn`, `_` (underscore), ...
* Identifiers starting with numbers or containing "non letters": `42_the_answer`, `third√of7`, `◆◆◆`, ...
* Many Emojis: 🙂, 🦀, 💩, ...

[Composed characters] like those used in the word `ḱṷṓn` can be represented in different ways with Unicode. These different representations are all the same identifier in Rust.

To disallow any Unicode identifiers in a project (for example to ease collaboration or for security reasons) limiting the accepted identifiers to ASCII add this lint to the `lib.rs` or `main.rs` file of your project:

```rust
#![forbid(non_ascii_idents)]
```

Some Unicode character look confusingly similar to each other or even identical like the Latin **A** and the Cyrillic **А**. The compiler may warn you about names that are easy to confuse with keywords, names from the same crate and imported items. If needed (but not recommended) this warning can be silenced with a `#[allow(confusable_idents)]` annotation on the enclosing function or module.

## Usage notes

All code written in the Rust Language Organization (*rustc*, tools, std, common crates) will continue to only use ASCII identifiers and the English language.

For open source crates it is suggested to write them in English and use ASCII-only. An exception can be made if the application domain (e.g. math) benefits from Unicode and the target audience (e.g. for a crate interfacing with Russian passports) is comfortable with the used language and characters. Additionally crates should consider to provide an ASCII-only API.

Private projects can use any script and language the developer(s) desire. It is still a good idea (as with any language feature) not to overdo it.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Identifiers in Rust are based on the [Unicode® Standard Annex #31 Unicode Identifier and Pattern Syntax][UAX31].

Note: The supported Unicode version should be stated in the documentation.

The lexer defines identifiers as:

> **<sup>Lexer:<sup>**  
> IDENTIFIER_OR_KEYWORD:  
> &nbsp;&nbsp; XID_Start&nbsp;XID_Continue<sup>\*</sup>  
> &nbsp;&nbsp; | `_` XID_Continue<sup>*</sup>  
>  
> IDENTIFIER :  
> IDENTIFIER_OR_KEYWORD <sub>*Except a [strict] or [reserved] keyword*</sub>

`XID_Start` and `XID_Continue` are used as defined in the aforementioned standard. The definition of identifiers is forward compatible with each successive release of Unicode as only appropriate new characters are added to the classes but none are removed. We effectively are using UAX 31's default definition of valid identifier, with a tailoring that underscores are included with `XID_Start`. (Note that this allows bare underscores to be identifiers, that is currently also the case with `_` in identifier contexts being a reserved keyword)

Rust lexers normalize identifiers to [NFC][UAX15]. Every API accepting identifiers as strings (such as `proc_macro::Ident::new` normalizes them to NFC and APIs returning them as strings (like `proc_macro::Ident::to_string`) return the normalized form. Procedural and declarative macros receive normalized identifiers in their input as well. This means two identifiers are equal if their NFC forms are equal.

A `non_ascii_idents` lint is added to the compiler. This lint is `allow` by default. The lint checks if any identifier in the current context contains a codepoint with a value equal to or greater than 0x80 (outside ASCII range). Not only locally defined identifiers are checked but also those imported from other crates and modules into the current context.

## Remaining ASCII-only names

Only ASCII identifiers are allowed within an external block and in the signature of a function declared `#[no_mangle]`.
Otherwise an error is reported.

Note: These functions interface with other programming languages
and these may allow different characters or may not apply normalization to identifiers.
As this is a niche use-case it is excluded from this RFC.
A future RFC may lift the restriction.

This RFC keeps out-of-line modules without a `#[path]` attribute ASCII-only.
The allowed character set for names on crates.io is not changed.

Note: This is to avoid dealing with file systems on different systems *right now*.
A future RFC may allow non-ASCII characters after the file system issues are resolved.

## Confusable detection

Rust compilers should detect confusingly similar Unicode identifiers and warn the user about it.

Note: This is *not* a mandatory for all Rust compilers as it requires considerable implementation effort and is not related to the core function of the compiler. It rather is a tool to detect accidental misspellings and intentional homograph attacks.

A new `confusable_idents` lint is added to the compiler. The default setting is `warn`.

Note: The confusable detection is set to `warn` instead of `deny` to enable forward compatibility. The list of confusable characters will be extended in the future and programs that were once valid would fail to compile.

The confusable detection algorithm is based on [Unicode® Technical Standard #39 Unicode Security Mechanisms Section 4 Confusable Detection][TR39Confusable]. For every distinct identifier X execute the function `skeleton(X)`. If there exist two distinct identifiers X and Y in the same crate where `skeleton(X) = skeleton(Y)` report it. The compiler uses the same mechanism to check if an identifier is too similar to a keyword.

Note: A fast way to implement this is to compute `skeleton` for each identifier once and place the result in a hashmap as a key. If one tries to insert a key that already exists check if the two identifiers differ from each other. If so report the two confusable identifiers.

## Exotic codepoint detection

A new `less_used_codepoints` lint is added to the compiler. The default setting is to `warn`.

The lint is triggered by identifiers that contain a codepoint that is not part of the set of "Allowed" codepoints as described by [Unicode® Technical Standard #39 Unicode Security Mechanisms Section 3.1 General Security Profile for Identifiers][TR39Allowed].

Note: New Unicode versions update the set of allowed codepoints. Additionally the compiler authors may decide to allow more codepoints or warn about those that have been found to cause confusion.

For reference, a list of all the code points allowed by this lint can be found [here][unicode-set-allowed], with the script group mentioned on the right.

There are some specific interesting code points that we feel necessary to call out here:

 - `less_used_codepoints` will warn on U+200C ZERO WIDTH NON-JOINER and U+200D ZERO WIDTH JOINER, despite these being useful in the  Perso-Arabic and some Indic scripts. In Indic scripts these characters force different visual forms, which is not very necessary for programming. These have further semantic meaning in Arabic where they can be used to mark prefixes or mixed-script words, which will not crop up so often in programming (we're not able to use `-` in identifiers for marking pre/suffixes in Latin-script identifiers and it's fine). Persian seems to make the most use of these, with some compound words requiring use of these. For now this RFC does not attempt to deal with this and follows the recommendation of the specification, if there is a need for it in the future we can add this for Persian users.
 - `less_used_codepoints` will not warn about U+02BB MODIFIER LETTER TURNED COMMA or U+02BC MODIFIER LETTER APOSTROPHE. These look somewhat like punctuation relevant to Rust's syntax, so they're a bit tricky. However, these code points are important in Ukrainian, Hawaiian, and a bunch of other languages (U+02BB is considered a full-fledged letter in Hawaiian). For now this RFC follows the recommendation of the specification and allows these, however we can change this in the future. The hope is that syntax highlighting is enough to deal with confusions caused by such characters.


## Adjustments to the "bad style" lints

Rust [RFC 0430] establishes naming conventions for Rust ASCII identifiers. The *rustc* compiler includes lints to promote these recommendations.

The following names refer to Unicode character categories:

* `Ll`: Letter, Lowercase
* `Lu`: Letter, Uppercase

These are the three different naming conventions and how their corresponding lints are specified to accommodate non-ASCII codepoints:

* UpperCamelCase/`non_camel_case_types`: The first codepoint must not be in `Ll`. Underscores are not allowed except as a word separator between two codepoints from neither `Lu` or `Ll`.
* snake_case/`non_snake_case`: Must not contain `Lu` codepoints.
* SCREAMING_SNAKE_CASE/`non_upper_case_globals`: Must not contain `Ll` codepoints.

Note: Scripts with upper- and lowercase variants ("bicameral scripts") behave similar to ASCII. Scripts without this distinction ("unicameral scripts") are also usable but all identifiers look the same regardless if they refer to a type, variable or constant. Underscores can be used to separate words in unicameral scripts even in UpperCamelCase contexts.

## Mixed script confusables lint

We keep track of the script groups in use in a document using the comparison heuristics in [Unicode® Technical Standard #39 Unicode Security Mechanisms Section 5.2 Restriction-Level Detection][TR39RestrictionLevel].

We identify lists of code points which are `Allowed` by [UTS 39 section 3.1][TR39Allowed] (i.e., code points not already linted by `less_used_codepoints`) and are "exact" confusables between code points from other `Allowed` scripts. This is stuff like Cyrillic `о` (confusable with Latin `o`), but does not include things like Hebrew `ס` which is somewhat distinguishable from Latin `o`. This list of exact confusables can be modified in the future.

We expect most of these to be between Cyrillic-Latin-Greek and some in Ethiopic-Armenian, but a proper review can be done before stabilization. There are also confusable modifiers between many script.

In a code base, if the _only_ code points from a given script group (aside from `Latin`, `Common`, and `Inherited`) are such exact confusables, lint about it with `mixed_script_confusables` (lint name can be finalized later).

As an implementation note, it may be worth dealing with confusable modifiers via a separate lint check -- if a modifier is from a different (non-`Common`/`Inherited`) script group from the thing preceding it. This has some behavioral differences but should not increase the chance of false positives.

The exception for `Latin` is made because the standard library is Latin-script. It could potentially be removed since a code base using the standard library (or any Latin-using library) is likely to be using enough of it that there will be non-confusable characters in use. (This is in unresolved questions)


## Reusability

The code used for implementing the various lints and checks will be released to crates.io. This includes:

 - Testing validity of an identifier
 - Testing for `less_used_codepoints` ([UTS #39 Section 3.1][TR39Allowed])
 - Script identification and comparison for `mixed_script_confusables`  ([UTS #39 Section 5.2][TR39RestrictionLevel])
 - `skeleton(X)` algorithm for confusable detection ([UTS #39 Section 4][TR39Confusable])

Confusables detection works well when there are other identifiers to compare against, but in some cases there's only one instance of an identifier in the code, and it's compared with user-supplied strings. For example we have crates that use proc macros to expose command line options or REST endpoints. Crates that do things like these can use such algorithms to ensure better error handling; for example if we accidentally end up having an `/арр` endpoint (in Cyrillic) because of a `#[annotation] fn арр()`, visiting `/app` (in Latin) may show a comprehensive error (or pass-through, based on requirements)

## Conformance Statement

* UAX31-C1: The Rust language conforms to the Unicode® Standard Annex #31 for Unicode Version 10.0.0.
* UAX31-C2: It observes the following requirements:
  * UAX31-R1. Default Identifiers: To determine whether a string is an identifier it uses UAX31-D1 with the following profile:
    * Start := XID_Start, plus `_`
    * Continue := XID_Continue
    * Medial := empty
  * UAX31-R1b. Stable Identifiers: Once a string qualifies as an identifier, it does so in all future versions.
  * UAX31-R3. Pattern_White_Space and Pattern_Syntax Characters: Rust only uses characters from these categories for whitespace and syntax. Other characters may or may not be allowed in identifiers.
  * UAX31-R4. Equivalent Normalized Identifiers: All identifiers are normalized according to normalization form C before comparison.



# Drawbacks
[drawbacks]: #drawbacks

* "ASCII is enough for anyone." As source code should be written in English and in English only (source: various people) no characters outside the ASCII range are needed to express identifiers. Therefore support for Unicode identifiers introduces unnecessary complexity to the compiler.
* "Foreign characters are hard to type." Usually computer keyboards provide access to the US-ASCII printable characters and the local language characters. Characters from other scripts are difficult to type, require entering numeric codes or are not available at all. These characters either need to be copy-pasted or entered with an alternative input method.
* "Foreign characters are hard to read." If one is not familiar with the characters used it can be hard to tell them apart (e.g. φ and ψ) and one may not be able refer to the identifiers in an appropriate way (e.g. "loop" and "trident" instead of phi and psi)
* "My favorite terminal/text editor/web browser" has incomplete Unicode support." Even in 2018 some characters are not widely supported in all places where source code is usually displayed.
* Homoglyph attacks are possible. Without confusable detection identifiers can be distinct for the compiler but visually the same. Even with confusable detection there are still similar looking characters that may be confused by the casual reader.

# Rationale and alternatives
[alternatives]: #alternatives

As stated in [Motivation](#motivation) allowing Unicode identifiers outside the ASCII range improves Rusts accessibility for developers not working in English. Especially in teaching and when the application domain vocabulary is not in English it can be beneficial to use names from the native language. To facilitate this it is necessary to allow a wide range of Unicode character in identifiers. The proposed implementation based on the Unicode TR31 is already used by other programming languages and is implemented behind the `non_ascii_idents` in *rustc* but lacks the NFC normalization proposed.

NFC normalization was chosen over NFKC normalization for the following reasons:

* [Mathematicians want to use symbols mapped to the same NFKC form](https://github.com/rust-lang/rfcs/pull/2457#issuecomment-394928432) like π and ϖ in the same context.
* [Some words are mangled by NFKC](https://github.com/rust-lang/rfcs/pull/2457#issuecomment-394922103) in surprising ways.
* Naive (search) tools can't find different variants of the same NFKC identifier. As most text is already in NFC form search tools work well.

Possible variants:

1. Require all identifiers to be already in NFC form.
2. Two identifiers are only equal if their codepoints are equal.
3. Perform NFKC mapping instead of NFC mapping for identifiers.
4. Only a number of common scripts could be supported.
5. A [restriction level][TR39Restriction] is specified allowing only a subset of scripts and limit script-mixing within an identifier.

An alternative design would use [Immutable Identifiers][TR31Alternative] as done in [C++]. In this case a list of Unicode codepoints is reserved for syntax (ASCII operators, braces, whitespace) and all other codepoints (including currently unassigned codepoints) are allowed in identifiers. The advantages are that the compiler does not need to know the Unicode character classes XID_Start and XID_Continue for each character and that the set of allowed identifiers never changes. It is disadvantageous that all not explicitly excluded characters at the time of creation can be used in identifiers. This allows developers to create identifiers that can't be recognized as such. It also impedes other uses of Unicode in Rust syntax like custom operators if they were not initially reserved.

It always a possibility to do nothing and limit identifiers to ASCII.

It has been suggested that Unicode identifiers should be opt-in instead of opt-out. The proposal chooses opt-out to benefit the international Rust community. New Rust users should not need to search for the configuration option they may not even know exists. Additionally it simplifies tutorials in other languages as they can omit an annotation in every code snippet.

## Confusable detection

The current design was chosen because the algorithm and list of similar characters are already provided by the Unicode Consortium. A different algorithm and list of characters could be created. I am not aware of any other programming language implementing confusable detection. The confusable detection was primarily included because homoglyph attacks are a huge concern for some members of the community.

Instead of offering confusable detection the lint `forbid(non_ascii_idents)` is sufficient to protect a project written in English from homoglyph attacks. Projects using different languages are probably either written by students, by a small group or inside a regional company. These projects are not threatened as much as large open source projects by homoglyph attacks but still benefit from the easier debugging of typos.


## Alternative mixed script lints

These are previously-proposed lints attempting to prevent problems caused by mixing scripts, which were ultimately replaced by the current mixed script confusables lint.

### Mixed script detection

A new `mixed_script_idents` lint would be added to the compiler. The default setting is to `warn`.

The lint is triggered by identifiers that do not qualify for the "Moderately Restrictive" identifier profile specified in [Unicode® Technical Standard #39 Unicode Security Mechanisms Section 5.2 Restriction-Level Detection][TR39RestrictionLevel].

Note: The definition of "Moderately Restrictive" can be changed by future versions of the Unicode standard to reflect changes in the natural languages used or for other reasons.

### Global mixed script detection with confusables

As an additional measure, we would try to detect cases where a codebase primarily using a certain script has identifiers from a different script confusable with that script.

During `mixed_script_idents` computation, keep track of how often identifiers from various script groups crop up. If an identifier is from a less-common script group (say, <1% of identifiers), _and_ it is entirely confusable with the majority script in use (e.g. the string `"арр"` or `"роре"` in Cyrillic)

This can trigger `confusable_idents`, `mixed_script_idents`, or a new lint.

We identify sets of characters which are entirely confusable: For example, for Cyrillic-Latin, we have `а, е, о, р, с, у, х, ѕ, і, ј, ԛ, ԝ, ѐ, ё, ї, ӱ, ӧ, ӓ, ӕ, ӑ` amongst the lowercase letters (and more amongst the capitals). This list likely can be programmatically derived from the confusables data that Unicode already has. It may be worth filtering for exact confusables. For example, Cyrillic, Greek, and Latin have a lot of confusables that are almost indistinguishable in most fonts, whereas `ھ` and `ס` are noticeably different-looking from `o` even though they're marked as a confusables.

The main confusable script pairs we have to worry about are Cyrillic/Latin/Greek, Armenian/Ethiopic, and a couple Armenian characters mapping to Greek/Latin. We can implement this lint conservatively at first by dealing with a blacklist of known confusables for these script pairs, and expand it if there is a need.

There are many confusables _within_ scripts -- Arabic has a bunch of these as does Han (both with other Han characters and with kana), but since these are within the same language group this is outside the scope of this RFC. Such confusables are equivalent to `l` vs `I` being confusable in some fonts.

For reference, a list of all possible Rust identifier characters that do not trip `less_used_codepoints` but have confusables can be found [here][unicode-set-confusables], with their confusable skeleton and script group mentioned on the right. Note that in many cases the confusables are visually distinguishable, or are diacritic marks.


# Prior art
[prior-art]: #prior-art

"[Python PEP 3131][PEP 3131]: Supporting Non-ASCII Identifiers" is the Python equivalent to this proposal. The proposed identifier grammar **XID_Start&nbsp;XID_Continue<sup>\*</sup>** is identical to the one used in Python 3. While Python uses KC normalization this proposes to use normalization form C.

[JavaScript] supports Unicode identifiers based on the same Default Identifier Syntax but does not apply normalization.

The [CPP reference][C++] describes the allowed Unicode identifiers it is based on the immutable identifier principle.

[Java] also supports Unicode identifiers. Character must belong to a number of Unicode character classes similar to XID_start and XID_continue used in Python. Unlike in Python no normalization is performed.

The [Go language][Go] allows identifiers in the form **Letter (Letter | Number)\*** where **Letter** is a Unicode letter and **Number** is a Unicode decimal number. This is more restricted than the proposed design mainly as is does not allow combining characters needed to write some languages such as Hindi.

# Unresolved questions
[unresolved]: #unresolved-questions

* Which context is adequate for confusable detection: file, current scope, crate?
* Should [ZWNJ and ZWJ be allowed in identifiers][TR31Layout]?
* How are non-ASCII idents best supported in debuggers?
* Which name mangling scheme is used by the compiler?
* Is there a better name for the `less_used_codepoints` lint?
* Which lint should the global mixed scripts confusables detection trigger?
* How badly do non-ASCII idents exacerbate const pattern confusion
  (rust-lang/rust#7526, rust-lang/rust#49680)?
  Can we improve precision of linting here?
* In `mixed_script_confusables`, do we actually need to make an exception for `Latin` identifiers?
* Terminal width is a tricky with unicode. Some characters are long, some have lengths dependent on the fonts installed (e.g. emoji sequences), and modifiers are a thing. The concept of monospace font doesn't generalize to other scripts as well. How does rustfmt deal with this when determining line width?
* right-to-left scripts can lead to weird rendering in mixed contexts (depending on the software used), especially when mixed with operators. This is not something that should block stabilization, however we feel it is important to explicitly call out. Future RFCs (preferably put forth by RTL-using communities) may attempt to improve this situation (e.g. by allowing bidi control characters in specific contexts).


[PEP 3131]: https://www.python.org/dev/peps/pep-3131/
[UAX31]: http://www.unicode.org/reports/tr31/
[UAX15]: https://www.unicode.org/reports/tr15/
[TR31Alternative]: http://unicode.org/reports/tr31/#Alternative_Identifier_Syntax
[TR31Layout]: https://www.unicode.org/reports/tr31/#Layout_and_Format_Control_Characters
[TR39Confusable]: https://www.unicode.org/reports/tr39/#Confusable_Detection
[TR39Restriction]: https://www.unicode.org/reports/tr39/#Restriction_Level_Detection
[C++]: https://en.cppreference.com/w/cpp/language/identifiers
[Julia Unicode PR]: https://github.com/JuliaLang/julia/pull/19464
[Java]: https://docs.oracle.com/javase/specs/jls/se10/html/jls-3.html#jls-3.8
[JavaScript]: http://www.ecma-international.org/ecma-262/6.0/#sec-names-and-keywords
[Go]: https://golang.org/ref/spec#Identifiers
[Composed characters]: https://en.wikipedia.org/wiki/Precomposed_character
[RFC 0430]: http://rust-lang.github.io/rfcs/0430-finalizing-naming-conventions.html
[TR39Allowed]: https://www.unicode.org/reports/tr39/#General_Security_Profile
[TR39RestrictionLevel]: https://www.unicode.org/reports/tr39/#Restriction_Level_Detection
[unicode-set-confusables]: https://unicode.org/cldr/utility/list-unicodeset.jsp?a=%5B%5B%3AIdentifier_Status%3DAllowed%3A%5D%26%5B%3AXID_Continue%3DYes%3A%5D%26%5B%3AConfusable_MA%3A%5D%5D&g=&i=Confusable_MA%2CScript_Extensions
[unicode-set-allowed]: https://unicode.org/cldr/utility/list-unicodeset.jsp?a=%5B%5B%3AIdentifier_Status%3DAllowed%3A%5D%26%5B%3AXID_Continue%3DYes%3A%5D%5D&g=&i=Script_Extensions