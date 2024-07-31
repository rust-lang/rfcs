- Start Date: 2024-07-31
- RFC PR: [rust-lang/rfcs#3674](https://github.com/rust-lang/rfcs/pull/3674)

# Summary
[summary]: #summary

This RFC proposes to remove "dev-dependencies" from the git and sparse index of crates.io. 

# Motivation
[motivation]: #motivation

Currently, the index of crates.io contains information about dev-dependencies of crates. However, this information is [apparently not used by cargo](https://rust-lang.zulipchat.com/#narrow/stream/246057-t-cargo/topic/Is.20dev-dependency.20information.20from.20the.20index.20used.3F) and never was. This unnecessary information increases the size of the index and thus the time it takes to download it. With the sparse index, the download time is already significantly reduced, but for crates with many dev-dependencies, the performance could still be improved.

crates.io currently has about 9 million regular dependencies and 1.5 million dev-dependencies in its database. The dependencies part only contributes to a subset of the total size of the index, so it is unlikely that we will see a 15% size reduction in the index. Still, having 1.5 million dev-dependencies in the index is a significant amount of data that is not used by cargo and is unnecessarily transferred to the users of crates.io.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

From the user's perspective, this change should not have any impact. The `cargo` command will continue to work as before. The only difference is that the index will be smaller and the download time will be reduced.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The crates.io server will still process and save dev-dependencies in the database, but it will no longer include them in the index. To be more precise, any item in the `deps` field with `"kind": "dev"` will be removed from the index.

To reduce the amount of unnecessary commits to download for users of the git index we could implement this in a way where dev-dependencies are only removed from an index file if a release for the corresponding crate is being published and the file needs to be touched anyway. We could keep running in this state for a couple of weeks/months and then later trigger a full sync when a bigger chunk of the actively maintained crates have already been updated, reducing the amount of commits needed for the migration.

# Drawbacks
[drawbacks]: #drawbacks

- This change will temporarily increase the size of the git index, due to the amount of file changes necessary to remove the dev-dependencies from the index. This could potentially be coupled with an index squash though, which would reduce the size of the index again.

- This change could potentially break other users of the index, if they rely on the dev-dependencies being present in the index. Part of the reason for this RFC is to see whether there are any users of the dev-dependencies in the index and what we could do to help them migrate to a different solution.

# Prior art
[prior-art]: #prior-art

[margo](https://github.com/integer32llc/margo) is a private crate registry implementation that explicitly does not include dev-dependencies in the index. The development of margo triggered the initial discussion on Zulip about whether dev-dependencies are used by cargo or not.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Are there any legitimate uses of the dev-dependencies in the index?

# Future possibilities
[future-possibilities]: #future-possibilities

- crates.io could consider allowing crates to be published if their **dev**-dependencies are not also available on crates.io.
