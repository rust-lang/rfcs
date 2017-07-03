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

Both of the registry entries take a value of a crates.io web server. If one is not
provided then the default crates.io URL is assumed, this ensures that it is
back compatible with all current crates.

Below is an example of the change that we are proposing to make:

```
[package]
name = "registry-test"
version = "0.1.0"
authors = ["Christopher Swindle <christopher.swindle@metaswitch.com>"]
registry = "https://www.my_awesome_fork.com/"

[dependencies]
libc = { version = "*", registry = "https://www.my_awesome_fork.com/" }
serde_json = "1.0.1"
```

## Mapping Between Registry and Git Repository
Currently the Git repository is used internally within Cargo, in order for Cargo to map
between the servers URL and a Git respository for the crates.io web server, the server
exposes a set endpoint which provides the Git repository to use, for example:

```
  http://crates.io/git
```

This will just return the Git repository to use, which the server already knows. When cargo
needs to perform an action on an alternate registry it just performs a GET request
on the URL and then uses the Git repository returned.

## Dependency resolution
This RFC proposes going to an alternative registry for a dependency only if the registry key
is present in the dependency. This means that in situations where there is the same name
crate on both crates.io and the alternate registry, if no registry is provided it will
use the one from crates.io. There are valid situations where people may wish to override
a particular crate with an alternate one, however the existing source replacement feature
seems a better fit to solve that scenario.

## Crate naming on alternate servers
It would be sensible, on an alternate server, for users not to publish using the same
name as a public crate as that will cause issues if someone need to use a public and private
crate with the same name. In order to minimise the risk of this happening I propose
that an optional field is added to the crates.io config which allows a prefix to be configured,
when a crate is published, the prefix is checked and the request rejected with a sensible error
if the crate name does not match.

## Changes for alternative registries for dependencies
This boils down to a very simple change, where we previously setup the crate source for the
crates.io registry, we now just need to check if a registry is provided, if it has the crate
source is created using the registry URL, otherwise the crates.io server is used.

### Index files changes
As Cargo requires the index file to include all the dependencies, the crates.io index file
format is updated to include the registry in the dependency. The registry is an optional field,
where by default it is None, and will only be set when using an alternate crate server. The
official public crates.io server will block any publish requests which contain a registry
in dependencies, so for crates.io this will always be set to None.

Validation of the depdencies on crates.io is also updated so that local registry crates are
checked for their existance on the local registry, they will not however validate any
external dependencies, instead they will be assumed to be valid.

## Blocking requests to push to a registry
Cargo will by try and publish to the registry that is provided in the Cargo.toml (if one is
provided), if the registry has been override and it does not match the entry in Cargo.toml it will
be rejected. The second part is a change to crates.io which will just reject the request to
publish the crate if the configured registry on the crates.io server does not match the
registry specified in the package or dependencies.

## Allow publishing when referencing external dependencies
We still want to support private crates having dependencies on the public crates.io server,
so we propose relaxing a check which ensures that the source for a dependency matches the
registry. We propose that this only performs the check only if the dependency is not the
default registry, thus allowing private crates to reference public crates on crates.io.

## Making it easier for users using an alternate crates.io registry
When a user selects a specific crate the Cargo.toml fragment would be updated to include the
registry URL, thus allowing users to easily copy and paste into their projects Cargo.toml
file. Below is an example of how this might look:

<img src="http://i.imgur.com/znMMwAc.jpg" alt="Example layout of crates.io with URL in fragment" />

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

## Validating external dependencies
It would be possible to check that external dependencies exist, however I think that this
would need to be optional as there could be situations where the alternate crates.io
server would be unable to contact the external dependencies and hence would not be
possible. This is certainly something that could be added at a later stage if there is
sufficient demand.

# Unresolved questions
[unresolved]: #unresolved-questions
As mentioned in the design section, this does not answer all of the questions for
supporting a private crates.io server, but it provides the first steps in that
direction, with the remaining areas considered out of scope for this RFC.
