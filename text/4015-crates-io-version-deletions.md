- Start Date: 2026-10-06
- RFC PR: [rust-lang/rfcs#4015](https://github.com/rust-lang/rfcs/pull/4015)

## Summary
[summary]: #summary

This RFC proposes a mechanism for crate authors to delete versions of their crates from crates.io
under certain conditions. This is an extension of [RFC #3660][rfc-3660] that allowed crate authors
to delete entire crates only.

## Motivation
[motivation]: #motivation

The functionality for deleting entire crates proposed in [RFC #3660][rfc-3660] has been available
[since Jan 25, 2025](https://github.com/rust-lang/rfcs/pull/3660#issuecomment-2593348002). This
feature has successfully lessened the support burden for crate deletions, and has not caused any
negative ecosystem impacts we're aware of.

[Increasingly](#show-me-the-numbers), however, crates.io is getting support requests from crate
owners who would like to delete particular _versions_ of their crates, but not the entire crate.
Examples of reasons for wanting to delete particular versions are accidental publishes of:

- Proprietary code or data
- Authentication tokens
- Incorrect license declarations
- Personal information

The owners have usually already yanked the relevant versions and understand that the contents may
have been mirrored elsewhere already, but they would like crates.io and docs.rs to stop serving the
content completely. By the time they have made a support request, they have published fixed
versions of the crate, so they don't want to delete the entire crate and risk losing the crate name.

Originally, [deleting individiual versions was part of RFC #3660][https://github.com/rust-lang/rfcs/pull/3660#discussion_r1647411989], but [was
removed](https://github.com/rust-lang/rfcs/pull/3731) because the scope of resolving versions to
determine there weren't crates depending on a particular version [turned out to be rather large and
complex](https://github.com/rust-lang/rfcs/pull/3660#discussion_r1647844396).

However, in most of these recent support cases, the versions belong to crates where the entire
crate would be eligible for deletion (because it has been published recently, has a small number of
downloads, and there are no other crates on crates.io depending on the entire crate). So this RFC
proposes allowing crate owners to delete particular versions of their crate in cases where they
would be allowed to delete the entire crate. Restricting version deletion to crates that have no
reverse dependencies thus avoids the complexity of version resolution that prevented us from
implementing version deletion initially, while still allowing crate owners to manage version
deletion themselves in this case.

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

For a crate that you own, you'll be able to go to the crate's "Settings" tab. Under the existing
"Danger Zone" heading where there is currently a "Delete this crate" button, there will be some
mechanism for deleting individual versions (the exact user interface will be worked out during
implementation).

When you go to delete individual versions of a crate, first you will get information on the
potential impact and the requirements for version deletion that will largely be the same as what is
shown today for crate deletion with relevant changes for versions. It will look similar to this:

> Delete versions of the [crate-name] crate?
>
> Are you sure you want to delete versions of the crate "[crate-name]"?
>
> Important: This action will permanently delete the versions you select. Deleting versions cannot
> be reversed!
>
> Potential Impact:
>
> - Users will no longer be able to download these versions.
> - Any dependencies or projects relying on these versions will be broken.
> - Deleted versions cannot be restored.
> - Publishing a version with the same number will be blocked for 24 hours.
>   (NOTE: see the [Unresolved Question][unresolved-questions] about blocking republishes)
>
> Requirements:
>
> A version can only be deleted if its crate is not depended upon by any other crate on crates.io.
>
> Additionally, a version can only be deleted if either:
>
> 1. the version has been published for less than 72 hours
>
> OR
>
> 2. a. the crate only has a single owner, *and*
>    b. the version has been downloaded less than 1000 times for each month it has been published.
>
> Reason:
>
> Please tell us why you are deleting these versions:
> [text input]
>
> [ ] I understand that deleting these versions is permanent and cannot be undone.

This interface will include a UI along the lines of checkboxes for every version so that multiple
versions can be deleted at once with one specified reason.

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Version deletion will have a new API endpoint:

- `DELETE /api/v1/crates/:crate_id/versions`

that will look for a query parameter `nums[]` to determine which version numbers should be deleted.

If there are no `nums` specified, this endpoint will return an error that the request was invalid
with a message indicating that `nums` is required. Nothing will be deleted.

If all of the crate's `nums` are specified, this endpoint will return an error that the request was
invalid with a message indicating that the requester should either delete the crate instead or
leave at least one version. Nothing will be deleted.

The deletions will be recorded in a `deleted_versions` table along with the reasons specified for
the deletion, in the same way as `deleted_crates` stores this information for crate deletions today.

## Drawbacks
[drawbacks]: #drawbacks

Much as in [RFC #3660](https://github.com/rust-lang/rfcs/pull/3660), this would make the crates.io registry slightly less immutable under these circumstances. This could cause confusion in cases where a version is deleted that is depended on by other projects that are not published on crates.io themselves.

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

[RFC #3660][rfc-3660] originally wanted to allow version deletions if there were no dependencies on
crates.io that must resolve to that version. We could try to find better solutions to that problem.

## Prior art
[prior-art]: #prior-art

[RFC #3660][rfc-3660] that allowed crate authors to delete their own crates in certain
circumstances has been going well. It has reduced the number of support requests for full crate
deletions, and hasn't caused any undesired ecosystem effects that we know of.

## Unresolved questions
[unresolved-questions]: #unresolved-questions

- Should version numbers be blocked from being republished for 24 hours, as crate names are?

## Future possibilities
[future-possibilities]: #future-possibilities

- Allow deletion of crates and versions whose only dependencies are owned by the account requesting
  deletion, so that the person doing the deletion is only potentially breaking their own crates.
  Sometimes owners get into cycles using dev-dependencies and the current restrictions don't allow
  them to delete the crates, even though they would be the only user affected. Changing this
  restriction would further reduce the support burden, but would likely need its own RFC.

[rfc-3660]: https://github.com/rust-lang/rfcs/pull/3660

## Show me the numbers

Here are support request counts by month in 2026 through September. It is left as an exercise for
the reader to imagine _why_ these requests are increasing.

| Month | Number of requests | Total number of crates affected | Total number of versions deleted |
|-------|--------------------|---------------------------------|----------------------------------|
| Jan 2026 | 0 | 0 | 0 |
| Feb 2026 | 0 | 0 | 0 |
| March 2026 | 0 | 0 | 0 |
| April 2026 | 0 | 0 | 0 |
| May 2026 | 3 | 21 | 26 |
| June 2026 | 0 | 0 | 0 |
| July 2026 | 2 | 2 | 2 |
| August 2026 | 4 | 5 | 7 |
| September 2026 | 5 | 9 | 832 |
