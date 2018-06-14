- Feature Name: verified_registry_commits
- Start Date: 2018-05-08
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Enable commits to crate registry indexes to be signed & verified, providing
stronger guarantees as to the authenticity of their content and (transitively)
guaranteeing the authenticity of the content of the packages in the registry.

# Motivation
[motivation]: #motivation

Crate registries like the crates.io registry are a way of distributing source
code which is ultimately built and executed on end user machines. It is vital
that users receive valid source files for the packages they download,
especially because a malicious package could be a major attack vector to harm
users of Rust, or users of those users' projects.

The index of a registry is a git repository downloaded by cargo over HTTPS or
SSH. This index contains information about each package, including a SHA-256
checksum of the content of that package. When cargo downloads a package from
the registry, its contents are verified against that checksum. This is intended
to guarantee that the content the user downloads is the same data this referred
to by the registry index.

However, a malicious or otherwise ill-behaved third party could conceivably
(such as with a MITM attack) intercept the download of the index and modify the
checksum of a package, allowing them later to modify the content of that
package when the user requests it. They could also, conceivably, modify the
index repository at its central storage, making similar malicious edits which
will be accepted by every user.

If the content of the registry index could be authenticated in a stronger way,
this would make it more difficult for an attacker to modify index data. Because
the registry is a git repository, and a git repository is a kind of [merkle
tree][merkle], signing a commit verifies the content of all the data that
commit contains (modulo the security properties of the repository's hash
function - a discussion of SHA-1 is later in this RFC). Because a hash of each
package contents is contained in the index repository, the index as a whole can
be thought of as a merkle tree, some of the leaves of which are all of the
packages in the registry.

Signing commits is an effective and cheap way to provide stronger authenticity
of package contents. It enables cheap key rotation, because a new signature on
the head of the index repository validates all of the content in the registry.
Because git already supports commit signing, it is a natural extension of our
existing practices, rather than a large scale re-engineering of the registry
system.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

For normal users, this change should be completely transparent. Whenever the
index is updated if the registry in question is a registry that ought to be
signed (a 'signed registry'), the signature of HEAD is verified against the
public keys which are permitted to sign commits to the index.

## The keys TOML

Inside the `.cargo` directory, a TOML file exists for each signed registry
containing the public key data for all signers. Public key data is an object
containing these members:

- `key`: An ASCII armored OpenPGP public key (conforming to RFC 4880)
- `info` (optional): Information about this key, such as the user it is
  supposed to belong to, which is not cryptographically verified.
- `can-commit` (optional): Whether or not this key can be used to create
  commits to the registry. Boolean; Defaults to false.
- `can-rotate` (optional): Whether or not this key can be used to perform a key
  rotation on this registry. Boolean; Defaults to false.

This file is maintained through a key rotation mechanism described later in the
RFC. It is plain text, and users may freely review or edit it for each registry
they depend on (though if they remove keys that are not considered invalid by
the registry administrators, they may break their dependency on that registry).

## When is a registry a signed registry?

Registries are considered signed registries if either of these hold true:

1. Current `HEAD` is a signed commit (by any key).
2. A keys file exists for that registry.

An attempt to update the `HEAD` of a signed registry to a commit that is not
signed by one of the existing committer keys is a hard failure that will
prevent cargo from resolving dependencies in that registry any longer. Until
the state is fixed, cargo will not generate lockfiles for crates that depend on
packages in that registry. This includes a commit which is not signed at all.

If the HEAD of a registry is signed but that registry has no signing keys TOML
file, that registry will be considered a signed registry, but in a broken state,
because HEAD is signed but there are no trusted signing keys. In general, this
definition of signed registry is supposed to "fail closed."

## Why PGP formatted keys?

This RFC specifies that keys and signatures are exchanged using PGP format,
even though it also adds that cargo will not ship a full PGP implementation to
verify signatures. The PGP format is a rather complex binary format which
supports many options that are not relevant for our use case: as a result,
we've specified that we only support a subset of the PGP format. One could
fairly ask why we use PGP at all instead of a more straightforward solution.

The primary reason to use the PGP format is to integrate with existing git &
gpg tooling. The subset of PGP we support is compatible with keys generated by
gpg2 and with signatures on commits made with `git commit -S`. This allows
users to manually produce and verify signatures as necessary either for
administrative purposes or to check the correctness of cargo's behavior.
Additionally, GitHub has a mechanism to host GPG keys associated with a GitHub
account, which would not be possible for a custom key format.

## Advisory for people running registries

cargo will only validate the signature of the `HEAD` commit, no intermediate
commits. All committers to the registry **MUST** verify the signature of
parents to their commits, or the security properties of this system will be
violated.

Registries are free to make their own policies regarding the distribution of
keys and when to perform rotations, but if a registry operator commits
without verifying the signature of that commit's parents, they have
nullified the benefits of running a signed registry over an unsigned
registry.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Signature schemes & data formats supported by cargo

Though keys are stored in the OpenPGP format, cargo will not actually verify
signatures using a complete OpenPGP implementation such as gpg, which would be
a significant new C dependency to ship to all users. Instead, cargo will use
pure Rust implementations of established signature schemes, as well as a pure
Rust parser for a subset of the OpenPGP format.

All signatures are valid OpenPGP signatures, which means they are signatures of
hashes of the data being signed as well as metadata in accordance with the
OpenPGP spec ([IETF RFC 4880][rfc4880]).

### OpenPGP subset

All signatures and public keys will be distributed in a subset of the data
format described in RFC 4880 which is consistent with the behavior of recent
versions of gpg2 (allowing the manual creation of signatures and keys as
necessary).

These are the requirements for a signature or public key to be supported by
cargo:

- It uses what RFC 4880 calls the "old format packet header" for all packets
- All packets have a two octet length (so the packet header is 3 octets long).
- Signatures and public keys are both version 4 of their format.
- It uses one of cargo's supported signature and hash algorithms.
- The first hashed subpacket of any signature is the public key fingerprint
  with subpacket type 33 (note this is the 20 byte fingerprint, not the 8 byte
  key ID).

This conforms to the default behavior of gpg2 and is what is accepted and
generated by [a Rust implementation of a subset of OpenPGP written by the
RFC author][pbp].

### Signature and hash algorithms

Our initial implementation will support only one signature algorithm and one
hash algorithm. This may be extended in time. The signature algorithm is EdDSA
with curve 25519 (ed25519) and the hash algorithm is SHA-256.

## Signature distribution format

Signatures are distributed in the same manner that git distributes commit
signatures - as an additional header on the commit, verifying the commits
contents, commit message, and other headers. This way, users can verify
index commits using their own version of git, and administrations making manual
edits to the registry can use git to generate the signatures.

## Key rotation

Key rotation is performed by adding an annotated tag to the index repository,
called a key rotation tag. A key rotation tag must be signed by a key with the
`can-rotate` privilege.

A key rotation tag should point to the HEAD commit of the registry index at the
time it is made. The commit it points to should be signed using a key which is
in the post-rotation key set.

The name of a key rotation tag must be `rotation-$FORMAT-$N` where $N is an
integer and $FORMAT is the format used for this rotation; the only format
specified by this RFC is `v1`. We recommend that registries use a counter
starting at `0` for `$N`. The number is only significant to distinguish
rotations from each other, and has no semantic role.

The message body of a key rotation tag in the `v1` format is a TOML document,
having the same structure as the keys toml which contains the authorized
signing keys described previously in the RFC.

When cargo updates the index, it will iterate through the new tags matching
this format that have been added to the repository in their order in the commit
history. cargo will verify that the tag is signed by a key with the
`can-rotate` privilege and then update the trusted key set by completely
replacing it with the signing keys.

## crates.io initial policy

crates.io keys will always contain a GitHub user account name in their `info`
field (possibly along with other info), and will publish those keys to that
GitHub account. The current set of crates.io keys will be published on
rust-lang.org as well. Users can use these other forms of publication to verify
that the keyset they have is valid.

A single key, belonging to the bors account, will be stored in the crates.io
service. This key will only have the commit privelege, not the rotate
privilege. Other keys, with various priveleges, will be stored in an offline
format, and belong to individual members of the Rust core team. These keys will
be used for administrative purposes.

Keys, especially the online bors key, may be rotated at irregular intervals,
not necessarily because of a known compromise. An explanation of the rotation
will always be published at rust-lang.org. We do not commit to any particular
rotation schedule.

## Security considerations

### Trust on first use

When a user first downloads a registry index, or transitions an index from
unsigned to signed, they have no pre-existing trusted keys for that registry.
For this reason, the first access of a signed registry is a leap of faith in
which the user also obtains the keys to trust for future updates. A successful
attack at that point would leave the user will with an invalid index.

We can harden crates.io against attacks at this point by distributing the
current trusted keys with the rustup distribution, allowing any security
mechanisms we put in place for the distribution of Rust and cargo binaries to
also secure this operation.

### An attacker with no keys

An attacker with no keys cannot sign commits or rotation tags. Because of this,
an attacker with no keys would not be able to modify the registry index, even
if they were able to defeat the existing security measures like our use of
HTTPS or SSH to transmit the index data over the network.

However, such an attacker could still prevent the user from receiving updates
to the index by - for example - MITMing their connection to the git server to
report that there is no update. Such an attack could keep users from receiving
essential security updates to their dependencies. Hardening cargo against this
sort of attack is left as future work.

### An attacker with a `can-commit` key

An attacker who has compromised a key with the `can-commit` privilege could
make commits to the index, modifying the data. This would essentially revert
the system to its security properties before this RFC.

However, if a key compromise of this sort is discovered, an automated key
rotation could remove the compromised key from the set, restoring the security
properties of the system.

### An attacker with a `can-rotate` key

An attacker who has compromised a key with the `can-rotate` privilege could
rotate the key set, adding and removing keys at will. This would allow them to
take control of the index, preventing legitimate updates (such as key
rotations) from reaching users. If a `can-rotate` key were compromised, it
would likely be very disruptive to all users.

For that reason, crates.io will adopt policy that `can-rotate` keys are stored
in an offline medium, with due security precautions. As a future hardening, we
could also support a threshold signature scheme, requiring signatures from
multiple `can-rotate` keys to perform a key rotation, reducing the impact of
compromising a single key with this privilege.

### SHA-1

The security of this system hinges on the security of the hash function used to
implement the git repository. Unfortunately, git currently uses SHA-1, a hash
function with known flaws that allow for collision attacks. A successful
collision attack against a crate index would nullify the security benefits of
this RFC: an attacker would be able to swap out one commit for another which
would both appear to be signed.

However, there are a few mitigating factors which make this RFC worthwhile to
pursue despite these problems:

1. The SHAttered collision attack depends on the ability to control data in
both colliding objects in order to create a collision (that is, they cannot
create a collision with an arbitrary hash). Hypothetically, an attacker could
upload a crate to the registry with index metadata that they can use to create
a collision; however, this increases the difficulty significantly in comparison
to the SHAttered case.
2. Even the SHAttered case was prohibitively expensive. Breaking into one of
our administrators' homes to copy their signing key is probably cheaper.
3. Our current git host, GitHub, checks for the signs of this sort of collision
attack and would not accept an object containing a collision.

That said, we take the weakness of SHA-1 seriously, and will commit to
switching to a stronger hash function as soon as it is possible for us to do
so. In order to do that, git needs to be updated to support a new hash
function, and that upgrade needs to be supported by both GitHub (which hosts
our index) and libgit2 (which cargo uses for git operations).

# Drawbacks
[drawbacks]: #drawbacks

The primary drawback of this is that it increases the operational complexity of
managing crates.io, and the complexity of the cargo codebase. The Rust
infrastructure and dev-tools teams would be taking on the burden of maintaining
this system & provisioning and protecting secret keys. The additional security
benefits of this RFC will depend on their key management practices.

# Rationale and alternatives
[alternatives]: #alternatives

The primary alternative to a security enhancement like this would be to switch
wholesale to [TUF][tuf], a complete, designed framework for secure upgrades.

TUF as described by its spec is not backward compatible with the existing
registry index format that cargo uses. We could not simply bolt on TUF to
cargo, we would have to do a complete system change over. Such a change over is
expensive in terms of our resource allocation and in terms of the user
experience of migration: realistically, we would not be able to prioritize it
in the near future. Even if we do some day switch to TUF, it is worthwhile to
moderately improve the security of our system *now*.

The specified format for TUF target files is also not designed to be an
efficient mechanism for storing the index for a package repository like cargo.
It stores the data cargo would store in its index in a single, large JSON file.
cargo's use of a git-based index was designed on the basis of its original
authors' experience with package managers like this - specifically, the issue
of monotonic, incremental updates, and of parsing only the information for the
specific packages needed in dependency resolution. For this reason, TUF would
likely need to be modified nontrivially to support crates.io, making it an even
larger task to switch to TUF.

Given all of this, an incremental change like signing the repository git
commits seems advantageous given that we cannot perform a migration to TUF at
this time.

# Prior art
[prior-art]: #prior-art

The most important prior art is the [TUF][tuf] specification, described in the
previous section. It would be good to continue to take learnings from TUF as we
try to improve the security of our infrastructure.

# Unresolved questions
[unresolved]: #unresolved-questions

No major unresolved questions as of this time.


[tuf]: https://theupdateframework.github.io/
[rfc4880]: https://tools.ietf.org/html/rfc4880
[pbp]: https://github.com/withoutboats/pbp
[merkle]: https://en.wikipedia.org/wiki/Merkle_tree
