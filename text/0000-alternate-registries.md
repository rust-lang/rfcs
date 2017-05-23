- Feature Name: alternate-registries
- Start Date: 2017-05-23
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Adding support for alternative crates.io servers to be used alongside the public crates.io server.

# Motivation
[motivation]: #motivation

Currently cargo only has support for getting crates from a public server, this is fine for
open source projects using Rust, however for closed source code this is problematic. Currently
the only real option is to use Git repositories to specify the packages, but that means that
all of the nice versioning and discoverability that cargo and crates.io provides is lost. I
would like to change this so that it is possible to have a local crates.io server which private
crates can be pushed to, plus still be able to make use of the public crates.io server as well.

# Detailed design
[design]: #detailed-design

There are a number of different areas which will likely need to be tackled in order to fully
support a local crates.io server in an enterprise. Below are some key areas:

* Add support for alternate crates.io registries (this RFC)
* Provide support for caching crates used on a crates.io proxy
* Add support to crates.io to allow authentication with other OAuth providers than github
* Support for private storage of crates on crates.io server rather than publicly available on S3

Rather than trying to get everything agreed in a single RFC I would like for these to be
dealt with as separate proposals and use this RFC as a stepping stone to be able to
meaningfully start to use private registries.

See https://github.com/rust-lang/cargo/pull/4036 for the code changes which this proposal was based on.

## Cargo.toml config changes
The following changes for Cargo.toml are proposed as part of this RFC:
* Add an optional 'registry' field to be specified as part of the package dependencies
* Add an optional 'registry' field to be specified in the package definition

Both of the registry entries take a value of a crates.io git repository. If one is not
provided then the default crates.io repository is assumed, this ensures that it is
back compatible with all current crates.

Below is an example of the change that we are proposing to make:

```
[package]
name = "registry-test"
version = "0.1.0"
authors = ["Christopher Swindle <christopher.swindle@metaswitch.com>"]
registry = "https://github.com/my_awesome_fork/crates.io-index"

[dependencies]
libc = { version = "*", registry = "https://github.com/my_awesome_fork/crates.io-index" }
serde_json = "1.0.1"
```

## Changes for alternative registries for dependencies
This boils down to a very simple change, where we previously setup the crate source for the
crates.io registry, we now just need to check if a registry is provided, if it has the crate
source is created using the registry URL, otherwise the crates.io server is used.

## Blocking requests to push to a registry
There are two parts to this, the first is a change to Cargo which checks if the registry
provided in the registry matches the host for the publish, if it does not it gets rejected.
The second part is a change to crates.io which will just reject the request to publish the
crate if the configured repository on the crates.io server does not match the registry
specified in the package or dependencies.

## Allow publishing when referencing external dependencies
We still want to support private crates having dependencies on the public crates.io server,
so we propose relaxing a check which ensures that the source for a dependency matches the
registry. We propose that this only performs the check only if the dependency is not the
default registry, thus allowing private crates to reference public crates on crates.io.

## Making it easier for users using an alternate crates.io registry
When a user selects a specific crate the Cargo.toml fragment would be updated to include the
registry URL, thus allowing users to easily copy and paste into their projects Cargo.toml
file.

# How We Teach This
[how-we-teach-this]: #how-we-teach-this

The term alternative registry would seem the most appropriate to describe this feature.

In the first instance I think that the Cargo.toml format documentation is sufficient to
provide access to the feature. However, in time once more of the pieces fall into place
it would be useful having a guide on how to setup/administer a crates.io server in an
enterprise setting (similar to the initial mirroring documentation).

# Drawbacks
[drawbacks]: #drawbacks

Currently this design requires that when you want to push to the private crates.io server
you need to override the host and token, it would be possible to update cargo to support
multiple registries tokens which can be used to login.

# Alternatives
[alternatives]: #alternatives

## Using a single server for cache and private registry
It was considered proposing a single crates.io server which performs both caching of crates.io,
plus has the ability to have crates pushed to it, however this has the following drawbacks:
* It requires crates.io to be able to combine two registries, or requires a radical change to the way crates.io works
* The current proposal could be extended to support this, if a caching server is added at a later stage

## Including registry definitions in a global location
We considered using a global configuration file (eg ~/.cargo/config) to allow a registry to
be specified, however this was ruled out on the basis that we believe that the registry to
use for dependencies is tightly linked to the project and hence it would be wrong to move
this into global configuration.

# Unresolved questions
[unresolved]: #unresolved-questions
As mentioned in the design section, this does not answer all of the questions for
supporting a private crates.io server, but it provides the first steps in that
direction, with the remaining areas considered out of scope for this RFC.
