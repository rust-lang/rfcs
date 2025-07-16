- Feature Name: `compat_math_identifiers`
- Start Date: 2025-07-16
- RFC PR: [TODO rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [TODO rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Rust already supports a wide range of unicode characters in identifiers - for example `α`, `номер`, `عدد`, `数`, `संख्या` are all valid Rust identifiers.
This feature extends the set of Unicode character which can be used in identifiers with [`ID_Compat_Math_Start`](https://util.unicode.org/UnicodeJsps/list-unicodeset.jsp?a=%5B%3AID_Compat_Math_Start%3DYes%3A%5D&g=&i=idtype) and [`ID_Compat_Math_Continue`](https://util.unicode.org/UnicodeJsps/list-unicodeset.jsp?a=%5B%3AID_Compat_Math_Continue%3DYes%3A%5D&g=&i=idtype), most notable: `∇`, `∂`, `∞`, subscripts `⁰¹²³⁴⁵⁶⁷⁸⁹⁺⁻⁼⁽⁾` and superscripts `₀₁₂₃₄₅₆₇₈₉₊₋₌₍₎`.
This can be a boon to implementers of scientific concepts as they can write for example `let ∇E₁₂ = 0.5;`.

# Motivation
[motivation]: #motivation

Programming languages have historically focused on the quite narrow set of ASCII characters, however developers from other cultures or specialized problem spaces can benefit from using characters which are native to their culture or domain.
The vast body of scientific literature uses a variety of characters to express concepts from physics, mathematics, biology, robotics and many others.
Symbols typically appearing in equations are Roman letters like `x`, Greek letters like `γ`, differentiation operators like `∂` (partial derivative) and `∇` (gradient).
Variables like `x` are often adorned with subscripts like `x₁₂` and regularly also with superscripts like `γ⁺` or `x⁽²⁾`.
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

If needed you can use mathematical symbols like `α`, `∇`, or `∂` as part of an identifier when implementing scientific concepts.

In addition you can use subscript and superscripts for your identifiers, for example you can write `a₁₂` instead of `a_12`, or `a⁺` instead of `a_plus`.
Note that you cannot start an identifier with a subscript or superscript, for example `₁a` will give a compiler error.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The Unicode sets [`ID_Compat_Math_Start`](https://util.unicode.org/UnicodeJsps/list-unicodeset.jsp?a=%5B%3AID_Compat_Math_Start%3DYes%3A%5D&g=&i=idtype) and [`ID_Compat_Math_Continue`](https://util.unicode.org/UnicodeJsps/list-unicodeset.jsp?a=%5B%3AID_Compat_Math_Continue%3DYes%3A%5D&g=&i=idtype) consist of the following characters:

1) `∂∇`
2) `∞`
3) `₀₁₂₃₄₅₆₇₈₉₊₋₌₍₎`
4) `⁰¹²³⁴⁵⁶⁷⁸⁹⁺⁻⁼⁽⁾`
5) `𝛁𝛛𝛻𝜕𝜵𝝏𝝯𝞉𝞩𝟃` (italic and bold versions of `∂∇`)

The characters 1) - 4) are added to the set of Rust identifiers.

The characters 5) are added to the set of Rust identifiers, but will trigger an NFKC warning when used:
```
warning: identifier contains a non normalized (NFKC) character: '𝛁'
```
similarly to how characters like `𝑥` (instead of `x`) or `𝑓` (instead of `f`) are triggering this warning today.

Note that if breaking precedence is desired I would suggest to not add the characters from 5).

# Drawbacks
[drawbacks]: #drawbacks

* Characters like `𝛁𝛛𝛻𝜕𝜵𝝏𝝯𝞉𝞩𝟃` are easily confusable with their base versions `∂∇` and can lead to subtle bugs. However the precedence in Rust seems to be to add them alongside their base version but trigger the NCKC warning.

* Some developers prefer to only use ASCII characters for programming. This paradigm can be enforced today via `deny(non_ascii_idents)`. This would disallow all characters added by this RFC.

* The superscript characters can be confused with actual mathematical operations. For example someone might write `let a = 2.0; let b = 3.0 * a²;` and be confused that this will result in a compiler error. There might also be the potential for subtle bugs like `let a² = 2; let a = 2; let x = a²;` and erronously assuming that `x = 4`, however one can argue that this is not due to the superscript characters as it can happen as well when only using ASCII characters: `let a_sq = 2; let a = 2; let x = a_sq;`.

* Some people might find it difficult to read superscript and subscript letters on lower resolution screens or when using small font sizes.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

If this RFC is not implemented then everyone has to keep using ASCII characters like `gradient_energy` or `a01`.

The impact of not implementing it should be fairly small, but implementing it could invite more scientific oriented people to the Rust language and make it easier for them to implement complex concepts.

# Prior art
[prior-art]: #prior-art

Rust has the philosophy to be open to various cultures and languages and allow them use their native symbols as identifiers.
Example which compiles in stable Rust without warning:
```
fn main() {
    let λ = 2.718_f32; // Greek letter lambda
    let 파이 = 3.141_f32; // Greek letter pi
    let значення = λ.abs() + 파이.abs();
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
- Should the italic and bold versions of characters `∂∇` be added?

# Future possibilities
[future-possibilities]: #future-possibilities

Rust has chosen the path of allowing non-ASCII characters as identifiers and this RFC adds some more characters which are useful to the scientific domain.

There might be other usefule sets of characters which could be added in the future.
