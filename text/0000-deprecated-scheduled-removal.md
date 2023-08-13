- Feature Name: `deprecated_scheduled_removal`
- Start Date: 2023-08-13
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC proposes the addition of a `scheduled_removal` paramter to the `deprecated` attribute, to allow library authors to specify when a
deprecated item is scheduled to be removed.

# Motivation
[motivation]: #motivation

Currently, library authors can specify when an item is deprecated via the `since` parameter. However, when these deprecated items are removed,
it is often confusing to some library users as to why a portion of the public API is removed, without noticing that it has been already deprecated.
Users may not be aware that certain items have become deprecated, but even if they notice, they may not migrate to something else immediately.
When the deprecated items do get removed, it is then difficult for users to migrate to another API. Removal of APIs can be sudden, and users are
usually not prepared for sudden removal of deprecated items.

Having a `scheduled_removal` attribute, that specifies the version of which a deprecated item is removed would mean users obtain the information
of when the deprecated item will be removed from the public API and hence can prepare or complete for the migration beforehand.


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This RFC proposes the addition of the `scheduled_removal` parameter to the `deprecated` attribute, as with the following example:
```rust
#[deprecated(since = "0.2.1", scheduled_removal = "0.3.0")]
struct ThisItemIsDeprecated;
```
Usages of this struct would result in the following warning:
```
warning: use of deprecated unit struct `ThisItemIsDeprecated`
 --> src/main.rs:5:17
  |
5 |     let _ = ThisItemIsDeprecated;
  |             ^^^^^^^^^^^^^^^^^^^^
  |
  = note: this deprecated item will be removed in version 0.3.0
  = note: `#[warn(deprecated)]` on by default
```
The added line `note: this deprecated item will be removed in version 0.3.0`  tells the user this deprecated item will be removed in version
`0.3.0` and would make it clear that the user needs to migrate to another API before `0.3.0` lands, otherwise their code would break and fail
to compile due to the removal of the API.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

# Drawbacks
[drawbacks]: #drawbacks

- More work for library authors to utilise this parameter for enhanced user experience of the users.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## A separate `scheduled_for_removal` attribute

A separate attribute named `scheduled_for_removal` will be introduced. Usage of this attribute looks like:
```rust
#[deprecated(since = "0.2.1")]
#[scheduled_for_removal(at = "0.3.0")]
struct ThisItemIsDeprecated;
```
Which would generate the following warning:
```
warning: use of unit struct `ThisItemIsDeprecated` that is scheduled for removal
 --> src/main.rs:5:17
  |
5 |     let _ = ThisItemIsDeprecated;
  |             ^^^^^^^^^^^^^^^^^^^^
  |
  = note: this item will be removed in version 0.3.0
  = note: `#[warn(scheduled_for_removal)]` on by default
```
The `scheduled_for_removal` lint would be introduced alongside with this attribute.

This was briefly considered but ultimately personally vetoed because another attribute is quite some work, and it is like an extension to the `deprecated`
attribute - describing when will this item will be removed. Hence, adding the `scheduled_removal` parameter is preferred hence this proposal is based
on a new parameter rather than a new attribute.

# Prior art
[prior-art]: #prior-art

- The [JetBrains Annotations Java package](https://github.com/JetBrains/java-annotations/blob/master/common/src/main/java/org/jetbrains/annotations/ApiStatus.java#L94-L111)
has a similar annotation: `@ApiStatus.ScheduledForRemoval`, which allows the library author to specify when an API will be removed from the public API entirely, and is mostly
the inspiration for this RFC.
- The `@Deprecated` attribute from the Java standard library, which has the boolean parameter [`forRemoval`](https://docs.oracle.com/javase%2F9%2Fdocs%2Fapi%2F%2F/java/lang/Deprecated.html#forRemoval--),
it is used to specify whether the deprecated item will be removed in a future version - albeit a bit vague (as it does not allow you to specify which version would the item be removed).

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- How should this new attribute be named? (Albeit `scheduled_removal` sounds kind of weird to me, it is the currently the best name after [discussion
on Rust Internals](https://internals.rust-lang.org/t/pre-rfc-scheduled-removal-parameter-for-deprecated-attribute/19324)).

# Future possibilities
[future-possibilities]: #future-possibilities

Perhaps other API status attributes can be exposed to library authors, for example `experimental` to mark experimental APIs and emit corresponding
warnings to the user about the usage of an unstable and experimental API of the library, that the code may break at any time without prior notice.
