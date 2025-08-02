- Feature Name: `compat_math_identifiers`
- Start Date: 2025-07-16
- RFC PR: [rust-lang/rfcs#3840](https://github.com/rust-lang/rfcs/pull/3840)
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
Having these symbols available as Rust identifiers could simplify the implementation of these concepts and stay closer to a reference publication, thus reducing confusion and implementation errors.

For example instead of:
```rust
let gradient_energy_1 = 2.0 * (position_1 - center_1);
let gradient_energy_2 = 2.0 * (position_2 - center_2);
```
one could write:
```rust
let ∇E₁ = 2.0 * (p₁ - c₁);
let ∇E₂ = 2.0 * (p₂ - c₂);
```

A longer example from the "wilds":
```rust
fn strain_energy_hessian_coeffs(l0: f64, l: f64) -> [f64; 4] {
    let l02 = l0.powi(2);
    let l03 = l0 * l02;
    let l04 = l0 * l03;
    let l05 = l0 * l04;
    let l2 = l.powi(2);
    let l3 = l * l2;

    let h = (l02 - l2) / (2.0 * l03);
    let dh = (3.0 * l2 - l02) / (2.0 * l05);

    [1.0 / l3, -1.0 / l03, dh, 1.0 / l0 - 1.0 / l + h]
}
```
With this RFC one could write:
```rust
fn strain_energy_hessian_coeffs(l₀: f64, l: f64) -> [f64; 4] {
    let l₀² = l₀.powi(2);
    let l₀³ = l₀ * l₀²;
    let l₀⁴ = l₀ * l₀³;
    let l₀⁵ = l₀ * l₀⁴;
    let l² = l.powi(2);
    let l³ = l * l²;

    let h = (l₀² - l²) / (2.0 * l₀³);
    let dh = (3.0 * l² - l₀²) / (2.0 * l₀⁵);

    [1.0 / l³, -1.0 / l₀³, dh, 1.0 / l₀ - 1.0 / l + h]
}
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

In other words this RFC proposes to adopt the "Mathematical Compatibility Notation Profile" which in accordance with [UAX31-R3b](https://www.unicode.org/reports/tr31/#R3b) allows these characters in identifiers and in turn prevents syntactic use.
For example `let a = 2.0; let b = a²;` will naturally give a compiler error that `a²` is an unknown identifier and not be interpreted as `let b = a * a;`.
Similarly `let a = [2, 0]; let b = a₁;` will naturally give a compiler error that `a₁` is an unknown identifier and not be interpreted as `let b = a[0];`.
`∞` will just be a character usable in identifiers and not be a synonym to the likes of `f32::INFINITY`.

The characters 5) are added to the set of Rust identifiers, but will trigger an NFKC or `uncommon_codepoints` warning when used depending on their Unicode classification.
For example using `𝛁` in an identifier will trigger:
```
warning: identifier contains a non normalized (NFKC) character: '𝛁'
```
similarly to how characters like `𝑥` (instead of `x`) or `𝑓` (instead of `f`) are triggering this warning in stable Rust today.
This follows the guidelines from the [Unicode Technical Standard #55 - Source Code Handling (UTS55)](https://www.unicode.org/reports/tr55/#General-Security-Profile) which recommends that "implementations should provide a mechanism to warn about identifiers that are not in the General Security Profile for Identifiers" as defined in the [Unicode Technical Standard #39 - Unicode Security Mechanisms (UTS39)](https://www.unicode.org/reports/tr39/#General_Security_Profile).
In particular the characters in 5) are identified as "Not_NFKC", i.e. characters that cannot occur in strings normalized to [NFKC](https://unicode.org/reports/tr15/#Norm_Forms).

Note that Unicode specifically [mentions Rust as a positive industry example](https://www.unicode.org/reports/tr55/#General-Security-Profile) that follows the recommendations from the General Security Profile.


# Drawbacks
[drawbacks]: #drawbacks

* Characters like `𝛁𝛛𝛻𝜕𝜵𝝏𝝯𝞉𝞩𝟃` are easily confusable with their base versions `∂∇` and can lead to subtle bugs. However the precedent in Rust is to add them alongside their base version, but trigger the NFKC warning.

* Some developers prefer to only use ASCII characters for programming. This paradigm can be enforced today via `deny(non_ascii_idents)`. This would disallow all characters added by this RFC.

* The superscript characters could be confused with actual mathematical operations. For example someone might write `let a = 2.0; let b = 3.0 * a²;` and be confused that this will result in a compiler error. There might also be the potential for subtle bugs like `let a² = 2; let a = 2; let x = a²;` and erronously assuming that `x = 4`, however one can argue that this is not due to the superscript characters as it can happen as well when only using ASCII characters: `let a_sq = 2; let a = 2; let x = a_sq;`.

* The subscript characters could be confused with indexing operation. For example someone might write  `let a = [2, 0]; let b = a₁;` and be confused that this will result in a compiler error.

* Some people might find it difficult to read superscript and subscript letters on lower resolution screens or when using small font sizes.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

If this RFC is not implemented then everyone has to keep using ASCII characters for identifiers in scientific code, for example `gradient_energy` or `a_12`.

The impact of not implementing it should be fairly small, but implementing it could invite more scientific oriented people to the Rust language and make it easier for them to implement complex concepts.

Alternatively Rust could decide to give the proposed characters syntatic meaning.

Superscript characters could be interpreted as exponentiation, for example `let a = 2; let b = a²;` could be a synonym to `let a = 2; let b = a * a;`.
This would open up a host of questions and potential issues, like:
- Should `a²⁻³` be interpreted as `1/a`?
- There is no superscript character for multiplication `*` or division `/`.

`∞` could be a synonym or replacement to `f32::INIFITY`, however there is no precedent for using non-ASCII characters in `core`/`std` and this would likely meet considerable opposition.

Derivatives could be added as a language feature using auto-differentiation techniques and `∇` and `∂` could be given syntactic meaning.
For example Mathematica supports the syntax `∂ₓf` for a partial derivative of `f` with respect to `x` and the syntax `∇ₓf` for the gradient with respect to `x`.
Moreover there is [an experimental feature](https://doc.rust-lang.org/nightly/std/autodiff/attr.autodiff_forward.html) for Rust which provides auto-differentiation via an attribute macro `#[autodiff_forward(name, ..)]` with a user-provided function name for the automatically generated derivative.
With this RFC `∇f` could be used as the function name, i.e. `#[autodiff_forward(∇foo, ..)]`. However Rust could also decide to automatically use `∇foo` as the derivatives of `foo`, and give `∇` syntactic meaning.

Subscript characters could be given syntatic meaning, for example `a₁` could be a synonym to `a[1]`, however this would be highly contentious and error prone due to the general disagreement between 0-based versus 1-based indexing and would suffer from similar problems as using superscripts for exponentiation.

# Prior art
[prior-art]: #prior-art

Rust has the philosophy to be open to various cultures and languages and allow them use their native symbols as identifiers.
Example which compiles in stable Rust without warning:
```rust
fn main() {
    let λ = 2.718_f32;  // Greek letter lambda
    let 파이 = 3.141_f32;  // Korean word for "pie"
    let значення = λ.abs() + 파이.abs();  // Cyrillic
    println!("{значення}");
}
```

Many of these characters are easily confusable if not attuned to the corresponding language or culture.
Example which compiles in stable Rust without warning:
```rust
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
```rust
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
```rust
fn ᅟ() {  // U+115F (Hangul Choseong Filler) renders as blank
    println!("boo");
}

fn main() {
    ᅟ();  // dito
}
```
Note that the character is the name of the function, even though it renders inside the parentheses in some browsers.

[C++ P3658R0](https://www.open-std.org/jtc1/sc22/wg21/docs/papers/2025/p3658r0.pdf) is a similar proposal with similar reasoning for the C++ language. In particular it states that the characters suggested in this RFC where allowed in C++11 to C++20 as originally published.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Are there other character sets which could be added as part of this RFC?
- Should the italic and bold versions of characters `∂` and `∇` be added?

# Future possibilities
[future-possibilities]: #future-possibilities

Rust has chosen the path of allowing non-ASCII characters as identifiers and this RFC adds some more characters which are useful to the scientific domain.

There might be other useful sets of characters which could be added in the future.
