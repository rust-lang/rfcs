- Start Date: 2024-06-20
- RFC PR: [rust-lang/rfcs#3660](https://github.com/rust-lang/rfcs/pull/3660)

# Summary

This RFC proposes a mechanism for crate authors to delete their crates from crates.io under certain conditions.


# Motivation

There are a variety of reasons why a crate author might want to delete a crate from crates.io:

* You published something accidentally.
* You wanted to test crates.io.
* You published content you didn't intend to be public.
* You want to rename a crate. (The only way to rename a package is to re-publish it under a new name)

The current [crates.io usage policy](https://crates.io/policies) says:

> Crate deletion by their owners is not possible to keep the registry as immutable as possible.

This restriction makes sense for the majority of crates that have been around for a while and are actively used, but the above list of reasons shows that there are valid use cases for allowing crate authors to delete their crates without having to contact the crates.io team.

To make this process easier for our users and to reduce the workload of the crates.io team dealing with such support requests, we propose to codify our current set of informal rules into a formal policy that allows crate authors to delete their crates themselves under certain conditions (see below).


# Proposal

We propose to allow crate authors to delete their **crates** from crates.io under the following conditions:

* The crate has been published for less than 72 hours,
* or if all the following conditions are met:
  * The crate has a single owner,
  * The crate is not depended upon by any other crate on crates.io (i.e. it has no reverse dependencies),
  * The crate has been downloaded less than 100 times for each month it has been published.

This crate owner action will be enabled by a new API endpoint:

- `DELETE /api/v1/crates/:crate_id`


# Drawbacks

> Why should we *not* do this?

The main drawback of this proposal is that it makes the crates.io registry less immutable.
This could lead to confusion if a crate is deleted that is depended on by other projects that are not published on crates.io themselves.
However, we believe that the conditions we propose are strict enough to prevent this from happening in practice due to the additional download threshold.


# Rationale and alternatives

> Why is this design the best in the space of possible designs?

The proposed design is based on the current informal rules that the crates.io team uses to decide whether to delete a crate.
These rules have been derived from the npm registry, which has a similar policy (see below).
We believe that the proposed conditions are strict enough to prevent accidental deletions while still allowing crate authors to delete their crates in the cases where it makes sense.

> What other designs have been considered and what is the rationale for not choosing them?

We considered not having restrictions on the number of reverse dependencies, but since that would leave the package index in an inconsistent state, we decided to require that the crate has no reverse dependencies.
Situations like the [`everything` package on npm](https://uncenter.dev/posts/npm-install-everything/) require manual intervention anyway, so we decided to keep the restrictions strict.

> What is the impact of not doing this?

The proposed design is based on the current informal rules that the crates.io team uses to decide whether to delete a crate or version. If we don't implement this proposal, we will continue to rely on the crates.io team to handle these requests manually, which is time-consuming and error-prone.

# Prior art

## npm

The main inspiration for this proposal comes from the npm registry, which has a similar policy for deleting packages and versions:

- https://docs.npmjs.com/policies/unpublish
- https://docs.npmjs.com/unpublishing-packages-from-the-registry

The npm registry started with a more permissive policy, but had to tighten it over time. 
It started out with a policy that allowed package owners to delete their packages at any time, but this led to a number of issues, [such as packages being deleted that were depended on by other packages](https://en.wikipedia.org/wiki/Npm_left-pad_incident).
Their policy was later changed to require that packages can only be deleted within 72 hours of being published, and then [changed again in January 2020](https://blog.npmjs.org/post/190553543620/changes-to-npmunpublish-policy-january-2020) to allow deletions outside the 72-hour window under certain conditions.


## PyPI

The Python Package Index (PyPI) still allows package owners to delete their packages (or a subset of released files) at any time.
A member of the PyPI team has proposed to [stop allowing deleting things from PyPI](https://discuss.python.org/t/stop-allowing-deleting-things-from-pypi/17227) due to the same issues that the npm registry faced. The most current proposed ruleset can be found [here](https://discuss.python.org/t/stop-allowing-deleting-things-from-pypi/17227/71).

Their proposal is also inspired by the npm registry policy, but notably does not include a reverse dependency restriction. It seems that PyPI might not currently be tracking dependencies between packages, which would make it harder for them to implement such a restriction.

## Others

<https://discuss.python.org/t/stop-allowing-deleting-things-from-pypi/17227/59> contains a list of other package registries and their deletion policies.


# Unresolved questions

## Should names of deleted crates be blocked so that they can't be re-used?

The reason for this would be to prevent someone else from re-publishing a crate with the same name, which could lead to potential security issues.
Due to the restrictions on the number of downloads and reverse dependencies, this seems like a low risk though.
The advantage of allowing others to re-use such names is that it allows name-squatted/placeholder crates to be released back to the community without the crates.io team having to manually intervene.

The npm registry blocks re-use of deleted package names for 24 hours.


# Future possibilities

It is conceivable that the restrictions could be adjusted in the future if the crates.io team finds that the proposed restrictions are too strict or too lenient. For example, the download threshold could be adjusted based on how well the proposed ruleset will work in practice.

Once the backend of crates.io has been updated to support this feature, we could also consider adding a web interface for crate owners to delete their crates and versions directly from the crates.io website. Similarly, we could add a subcommand to the `cargo` CLI, either implemented as a plugin or as part of the main `cargo` codebase.
