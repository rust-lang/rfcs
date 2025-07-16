- Feature Name: `compat_math_identifiers`
- Start Date: 2025-07-16
- RFC PR: [TODO rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [TODO rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Rust already supports a wide range of unicode characters in identifiers - for example `α`, `номер`, `عدد`, `数`, `संख्या` are all valid Rust identifiers.
This RFC extends the set of Unicode character which can be used in identifiers with [`ID_Compat_Math_Start`](https://util.unicode.org/UnicodeJsps/list-unicodeset.jsp?a=%5B%3AID_Compat_Math_Start%3DYes%3A%5D&g=&i=idtype) and [`ID_Compat_Math_Continue`](https://util.unicode.org/UnicodeJsps/list-unicodeset.jsp?a=%5B%3AID_Compat_Math_Continue%3DYes%3A%5D&g=&i=idtype), most notable: `∇`, `∂`, `∞`, subscripts `⁰¹²³⁴⁵⁶⁷⁸⁹⁺⁻⁼⁽⁾` and superscripts `₀₁₂₃₄₅₆₇₈₉₊₋₌₍₎`.
This can be a boon to implementers of scientific concepts as they can write for example `let ∇E₁₂ = 0.5;`.

# Motivation
[motivation]: #motivation

Programming languages have historically focused on the quite narrow set of ASCII characters, however developers from other cultures or specialized problem spaces can benefit from using characters which are native to their culture or domain.
The vast body of scientific literature uses a variety of characters to express concepts from physics, mathematics, biology, robotics and many others.
Symbols often appearing in equations are Roman letters like `x`, Greek letters like `θ`, and differentiation operators like `∂` and `∇`.
Variables are often adorned with subscripts like `x₁₂` or superscripts like `x⁺` or `x⁽²⁾`.
Having these symbols available as Rust identifiers could simplify the implementation of these concepts and stay closer to a reference publication, thus reducing confusing and implementation errors.

For example instead of:
```
let gradient_energy_1 = 2.0 * (position_1 - center_1);
let gradient_energy_2 = 2.0 * (position_2 - center_2);
```
one could write:
```
let ∇E₁ = 2.0 * (p₁ - c₁);
let ∇E₂ = 2.0 * (p₂ - c₂);
```

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

If needed you can use mathematical symbols like `θ`, `∇`, or `∂` as part of an identifier when implementing scientific concepts.

In addition you can use subscript and superscripts for your identifiers, for example you can write `x₁₂` instead of `x_12`, or `x⁺` instead of `x_plus`.
Note that you cannot start an identifier with a subscript or superscript, for example `₁x` will give a compiler error.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The Unicode sets [`ID_Compat_Math_Start`](https://util.unicode.org/UnicodeJsps/list-unicodeset.jsp?a=%5B%3AID_Compat_Math_Start%3DYes%3A%5D&g=&i=idtype) and [`ID_Compat_Math_Continue`](https://util.unicode.org/UnicodeJsps/list-unicodeset.jsp?a=%5B%3AID_Compat_Math_Continue%3DYes%3A%5D&g=&i=idtype) as defined in [Unicode Standard Annex #31 (UAX31)](https://www.unicode.org/reports/tr31/#Standard_Profiles) are part of the Unicode mathematical compatibility notation profile and consist of the following characters:

1) `∂` and `∇` from [Miscellaneious mathematical symbols](https://util.unicode.org/UnicodeJsps/list-unicodeset.jsp?a=%5Cp%7BNames_List_Subheader=Miscellaneous%20mathematical%20symbols%7D),
2) `∞` from [Miscellaneous mathematical symbol](https://util.unicode.org/UnicodeJsps/list-unicodeset.jsp?a=%5Cp%7BNames_List_Subheader=Miscellaneous%20mathematical%20symbol%7D),
3) `₀₁₂₃₄₅₆₇₈₉₊₋₌₍₎` from [Subscripts](https://util.unicode.org/UnicodeJsps/list-unicodeset.jsp?a=%5Cp%7BNames_List_Subheader=Subscripts%7D),
4) `⁰¹²³⁴⁵⁶⁷⁸⁹⁺⁻⁼⁽⁾` from [Superscripts](https://util.unicode.org/UnicodeJsps/list-unicodeset.jsp?a=%5Cp%7BNames_List_Subheader=Superscripts%7D) and [Latin-1 punctuation and symbols](https://util.unicode.org/UnicodeJsps/list-unicodeset.jsp?a=%5Cp%7BNames_List_Subheader=Latin-1%20punctuation%20and%20symbols%7D),
5) `𝛁𝛛𝛻𝜕𝜵𝝏𝝯𝞉𝞩𝟃`  (italic and bold versions of `∂` and `∇`) from various sets like [Bold Greek symbols](https://util.unicode.org/UnicodeJsps/list-unicodeset.jsp?a=%5Cp%7BNames_List_Subheader=Bold%20Greek%20symbols%7D).

The characters 1) - 4) are added to the set of characters allowed in Rust identifiers.
[UAX31](https://www.unicode.org/reports/tr31/#Standard_Profiles) notes that "supporting these characters is recommended for some computer languages because they can be beneficial in some applications".
These characters will not have syntactic use and are only added to the set of characters allowed in identifiers following the recommendations of [UAX31-R3b](https://www.unicode.org/reports/tr31/#R3b).
For example `let a = 2.0; let b = a²;` will naturally give a compiler error that `a²` is an unknown identifier and not be interpreted as `let b = a * a;`.
Similarly `let a = [2, 0]; let b = a₁;` will naturally give a compiler error that `a₁` is an unknown identifier and not be interpreted as `let b = a[0];`.

The characters 5) are added to the set of Rust identifiers, but will trigger an NFKC warning when used:
```
warning: identifier contains a non normalized (NFKC) character: '𝛁'
```
similarly to how characters like `𝑥` (instead of `x`) or `𝑓` (instead of `f`) are triggering this warning in stable Rust today.
This follows the guidelines from the [Unicode Technical Standard #55 - Source Code Handling (UTS55)](https://www.unicode.org/reports/tr55/#General-Security-Profile) which recommends that "implementations should provide a mechanism to warn about identifiers that are not in the General Security Profile for Identifiers" as defined in the [Unicode Technical Standard #39 - Unicode Security Mechanisms (UTS39)](https://www.unicode.org/reports/tr39/#General_Security_Profile).
In particular the characters in 5) are identified as "Not_NFKC", i.e. characters that cannot occur in strings normalized to [NFKC](https://unicode.org/reports/tr15/#Norm_Forms).

It shall be pointed out that Unicode specifically [mentions Rust as a positive industry example](https://www.unicode.org/reports/tr55/#General-Security-Profile) following the recommendations from the General Security Profile

# Drawbacks
[drawbacks]: #drawbacks

* Characters like `𝛁𝛛𝛻𝜕𝜵𝝏𝝯𝞉𝞩𝟃` are easily confusable with their base versions `∂∇` and can lead to subtle bugs. However the precedence in Rust seems to be to add them alongside their base version but trigger the NFKC warning.

* Some developers prefer to only use ASCII characters for programming. This paradigm can be enforced today via `deny(non_ascii_idents)`. This would disallow all characters added by this RFC.

* The superscript characters could be confused with actual mathematical operations. For example someone might write `let a = 2.0; let b = 3.0 * a²;` and be confused that this will result in a compiler error. There might also be the potential for subtle bugs like `let a² = 2; let a = 2; let x = a²;` and erronously assuming that `x = 4`, however one can argue that this is not due to the superscript characters as it can happen as well when only using ASCII characters: `let a_sq = 2; let a = 2; let x = a_sq;`.

* The subscript characters could be confused with indexing operation. For example someone might write  `let a = [2, 0]; let b = a₁;` and be confused that this will result in a compiler error.

* Some people might find it difficult to read superscript and subscript letters on lower resolution screens or when using small font sizes.

* Rust might want to decide in the future to give certain superscripts and subscripts syntactic meaning. For example they might want to interpret `a²` as `a * a` or `a₁` as `a[0]`. The latter sounds espcially unlikely though due to the general disagreement of 0-based vs 1-based indexing.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

If this RFC is not implemented then everyone has to keep using ASCII characters for identifier in scientific code, for example `gradient_energy` or `a_12`.

The impact of not implementing it should be fairly small, but implementing it could invite more scientific oriented people to the Rust language and make it easier for them to implement complex concepts.

# Prior art
[prior-art]: #prior-art

Rust has the philosophy to be open to various cultures and languages and allow them use their native symbols as identifiers.
Example which compiles in stable Rust without warning:
```
fn main() {
    let λ = 2.718_f32;  // Greek letter lambda
    let 파이 = 3.141_f32;  // Korean word for "pie"
    let значення = λ.abs() + 파이.abs();  // Cyrillic
    println!("{значення}");
}
```

Many of these characters are easily confusable if not attuned to the corresponding language or culture.
Example which compiles in stable Rust without warning:
```
fn main() {
    let 鳯 = 3;  // U+9CF5: "phoenix" (old variant)
    let 鳳 = 4;  // U+9CF4: modern simplified/traditional
    let 隱 = 5;  // U+96B1: “hidden”
    let 隠 = 6;  // U+96B0: nearly identical glyph
    println!("鳯 = {}, 鳳 = {}", 鳯, 鳳);
    println!("隱 = {}, 隠 = {}", 隱, 隠);
}

```

A vast set of characters added as part of Unicode character sets are easily confusable with other characters.
Example which compiles in stable Rust and triggers a "warning: identifier contains a non normalized (NFKC) character":
```
fn main() {
    let l = 1.0;
    let ℓ = l + 2.0;
    let 𝑓𝑢𝑛𝑐 = |𝑥: f32| 𝑥 * ℓ + l;
    let Σ = (1..5).map(|𝑖| 𝑓𝑢𝑛𝑐(𝑖 as f32)).sum::<f32>();
    println!("∑: {Σ}");
}
```

There are characters which one might argue should never have been added but are part of allowed Unicode sets.
Example which compiles in stable Rust and triggers a "warning: identifier contains an uncommon character":
```
fn ᅟ() {  // U+115F (Hangul Choseong Filler) renders as blank
    println!("boo");
}

fn main() {
    ᅟ();  // dito
}
```

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Are there other character sets which could be added as part of this RFC?
- Should the italic and bold versions of characters `∂` and `∇` be added?

# Future possibilities
[future-possibilities]: #future-possibilities

Rust has chosen the path of allowing non-ASCII characters as identifiers and this RFC adds some more characters which are useful to the scientific domain.

There might be other useful sets of characters which could be added in the future.
