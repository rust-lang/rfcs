- Feature Name: error_handling_project_group
- Start Date: 2020-07-23
- RFC PR: [rust-lang/rfcs#2965](https://github.com/rust-lang/rfcs/pull/2965)
- Rust Issue: [rust-lang/libs-team#3](https://github.com/rust-lang/libs-team/issues/3)

# Summary
[summary]: #summary

This RFC establishes a new project group, under the libs team, to drive efforts to improve error handling in Rust.

# Motivation
[motivation]: #motivation

The error handling project group aims to reduce confusion on how to structure error handling for users in the Rust community. This will be accomplished by creating learning resources and pushing effort to upstream widely used crates into the standard library. As a secondary goal, this project group will also try to resolve some known issues with the `Error` trait and reporting errors in panics/termination.

# Charter
[charter]: #charter

## Goals

### Agree on and define common error handling terminology

- Recoverable error: An error that can be reacted and recovered from when encountered e.g. a missing file.
- Unrecoverable error: An error that cannot reasonably be reacted to or recovered from and which indicates a bug e.g. indexing out of bounds.
- Error Type: A type that represents a recoverable error. Error types can optionally implement the `Error` trait so that it can be reported to the user or be converted into a trait object.
- Reporting Type: A type that can store all recoverable errors an application may need to propagate and print them as error reports.
    - Reporting types can represent the recoverable errors either via concrete types, likely parameterized, or trait objects.
    - Reporting types often bundle context with errors when they are constructed, e.g. `Backtrace`.
    - Reporting types often provide helper functions for creating ad hoc errors whose only purpose is to be reported e.g. `anyhow::format_err!` or `eyre::WrapErr`.

### Come to a consensus on current best practices

Here is a tentative starting point, subject to change:

- Use `Result` and `Error` types for recoverable errors.
- Use `panic` for unrecoverable errors.
- Implement `Error` for error types that may need to be reported to a human or be composed with other errors.
- Use enums for types representing multiple failure cases that may need to be handled.
    - For libraries, oftentimes you want to support both reporting and handling so you implement `Error` on a possibly non-exhaustive enum.
- Error kind pattern for associating context with every enum variant without including the member in every enum variant.
- Convert to a reporting type when the error is no longer expected to be handled beyond reporting e.g. `anyhow::Error` or `eyre::Report` or when trait object + downcast error handling is preferable.
- Recommend `Box`ing concrete error types when stack size is an issue rather than `Box`ing and converting to `dyn Error`s.
- What is the consensus on handling `dyn Error`s? Should it be encouraged or discouraged? Should we look into making `Box<dyn Error...>` implement `Error`?


### Identify pain points in error handling today

- Backtrace capture is expensive, but without one it can be difficult to pinpoint the origin of errors
- unwrap on errors without first converting to a reporting type will often discard relevant information
- errors printing from main have to assume a prefixed `Error: `, sub par control of output format when printing during termination.
- Error trait only exposes 3 forms of context, can only represent singly linked lists for chains of errors

### Communicate current best practices

- Document the consensus.
- Communicate plan for future changes to error handling, and the libraries that future changes are being based off of.
- Produce learning resources related to current best practices.
    - New chapters in the book?

### Evaluate options for error reporting type a.k.a. better `Box<dyn Error>`

- Survey the current libraries in the ecosystem:
    - `anyhow`
    - `eyre`
- Evaluate value of features including:
    - Single word width on stack
    - Error wrapping with display types and with special downcast support.
    - Report hook and configurable `dyn ReportHandler` type for custom report formats and content, similar to panic handler but for errors.
    - libcore compatibility.

### Consolidate ecosystem by merging best practice crates into std

- Provide a derive macro for `Error` in std.
- Stabilize the `Backtrace` type but possibly not `fn backtrace` on the `Error` trait.
    - Provide necessary API on `Backtrace` to support crates like `color-backtrace`.
- Move `Error` to core.
    - Depends on generic member access.
    - Requires resolving downcast dependency on `Box` and blocking the stabilization of `fn backtrace`.
- Potentially stabilize an error reporting type based on `anyhow` and `eyre` now that they're close to having identical feature sets.

### Add missing features

- Generic member access on the `Error` trait.
- `Error` return traces:
    - Depends on specialization and generic member access.
- Fix rough corners around reporting errors and `Termination`.

## Non Goals

- This group should not be involved in design discussions for the `Try` trait, `try` blocks, or `try` fns.

## Membership Requirements

- Group membership is open, any interested party can participate in discussions, repeat contributors will be added to appropriate teams.

## Additional Questions

### What support do you need, and separately want, from the Rust organization?

I'm not sure, my main concern is getting prompt feedback on RFCs.

### Why should this be a project group over a community effort?

There isn't anything in this project group that can't be handled as a
community effort, but centralizing work into a project group should help
speed things. Error handling is a core aspect of the language and changes in
error handling have large impacts on the ecosystem. Ensuring that efforts to
refine error handling within Rust have sufficient resources and don't stall
out is in the best interests of the community. By organizing efforts as a
project group we will hopefully have an easier time recruiting new members,
getting attention on RFCs from members of the libs team, and using the
established resources and expertise of the rust organization for coordinating
our efforts.

### What do you expect the relationship to the team be?

The project group will create RFCs for various changes to the standard library and the team will review them via the standard RFC process.

### Who are the initial shepherds/leaders? (This is preferably 2–3 individuals, but not required.)

Jane Lusby(@yaahc_), Andrew Gallant(@BurntSushi), and Sean Chen(@seanchen1991).

### Is your group long-running or temporary?

Temporary.

### If it is temporary, how long do you see it running for?

This depends pretty heavily on how quickly the RFCs move, anywhere between 6 months and 2 years I'd guess but don't quote me on this.

### If applicable, which other groups or teams do you expect to have close contact with?

Primarily the libs team, but there may be some small interactions with the lang team, compiler team, and traits working group.

### Where do you see your group needing help?

Primarily in drafting RFCs, writing is not this author's strong suit.
