- Feature Name: cargo_asymmetric_tokens
- Start Date: 2022-02-02
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Add support for Cargo to authenticate the user to registries without sending secrets over the network. 

# Motivation
[motivation]: #motivation

The word "token" is going to be used a lot in this document. For clarity the tokens created for the way things work before this RFC will be referred to as "secret tokens" and tokens created for the scheme described in this RFC are referred to as "asymmetric tokens". A "hardware token" on the other hand, refers to a physical device that stores key pairs and provides an API to interact with them without providing any way to get at the raw private key.

When Cargo authenticates to a registry it passes along a token.
This secret token is both shared over the network and sufficient to do authentication.
Persistent shared secrets are rife with opportunities for things to go wrong.
For some examples:
- The user can unintentionally share the file containing the token. This was unfortunately common when it was stored in `.cargo/config`, which is why it is now stored in `credentials.toml` by default.
- The file containing the token can be read at rest. File permissions are used to protect it, but can only go so far. [Credential processes](https://github.com/rust-lang/rfcs/blob/161ce8a26e70226a88e0d4d43c7914a714050330/text/2730-cargo-token-from-process.md) can do better *if* they are used.
- If the token is ever logged and the logs are public, then the token is public. This is fairly easy to do accidentally in CI contexts. Cargo now redacts the token in its own logging, but if network traffic is logged there is still an issue.
- If a user configures a custom registry to use `http` instead of `https`, then anyone on the network can see the token go by.
- If a user misconfigures a token to go to the wrong registry (typosquatting, homoglyph, or copy-paste error), then the recipient has the token.
- If a registry does not adequately protect its copy of the tokens then a database disclosure can leak all the users' tokens. ([cc: crates.io security advisory](https://blog.rust-lang.org/2020/07/14/crates-io-security-advisory.html))
- If you have a creative problem that's not on this list, then this is probably not the right venue to discuss it. ([Security Reporting policy](https://www.rust-lang.org/policies/security))

Fundamentally these are all problems only because once an attacker has seen a secret token they have all that is needed to act on that user's behalf. The secret token is sufficient for the attacker to call publish or yank. Even if the request that the attacker saw was a simple read (assuming that ["Cargo alternative registry auth #3139"](https://github.com/rust-lang/rfcs/blob/f3aecb96eeb95542d81d6dc6b0a22c1245383604/text/0000-cargo-alternative-registry-auth.md) is accepted) once the attacker has the token it is all over.

When using asymmetric cryptography the important secret (the private key) never leaves the user's computer.
With a credential provider, the secret material can even stay on a hardware token.
Furthermore, an asymmetric token can only be used for the intended action, and only for a short time window. The opportunity for replay is smaller, and can be tightened by the registry to meet its threat model. (See the [Appendix: Threat Model](#threat-model) for a detailed comparison of how asymmetric tokens helps with each problem.)
After the asymmetric token has expired, the data sent over the network can be made public, without risking the private material. A registry can keep or publish an audit log of asymmetric tokens without risk of them being reused, in case a security auditor would like to look for abnormal or unusual behavior.

Different registries will have different users in mind and have different use cases. Therefore, they will need to have different behaviors. So, there are many decisions a registry has to make that this RFC has no opinion on. Some examples:

- Bootstrapping trust: how does the registry decide to trust a new user?
- Key generation: where and how is the key pair made?
- Key rotation: how often do existing users need to make a new key pair?
- Revocation: how does the registry decide to stop trusting an existing key pair?

In order for crates.io to support asymmetric tokens these questions will need to be answered for crates.io. If and how crates.io will implement compatibility with these new tokens will be left for a follow-up discussion/RFC.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Private registries that require authentication use asymmetric cryptography as a more secure way for cargo to log in. Each registry works a little different, but the most common workflow is:
1. Generate a key pair. (For many registries, you can generate the key pair using the cargo command `cargo login --registry=name --generate-keypair`, which will print the public key for use in the next step.)
2. Log into the registries website
3. Go to the "register a key pair" page, upload your public key. and get the user ID for that key pair.

Most do not, but some registries require one more step:

4. if the registry gave you a `key-subject` then on the command line run `cargo login --registry=name --key-subject="the provided data"`

There are credential processes for using key pairs stored on hardware tokens. Check crates.io to see if there's one available for your hardware. Each one is a little different, but the general workflow is:
1. `cargo install credential-process-for-your-hardware-token`
2. Run `cargo credential-process-for-your-hardware-token setup registryURL` to get your public key.
3. Edit `credentials.toml` to have a `credential-process` field as described by `credential-process-for-your-hardware-token` docs. (The credential process command may help do this for you.)
4. Log into the registries website
5. Go to the "register a key pair" page, upload your public key.

Some registries prioritize user experience over strictest security. They can simplify the process by providing key generation in the browser. If your registry works this way the workflow will be:
1. Log into the registries website
2. Go to the "generate a key pair" page, and copy the command it generated for you. It will disappear when you leave the page, the server will not have a copy of the private key!
3. Run it on the command line. It will look like  `cargo login --registry=name --private-key="key"`

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Setting and storing login information

In [`config.toml`](https://doc.rust-lang.org/cargo/reference/config.html) and `credentials.toml` files there is a field called `private-key`, witch is a private key formatted in the secret [subset of `PASERK`](https://github.com/paseto-standard/paserk/blob/master/types/secret.md) and is used to sign asymmetric tokens

A keypair can be generated with `cargo login --generate-keypair` which will:
- generate a public/private keypair in the currently recommended fashion.
- save the private key in `credentials.toml`.
- print the public key in [PASERK public](https://github.com/paseto-standard/paserk/blob/master/types/public.md) format.

It is recommended that the `private-key` be saved in `credentials.toml`. It is also supported in `config.toml`, primarily so that it can be set using the associated environment variable. Witch is the recommended way to provide it in CI contexts. This set up is what we have for the `token` field for setting a secret token.

There is also an optional field called `private-key-subject` which is a string chosen by the registry.
This string will be included as part of an asymmetric token and should not be secret.
It is intended for the rare use cases like "cryptographic proof that the central CA server authorized this action". Cargo requires it to be non-whitespace printable ASCII. Registries that need non-ASCII data should base64 encode it.

Both fields can be set with `cargo login --registry=name --private-key="key" --private-key-subject="subject"`.

A registry can have at most one of `private-key`, `token`, or `credential-process` set.

## The authentication process

When authenticating to a registry, Cargo will generate a PASETO in the [v3.public format](https://github.com/paseto-standard/paseto-spec/blob/master/docs/01-Protocol-Versions/Version3.md). This format uses P-384 and 384-bit ECDSA secret keys, and is compatible with keys stored in contemporary hardware tokens. The generated PASETO will have specific "claims" (key-value pairs in the PASETO's JSON payload).

The claims within the PASETO will include at least:
- The current time. (ISO 8601 compliant DateTime string in the `iat` key.)
- The challenge, if cargo has received a challenge from a 401/403 from this server this session. A server that issues challenges should have some stateful way of knowing which challenges have been used and which ones are still available. (The string exactly as received in the `challenge` key.)
- The `private-key-subject` if it was set. (The string exactly as set in the `sub` key.)
- If this is a mutation: which one (publish or yank or unyank), the package, the version, the SHA256 checksum of the `.crate` file as stored in the `cksum` in the index. (`mutation`, `name`, `vers`, `cksum` keys respectively.)

The "footer" (which is part of the signature) will be a JSON string in UTF-8 and include the registry base URL (in the `aud` key) and the `key ID` (in the `kid` key).
The `key ID` can be obtained from the public key using the [PASERK IDs](https://github.com/paseto-standard/paserk/blob/master/operations/ID.md) standard.

PASETO includes the message that was signed, so the server does not have to reconstruct the exact string from the request in order to check the signature. The server does need to check that the signature is valid for the string in the PASETO and that the contents of that string matches the request.

The registry server will validate the PASETO, and check the footer and claims:

- The PASETO is in v3.public format.
- The PASETO validates using the public key it looked up based on the `key ID`.
- The URL matches the registry base URL (to make sure a PASETO sent to one registry can't be used to authenticate to another, and to prevent typosquatting/homoglyph attacks)
- The PASETO is still within its valid time period (to limit replay attacks). We recommend a 15 minute limit, but a shorter time can be used by a registry to further decrease replayability. Or a longer one can be used to better accommodate clock skew.
- If the server issues challenges, that the challenge has not yet been answered.
- If the operation is a mutation, that the package, version, and hash match the request.

## Credential Processes

Credential Processes as defined in [RFC 2730](https://github.com/rust-lang/rfcs/pull/2730) are outside programs cargo can call on to change where and how secrets are stored. That RFC defines `special strings` which go in the `credential-process` field to describe what data the process needs from cargo. This RFC adds `{claims}`. If used Cargo will replace it with a JSON encoded set of key value pairs that should be in the generated token. Cargo will check that the output of such a process looks like a valid PASETO v3.public token that Cargo would have generated, and that the PASETO token includes all the claims Cargo provided. The credential process may add additional claims (e.g. 2fa, TOTP), as long as they are nested in `custom`.

Some credential processes that might be useful for people to develop include:
- The ability to store keys in operating systems specific secure enclaves.
- the ability to use keys embedded in common hardware tokens.
- The ability to read keys in formats used by other tools (GPG, SSH, PKCS#12, ect.)

## Note on stability

This is just a reminder to check if there are newer RFCs that have had to deprecate, remove, or replace parts of this one. RFCs can always be adjusted by new RFCs. In general the Rust community takes backwards compatibility very seriously, so if an RFC says you can do something no future RFC is likely to say that you cannot do that thing. It has happened, RFCs have been amended or changed by subsequent RFCs. The content of this RFC is full of details with security implications. It is not unlikely that in the course of human events changes will need to be made to it. Hopefully, they can be made by loosening restrictions or supporting new formats. But, because security is involved the Rust community may be more likely to break backward compatibility than is our norm.

# Drawbacks
[drawbacks]: #drawbacks

This gets Cargo involved in the cryptographic standards used by registries, which puts a lot of complexity on ourselves. Now rust teams need to be involved in conversations about what cryptographic standards alternative registries choose to use.

Furthermore, this RFC attempts to make a start on solving several problems at the same time. It may be that in time we discover these problems need to be solved separately. If we end up with a separate system for code signing and a separate system for authorization, then a simpler more direct method of authentication might have been a better choice.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Continue with the existing secret tokens. Private registries that want to provide this kind of functionality can create a bespoke system for their exact needs. For example, only generating short-lived tokens and having the user log in daily.
In practice, I suspect many registries will not, leading to an ecosystem where most registries use less secure authentication, and creating more hazards for users. Some security properties (e.g. not supporting tokens from one registry on another) work better when all registries support them.

We could use PASETO `v4.public` instead of `v3.public`. The `v4` standard uses more modern crypto, based on Ed25519.
Unfortunately, most existing hardware tokens do not support Ed25519. By using `v3.public` based on P-384 we allow a `credential-process` to keep keys on the hardware.

We could use [Amazon's SigV4](https://docs.aws.amazon.com/general/latest/gr/signature-version-4.html). In SigV4 the client constructs a string from the request (url, headers, and body). The client signs the string. It sends the signature and only the signature as the authentication with the request. Importantly the client does not send the constructed string. The server looks at the request it receives and construct a new copy of the string. It then checks that the signature it got is valid for the string it constructed. This scheme means that the authentication field stays the same size no matter how much is being signed. Also, any large data sent in the request is not duplicated in the authentication header. Most importantly there is no way for a server to have a bug where it forgets to check that some fields in the token do not match the request it came with! 
Unfortunately this scheme is more complicated than it seems. There is a lot of complexity hidden in "constructs a string". SigV4 does not get us out of  having to specify exactly which fields are important for each request. Furthermore, HTTP headers and urls can be canonicalised differently by different hops on the network. when calling Amazon's services Amazon provides client libraries that do all the heavy lifting of making sure the fields are canonicalised the same on the client and the server if and only if the requests are for the same resource. A lot of this complexity's has been standardized and generalized in the [HTTP Message Signatures](https://www.ietf.org/archive/id/draft-ietf-httpbis-message-signatures-08.html) draft specification. Unfortunately, implementations of the specification are not yet widely available. 

Mutating operations include signed proof that the asymmetric token was intended for that package, version, and hash. Why not do the same for read operations? When reading from an HTTP based index, we may need to request many files in quick succession without being able to enumerate them in advance. When using a credential process to communicate with a hardware token that requires human interaction for each signing operation we do not want to require hundreds of interactions.

Use [Biscuit](https://www.biscuitsec.org/) instead of PASETO. Biscuit is a format that adds delegation and a logic-based policy engine for attenuation and fine-grained usage controls to the other properties tokens have. The Biscuit logic language provides a centralized place to do authorization. As part of the token format, for example, a token can be made that can only publish one crate on a particular day (good for a CI/CD use case), or a token that can only yank particular crates (good for giving to a security scanner). Once biscuit is adopted as your token format [the crates.io token scopes RFC](https://github.com/rust-lang/rfcs/pull/2947) becomes easy to implement. Authorization with tokens that have limited scope are definitely something more widely used registries should definitely support.
If we use Biscuit all the controls anyone could ask for are just part of the system.

However:
- Introducing it here for authentication means that all registries need to use the biscuit language for their authorization. For some small registries this will be a lot more controls than they need. For large registries they will need to build compatibility between whatever existing authorization system they have and their biscuit implementation.
- The biscuit language has some pretty complicated primitives, including regular expressions. Registries that require thorough correctness audits for all code related to Auth may find this prohibitively expensive.
- The current biscuit specification (2.0) does not have a rich model of authentication. If you have a token that was authorized to do the action you are attempting to do then you must be someone who is allowed to do that action. Which has a lot of the same limitations of the existing secret token system as outlined in the motivation section of this RFC.
- It is still possible to do scopes for tokens without using biscuits. A user ID can be created for each authorized role, and then the server can make sure that the used user ID is authorized to do the intended action.

# Prior art
[prior-art]: #prior-art

NuGet has support for [author signing](https://github.com/NuGet/Home/wiki/Author-Package-Signing), which can be used to make sure that publishes only happen from somebody who has a private key. This system allows authenticity to be checked looking only at the crate that is downloaded. 
However, in order to participate the author must have a "code signing certificate" from a "trusted root authority", making the barrier to participation to high for most users and certainly too high to be considered a norm of the community.

Maven Central [requires](https://maven.apache.org/repository/guide-central-repository-upload.html#pgp-signature) all uploads to be [signed by PGP](https://central.sonatype.org/publish/requirements/gpg/) and that the keys are registered with a public key server. Following the UNIX philosophy, they leave the actual act of signing up to independent implementations of PGP.
It takes a lot of documentation to explain how to hook up all of these different parts to work together correctly. Furthermore, no assurance is made that the GPG signature and the Maven Central token used for upload represent the same identity.

The npm client can pass along a `otp` option on the command line to act as [proof of 2FA](https://docs.npmjs.com/configuring-two-factor-authentication#sending-a-one-time-password-from-the-command-line). This provides a lot of the "over the wire" benefits of this RFC for the npm registry, but cannot be used by a third party after the fact to verify the uploaded identity.

[TUF](https://theupdateframework.io/) exclusively deals with how a client downloading packages through a mirror can be assured they came from a non-compromised copy of the registry. Which is not the problem this RFC is addressing.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

How aggressively to push people off secret tokens? This RFC does not remove the existing use of secret tokens for publishing and yanking on private registries nor suggests a timeline for crates.io to use asymmetric tokens. There is an [RFC to allow authentication on more operations](https://github.com/rust-lang/rfcs/blob/f3aecb96eeb95542d81d6dc6b0a22c1245383604/text/0000-cargo-alternative-registry-auth.md), the expectation is that we will require the use of asymmetric tokens for this new functionality. This is a question that we will have to decide as we go through implementation and stabilization.

What default settings should `cargo login --generate-keypair` use? What process should be used for changing these defaults as best practice changes? Where should it put the private keys?

More generally, is all the user experience exactly correct for all the new CLI flags? The expectation is that these will need to be changed and tweaked as we try using them after implementation.

# Future possibilities
[future-possibilities]: #future-possibilities

Figuring out how and when crates.io should support these kinds of tokens is left to a follow-up discussion/RFC. The motivation section describes some of the things that will need to be figured out.

Only after crates.io is not using secret tokens should we consider removing the support for them in private registries (and the code in cargo to support it).

After that an audit log of what tokens were used to publish on crates.io and why that token was trusted, would probably be a rich data source for identifying compromised accounts. As well as making it possible to do end to end signature verification. The crate file I downloaded matches the `cksum` in the index; the index matches the `cksum` in the audit log; the public key used in the audit log is the one I expected.

This scheme could be augmented to allow the use of several signing technologies. We would need to add a way for a registry to express what formats it will accept. We would need to add code for cargo to check that the credential provider was following one of the accepted formats. We would need to add code for cargo to generate the additional formats. But none of this is out of the question, so there is a clear path forward when algorithm agility is required.

# Appendix

## Threat Model

If a registry were set up to exclusively use the new asymmetric tokens, how well would it handle the issues in the motivation?

> The user can unintentionally share the file containing the token. This was unfortunately common when it was stored in `.cargo/config`, which is why it is now stored in `credentials.toml` by default.

`credentials.toml` name identifies that it should not be shared. Unfortunately, this RFC does not make things better.

> The file containing the token can be read at rest. File permissions are used to protect it, but can only go so far. [Credential processes](https://github.com/rust-lang/rfcs/blob/161ce8a26e70226a88e0d4d43c7914a714050330/text/2730-cargo-token-from-process.md) can do better *if* they are used.

Many more kinds of security hardware devices can protect a private key then can protect an arbitrary secret token. Hardware devices can store a private key and only perform operations using that key, without making the key itself available.

> If the token is ever logged and the logs are public, then the token is public. This is fairly easy to do accidentally in CI contexts. Cargo now redacts the token in its own logging, but if network traffic is logged there is still an issue.

It is still possible for someone to log the private key. However, the signed asymmetric token is not secret. So all other things (like network traffic) can be logged safely.

> If a user configures a custom registry to use `http` instead of `https`, then anyone on the network can see the token go by.

Content shared over the network is not secret. The opportunity for replay attacks is significantly limited. If the operation is mutating then the token can only be used for the intended operation. If it is a read operation, if the request returns meaningful results then the attacker can already see it without reusing the token. But, as the token includes the URL it can not be used on the `https` address.

> If a user misconfigures a token to go to the wrong registry (typosquatting, homoglyph, or copy-paste error), then the recipient has the token.

The asymmetric token includes the URL so the signature is only valid for that URL, the token is not valid for the real registry.

> If a registry does not adequately protect its copy of the tokens then a database disclosure can leak all the users' tokens. ([cc: crates.io security advisory](https://blog.rust-lang.org/2020/07/14/crates-io-security-advisory.html))

There is no reason for the registry to even see the private key. Even if the registry wants to generate keys for its users there is no need to store private keys. Disclosure of public keys is not a security risk, as they can not be used to sign new asymmetric tokens.

To be fair, there's no reason for a registry based on secret tokens to store them in a recoverable format. The registry can store secret token hashes instead, and avoid this problem without inconveniencing the user. Since secret tokens are already random, you can avoid a lot of the complexities of storing passwords.

Storing plain text secret tokens is only a problem in practice not in theory. However, the link is to an example of crates.io getting this wrong. I can only assume if we have seen one registry get this wrong, then there are others and there will be more in the future.

> Fundamentally these are all problems only because once an attacker has seen a secret token they have all that is needed to act on that user's behalf.

Without the private key an asymmetric token can only be used for the intended registry, for the intended action, and for a limited amount of time. This mitigates the risk of disclosure.