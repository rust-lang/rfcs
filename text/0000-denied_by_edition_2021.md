- Feature Name: `denied_by_edition_2021`
- Start Date: 2021-02-23
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

The Rust standard library contains many items marked as "deprecated", implying that users should avoid using them in favor of alternatives. Despite their undesirability, these deprecated items can never be outright removed from the standard library, in keeping with Rust's stability guarantee. However, with the aid of the edition mechanism, the use of a deprecated item can be made into a compile-time error, thereby allowing such items to be "removed" from the standard library in a way that remains fully edition-compatible.

This RFC proposes the following:

1. A mechanism by which selected deprecated items in the Rust standard library can have their lint level upgraded from "warn" to "deny"  based on the Rust edition that the user has selected.
2. A policy of applying this mechanism to items that have previously been deprecated **for at least one full edition cycle**.
3. A policy of additionally marking such items as `#[doc(hidden)]`, thereby removing them from the generated documentation.
4. A policy of employing Rustdoc aliases to explicitly redirect users of deprecated items to their replacements in Rustdoc search results.
5. An exhaustive list of items that have been deprecated since before the Rust 2018 edition that would be subject to these policies for the Rust 2021 edition.

# Motivation
[motivation]: #motivation

One of Rust's guiding principles is "stability without stagnation". The edition system is the ultimate expression of this philosophy, giving users the option to endure small amounts of potential breakage while giving Rust itself the leeway to evolve over time. But while the language itself has made good use of the edition system for this purpose, the standard library has not yet attempted to leverage it.

Historically, evolution of the standard library has proceeded by leveraging the `deprecated` lint, achieved by tagging an item (a function, type, module, etc.) with the `#[rustc_deprecated]` attribute. The `deprecated` lint is defined to be "warn-by-default", which means that any tagged items will trigger a compiler warning when used.

When first learning about the edition system, it is common to see people ask whether or not it also applies to the deprecated items in the standard library; it is natural and intuitive to assume that it does. In these discussions, it is also common to see people asserting that the edition system cannot be used to remove deprecated items from the standard library; while not incorrect, this also obscures the truth of the matter.  An item cannot be *deleted* from the standard library; all Rust editions share the same standard library, and so deleting any item would make it unavailable to not just one edition, but to every edition. However, it is actually quite easy to effectively "remove" an item via an edition by using lints.

One of the premier capabilities of an edition is its capacity for "promoting" a lint from "warn-by-default" to "deny-by-default". As of this writing, preliminary planning for the Rust 2021 edition suggests such that [such lint promotions](https://github.com/rust-lang/rust/issues/80165) will in fact make up the majority of the changes seen in the 2021 edition.

Because deprecation is already governed by the lint system, it is natural to leverage this capability to promote certain deprecations from warn to deny. Of course, care must be taken: unlike other lint promotions, we don't want to outright deny the `deprecated` lint on the 2021 edition, because that would  preclude the ability to ever trigger a warn-by-default lint for deprecations in the future. Instead, we will define a new deny-by-default lint and conditionally apply it only to select deprecated items. Furthermore, out of courtesy for end users, there should be some minimum amount of time between a deprecated item starting to emit a warning and starting to emit a denial. The guideline for edition-related lint promotions is that the lint must have been set to warn-by-default since the release of the previous edition; we adopt that guideline here as well. Therefore, we propose applying this new deny-by-default lint only to items that have been deprecated since at least Rust 1.31 (coterminous with the 2018 edition).

Finally, one of the largest benefits of "removing" an item in this way is its effect on documentation. Thanks to the compiler warning, many find it easy enough to avoid using deprecated items in their own code; however, the mere existence of deprecated items imposes an ever-increasing tax on all consumers of the Rust standard library documentation. Deprecated items introduce distracting noise to top-level documentation, module-level documentation, and search results. As the standard library continues to evolve, the amount of clutter will increase without bound unless steps are taken to remove deprecated items from the documentation. The measures proposed above provide the perfect opportunity to do so.

# Guide-level explanation

At the beginning of the planning phase for each new edition, a list will be compiled of all items in the standard library that have been deprecated for the entire lifetime of the current edition. These items will be marked for denial in the upcoming edition, using the mechanisms explained below. When the new edition is set to be released (or shortly after, there is no real urgency), these items will additionally be marked as `#[doc(hidden)]`. For all items that have been superseded by other items in the standard library, the replacement items will be annotated with Rustdoc aliases that refer to the name of the deprecated item.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## New lint machinery

**A proof-of-concept implementation of this section can be found here: https://github.com/bstrie/rust/tree/dbe_poc**

Introduce a new deny-by-default lint, `denied_by_edition`. This is the only user-facing language change proposed by this RFC; bikeshed away.

On the `#[rustc_deprecated]` attribute, introduce a new optional field named `denied_by_edition`. The value of this field must be a string that parses to a valid [`Edition`](https://doc.rust-lang.org/nightly/nightly-rustc/rustc_span/edition/enum.Edition.html). For example:

```rust
#[rustc_deprecated(since="1.0.0", reason="foo", denied_by_edition="2021")]
```

It is a compilation error for the `denied_by_edition` field to be defined if the `deprecated_in_future` lint would take effect, i.e. on a `#[rustc_deprecated]` attribute whose `since` value is in the future.

During compilation, when the compiler encounters an item decorated with `#[rustc_deprecated]`, it will check if the `denied_by_edition` field is defined. If it is defined, the compiler will compare the value of this field to the value of the edition associated with the current compilation session. If the field is less than or equal to the current session's edition, the attribute will emit the `denied_by_edition` lint instead of the `deprecated` lint.

## Changes to the standard library

The following items are to be marked as `#[rustc_deprecated(denied_by_edition = "2021"` and `#[doc(hidden)]`. Items are listed in ascending order of the Rust version in which they were deprecated. The most recently-deprecated of these items was in Rust 1.29, on 2018-09-13.


#### Rust 1.0
```
f64::{
	is_negative
	is_positive
}
```

#### Rust 1.1
```
fs::soft_link
```

#### Rust 1.2
```
mem::{
	min_align_of
	min_align_of_val
}
```

#### Rust 1.3
```
slice::connect
```

#### Rust 1.4
```
str::{
	lines_any
	LinesAny
}
```

#### Rust 1.6
```
sync::Condvar::wait_timeout_ms
thread::{
	park_timeout_ms
	sleep_ms
}
```

#### Rust 1.8
```
fs::Metadata::as_raw_stat
os::{
	android::{
		fs::as_raw_stat
		raw::*
	}
	dragonfly::{
		fs::as_raw_stat
		raw::*
	}
	emscripten::{
		fs::as_raw_stat
		raw::*
	}
	freebsd::{
		fs::as_raw_stat
		raw::*
	}
	fuchsia::raw::*
	haiku::fs::as_raw_stat
	illumos::{
		fs::as_raw_stat
		raw::*
	}
	ios::{
		fs::as_raw_stat
		raw::*
	}
	linux::{
		fs::as_raw_stat
		raw::*
	}
	macos::{
		fs::as_raw_stat
		raw::*
	}
	netbsd::{
		fs::as_raw_stat
		raw::*
	}
	openbsd::{
		fs::as_raw_stat
		raw::*
	}
	redox::{
		fs::as_raw_stat
		raw::*
	}
	solaris::{
		fs::as_raw_stat
		raw::*
	}
	unix::raw::*
}
```

#### Rust 1.10
```
f32::abs_sub
f64::abs_sub
```

#### Rust 1.13
```
hash::{
	SipHasher::{
		self
		new
		new_with_keys
	}
	SipHasher13::{
		self
		new
		new_with_keys
	}
	SipHasher24
}
```

#### Rust 1.16
```
net::TcpListener::{
	only_v6
	set_only_v6
}
```

#### Rust 1.24
```
fmt::Formatter::flags
```

#### Rust 1.26
```
ascii:AsciiExt
```

#### Rust 1.29
```
env::home_dir
str::{
	slice_unchecked
	slice_mut_unchecked
}
```

The following items are to receive the corresponding rustdoc aliases:

* `f64::is_sign_negative`: "is_negative"
* `f64::is_sign_positive`: "is_positive"
* `os::unix::fs::symlink`: "soft_link"
* `os::windows::fs::symlink_file`: "soft_link"
* `os::windows::fs::symlink_dir`: "soft_link"
* `mem::align_of`: "min_align_of"
* `mem::align_of_val`: "min_align_of_val"
* `slice::join`: "connect"
* `str::lines`: "lines_any"
* `str::Lines`: "LinesAny"
* `sync::Condvar::wait_timeout`: "wait_timeout_ms"
* `thread::park_timeout`: "park_timeout_ms"
* `thread::sleep`: "sleep_ms"
* `collections::hash_map::DefaultHasher`: "SipHasher", "SipHasher13", SipHasher24"
* `fmt::Formatter::sign_plus`: "flags"
* `fmt::Formatter::sign_minus`: "flags"
* `fmt::Formatter::alternate`: "flags"
* `fmt::Formatter::sign_aware_zero_pad`: "flags"
* `slice::get_unchecked`: "slice_unchecked"
* `slice::get_unchecked_mut`: "slice_mut_unchecked"

# Drawbacks
[drawbacks]: #drawbacks

This imposes the same potential cost as any other edition-related lint promotion.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Instead of only applying this RFC to items that have been denied since Rust 2018, we could instead apply it to all items marked as deprecated prior to the Rust 2021 release. This would accelerate the "removal" process, at the expense of giving users less leeway before being expected to update.

Instead of defining the new lint at the "deny" level, we could instead define it at the "forbid" level. The difference between the two is that the former can be silenced and overridden by an `#[allow]` attribute, whereas the latter cannot. Since other edition-related lints seem content to use "deny" rather than "forbid", this RFC follows suit.

# Prior art
[prior-art]: #prior-art

The `deprecated_in_future` lint is inspiration here; specifically in how it leverages a field on the `#[rustc_deprecated]` attribute to dynamically select the correct lint to emit based on environmental factors.

Philosophical inspiration was provided by the lint system itself, specifically in the three major levels: "allow", "warn", and "deny". With this RFC, there will be symmetry between these lint levels and the conceptual deprecation "levels": `deprecated_in_future` for "allow", `deprecated` for "warn", and `denied_by_edition` for "deny".

[The edition-related lint promotions](https://github.com/rust-lang/rust/issues/80165) are further inspiration here. Specifically, any objection based on the premise that a deny-by-default lint (which can be overridden) does not sufficiently dissuade people from using these items is obviated by the precedence set by the other edition-related lint promotions. If it's good enough for those lints, then naturally it's good enough for us.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

There is perhaps a better name than `denied_by_edition`; at face value it's a bit overbroad as a name, and might want to mention "deprecated" somewhere.

# Future possibilities
[future-possibilities]: #future-possibilities

If this RFC is accepted, it's reasonable to expect that analogous RFCs will be proposed for denying items in future Rust editions.
