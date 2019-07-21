- Feature Name: float_gen_debug
- Start Date: 2019-07-21
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Support `{:g?}` and `{:G?}` as formatting flags to modify the formatting of floating point numbers in `core::fmt::Debug`. These formats dynamically switch between fixed-point formatting and the exponential formats `:e` and `:E` based on the magnitude of a value.  This addition largely follows the model set forth by [RFC #2226](https://github.com/rust-lang/rfcs/pull/2226), which added `{:x?}`.

Though it sets the stage for their eventual existence, this RFC does **not** currently propose the addition of `{:g}` and `{:G}`.

# Motivation
[motivation]: #motivation

## Inadequate formatting facilities
[inadequate-formatting-facilities]: #inadequate-formatting-facilities

Rust currently has two ways to format floating point numbers:

* Simple/fixed (through `Debug` and `Display`)
* Exponential (through `LowerExp` and `UpperExp`)

Either of these additionally support a mode of "round-trip precision," when no precision (`.prec`) is provided in the format specifier.  However, neither of these two formats are suitable for human-oriented interfaces in contexts where numbers may be of arbitrary magnitude.

The simple formatting scheme can sometimes force the reader to play a game of "count the zeros":

```rust
assert_eq!(
    format!("{:?}", std::f64::MAX),
    "179769313486231570000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000.0",
);
```

This can frequently be an issue with values like `1e-10`, which may often show up as fuzz factors and tolerances in floating point computations.  The only solution offered by the standard library is exponential formatting through `:e` and `:E`. However, while useful for values of extreme magnitudes, exponential format can be taxing for humans to read for values on the order of `1`:

```rust
assert_eq!(format!("{:e}", 22.0), "2.2e1");
assert_eq!(format!("{:e}", 1.0), "1e0");
assert_eq!(format!("{:e}", 0.9), "9e-1");
assert_eq!(format!("{:e}", 0.0), "0e0");
```

Many other languages and utilities have a "general" or "generic" formatting mode which dynamically switches between simple and exponential format, making it useful for a much wider selection of data. Some of these languages even use it as their default formatting format. Meanwhile, Rust's standard library does not even provide the *functionality.*

Some may see that as a positive; after all, Rust is by no means a "batteries-included" language, and the community takes pride in having such easy access to third-party libraries on crates.io, where solutions are free to grow and evolve without being subject to the extremely harsh backwards compatibility guarantees of the standard library.  But there is one part of the standard library that suffers greatly from not having it:

## General-purpose tools demand general-purpose formatting

Let us call attention in particular to the standard library's `core::fmt::Debug`.  `Debug` is a general-purpose tool for rendering arbitrary datatypes to developers with as little developer effort as possible.  Unfortunately, the current implementation of `Debug` is hopelessly inadequate for many kinds of structs that contain floats:

```rust
#[derive(Debug)]
pub enum StepKind<F> {
    Fixed(F),
    Ulps(u64),
}

#[derive(Debug)]
struct FloatRange<F> {
    min_inclusive: F,
    max_inclusive: F,
    step: StepKind<F>,
}

let positive_normal_f32s = FloatRange {
    min_inclusive: std::f32::MIN_POSITIVE,
    max_inclusive: std::f32::MAX,
    step: StepKind::Ulps(1),
};

assert_eq!(
    format!("{:?}", positive_normal_f32s),
    "FloatRange { min_inclusive: 0.000000000000000000000000000000000000011754944, max_inclusive: 340282350000000000000000000000000000000.0, step: Ulps(1) }",
);
```

There is no way to format this struct with exponential notation, and even if you could, the output would look absurd when you later find yourself using `FloatRange { min_inclusive: 0.0, max_inclusive: 1.0, step: Fixed(0.25) }`.  With the proposed functionality, users will be able to utilize the existing `Debug` machinery to easily inspect arbitrary data structures with floating point numbers of heterogenous magnitude:

```rust
// using enum-map = "0.4.1"
#[derive(enum_map::Enum, Debug)]
enum Kind { Default, Simple }

#[derive(Debug)]
struct Settings {
    initial: f64,
    step: f64,
}

fn main() {
    let map = enum_map::enum_map!{
        Kind::Default => Settings { initial: 7654.32101234, step: 1e-6 },
        Kind::Simple => Settings { initial: 0.0, step: 0.1 },
    };
    println!("{:g?}", map);
}
```
**Output:**
```
{Default: Settings { initial: 7654.32101234, step: 1e-6 }, Simple: Settings { initial: 0, step: 0.1 }}
```

Accomplishing such a feat through an external library is nearly impossible without massive buy-in.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## Formatting flags for `Debug`

When formatting values using `Debug`, the flag `g` or `G` may be added before the `?`; this changes the formatting of any floating point values recursively contained in the type to use a general-purpose formatting scheme which switches between exponential and plain format based on magnitude:

```rust
assert_eq!(format!("{:g?}", 5.0), "5");
assert_eq!(format!("{:g?}", vec![5.0, 5.1, 1.234e9, 1.234e-9]), "[5.0,  5.1, 1.234e9, 1.234e-9]");
assert_eq!(format!("{:G?}", vec![5.0, 5.1, 1.234e9, 1.234e-9]), "[5.0,  5.1, 1.234E9, 1.234E-9]");
```

This format prints to round-trip precision by default.  When a precision is added, it is used as the maximum number of significant figures to display. (contrast with `{}` and `{:e}`, where it used as the number of places after the decimal point). To this end, it also changes the maximum number of digits that large numbers are allowed to contain before they are switched to exponential format:

```rust
assert_eq!(format!("{:.3g?}", vec![50.0, 500.0, 1.234e9]), "[50.0, 5e3, 1.23e9]");
```

When the alternate flag `#` is added, `{:#g?}` will pretty-print the struct but will *not* switch the floats to an alternate formatting scheme, similar to the behavior of `{:#x?}`.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Formatting specifier syntax

The following grammar of formatting strings was presented in [RFC #2226](https://github.com/rust-lang/rfcs/blob/master/text/2226-fmt-debug-hex.md):

```
format_string := <text> [ maybe-format <text> ] *
maybe-format := '{' '{' | '}' '}' | <format>
format := '{' [ argument ] [ ':' format_spec ] '}'
argument := integer | identifier

format_spec := [[fill]align][sign]['#']['0'][width]['.' precision][radix][type]
fill := character
align := '<' | '^' | '>'
sign := '+' | '-'
width := count
precision := count | '*'
type := identifier | '?'
count := parameter | integer
parameter := argument '$'
radix := 'x' | 'X'
```

This grammar is ambiguous for `{:x}`, however, so we will first revise it to pull `radix` directly into the `type`. 

```
format_spec := [[fill]align][sign]['#']['0'][width]['.' precision][type]
type := identifier | debug-type
debug-type := [radix] '?'
radix := 'x' | 'X'
```

This RFC extends it to additionally support `g` or `G` in place of a radix:

```
debug-type := [radix | floatmod] '?'
floatmod := 'g' | 'G'
```

`{:g?}` and `{:x?}` are mutually exclusive in this initial proposal, though `{:gx?}`/`{:xg?}` remain as backwards-compatible addition.  This decision was made to keep the option of `{:xe}` open for hexadecimal floats (though `{:a}` seems to be more common in other languages), which was implied to be possible by the original ambiguous grammar in RFC #2226.

## `Formatter` API and trait impls

RFC #2226 proposed a public API for checking these flags on an instance of `core::fmt::Formatter`.  However, at the time of this RFC, the public API is still in limbo; there are not even any feature-gated methods for this, only [private methods](https://github.com/rust-lang/rust/blob/83dfe7b27cf2debecebedd3b038f9a1c2e05e051/src/libcore/fmt/mod.rs#L1698-L1702).

For now, this RFC can be similarly implemented using only private methods on `Formatter`, which can be checked by the impls of `Debug` for `f32` and `f64`.  A public API for this RFC can be decided in tandem with RFC #2226.

## Specific formatting examples (tentative)

Because the  following is a table of example outputs that showcase a number of the tunable knobs in the format. The columns for `{:g}` in these tables propose one possible set of decisions.  **The decisions in this table are tentative and up to bikeshedding.**

The format below is largely based on Python's default formatter (`{}`).  Like Rust's Debug, this format displays a trailing `.0` on integers and a leading `-` for `-0.0`. There are two notable modifications:
* Python's `{}`'s switches to exponential format at `10**16`.  This would largely defeat the purpose of `{:g?}`, so a smaller threshold is chosen.
* Python formats exponents as `e+01`.  This uses `e1` for consistency with Rust's `{:e}`.

**Without precision flags:**

Value | `{:?}` | `{:e}` | `{:g?}` | Notes
---|---|---|---|---
`1.0`    |  `1.0` | `1e0` | **`1.0`** | Always show at least one place after the decimal point.
`0.0`    |  `0.0` | `0e0` | **`0.0`** |
`-0.0`   | `-0.0` | `0e0` | **`-0.0`** |
`1.234` | `1.234` | `1.234e0` | **`1.234`** | 
`100`      | `100`      | `1e2` | **`100.0`** |
`1000`     | `1000`     | `1e3` | **`1000.0`** | Even though `1e3` is shorter
...        | ...        | ... | ... |
`100000`  | `100000`  | `1e5` | **`100000.0`** |
`1000000` | `1000000` | `1e6` | **`1e6`** | Suggested default high cutoff
`0.0001`  | `0.0001`  | `1e-4` | **`0.0001`** |
`0.00001` | `0.00001` | `1e-5` | **`1e-5`** | Suggested low cutoff
`(1.0f32 + EPSILON)` |  `0.10000001` | `1.0000001e-1` | **`0.10000001`** |
`1e-7 * (1.0f32 + EPSILON)`  | `0.000000100`<br>`00001` | `1.0000001e-7` |  **`1.0000001e-7`** |

**With precision flags:**

Notice that the `g?` column in this table generally uses a precision that is one greater than the other columns (`p$` versus `s$`), to make the output more comparable.

Value | Precision | `{:.p$?}` | `{:.p$e}`  | `{:.s$g?}`    | Notes |
---|---|---|---|---|---|
`1.234`  | `p=2,s=3`     | `1.23`    | `1.23e0`   | **`1.23`**   | Precision is # sig-figs
`1.234`  | `p=3,s=4`     | `1.234`   | `1.234e0`  | **`1.234`**  |
`1.234`  | `p=5,s=6`     | `1.23400`  | `1.23400e0` | **`1.234`**  | Strip trailing zeros...
`1.0`    | `p=3,s=4`     | `1.000`   | `1.000e0`  | **`1.0`**  | ...but keep at least one place after the decimal point
`10000.1`    | `p=5,s=6`     | `10000.1000` | `1.0000e4` | **`10000.0`** | 
`10000.1`    | `___,s=5`     |  |  | **`1e4`** | High cutoff is when we can't fit the digit after the decimal.
`-0.0`    | `p=3,s=4`     | `-0.0000`   | `-0.000e0`  | **`-0.0`**  | 
`1e-3`    | `___,s=1`  |  | | **`0.001`** | Low cutoff is independent of precision
`1e-3`    | `___,s=0`  |  | | **`0.001`** | `{:.0p?}` is same as `{:.1p?}`
`(1f32 + ε)` | `p=10,s=11` | `1.0000001192` | `1.0000001192e0` | **`1.0000001192`** | Excess digits faithfully represent the binary value
`1e-7` | `p=5,s=6`  | `0.00000` | `1.00000e-7` | **`1e-7`** |

# Drawbacks
[drawbacks]: #drawbacks

## Implementation difficulty

Efficient floating point formatting is not an easy problem.  However, the author of this RFC has little expertise on the topic.

## `{:g?}` without `{:g}` is weird; could be abused

This RFC punts on `{:g}` for reasons that will be explained in the alternatives section.  `{:g?}` is not intended to be used in user-facing output, leaving that problem space to be fulfilled by third-party crates like [`dtoa`](https://crates.io/crates/dtoa).  Regardless, desparate users will likely use it as a substitute for the missing `{:g}`.

## `{:g?}` may become preferred over `{:?}`

There is tons of code that (a) already exists, (b) uses `{:?}`, and (c\) ...probably would be better off using `{:g?}` instead.  Such code will likely be fixed very slowly, and much of it won't ever be fixed at all.

This is a natural part of code evolution.  Most alternatives share this drawback; the only way to overcome it would be with breaking changes to the standard library formatting impls.

## No `{:#g}` mode

The vast majority of languages sampled by the author that have both a `{:g}` formatter and an "alternate" flag (`#`) ascribe the following behavior to the `#` flag when used on floating point numbers:

* In all floating point formats, `#` causes a trailing `.` to be kept even if there would be no digits after it.
* Furthermore, for `%g` and `%G`, the behavior that strips trailing zeros will be suppressed.

However, like `{:#x?}`, this proposal does not cause `{:#g?}` to exhibit the output of a would-be `{:#g}` format, making this behavior unavailable.

(Worth noting however is that `#` for Rust's floats already does not follow the first bullet point, either...)

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Why not include `{:g}` and `{:G}`?

To address the elephant in the room, why propose `{:g?}` and `{:G?}` without `{:g}` and `{:G}`? It is true that having all four would waste less of our strangeness budget.  But the upfront cost is a great deal higher, all to provide something that's not nearly as important to have.

Simply put, **`{:g?}`** is something direly needed, and **`{:g}`** is not.

* **We can't please everyone:** It's impossible to make a `{:g}` implementation that's perfect for everyone.  Limiting ourselves to `Debug` lowers the bar to something much more attainable: The implementation only needs to be *good enough for developers.*
* **Higher level of commitment:** Limiting ourselves to `Debug` gives us more slightly freedom to improve things, as `Debug` is subject to somewhat lighter backwards-compatibility guarantees than the other formatting traits.
* **Unclear design questions:** Should we introduce new traits for the new formatting modes, just for consistency's sake? ([@rkruppe argued against this](https://internals.rust-lang.org/t/pre-rfc-draft-g-or-floating-points-for-humans/9110/12?u=exphp) on the pre-RFC).  Should we have the Formatter method become *the* API?
* **It's not necessary!:** The purpose of `{:g}` and `{:G}` would be for formatting numbers for the end user.  Considering that all such possible use cases should already be using either `{}` or `{:e}` on individual floats, third-party crates (which can easily provide newtype wrappers around floats to adjust their `Display` impl) already suffice for all such possible use cases.

## Alternative: Change the output of `{:?}`

A far more direct approach to the motivation: Introduce nothing new, and instead change the `{:?}` representation for `f32` and `f64` to work more like `{:g}` proposed here.

* **Pro: No new APIs.** No new traits, no changes to format specifiers.
* **Pro: Automatic adoption all over.** The benefits will be reaped in many more places, such as the `assert_eq!` macro.
* **Pro: Consistency with other languages.** The author of this RFC was unable to find any language with a general `T -> String` conversion facility where the default behavior for floats does *not* dynamically switch to exponential notation. 
* **Con: Massive breaking change!**  Although _ideally_ there ought to be no code depending on `Debug` output representations, in reality this is far from the truth, and in practice there are even places that *should* depend on it (e.g. `should_panic` patterns under certain conditions).  About a year prior to the posting of this RFC, [the `Debug` representation of floats was changed to include a trailing `.0` for integer values](https://github.com/rust-lang/rust/pull/46831), and [this did not go unnoticed](https://github.com/rust-lang/rust/issues/47619).  The changes listed here are of far greater magnitude.
* **Pro/Con: Potential for misuse:** Like the current proposal, people may use `{:?}` in human-oriented output because it "looks nicer."

## Alternative: Add `{:e?}` and `{:E?}` instead.

Without introducing any implementation of general floating-point formatting, just add `{:e?}` specifiers.  This would solve the issue presented in the `positive_normal_f32s` example.  However, the author of this RFC would conjecture that the set of clear-cut good use cases for `{:e?}` is vanishingly small compared to `{:g?}`.

## Alternative:  Make the formatting system extensible

(thanks to @crlf0710 for reminding me to add this) 

This RFC proposes adding a new format, but as an alternative, we could make formatting extensible in a way that allows third party libraries to provide a new format.  The big question is: ....*how,* exactly? While this does dodge some difficult questions and allow the standard library to remain general-purpose, it could be a massive design effort that will require a much greater and far more complicated RFC.

It is posited that there are not many more common formatting modes that are missing from Rust.  Browsing around other languages, many languages with an `a`/`A` format for hexadecimal exponential format were found, and that was largely it.

## Alternative: Make this the alternative format `{:#?}` for floats

(thanks to @ekuber)

Like some other alternatives, this is a breaking change.  Unfortunately, this would force people to use `{:#?}` on structs as well.  `{:#?}` is a very space-consuming representation that is far from ideal for most use-cases.

# Prior art
[prior-art]: #prior-art

## `{:g}` in other languages

A variety of popular languages were sampled by the author.  Without exception, *every single one* was found to provide a general number formatting facility that dynamically switches to exponential based on value; though the exact output varies from language to language.

* [C's `printf`](https://en.cppreference.com/w/c/io/fprintf) is obviously a seminal example, and supports `%g`/`%G`. It also has `#`, with the behavior documented above.
* [Perl's formatting options](https://perldoc.perl.org/functions/sprintf.html) are just like C.
* [Lua](http://pgl.yoyo.org/luai/i/string.format) is documented as being like C.
* [Go](https://golang.org/pkg/fmt/)'s `%g`/`%G` appears to be like C. (including `#`).
* [Python](https://docs.python.org/3/library/string.html#formatspec) has `{:g}`/`{:G}` and `#`.
    * It also has a default formatter `{}` which is like `{:g}` except that it always shows at least one place after the decimal point.  To accomodate this extra digit, it also switches to exponential sooner.  (at `>= 10 ** (PREC - 1)` rather than `>= 10 ** PREC`) 
* Haskell's [`Text.Printf`](http://hackage.haskell.org/package/base-4.12.0.0/docs/Text-Printf.html) supports `%g`/`%G`.
* [Java](https://docs.oracle.com/javase/7/docs/api/java/util/Formatter.html#syntax) supports `%g`/`%G`.  Its `%g` does not strip trailing zeros, and `%#g` is forbidden.
* [Clojure](https://clojuredocs.org/clojure.core/format) has only a thin wrapper around Java's functionality.
* [.NET](https://docs.microsoft.com/en-us/dotnet/standard/base-types/standard-numeric-format-strings?view=netframework-4.8) has `{:g}`/`{:G}`, but it is unusual.  This language also lacks the common `#`, `+`, and `0` flags.
* Nim [appears to have](https://nim-lang.org/docs/strformat.html#standard-format-specifier-for-strings-integers-and-floats) the same set of modes as Python (including the default mode).
* [Erlang](http://erlang.org/doc/man/io.html#type-format) has `~g`.  It appears to keep trailing zeros, formats exponents as `e+1`, and has surprisingly small thresholds of `< 0.1` and `>= 1e4`.
* Javascript is unusual.  `console` logging facilities only have `%f`.  There is a method [`number.toPrecision`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_objects/Number/toPrecision) which seems to behave like `{:#g}` formatting.
    * `Number.toString()` switches to exponential notation at 1e21, at least in Chrome and NodeJS.  Oddly enough, so does e.g. `Number.toFixed(2)`!
* At least on Clang, the default behavior of C++'s [`operator<<(ostream&, double)`](https://en.cppreference.com/w/cpp/io/basic_ostream/operator_ltlt) appears to behave like `%.6g` in C.  `setprecision(8)` changes it to `%.8g`, and etc.
    * C++11 added `std::to_string(double)`, which, bizarrely, formats the number using `%.6f`.

## Treatment of floats by `{:?}` in other languages

Some of the above languages were found to have analogues to `Debug` for recursively printing values with extremely little developer effort.  Rust is the only language the author is aware of wherein such functionality does not dynamically switch to exponential notation for extremely large and small floats.

* **Haskell**: The instance of `Show` for `Double` switches to exponential on  `< 0.1` and `>= 1e7`.
* **Erlang**: `~w` (for recursively printing terms) dynamically switches to exponential format for floats, but unlike `~g` it aggressively favors the smallest possible representation; e.g. `[12345.0, 10000.0]` renders as `[12345.0,1e4]`.
* **Nim**: The [`repr` function](https://nim-lang.org/docs/system.html#repr%2CT) switches to exponential on `< 1e-4` and `>= 1e16`.
* **JavaScript**: On NodeJS and Chrome, `console.log` can be used on arbitrary objects, and will use exponential format for numbers `>= 1e21` or `< 1e-6`.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

* The precise format is subject to heavy bikeshedding.
* When should the format be considered final?  On stabilization of `{:g?}`? On stabilization of `{:g}` if it occurs?

# Future possibilities
[future-possibilities]: #future-possibilities

* `{:g}` and `{:G}` could be pursued after this RFC.