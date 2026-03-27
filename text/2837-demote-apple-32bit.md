- Feature Name: `demote_apple_32bit`
- Start Date: 2019-12-10
- RFC PR: [rust-lang/rfcs#2837](https://github.com/rust-lang/rfcs/pull/2837)
- Rust Issue: [rust-lang/rust#67724](https://github.com/rust-lang/rust/issues/67724)

## Summary
[summary]: #summary

This RFC proposes to demote the `i686-apple-darwin` rustc target from Tier 1 to
Tier 3, and to demote the `armv7-apple-ios`, `armv7s-apple-ios` and
`i386-apple-ios` rustc targets from Tier 2 to Tier 3.

## Motivation
[motivation]: #motivation

Apple [publicly announced][macos-announcement] that macOS 10.14 Mojave is the
last OS supporting the execution of 32bit binaries, and macOS 10.15 (and later)
prevents running them at all. It's been years since the last 32bit Apple
hardware was sold, so providing 64bit binaries should cover most of the macOS
userbase.

Apple [also announced][ios-announcement] that iOS 10 is the last one supporting
the execution of 32bit apps, and they won't work at all on iOS 11 and later.
All iPhones after the iPhone 5 and the iPhone 5C support 64bit apps, which
means all the supported ones can run them.

Along with the deprecation, Apple removed support for building 32bit binaries
since Xcode 10, and that makes building rustc itself on the project's CI harder
(as we're limited to providers still offering Xcode 9).

It makes little sense for the Rust team to continue providing support for a
platform the upstream vendor abandoned, especially when it requires extra
effort from us infrastructure-wise.

[macos-announcement]: https://support.apple.com/en-us/HT208436
[ios-announcement]: https://developer.apple.com/documentation/uikit/app_and_environment/updating_your_app_from_32-bit_to_64-bit_architecture

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The first release after this RFC is merged will be the last one with Tier 1
support for the `i686-apple-darwin` target and Tier 2 support for the
`armv7-apple-ios`, `armv7s-apple-ios` and `i386-apple-ios` targets. The release
after that will demote the targets to Tier 3, which means no official build
will be available for them, and they will not be tested by CI.

Once this RFC is merged a blog post will be published on the main Rust Blog
announcing the change, to alert the users of the target of the demotion. The
demotion will also be mentioned in the release announcement for the last
release with Tier 1 and Tier 2 support, as well as the first release with Tier
3 support.

This RFC does **not** propose removing the targets completely from the
codebase: that will be decided either by another RFC just for those targets, or
by an RFC defining a general policy for Tier 3 target removal.

Once the targets are demoted to Tier 3, users on other platforms with one of
those targets' `rust-std` installed won't be able to update the toolchain until
they remove that target. Users using an Apple 32bit compiler as their host
platforms will instead be prevented from updating at all, as no new binary
artifact will be available.

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The `i686-apple-darwin`, `armv7-apple-ios`, `armv7s-apple-ios` and
`i386-apple-ios` targets will be considered Tier 3 from the second release
after this RFC is merged. The code supporting the target will not be removed
from the compiler, even though we won't guarantee it will continue to work.

The following CI builders will be removed:

- `dist-i686-apple`
- `i686-apple`

In addition, the `armv7-apple-ios`, `armv7s-apple-ios` and `i386-apple-ios`
targets will be removed from the `dist-x86_64-apple` builder.

## Drawbacks
[drawbacks]: #drawbacks

Users might depend on the target, and approving this RFC means they'll be stuck
on an old compiler version forever, unless they build their own compiler and
fix the regressions introduced in newer releases themselves.

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Support for building 32bit binaries for Apple targets is shrinking as time goes
by: the latest SDKs from Apple don't support building them at all, and CI
providers are slowly starting to upgrade their minimum supported SDK versions:

* Azure Pipelines (the provider rustc currently use) doesn't have any public
  information on when Xcode 9 will be deprecated.
* GitHub Actions doesn't support Xcode 9 at all.
* Travis CI [deprecated Xcode older than 9.2][travis-ci-xcode-deprecation] in
  July 2018.

If this RFC is not accepted, we'll eventually reach a point when we'll have to
make considerable investments both in terms of money and time to keep building
on Apple 32bit.

[travis-ci-xcode-deprecation]: https://blog.travis-ci.com/2018-07-19-xcode9-4-default-announce

## Prior art
[prior-art]: #prior-art

There is no precedent inside the project for the deprecation of Tier 1 targets
to Tier 3.

Go is taking a [similar approach to us][go-34749], documenting that the last
release supporting Apple 32bit is going to be Go 1.14 (the next one), with
support for the target being dropped in Go 1.15.

[go-34749]: https://github.com/golang/go/issues/34749

## Unresolved questions
[unresolved-questions]: #unresolved-questions

*Nothing here.*

## Future possibilities
[future-possibilities]: #future-possibilities

*Nothing here.*
