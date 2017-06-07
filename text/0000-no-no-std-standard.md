- Feature Name: no_no_std_standard
- Start Date: 2017-01-03
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Reject crates.io uploads which declare a feature named `no_std`. Instead, suggest changing the crate to use a `std` feature that is enabled by default. This convention is more in line with how dependency resolution works in Cargo, and so enforcing it will lead to more consistent behavior.

Note that the Rust `#![no_std]` attribute is unrelated to this proposal, and will not be affected by it.

# Motivation
[motivation]: #motivation

Crate features are *additive*. If two crates need different features from the same dependency, then Cargo will take the union of both. In other words, you can only *enable* a feature on a dependency, never disable it.

This can cause issues when one of these features is *negative*. In particular, a crate may provide a `no_std` feature, which restricts its API so that it works with just the `core` library. But because features can only be enabled, not disabled, any consumer which enables `no_std` here will also enable it for every other user of this crate. Most of the time, this is not what the user wants to do.

While there are other examples of negative features out there, `no_std` is by far the most prominent. This is because without knowing about Cargo's algorithms, it would seem logical to toggle `#![no_std]` support using a feature of the same name.

As of this writing, there are [28 crates] on crates.io (0.4% of 7,426 total) with a feature that contains `no_` as a substring. Out of these, 21 of them have `no_std`. As for the remaining seven:

- `gif` has a `raii_no_panic` feature. When enabled, any I/O errors are ignored when the write buffer is dropped. In the author's opinion, the library should expose an explicit `.flush()` method and avoid panicking altogether. Verdict: **change**.

- `lazy_static` has a `spin_no_std` feature which replaces the usual implementation with a spin lock. This has a similar use case to `no_std`, and so has the same verdict of **change**.

- `libbindgen` has a `assert_no_dangling_items` feature. This appears to be a [debug tool][libbindgen commit]. Verdict: **keep**.

- `linked-tail-list` has a `test_no_validate` feature, which disables some sanity checks. In the author's opinion, this library should either make this positive (like `libbindgen`) or use `#[cfg(test)]` instead. Verdict: **change**.

- `quickercheck` has a feature called `no_function_casts` which changes some impl items to use the `Fn` trait directly. Given how it's used, the feature could be renamed to `nightly` instead. There's no clear benefit to this though. Verdict: **keep**.

- `ralloc` has two features, `unsafe_no_mutex_lock` and `no_log_lock`, which trade thread safety for increased performance. But even if one crate knows that it uses `ralloc` in a single-threaded way, it cannot guarantee that other crates do as well. On the other hand, given the syntax:

    ```toml
    [dependencies.ralloc]
    default-features = false
    features = []  # disable: unsafe_mutex_lock, log_lock
    ```

    it feels strange opting out of safety by *omitting* a word.

    Verdict: **uncertain**.

- `rscam` has a feature called `no_wrapper` which disables linking to the C library. This should be changed to a positive `use_libv4l2` feature instead. **Change**.

Given the variation between these crates, we cannot justify a blanket ban on all features that contain the string `no_`. We conclude that the best solution is to blacklist `no_std` only.

[28 crates]: https://gist.github.com/lfairy/5767bd29de07554e059981e18449ba44
[libbindgen commit]: https://github.com/servo/rust-bindgen/commit/15c687e0f1bec97448168f981ec93d207884a775

# Detailed design
[design]: #detailed-design

This change should be rolled out incrementally, in a similar way to the [ban on wildcard dependencies](./1241-no-wildcard-deps.md).

In the next stable Rust release, Cargo will issue warnings for all crates that have a `no_std` feature when publishing. However, the publish will still succeed.

In time with the stable release after that, crates.io will be updated to reject crates which declare this feature. Note that the check will only run on the crates.io server: other package hosts will be free to implement another policy.

# How We Teach This
[how-we-teach-this]: #how-we-teach-this

This change should be announced through the usual channels (Discourse, Reddit, TWiR, Twitter, etc.).

The [Cargo manual] includes a section on how to use crate features. The concepts discussed in this RFC can be added there. Something like this:

> Prefer features that *add* items, rather than remove them. For example, rather than having a feature `"no_unicorns"` that removes unicorns, have a feature `"unicorns"` that adds unicorns instead. Since features can only be enabled, not disabled, this will make sure that users have access to unicorns if they need them. In fact, crates.io rejects packages that have a feature called `"no_std"`!

It would be best not to use the words "positive" and "negative" in user-facing documentation. This proposal uses these words to avoid the unwieldy "feature that adds/removes items", but outside of this RFC the longer phrasing would be more self-evident.

[Cargo manual]: http://doc.crates.io/manifest.html#the-features-section

# Drawbacks
[drawbacks]: #drawbacks

This will add a bit of complexity to Cargo and crates.io.

This adds another barrier to publishing a crate. Though, given the low proportion of affected crates, it shouldn't be too much of an issue.

# Alternatives
[alternatives]: #alternatives

## Only warn, not reject

As noted in the wildcards RFC: we can continue to allow these features, but complain in a "sufficiently annoying" manner to discourage their use. It is not clear how this is an improvement though, given that this proposal takes care to leave no false positives anyway.

## Allow disabling features in Cargo

This is a much more significant change than the original proposal, and would need more use cases to justify it.

## Do nothing

We can always do nothing. However, given the issues brought up in the motivation, it is probably better to do something about this.

# Unresolved questions
[unresolved]: #unresolved-questions

None so far.
