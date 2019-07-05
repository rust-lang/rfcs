- Feature Name: N/A
- Start Date: 2019-07-05
- RFC PR: [rust-lang/rfcs#2719](https://github.com/rust-lang/rfcs/pull/2719)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

The ability to authenticate for all requests on private registries is added.
This includes API requests, crate downloads, and index retrievals.
The global authentication is enabled in Cargo's configuration in a registry definition.

# Motivation
[motivation]: #motivation

Businesses with projects in Rust will need somewhere to keep their crates, and the current
implementation of custom registries only sends authentication at times when it is needed on
crates.io. There is no option to send the token when downloading crates or making other
API calls.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

If a custom registry is to be considered private, the registry can require authentication on all
requests. This can be enabled on the client side by modifying the registry definition:
```toml
[registries.my-registry]
index = "https://my-intranet:8080/git/index"
auth_required = true
```
This will cause Cargo to send the token obtained via `cargo login` to the registry with all requests.


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The option `registries.<registry>.auth_required` is a boolean that enables the sending of the token in the `Authorization` header on all requests, including downloads and API calls. This will be implemented with a check on any requests to the registry that sends the header if this value is true.

<!-- This is the technical portion of the RFC. Explain the design in sufficient detail that:

- Its interaction with other features is clear.
- It is reasonably clear how the feature would be implemented.
- Corner cases are dissected by example.

The section should return to the examples given in the previous section, and explain more fully how the detailed proposal makes those examples work. -->

# Drawbacks
[drawbacks]: #drawbacks

This is probably biased, but I know of no drawbacks.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This design may not be the best design, so feedback is appreciated.

Another considered design was to use a cargo plugin to proxy requests through a local server that sends authentication with the server with requests. This design was decided against due to concerns about battery life, and the requirement to ensure the proxy is running whenever it is required, meaning possibly requiring a system service to keep it running.

The impact of not doing this would be that large organizations would be hesitant to use Rust for internal/confidential projects, so there would not be as many large organizations supporting the growth of Rust.

<!-- - Why is this design the best in the space of possible designs?
- What other designs have been considered and what is the rationale for not choosing them?
- What is the impact of not doing this? -->

# Prior art
[prior-art]: #prior-art

Many other languages have package managers with support for private repositories. Not all will be listed here, but a select few as examples.

- Python (Pio)
- .NET (NuGet)
- Java (Maven)
- JavaScript (npm)
- PHP (Composer)

<!-- Discuss prior art, both the good and the bad, in relation to this proposal.
A few examples of what this can include are:

- For language, library, cargo, tools, and compiler proposals: Does this feature exist in other programming languages and what experience have their community had?
- For community proposals: Is this done by some other community and what were their experiences with it?
- For other teams: What lessons can we learn from what other communities have done here?
- Papers: Are there any published papers or great posts that discuss this? If you have some relevant papers to refer to, this can serve as a more detailed theoretical background.

This section is intended to encourage you as an author to think about the lessons from other languages, provide readers of your RFC with a fuller picture.
If there is no prior art, that is fine - your ideas are interesting to us whether they are brand new or if it is an adaptation from other languages.

Note that while precedent set by other languages is some motivation, it does not on its own motivate an RFC.
Please also take into consideration that rust sometimes intentionally diverges from common language features. -->

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Currently the `Authorization` header does not use an auth scheme. This is against standards. [rust-lang/cargo#4989](https://github.com/rust-lang/cargo/issues/4989) has been opened because of this. It is undecided whether to include a fix for this in this RFC or another. This would be a breaking change if not implemented right.

# Future possibilities
[future-possibilities]: #future-possibilities

Possibly allowing non-Git indices, but it appears to be a large amount of work, so it is not vital to this RFC.
