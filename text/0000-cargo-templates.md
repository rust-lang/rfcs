- Feature Name: `cargo-templates`
- Start Date: 2021-02-15
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary

[summary]: #summary

This RFC proposes to add templates to Cargo. Rust users can submit templates called Cargo templates.
An example of this would be to create a CLI template which already has a Clap "Hello World" CLI set up.
Users can submit their templates like how they would create a crate. Once submitted,
one can create a new project with their template by running `cargo new NAME --template NAME_OF_TEMPLATE`
or `cargo init --template NAME_OF_TEMPLATE`.

# Motivation

[motivation]: #motivation

Lot's of Rust users tend to create common boilerplate's for each of their libraries or applications
which contain util functions, macros, error handling functions, general boilerplate, or something else.
With Cargo templates, one can easily create their boilerplate and use it as many times as they would like.

# Guide-level explanation

[guide-level-explanation]: #guide-level-explanation

Cargo templates is a feature which allows one to use some boilerplate code/template in their project.
For example, imagine you were creating a CLI application with Clap and you always use the same "Hello World"
CLI template as a starter. Instead of coding that out every time, with Cargo templates, you can create
a new template and create a new Cargo project with that template. Pass the `--template` flag followed
by the name of the template when running `cargo new` or `cargo init` to create a new project with the
template specified. You can use this in binaries or in libraries by passing the `--bin` or `--lib`
flags respectively.

What if I want to create a template? Well, you can initialize a new Cargo project which will be a template,
just like how you initialize a binary or library. Run `cargo new NAME --template` or `cargo init --template`.
You can publish the templates exactly how you would publish a crate, push the crate to GitHub and then
run `cargo publish`. You can see all the Cargo templates, and the one you just created on [crates.io](https://crates.io/templates).
You can see the one you just created by going to [crates.io/templates/NAME](https://crates.io/templates/NAME)
(NAME is equal to the same name you used for your template). Also, just like when creating a crate,
you can use doc comments (`///`) to automatically create your documentation.

# Reference-level explanation

[reference-level-explanation]: #reference-level-explanation

To make Cargo templates work, we would need updates to Cargo, docs.rs (maybe), and crates.io.

## Cargo

- CLI would need to be adjusted to take the `--template` flag

- Add templates to the Cargo platform
  - Publishing templates
  - Persisting to database, etc

## docs.rs

- Allowing for documentation to be built with templates too
  - TODO: should the design of a template's docs be different than a crates or the same
    (if same, we don't need to make any changes to docs.rs)

## crates.io

- Adding 2 new paths

  - `/templates`
    - Similar to `/`, `/templates` shows the newest templates, most downloaded, just updated templates,
      templates downloaded, templates in stock, most recent downloads, popular keywords, popular
      categories, and a description on what templates are
  - `/templates/NAME`
    - Similar to `/crate/NAME`, `/templates/NAME` would show the same data from the `Cargo.toml` config,
      installation info, and other stats such as last updated, template size, etc

- Edit dashboard to include templates and template statistics

# Drawbacks

[drawbacks]: #drawbacks

None yet.

# Rationale and alternatives

[rationale-and-alternatives]: #rationale-and-alternatives

None yet.

# Prior art

[prior-art]: #prior-art

- [Repl.it templates](https://repl.it/templates)

  - Allows you to create templates for any language (supported by Repl.it) and to create a new project within
    the Repl.it editor with that template

- [Create React app custom templates](https://create-react-app.dev/docs/custom-templates/)

  - Create React app custom templates allows you to create a React template and use that for your future
    projects

# Unresolved questions

[unresolved-questions]: #unresolved-questions

- Should the design of a template's docs be different than a crates or the same (in other words,
  would any changes to docs.rs be necessary)?

# Future possibilities

[future-possibilities]: #future-possibilities

None yet.
