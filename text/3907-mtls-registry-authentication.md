- Feature Name: `mtls-registry-authentication`
- Start Date: 2026-01-16
- RFC PR: [rust-lang/rfcs#3907](https://github.com/rust-lang/rfcs/pull/3907)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This is an RFC aimed at allowing Cargo to present client certificates when forming HTTP connections and support mutual TLS authentication with registries.

# Motivation
[motivation]: #motivation

Some organizations require client identity verification when interacting with privately hosted services. This can be achieved a number of ways, but is commonly done with certificates in a process called "mutual TLS" (mTLS).

Cargo does not currently support forwarding client certificate information when it configures its `libcurl` HTTP handle. This poses an issue for organizations that host private crate registries and perform client authentication via certificates, since there is no alternative way to forward these client provided certificates.

Authentication at the TLS level is different from the token-based methods for [Registry Authentication](https://doc.rust-lang.org/cargo/reference/registry-authentication.html) exposed by the [Credential Provider Protocol](https://doc.rust-lang.org/cargo/reference/credential-provider-protocol.html) since it takes place before the connection to the registry is established. It is not currently possible to write a [credential plugin](https://doc.rust-lang.org/cargo/reference/registry-authentication.html#credential-plugins) that enables this type of authentication with a registry, but an extension to that protocol would make this possible.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Credential providers will be able to provide client certificates and private keys to Cargo via new request and response messages.

Cargo will issue a tls-identity request when configuring an HTTP client for a registry, and the returned identity will be used for subsequent communication to the same registry (within the current Cargo session).

## TLS client identity request

* Sent by: Cargo
* Purpose: Get client certificates and private keys for HTTP communication

```json
{
    // Protocol version
    "v":2,
    // Request kind: set TLS client identity
    "kind":"tls-identity",
    // Registry information (see https://doc.rust-lang.org/cargo/reference/credential-provider-protocol.html#registry-information)
    "registry":{"index-url":"sparse+https://registry-url/index/"},
    // Additional command-line args (optional)
    "args":[]
}
```

## TLS client identity response

* Sent by: credential provider
* Purpose: Set client certificates and private keys for HTTP communication

```json
{"Ok":{
    // Response kind: this was a TLS client identity request
    "kind":"tls-identity",
    // Client certificate chain in PEM format, with escaped newlines (empty if unset)
    "certificate":"-----BEGIN CERTIFICATE-----\n[Base64 encoded client certificate data]\n-----END CERTIFICATE-----",
    // Private keys in PEM format, with escaped newlines (empty if unset)
    "key":"-----BEGIN PRIVATE KEY-----\n[Base64 encoded private key data]\n-----END PRIVATE KEY-----"
}}
```

## Certificate and key formats

The `certificate` and `key` fields are expected to correspond to the same TLS client identity. If a credential provider is unable to supply a usable client identity, it may return empty fields.

The `certificate` field contains the client certificate chain in PEM format, with newlines escaped using `\n`. If multiple certificates are present, they are expected to be concatenated PEM blocks.

The `key` field contains the private key corresponding to the client certificate, in PEM format, with newlines escaped using `\n`.

Encrypted private keys are not supported. Credential providers are responsible for decrypting user-provided material before returning it to Cargo.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The currently used crate for `libcurl` exposes methods for setting these certificates and keys, and can be used to set these configuration options when HTTP handles are being configured. These methods are:
* `curl::easy::Easy::ssl_cert_blob`
* `curl::easy::Easy::ssl_key_blob`

These "easy" methods wrap well tested code in the curl source:
* https://github.com/curl/curl/blob/master/docs/libcurl/opts/CURLOPT_SSLKEY.md
* https://github.com/curl/curl/blob/master/docs/libcurl/opts/CURLOPT_SSLCERT.md

# Security Considerations

Cargo MUST treat all certificate and private key data returned by a credential provider as sensitive material.

Cargo MUST NOT persist tls-identity response data to disk.

All certificate and key material should be held in memory only for the lifetime required to configure the HTTP client.

Cargo MUST NOT log, print, or otherwise expose the contents of these blobs, including in debug or trace output.

Credential providers are responsible for securely sourcing and protecting private key material.

# Drawbacks
[drawbacks]: #drawbacks

This adds additional complexity to Cargo's HTTP configuration and could have impacts on where in the code HTTP handles are configured, and which handles are used for communication with different registries.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Allow custom credential plugins to handle certificates

- Credential management is hard, and Cargo does not need to be directly involved in managing these certificates.

- There are near endlessly niche ways that a user may want to provide their client certificates, and this protocol extension will allow users to write custom plugins to handle their situation.

- This RFC does not introduce new trust boundaries beyond those already present for credential providers, which are treated as fully trusted by the user.

## Avoid backend lock-in

- Today Cargo is using `libcurl` for its backend HTTP client. There might be a future where a `rustls` based backend would be preferred. Nearly all TLS libraries support client certificates in some form, and this protocol extension gives Cargo ability to convert from the widely used PEM format to whatever may be needed in the future.

# Prior art
[prior-art]: #prior-art

Mutual TLS authentication is widely supported across TLS libraries, developer tools, and artifact distribution systems.

libcurl has supported client certificates via CURLOPT_SSLCERT and CURLOPT_SSLKEY since version 7.1 (released August 2000), and these options are commonly used by applications that require authenticated HTTPS connections. Other widely used TLS implementations, including OpenSSL, BoringSSL, NSS, and rustls, also provide first-class support for configuring client certificates and private keys for TLS connections.

Many developer tools and package managers support mutual TLS when interacting with private registries or artifact repositories. For example, Python package management tools such as Poetry and uv allow users to configure client certificates for authenticated registry access. Other ecosystems similarly support client certificate authentication, including pip, npm, Maven, and Gradle, where mutual TLS is commonly used in enterprise environments.

Private artifact repository systems and registry infrastructure, such as JFrog Artifactory, Sonatype Nexus, GitHub Enterprise, and GitLab, frequently support or encourage mutual TLS as an authentication mechanism for internal services. These systems are often deployed in environments with existing public key infrastructure, where TLS-level client authentication integrates naturally with organizational security policies.

Within Cargo itself, this RFC builds on existing design patterns established by the credential provider protocol. Cargo already delegates authentication concerns to external credential providers and avoids managing long-lived secrets directly. Extending this protocol to allow credential providers to supply TLS client identity material follows the same approach and enables mutual TLS support without introducing new secret management responsibilities into Cargo.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

This RFC is intentionally limited to providing client certificate and private key material to Cargo. It does not address configuring additional certificate authority (CA) roots, interacting with platform trust stores, or performing certificate signing request (CSR)–based authentication flows (which would be needed to support hardware security modules). Future extensions to the credential provider protocol might allow credential providers to supply additional trust anchors or to participate in dynamic certificate issuance mechanisms, but these design decisions would likely be influenced by particular TLS backend choices.
