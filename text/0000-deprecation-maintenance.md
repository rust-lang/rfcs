- Feature Name: N/A
- Start Date: 2019-01-13
- RFC PR: (leave this empty)
- Rust Issue: N/A

# Summary
[summary]: #summary

Amend [RFC 1242] to specify what “minimal maintenance” means for [rust-lang-deprecated] crates.

[RFC 1242]: https://github.com/rust-lang/rfcs/blob/master/text/1242-rust-lang-crates.md
[rust-lang-deprecated]: https://github.com/rust-lang-deprecated/

# Motivation
[motivation]: #motivation

From time to time, official rust-lang crates may be deprecated according to [RFC 1242]:

> At some point a library may become stale -- either because it failed to make it out of the nursery, or else because it was supplanted by a superior library.

Especially in the latter case, use of the original crate may still be pervasive throughout the crates.io ecosystem. This evidenced by the crates.io download numbers (for lack of a better popularity metric):

| Deprecated crate | Recent downloads | All-time downloads |
| ---------------- | ---------------- |------------------- |
| time             |             789k |             5,346k |
| rustc-serialize  |             391k |             5,558k |
| tempdir          |             280k |             2,616k |
| hexfloat         |              258 |              3,027 |

(numbers as of 2019-01-13)

The community moving away from recently-deprecated crates will take time.

Generally, dependencies on deprecated crates are not consciously added by crate authors. The deprecation messaging is well-recieved and new functionality is generally implemented with non-deprecated alternatives. However, crates can easily depend on deprecated crates *transitively*. To further drive this point home, `time` is used by such popular crates as `hyper`, `chrono`, `cookie`. `rustc-serialize` by `rust-crypto` and the RLS. `tempdir` by `cc` and `semver`.

Crates with existing dependencies on deprecated crates will only replace them slowly. Owners of deprecated crates or the crates that replace them do not very often push changes to their downstream dependencies (no doubt due to limited resources). Downstream crates may also be reluctant to make a semver-incompatible change (in case the deprecated crate was a public dependency).

In the mean time, these deprecated crates continue to be used and depended on by the community, which means they should recieve some form of maintenance, as [RFC 1242] specifies:

> Deprecated crates move to rust-lang-deprecated and are subsequently minimally maintained.

However, minimal maintenance is not defined and practically speaking some of these crates receive no maintenance. The popular deprecated crates each have their own policy:

* `rustc-serialize`: “No new feature development will happen in this crate, although bug fixes proposed through PRs will still be merged”
* `time`: “This library is no longer actively maintained, but bugfixes will be added”
* `tempdir`: “Please direct new issues and pull requests to tempfile”

Although in some cases, maintainers of these crates are actively refusing to merge maintenance PRs meeting the crate's policy.

Considering the critical role some of the deprecated crates play in the ecosystem, this RFC aims to do the following:

* Create a consistent policy for the maintenance of all deprecated crates.
* Improve guidance on what kind of maintenance a deprecated crate may be expected to receive.

This is done to improve transparency of the decision-making process and avoid arbitrary choices by crate owners, providing clarity to the community.

## A special note about portability

Rust has generally been interested in being available on platforms far and wide, boasting a large list of [supported platforms] and including what some may consider esoteric platforms on the [2018 Roadmap].

However, compiler and `std` support for platforms is not enough. Crates *also* sometimes need platform support. Crates being unavailable on a specific platform may prevent wider adoption of that platform. Again, considering the key role of some of the deprecated crates, this RFC proposes to explicitly include adding platform support to the definition of “minimal maintenance”.

[supported platforms]: https://forge.rust-lang.org/platform-support.html
[2018 Roadmap]: https://blog.rust-lang.org/2018/03/12/roadmap.html

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

In evaluating a PR for a deprecated crate meeting the popularity threshold, the reviewer could ask the following questions:

* Does this PR fix a bug in the crate? Alternatively, are these the minimal changes required to make the crate function properly on a new platform?

* Does this PR keep the public API of the crate the same? (This is a bellwether for changes that are not bug fixes).

If the answer to these questions is yes, then the PR is a likely candidate for merging, and should also trigger a point release of the crate. Of course, the reviewer should also do a normal code review. The reviewer may reasonably use other information to come to a decision regarding whether the PR is a reasonable bug fix or adds platform support.

For example, https://github.com/rust-lang-deprecated/time/pull/169

```diff
diff --git a/src/duration.rs b/src/duration.rs
index 419af0fc7..73aeb5c4f 100644
--- a/src/duration.rs
+++ b/src/duration.rs
@@ -133,10 +133,10 @@ impl Duration {

     /// Runs a closure, returning the duration of time it took to run the
     /// closure.
-    pub fn span<F>(f: F) -> Duration where F: FnOnce() {
+    pub fn span<F,R>(f: F) -> (Duration, R) where F: FnOnce() -> R {
         let before = super::precise_time_ns();
-        f();
-        Duration::nanoseconds((super::precise_time_ns() - before) as i64)
+        let r = f();
+        (Duration::nanoseconds((super::precise_time_ns() - before) as i64), r)
     }

     /// Returns the total number of whole weeks in the duration.
```

would likely not be merged. https://github.com/rust-lang-deprecated/rustc-serialize/pull/195

```diff
diff --git a/src/serialize.rs b/src/serialize.rs
index 296f3d4..f330756 100644
--- a/src/serialize.rs
+++ b/src/serialize.rs
@@ -1356,7 +1356,7 @@ array! {
 }

 impl Encodable for path::Path {
-    #[cfg(target_os = "redox")]
+    #[cfg(not(any(unix, windows)))]
     fn encode<S: Encoder>(&self, e: &mut S) -> Result<(), S::Error> {
         self.as_os_str().to_str().unwrap().encode(e)
     }
@@ -1380,7 +1380,7 @@ impl Encodable for path::PathBuf {
 }

 impl Decodable for path::PathBuf {
-    #[cfg(target_os = "redox")]
+    #[cfg(not(any(unix, windows)))]
     fn decode<D: Decoder>(d: &mut D) -> Result<path::PathBuf, D::Error> {
         let string: String = try!(Decodable::decode(d));
         let s: OsString = OsString::from(string);
```

would be merged under this policy. Also, https://github.com/rust-lang-deprecated/rustc-serialize/pull/178 would likely have been merged.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The exact amendment is included as a change to the RFC in this PR. [View the amended text](1242-rust-lang-crates.md).

# Drawbacks
[drawbacks]: #drawbacks

### Maintenance of deprecated crates doesn't sufficiently discourage their use

People might feel that letting the code of deprecated crates bit-rot will force people to seek out alternative solutions. This is likely accurate, but consider that these crates were once non-deprecated under the `rust-lang` umbrella! People started using these crates with certain expectations. Just “throwing code over the wall” is generally not the way the Rust community works.

A similar argument might be made for platform support: if a sufficiently popular platform doesn't support a crate, this again will force people to seek out alternative solutions. However, at the time the platform support is added to the deprecated crates, the new platform likely doesn't have enough “pull” to force any kind of change whatsoever. These platforms are generally small enough that they won't have a noticeable impact on the usage numbers of deprecated crates (positive or negative). The only thing not adding platform support to popular crates does is impede the growth of those new platforms.

### Performing maintenance on deprecated crates costs developer cycles

While there is always some effort required for reviewing any PR, the types of maintenance PRs this RFC proposes to consider generally don't take a lot of time. Bug fixes don't usually involve a lot of code churn. Platform support generally looks very similar to existing platform support.

### Unifying the maintenance policy for all popular deprecated rust-lang crates isn't flexible enough

The RFC author believes a unified policy is beneficial for the current crates. If a future deprecated crate needs a different policy, that can be discussed in that crate's deprecation RFC.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

### Change nothing

The status quo is that crate owners set their own policies and are free to interpret “minimal maintenance” however they see fit.

### Minimal maintenance doesn't include platform support

Still adopt a unified maintenance policy, but explicitly exclude platform support from the type of maintenance that can be done.

There is community demand for platform support. Of the 12 [`time` PRs] that are open or have been merged in the past year, 5 are related to platform support. Of the 6 [`rustc-serialize` PRs] that are open or have been merged in the past year, 4 are related to platform support. Just ignoring platform support seems like a weird choice.

[`time` PRs]: https://github.com/rust-lang-deprecated/time/pulls
[`rustc-serialize` PRs]: https://github.com/rust-lang-deprecated/rustc-serialize/pulls

### Adopt a strict “no maintance” policy

Stop doing accepting any PRs for deprecated crates and guarantee no future release. This includes no more security fixes.

### Implement workarounds using Cargo

There has been some discussion of how to handle platform support for uncooperative upstreams in https://github.com/rust-lang-nursery/portability-wg/issues/6. One idea was to add additional support for source replacement focussed on portability in Cargo. This seems like a lot of work compared to the alternatives.

# Prior art
[prior-art]: #prior-art

### `std`

The main relevant prior art is how deprecation works in `std`. Because of Rust's stability guarantee, deprecated functionality will be maintained for a very long time. It will have bugs fixed and it will be ported to new platforms.

### Linux

Linux might be another informative example. In Linux, subsystems are never deprecated. They may be supplanted by superior subsystems, however, the old subsystems will keep functioning. After a long time, if all users have switched over to the new subsystem and there are no users of the old system left, the old subsystem may be removed entirely (with no deprecation notice/period).

# Unresolved questions
[unresolved-questions]: #unresolved-questions

### What is a popular crate?

This RFC arbitrarily defines a popular crate as one that has had at least 50,000 crates.io downloads in the past 90 days. Is this number good? Will this number need to change in the future?

### Should “bug fix” be further defined?

Should https://github.com/rust-lang-deprecated/time/pull/168 have been merged? This RFC doesn't explicitly define “bug fix” but leaves it up to the maintainer.

# Future possibilities
[future-possibilities]: #future-possibilities

None considered right now.
