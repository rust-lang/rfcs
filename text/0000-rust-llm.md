# RFC: Add large language models to Rust

This adds a new module to the standard library containing interfaces to large language models.

The author highlights that no large language models were used in the making of this RFC exactly because Rust does not support them today.

## Motivation

Large language models (LLMs for short) have become ubiquitous.
But using AI language models in Rust has been a lot harder than one would expect.
The standard library provides neither an interface to nor an implementation of a large language model.
This will significantly hurt adoption in the future as Rust continues to fall behind other ecosystems, like NodeJS built-in `node:llm` or C23's `llm.h`.
To ensure continued adoption of Rust in enterprise contexts, this RFC proposes enhancing Rust by adding first-class large language model capabilities to its standard library.

## Guide-level explanation

This RFC contains the following changes

- add `std::llm`, containing several interfaces and potentially an implementation of a large language model
- add a `cargo llm` subcommand for interfacing with `std::llm` from the command line.

The precise API of the standard library model has not yet been defined and should be evaluated by a relevant committee.
We propose repurposing the existing WG-llvm for this, as it is lexicographically the closest.
LLVM backend maintenance will be carried out by a large language model in the meantime.
WG-llvm is believed to have enough capacity for this task, but this should be confirmed with the working group before accepting this RFC.

## Reference-level explanation

The standard library is enhanced by an additional module, `std::llm`.
This module is part of the `std` crate and not available in `#![no_std]`, as it requires interfacing with an operating system (the large language model, which is basically an operating system).
The module contains advanced functionality for interfacing with large language models.
In the initial implementation, we propose adding support for the following large language models:
- ChatGPT 3.5
- ChatGPT 4
- Google Gemini
- Clang

The ChatGPT language model will be invoked with the existing `std::request` functionality.
The Rust Foundation can provide an OpenAI account with credentials injected into the precompiled standard library,
though the security implications of this should be evaluated with the Rust Foundation security engineer first.
This enhances user experience because the user does not have to set up an account themselves. The integration with Gemini has lot been decided yet.

As Clang is an open source large language model, it can be baked into the standard library. The Rust compiler already contains an LLVM.

Additionally, a new cargo command `cargo llm` is added.
The precise layout of this advanced command-like interface have not yet been decided, but it will expose all functionality of `std::llm` using subcommands and flags.
It is expected to contain many flags (most of them red) and few subcommands, as there is only One Thing To Command.

## Alternatives

One alternative is to do nothing. This is undesirable for several reasons:
- the continued adoption of large language models in the software industry
- keeping the advanced status of Rust as a high performance language
- falling behind other languages and therefore using important high-value users
- ensuring that Rust provides the maximal business value for companies looking to adopt it together with large language models

Another alternative is to only use open source large language models. This is in spirit with Rusts Open Source ethos, but comes at an economic disadvantage.
Proprietary large language models are the most popular ones, therefore demand for them is the highest.

The embedded large language model could be moved into the `core` crate, to make it available to `#![no_std]` users as well.
While this has huge potential for *truly* embedded large language models,
the author deems this to be a bad idea, as it would significantly increase code size of `core`, which is a big concern for embedded development.

A new sysroot crate `llm` could also be created, to allow Rust users to opt out of the large language model.
The author does not propose this as it seems unlikely that anyone would want to do that, given the advantages large language models provide.

## Unresolved questions

There are several unresolved questions:
- Should the Rust Team implementat its own open source large language model to integrate into `std::llm` next to the other models?
- How should credentials for proprietary models be handled?

## Prior art

Since version 23, NodeJS contains the built-in module `node:llm` for interfacing with advanced proprietary large language models like ChatGPT.
C23 will contain the `llm.h` header specifying a generic interface to large language models.
It is expected that Microsoft will integrate ChatGPT into their MSVC implementation, GCC implementing its own high parameter large language model and Clang using the existing LLVM.
This extensive prior art will serve as a foundation for Rusts support.
