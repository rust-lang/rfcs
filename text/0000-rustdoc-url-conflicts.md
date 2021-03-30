- Feature Name: rustdoc_url_conflicts
- Start Date: 2021-03-29
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC describes how the rustdoc URLs will be modified to fix the current issue on case insensitive file systems.
For example, in the libstd there is:
 * keyword.self.html
 * keyword.Self.html

If macOS or Windows users try to access to both URLs, they will always see the same content because for both file systems, it's the same file.

# Motivation
[motivation]: #motivation

This is one of the oldest rustdoc issue and has been problematic for years and a lot of users. You can see that a few issues about it were opened:
 * <https://github.com/rust-lang/rust/issues/25879>
 * <https://github.com/rust-lang/rust/issues/39926>
 * <https://github.com/rust-lang/rust/issues/46105>
 * <https://github.com/rust-lang/rust/issues/51327>
 * <https://github.com/rust-lang/rust/issues/76922>
 * <https://github.com/rust-lang/rust/issues/80504>
 * <https://github.com/rust-lang/rust/issues/83154> (and <https://github.com/rust-lang/rustup/issues/2694>)

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The idea here is to replace every capitalized letter in the item with `-[minimized capital]`. So for example, `enum.FooBar.html` will become `enum.-foo-bar.html`.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Multiple attempts were made but all of them had huge drawbacks. The last one suggested to handle "conflicted" paths by doing the following:
 * Change name of conflicted files and store them somewhere else.
 * Create one file which will then load the correct file based on the URL semantic.

There are multiple issues with this approach:
 * It requires JS to work.
 * It doesn't work on case-sensitive file systems (unless we make links to support all URLs but then it would be problematic on case-insensitive file systems).

Other approaches were discussed but their drawbacks were even bigger and more numerous. This approach seems to be the most viable.

# Drawbacks
[drawbacks]: #drawbacks

 * It'll make the URL harder to read.
 * It'll change the existing URLs (this is mostly an issue for blogs and external content not using rustdoc or old rust documentation not using intra-doc links)

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The other alternatives require either JS to be enabled all the time or just don't work on both case-sensitive and case-insensitive file systems with the same generation process, forcing us to differentiate between both and make documentation generation case sensitivity-related.

# Prior art
[prior-art]: #prior-art

Like said previously, there were multiple attempts done before:
 * <https://github.com/rust-lang/rust/pull/83612>
 * <https://github.com/rust-lang/rust/pull/64699>
 * <https://github.com/rust-lang/rust/pull/59785>
 * <https://github.com/rust-lang/rust/pull/35020>

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None.

# Future possibilities
[future-possibilities]: #future-possibilities

Maybe change the way we replace capitalized letters?
