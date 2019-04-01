- Feature Name: moooooooooooooooo
- Start Date: 2019-04-01
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Re-export `std::borrow::Cow` from the standard library, along with each of its
variants, to encourage more widespread use. To avoid backwards compatibility
issues, these types/variants will need to be exported using more unique aliases.

# Motivation/Explanation

`std::borrow::Cow` is one of the most under-used portions of the standard
library. Many issues that newcomers understanding ownership could be improved if
`Cow` were used more prolifically throughout the ecosystem. The easiest way for
us to encourage this is to remove the need to explicitly import `Cow` by
exporting it from prelude.

Anything exported in the prelude is generally expected to refer to what it
refers to in the prelude. If a crate defines its own type called `Ok`, it will
likely be confusing to readers, and will break any macros which don't refer to
`std::result::Result::Ok` through its full path.

Unfortunately, `Cow` is an extremely overloaded term in Rust today, so we can't
reasonably export that identifier in the prelude. Applications today might be
using this name to refer to:

- California, Oregon, Washington (the most common usage)
- Can of Worms
- City of Waterloo
- Crush of the Week
- Container on Wheels
- Cost of Waste

to name a few. For this reason, the re-export of this type must be something
which is unlikely or impossible to appear in Rust programs today. For this
reason, the export will be defined as `pub use core::borrow::Cow as 🐄;`. As the
`non_ascii_idents` feature is not yet stable, this alias cannot possibly shadow
any type defined in stable Rust today. By defining this non-ascii identifier to
mean `Cow` today, we can ensure that it is associated with this type, and does
not become a common term for other uses as `Cow` clearly already is today.

Every enum exported in the prelude today also exports its variants at the top
level (e.g. `Option`, `Result`, etc). Clearly we should do the same for `🐄`.
However, similar to how we can't reasonably begin exporting `Cow` from the
prelude today, we clearly can't export `Borrowed` or `Owned` either (the
alternative uses of these terms should be clear and thus are not listed here).

This RFC proposes two potential alternatives for `Borrowed` and `Owned`. These
are both presented as equal options, and this RFC does not propose either as a
superior option. It is proposed that both be implemented, and the more popular
choice be chosen during the stabilization period.

Option A is as follows:

`Cow::Owned` would be exported as U+1F42E COW FACE, `🐮`. The choice of this
identifier should be clear -- A value of variant `🐮` has been copied for write,
or "cowed". We don't believe that the similarities between 🐄 and 🐮 will be
confusing in practice, all Rust developers will clearly pronounce the former
"cow" and the latter "owned/cowed".

`Cow::Borrowed` would be exported as U+262D HAMMER AND SICKLE, `☭`. This is
because ownership is theft. No further explanation is required.

The main drawback to the use of these characters is that 🐮 resides outside of
the Unicode BMP, and thus requires multiple code units to represent in UTF-16.
Because UTF-16 is a fixed width encoding, this may be confusing to many
developers. For this reason, the following alternative is proposed:

As no identifier or glyph today meets the needs of sufficiently representing the
embodiment of `Borrowed` and `Owned`, while being sufficiently unique from
identifiers which may be used in Rust today, two characters from the Unicode
private use area will be used instead.

`Borrowed` will be exported as U+FF8E, and `Owned` will be exported as U+FF8F.
Since both of these characters reside on the BMP, they can both be represented
as expected in the fixed width UTF-16 encoding. As these characters are not
displayed in most fonts, we will need to define a conventional representation of
these characters. We expect that Rustup will be updated to patch all system fonts
for these characters upon installation. This seems to be a reasonable solution
to this problem.

The conventional display of these characters will be defined as:

`Borrowed` will be an image of Ferris, the official unofficial Rust mascot.
Borrowing is a uniquely Rust concept, what better image to represent this than
our language's mascot.

`Owned` will be displayed as a scaled down version of [this picture of Steve
Klabnik wearing eclipse glasses standing in front of a nuclear
explosion](../steve-nuke.png). Because it owns.

Again, these options are presented with no preference for one or the other, and
it is expected the community will naturally pick one during the stabilization
process.

# Drawbacks
[drawbacks]: #drawbacks

While there are no drawbacks to the feature overall, this does slightly
complicate the `non_ascii_idents` feature. Since it is expected these re-exports
will become commonly used after stabilization, we will need to ensure that 🐄
and whatever characters are chosen for `Borrowed` and `Owned` are excluded from
the `non_ascii_idents` lint. This means that Rust will effectively consider 🐄
to be ascii (note that whether this should affect the behavior of
`(str|char)::is_ascii` is out of scope of this RFC, though no drawbacks to
defining `"🐄".is_ascii() == true` are immediately apparent).

Additionally, we must ensure that 🐄 and 🐮 do not trigger the
`confusable_idents` lint.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- Continue to write `use std::borrow::Cow`, and wallow in sorrow
- Export these types from the prelude, but continue to use incredibly boring
  terms like "Cow", "Borrowed", and "Owned"

# Prior art
[prior-art]: #prior-art

We will be trailblazers as the first programming language to define 🐄 in its
standard library.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Should we also define `fn moo(self)` as a conventient alias for
  `🐄::from(expr)`?

# Future possibilities
[future-possibilities]: #future-possibilities

As this feature becomes widely adopted, we can introduce a `boring_cow` lint to
prevent the previous path from being used, potentially even deprecating it in
Rust 2021.
