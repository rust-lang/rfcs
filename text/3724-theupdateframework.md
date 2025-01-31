- Feature Name: Rust Release & Crate Signing using TUF
- Start Date: 2024-10-31
- Authors: Josh Triplett, Walter Pearce
- RFC PR: [rust-lang/rfcs#3724](https://github.com/rust-lang/rfcs/pull/3724)

# Summary
[summary]: #summary

`The Update Framework` (Reference: https://theupdateframework.github.io/specification/latest/) is designed to provide a standard methodology and tooling for a chain of trust to exist against a given repository of files. TUF provides mitigations and mechanisms which were provided for in the previous PKI RFC - much of the work and design in that RFC was mainly inspired by TUF.  The project currently has two major use cases in which we could benefit from artifact signing and key management: Releases and crates served from crates.io. This RFC specifies an implementation which can provide key management, artifact signing and mirroring for both cases. At its core, TUF provides a framework for specifying trusted keys and signing of artifacts resident in a repository, while providing defense in depth measures to mitigate against common attacks. Implementation of this RFC will provide the project with the following:
- Two distinct trusted roots for: Releases and Crates, with a relationship of trust from root->crates
- Crates will be secured by signing the crates.io index, and not the artifacts directly
- A quorum model for trusting of delegate keys and signing members
- Online and offline verification of files from the repository and the index itself
- Future ability for verified and out-of-band mirroring

We propose the creation two TUF repositories utilized for the distribution of signed content by the Rust Project, `rust-lang/tuf-root` and `rust-lang/tuf-crates` respectively. Given the disparity in the cadence of changes to each distribution channel, and the different chains of trust in each, we propose two distinct but related repositories. Finally, because metadata for all artifacts in the repository exist within a given TUF instance, we considered it best to separate these two sets of release files into their own metadata repositories.

These shall provide:

`rust-lang/tuf-root`
- Rust Stable/Beta Releases
- Rust Nightly Releases
- Rustup Releases
- Crates.io root role, to create a chain of trust

`rust-lang/tuf-crates`
- Crates.io crate index

# Motivation
[motivation]: #motivation

Rustaceans need to be able to download crates and know that they're getting the crate files that were published to crates.io without modification.

There are places where Rust is difficult to use right now. Using Cargo with crates.io works well for Rustaceans with unfirewalled access to high speed Internet, but not all are so lucky. Some are behind restrictive firewalls which they are required to use. Some don't have reliable access to the Internet. In cases like these, we want to support mirrors of crates.io in a secure way that provides cryptographic guarantees that they are getting the same packages as are provided by the Rust Project, without any risk of tampering.

Another reason for wanting to be able to better support mirrors is to address cost pressures on Rust. Approximately half of Rust release and crate traffic is from CI providers. Being able to securely distribute Rust crates from within CI infrastructure would be mutually beneficial, since it would both allow the Rust Foundation to reallocate budget to other uses and would make Rust CI actions faster and more reliable on those platforms.

Finally, supply chain security is a growing concern, particularly among corporate and government users of Rust. The Log4j vulnerability brought much greater attention to the problems that can occur when a single dependency nested arbitrarily deep in a dependency graph has a critical vulnerability. Many of these users are putting significant resources into better understanding their dependencies, which includes being able to attest that their dependencies verifiably came from specific sources like crates.io.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

For an understanding of terms utilized in this section, please see the [TUF reference](https://theupdateframework.github.io/specification/latest/).

#### What is TUF?

[The Update Framework](https://theupdateframework.io/) (TUF) is a software framework designed to protect software update repositories that automatically identify and download updates to software. TUF uses a series of roles and keys to provide a means to retain security, even when some keys or servers are compromised. It does this with a stated goal of requiring minimal changes and effort from repository administrators, software developers, and end users. ([Wikipedia](https://en.wikipedia.org/wiki/The_Update_Framework))

TUF provides a methodology and framework for managing and verifying a modern chain of trust between what it terms "roles", stemming from a root and allowing for a further delegation of roles for different functions and paths within the repository.

At the simplest level, TUF is a collection of signing key quorums which verify its own integrity, and establishes trust between the quorums and files within a repository.

#### Terminology

- `tuf`: The Update Framework and its specification
- `artifacts`: The actual content and files distributed and to be signed
- `targets role`: The role within TUF which can be N keys in a quorum or singular for signing artifacts.
- `repository`: One of the two TUF repositories which is where metadata for artifacts exist. This metadata is the signatures of files, snapshots, and TUF keying information.
- `root`: The top-level set of signers/keys which are used to validation the repository
- `delegate`: Another set of signers/keys which are used for performing specified authority within the repository

## Summary & Motivations

We propose the creation of two distinct TUF repositories for signing of Rust Project content and crates, respectively. Two main motivations exist for separating these concerns: The cadence of content published within each, and the trust of each. Rustup and Rust releases (both nightly and stable) are conducted under a controlled and predictable manner which is managed by the Project. However, crates are published by the community, and as such we see a larger and much more varied volume of content which may exist within this repository. We have additionally modeled signing the root of one repository by the other - this implicitly grants us a chain of trust from the "Project" (tuf-root) to the separate crates.io repository (tuf-crates). The sections below go into more detail on each repository and its configuration.

### `rust-lang/tuf-root`

tuf-root shall be used for signing Rust releases, rustup releases, and an independent and updated version of the tuf-crates root role metadata file (root.json).

#### Roles

##### Root Role

The root role of the tuf-root shall be a TUF role consisting of 9 members with a 5 member threshold for signing (5-of-9); please reference the Root Quorum Model section below for details on how this role should be managed and its members selected. The sole purpose of this role shall be delegating authority to the other roles within the tuf-root repository (when members of these roles change). Finally, this role shall also be used for signing the tuf-crates root.json - thus protecting the chain of trust between tuf-root and tuf-crates.

##### Targets Role

The repository shall have a top-level Targets role, as specified by TUF, which then delegates authority to the following delegate target roles:

##### Release (Stable/Beta/Rustup) Role

The Release role shall have the authority to only sign official releases. We propose this role also consist of a quorum model, consisting of all members of the release team. This role should have a 3 member threshold, and always consist of all members of the release team. The release team shall be responsible for the creation, management and administration of delegate keys utilized for its various releases in which it has authority (stable, beta, rustup). We recommend that any delegate automation keys be stored in a secure keystore and have a regular update and rotation schedule which shall require a quorum of the release  team to conduct; a timeframe of 3-6 month rotations is recommended.

###### Nightly Role

Nightly releases shall be conducted by a single signing key, trusted for only signing nightly releases of the Rust Compiler. This will allow for nightly releases to remain automated and not require the active participation of any rust members. This key shall remain separate than the official signing key.

###### Snapshots & Timestamps Role

These roles shall be a single-member role with a key utilized for automation.

### `rust-lang/tuf-crates`

The actual target for tuf-crates shall be the crates index and not the artifacts themselves. This means that the TUF repository for crates.io is performed on much smaller payloads, which still provides us with cryptographic security due to the fact the index contains SHA-256 hashes of the crate file artifacts.  Given the index already consists of SHA-256 signatures of all files, we are then utilizing TUF to validate the index, which in turn is utilized to validate the actual downloaded artifacts. This allows us to perform validation on index updates and not on final downloads, also reducing the overhead of performing multiple hashing and validation procedures on the larger crate artifact files.

#### Roles

##### Root Role

The root role of the tuf-crates repository shall consist of all members of the crates.io rust team with a threshold of 3. As a special case, updating this role shall also require a resigning by the root role of the tuf-root repository (sign a metadata entry existing within tuf-root). This means any changes to the membership of the crates.io team will also require a signing ceremony via github by the root quorum.

##### Targets Role

The repository shall have a top-level Targets role which is utilized directly for all actions on crates.io. This role shall be a 1-key member role, allowing for the automation of actions by crates.io for publishing of crates. In the future, we propose the delegation of roles for trusted publishing within organizations or crates.

###### Snapshots & Timestamps Roles

These roles shall each be a single-member role with a key utilized for automation.

## TUF Management

We propose the adaptation and implementation of TUF-on-CI (https://github.com/theupdateframework/tuf-on-ci) to manage roots and signing events via GitHub CI. This provides a GitHub-centric workflow for performing signing ceremonies via Pull Requests directly on the TUF repositories in question.

Online signing needs shall be implemented with AWS KMS.

All members of all signing quorums within the Rust Project will require hardware keys, the expenses for which will be covered by the Rust Foundation.



## Root Quorum Model

The root key shall follow a `5-of-9` authentication model for all operations. We consider this a reasonable middle ground of quorum to prevent malicious activity, while allowing for rapid response to an event requiring a quorum. These events are iterated below in [When the Quorum will be needed][when-the-quorum-will-be-needed].

This proposal delegates the authority of selecting and managing the quorum membership to the Rust Project's Leadership Council. We recommend that the quorum be selected from trusted individuals within the Project and Foundation. This is a position of trust, not one of authority over anything other than safe handling of key material and flagging of unusual activity; the 9 members are responsible for executing quorum operations, and providing transparency and trust (through their refusal to participate in any malicious operations), but not for deciding independently what key operations should happen.

These individuals should be available in the event quorum is needed for root key operations. These roles can and should be reappointed as needed, but to simplify logistics, these roles should not require rotation more often than 2-3 years. (Operations requiring quorum are expected to be rare, and an annual rotation would make rotation of the quorum group the vast majority of quorum operations.)

##### When the Quorum will be needed
[when-the-quorum-will-be-needed]: #when-the-quorum-will-be-needed

- Changes to quorum membership or thresholds shall require a quorum (enforced by the HSM authentication)
- Creation, signing, and issuance of new roles
- Creation, signing, and modification of roles in the event of expiration or compromise

##### Quorum Threat Mitigations

- To reduce the feasibility of coercion or collusion, we recommend that no more than 3 (M-2) of the 9 people be affiliated with any one company or institution, or be residents or citizens of the same country. (If in the future the quorum threshold is changed, this limit should be changed accordingly to remain M-2.)
- If in the future a desire arises to modify the quorum, we strongly recommend growing the size of membership rather than reducing the quorum threshold. The quorum exists to mitigate against bad actors, and is not implemented to prove a majority.
- To reduce the feasibility of coercion or collusion, we recommend that members of the quorum be selected from trusted individuals within the Rust Project, community, and Foundation staff/board. This allows us to spread trust and responsibility among socially and fiduciarily responsible parties.
- There is value in the members of the quorum generally residing in geographically and politically distributed locations most of the time. This RFC does not place any requirements regarding quorum member colocation for events or travel; the Rust Project and the quorum members can appropriately evaluate the value of such events and the corresponding risks.
- Hardware keys shall be distributed for management and storage of signing keys by active members of the quorum. The Rust Foundation shall provide the hardware tokens at zero cost to the quorum members.
- Hardware key loss or theft or similar will be handled by the quorum rotating out that key and rotating in a new key. Quorum members are responsible for reporting key loss or theft or similar to the infrastructure team in a timely fashion.

# Reference Level Explanation
[reference-level-explanation]: #reference-level-explanation

## Repository Workflows
[tuf-on-ci](https://github.com/theupdateframework/tuf-on-ci) shall be used for workflows on each repository. This is a tried and tested implementation currently in use by GitHub to manage their attestations at scale.

tuf-on-ci is a set of CI tools which are integrated with GitHub Actions for providing TUF quorum and hardware key support for managing TUF repositories via pull requests on GitHub. This is a ready made and production ready suite of CI that is used by the sigstore root signing for managing TUF quorums.

### (tuf-root) Rustup Release

- A signing event will be triggered via pull request in the `tuf-root` repository, which is assigned to the `Release Role` quorum, who must sign the request.

### (tuf-root) Stable/Beta Release

- A signing event will be triggered via pull request in the `tuf-root` repository, which is assigned to the `Release Role` quorum, who must sign the request.

### (tuf-root) Nightly Release

- An online signing delegate key will be utilized for signing updates to the repository. This key shall live in AWS KMS and allow for automated signing within the `tuf-root` repository.

### (tuf-crates) Crate Release/Yank

- An online signing delegate key will be utilized for signing updates to the repository. This key shall live in AWS KMS and allow for automated signing within the `tuf-crates` repository.

### (tuf-root + tuf-crates) Crates.io Membership Change

- The crates.io team will update the root role in the `tuf-crates` repository, triggering a signing event that the existing crates.io team must sign via Pull Request.
- An update to the tuf-crates-root.json file will occur in the `tuf-root` repository, which shall trigger a new signing event Pull Request, which the root quorum must perform.

## TAP-16 Implementation

We're proposing to use [TAP-16](https://github.com/theupdateframework/taps/blob/master/tap16.md) to provide efficient update checking and download sizes. TAP-16 uses Merkle trees rather than full lists for the download of a snapshot of the inventory of a repository (`snapshot.json`). We want to ensure that, as crates.io grows, the total size clients have to download when checking for updates remains small.

### Shared local folder changes

Creation of a new `~/.cargo/tuf` directory. (If Cargo stores its registry information in another directory, the `tuf` directory should be stored alongside the `registry` directory.) This directory shall be used for all TUF operations by project tools (both Rustup and Cargo). The cargo folder was chosen as the main location of residence for these files given that although Rustup will be performing the initialization of these folders, there is already a precedent set for shared files living within the cargo folder.

- `~/.cargo/tuf` The top-level directory of local copies of TUF repositories
- `~/.cargo/tuf/root` a copy of the `tuf-root` repository locally synchronized
- `~/.cargo/tuf/crates/<repository>` a copy of the `tuf-crates` repository locally synchronized on a per-repository basis. (for example, `~/.cargo/tuf/crates/crates.io/`)

### Controlling enablement of TUF

This RFC does not specify how to handle non-`crates.io` repositories. Cargo can choose to enable TUF for third-party repositories in the future, or may default to only using TUF for crates.io unless otherwise configured. Cargo might choose to use an environment variable (e.g. `CARGO_TUF_DISABLE`) to disable all usage of TUF on third-party repositories.

## Cargo changes
- Feature addition which shall be used for doing TUF synchronization and update procedures. This code shall use the `rust-tuf` crate (https://github.com/rustfoundation/rust-tuf).
- Utilize an implementation of `TAP-16` which will provide the following functions (these names being placeholders to show functionality):
- `crate_tuf_update(repository, crate, version)` - Shall download the updated signing information for a given crate to the appropriate tuf crates folder.
- `crate_tuf_verify(repository, crate, version)` - Shall verify the signing information for a given crate. This function must be performed separately from update on every use due the nature of signatures timing out and requiring an update (and a fresh call to `crate_tuf_update`)

## Rustup changes
- Feature addition which shall be used for doing TUF synchronization and update procedures. This code shall use the `rust-tuf` crate (https://github.com/rustfoundation/rust-tuf).
- Utilize the default `rust-tuf` and TUF behavior for synchronization and performing snapshot validation for releases.
- `release_sync()` - Shall download the updated signing information for the rust releases repository and verify the snapshot.
- `release_verify(file)` - Shall verify the signing information for a given release file.

## Crates.io changes

- Prior to updating the index, crates.io shall perform the online signing of the index entry to update the targets and sign the index entry, saving this as a like-pathed artifact in the TUF repository.

## Infra Changes

The infrastructure team.
- Creation of `rust-lang/tuf-root` and `rust-lang/tuf-crates` repositories on GitHub
- Initiation of the root signing ceremony via tuf-on-ci on each repository
- Facilitate the initial and subsequent signing events
- Determine how to mirror and distribute both repositories via CDN. We recommend that synchronization of the repository for the `rust-tuf-lib` should be done via HTTPS downloads from the CDN to prevent the end-user downloads being reliant on `git` or `GitHub`

# Drawbacks
[drawbacks]: #drawbacks

## Reliance on GitHub
This proposal has a reliance on GitHub and GitHub actions, due to the usage of `tuf-on-ci` and implementing a PR-based workflow for signing events. We consider this an acceptable decision given that the majority of the projects workflows exist here already; and under duress, the actions and process can be migrated to other providers who support similar CI workflows.

## Additional Synchronization & Storage Overhead
We will need to synchronization two more sources of data locally on systems with Rustup and Cargo - the TUF repositories. This has a storage and network cost for users, although it is considered minimal. We point to other repositories who have implemented this solution (PyPi, NPM) as examples that the overhead is acceptable at much larger scales.

## Legacy PKI Understanding
People's understanding of PKI often starts and ends with single root CA certificates, and the idea of a quorum model may seem excessively innovative rather than being a safe and well-established choice. We will need to carefully communicate that quorum models based on thresholds of geographically distributed individuals have a well-established history for a variety of purposes (and that TUF is an industry-leading implementation of such a model, used by PyPi and npm). We will need to provide clear documentation and announcement materials for a variety of audiences, including both developers and business leaders.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Threat Model

### Repositories / Mirrors

- **Threat**: Introduce rogue crate versions
  **Addressed**: The entire index for each crate is signed, so no new entry can be added to the index for a crate.
- **Threat**: Malicious Dependency Injection (modifying index entries without modifying crates)
  **Addressed**: The entire index for each crate index is signed, which includes the dependencies in the index used by Cargo for dependency resolution. Between this and the crate hash, whether Cargo uses the dependency information from the index or the crate itself, the dependency information is signed.
- **Threat**: Modify existing crate versions
  **Addressed**: The signed index for each crate includes the SHA-512 hashes of every crate file. Thus a modification of the file must be reflected in the index and signature.
- **Threat**: Rearrange validly signed data to present a crate that exists on crates.io as a different crate
  **Addressed**: The entire index for any given crate is signed, and the index includes the name of the crate, its versions, dependencies, and file hashes. Cargo will validate that the index entry matches the crate name it expected and the downloaded crate file for a version matches the file hash. So if an index for one crate is presented as the index for another, its signature will be valid but Cargo will not accept any entries from that index file.
- **Threat**: Malicious modification of index on storage
  **Addressed**: The entire index entry for any given crate is signed, so any modification of a crate file or its index entry requires a valid signature.
- **Threat**: Rollback attacks (prevent a user from getting updates or from noticing that a crate has been yanked/removed on crates.io by presenting an outdated mirror)
  **Addressed**: We have chosen to sign the entirety of a crates index entry, which includes all versions, for this signature scheme. This means the removal or addition of a version is not possible without a new signature. The Timestamp Role of TUF is required to resign the repository within a configured time frame, validly signed signatures cannot be used for more than a reasonable timeframe.
- **Threat**: Modification of crate/index while in transit
  **Addressed**: The entire index entry for any given crate is signed, so any modification of a crate file or its index entry requires a valid signature.
- **Threat**: Removal of a Crate from the index
  **Addressed**: The Snapshot Role of TUF addresses this; a snapshot will communicate the changes made to the repository to the user, so Cargo can notice that a referenced index does not exist.

### Actors & Secrets

- **Threat**: Compromise of a Crate Owner Account
  **Addressed**: This threat is ***not*** directly addressed in this RFC. However, once crates.io removes any malicious crates that have been uploaded, this RFC's verification ensures that users will stop seeing the malicious crates in a timely fashion, even on mirrors.
- **Threat**: Compromise of the `Crates Role`
  **Addressed**: Rotation of the key by the `tuf-crates` root and inclusion of the new key in tuf-crates, signed by the crates.io quorum.
- **Threat**: Misuse of the `Crates Role` to sign a malicious crate index file (whether by an entity with access to that key or through compromise)
  **Addressed**: The snapshot/timestamp roles handle revoking the malicious signature, and this is otherwise handled identically to compromise of the `Crates Role`.
- **Threat**: Compromise of the `tuf-crates Root Role`
  **Addressed**: This scenario requires the compromise of `N` crates.io team members. Handled via the revocation and rotation via the Rust root key quorum.
- **Threat**: Compromise or loss of 1-4 keys in the Rust root quorum
  **Addressed**: Handled via 5+ members of the root quorum signing a new root with those keys rotated out and new keys rotated in.
- **Threat**: Compromise or loss of 5+ keys in the Rust root quorum
  **Addressed**: Manual replacement and transition of Rust root quorum, featuring loud announcement from Rust project, signed by remaining quorum members, corroborated by news of whatever disaster led to this event.
- **Threat**: Compromise or MITM of the `crates.io` TLS certificate used for HTTPS
  **Addressed**: All signatures are independent of TLS, so the compromised/MITMed connection cannot tamper with the indexes (see threats in previous section). The `Crates Role` provided by crates.io chains up to the Rust root role, so the compromised/MITMed connection cannot present a different `Crates Role`.
- **Threat**: Presentation of an old/revoked/expired `tuf-crates` signature
  **Addressed**: TUF implements quorum history in a way that cargo will identify that it is not the latest quorum. The `Timestamp Role` will also mitigate the usage of an old signature by limiting the attack period to N-hours, as configured by crates.io.
 
## Artifact Signing (`tuf-crates`)

Instead of signing the index entries and transitively inferring security via those signatures, we could alternatively or additionally have the TUF repository include direct signatures of the artifacts. However, omitting signing of the index entries would allow various threats (see "Threat Model"), and signing the individual crate files does not provide added security above and beyond signing index files that include cryptographic hashes of the individual crate files.

# Prior art
[prior-art]: #prior-art

Previous RFC, not based on TUF; closed in favor of this one:

- [RFC #3579: Public Key Infrastructure for Rust Project](https://github.com/rust-lang/rfcs/pull/3579)

Relevant past RFCs:

- [RFC #3403: sigstore and cargo/crates.io](https://github.com/rust-lang/rfcs/pull/3403)
- [TUF for crates.io discussion](https://github.com/withoutboats/rfcs/pull/7)
- [Trusted Publishing support for crates.io](https://github.com/rust-lang/rfcs/pull/3691)

Other posts on TUF usage:
- [Python PEP #458: Secure PyPI downloads with signed repository metadata](https://peps.python.org/pep-0458/) (This was designed but not implemented)
- [Square blog on securing RubyGems with TUF](https://developer.squareup.com/blog/securing-rubygems-with-tuf-part-1/)
  - [part 2](https://developer.squareup.com/blog/securing-rubygems-with-tuf-part-2/)
  - [part 3](https://developer.squareup.com/blog/securing-rubygems-with-tuf-part-3/)
- [Securing Haskell's Hackage repository with TUF](https://www.well-typed.com/blog/2015/07/hackage-security-alpha/)
- The Node Package Manager (NPM) is exploring usage of TUF as well.

The Debian archive format provides prior art for various aspects of signatures and mirror verification. For instance, Debian addresses the threat model of outdated mirrors leading people to retain versions with security vulnerabilities, by having a `Valid-Until` field in their Release files. Sample of a Debian signed release file: <https://deb.debian.org/debian/dists/sid/InRelease>. Debian also provides prior art for downloading items by hash.

Sigstore TUF Repository - https://github.com/sigstore/root-signing

# Unresolved Questions
[future-possibilities]: #unresolved-questions

## TAP-16

We're proposing to use [TAP-16](https://github.com/theupdateframework/taps/blob/master/tap16.md) to provide efficient update checking and download sizes. TAP-16 uses Merkle trees rather than full lists for the download of a snapshot of the inventory of a repository (`snapshot.json`). Before shipping this, we need to ensure that checking for updates and downloading updates are both efficient operations that do not require downloading a large and growing amount of data. We should test this for the case of a crate with a single dependency and a crate with a thousand dependencies, and project these costs into the future when we have tens of millions of crates and billions of versions.

# Future possibilities
[future-possibilities]: #future-possibilities

## Automatic Integration with teams repo and sync-team

Utilizing `tuf-on-ci` and further GitHub actions on merges in the teams repository, it could be possible to automate the process of initiating signing events on team changes. Additionally, the teams repo could be later utilized as the source of keys which are present in TUF - allowing for the teams repo to be the single source of truth. This RFC remains independent on the `team` and `team-sync` projects, and leave this a future possibility for integration and automation.

## More efficient mirroring procedures

This RFC inherently provides all the guarantees required for having cryptographically verifiable arbitrary mirrors for Rustup, Nightlies, Releases and Crates. We will need to establish processes for creating such mirrors, and keeping them up to date in a timely fashion with minimal load on Rust infrastructure.

## Mechanism to specify a mirror in Cargo

This RFC does not specify a mechanism to specify an alternate mirror in Cargo. We expect that Cargo will want to add a mechanism for specifying a mirror of crates.io, and then Cargo will apply the same crates.io verification for that mirror.

TUF also provides a Mirrors role which is not addressed in this RFC. In the future, the Mirror role may become a quorum of the Infra team - allowing them to manage and verify official mirror locations and other sources of the index and releases. This is not required for cryptographically verified mirrors, but it would be required for maintaining a verifiable official list of such mirrors.

## Automatic mirror discovery

The unused TUF mirror role, not addressed in this RFC, provides a mechanism for the various teams to manage a list of official mirrors which could be used for automatic mirror discovery and selection by cargo and rustup in the future.

We may want to consider other mechanisms for automatic mirror discovery that support local provision of local mirrors, such as DNS-based discovery. This would allow organizations or CI services to provide local mirrors that will automatically be used to save bandwidth.

## Crate, Organization or Namespace-level Trusted Authors

TUF delegation allows us to specify new roles for paths within the tree of the `tuf-root` index. In the future, we may create further nested quorums and roles and allow them to be crated and configurable via crates.io. This could, in theory, allow for managing quorums, namespaces, groups and distinct authors across crates.io and automatically provide us with cryptographic guarantees for those different levels of organization.

## Support signed private registries

In the future, we can use this same infrastructure to support signed registries other than crates.io. This initial proposal covers crates.io and ties it to the Rust Project TUF infrastructure. Future designs for non-crates.io registries will need to ensure that the configurability provided for handling such registries does not compromise the chain of trust to crates.io.