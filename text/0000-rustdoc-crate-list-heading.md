- Feature Name: `rustdoc_crate_list_heading`
- Start Date: 2026-10-31
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

## Summary
[summary]: #summary

In Rustdoc, the build system can organize the list of crates in the sidebar with `--crate-list-heading`.
Cargo will use this to separate crates in the local workspace from crates.io dependencies.

## Motivation
[motivation]: #motivation

Rustdoc supports having more than one crate in a single documentation site.

This is used when you run `cargo doc` to produce a local documentation site
with your project plus all of its dependencies.
Large projects with custom or semi-custom build systems also use this feature to
build searchable docs across their internal ecosystems.

When a documentation site has hundreds of crates, the sidebar list becomes unwieldly.
But crates are a flat namespace as far as Rustdoc itself is concerned (except [RFC 3243][]),
so there isn't any way for Rustdoc to organize them without something giving it more information.

This RFC proposes a way for build systems to do that.

[RFC 3243]: https://rust-lang.github.io/rfcs/3243-packages-as-optional-namespaces.html

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

<table><thead><tr><th>Before</th><th>After</th></tr></thead>
<tbody><tr><td>

<h3>Crates</h3>
<ul>
<li><a href="https://kpreid.github.io/rust-138143-tmp-output/either/index.html">either</a></li>
<li><a href="https://kpreid.github.io/rust-138143-tmp-output/exhaust/index.html">exhaust</a></li>
<li><a href="https://kpreid.github.io/rust-138143-tmp-output/exhaust_macros/index.html">exhaust_macros</a></li>
<li><a href="https://kpreid.github.io/rust-138143-tmp-output/itertools/index.html">itertools</a></li>
<li><a href="https://kpreid.github.io/rust-138143-tmp-output/mutants/index.html">mutants</a></li>
<li><a href="https://kpreid.github.io/rust-138143-tmp-output/proc_macro2/index.html">proc_macro2</a></li>
<li><a href="https://kpreid.github.io/rust-138143-tmp-output/quote/index.html">quote</a></li>
<li><a href="https://kpreid.github.io/rust-138143-tmp-output/syn/index.html">syn</a></li>
<li><a href="https://kpreid.github.io/rust-138143-tmp-output/unicode_ident/index.html">unicode_ident</a></li>
</ul>

</td><td>

<h3>Crates</h3>
<ul>
<li><a href="https://kpreid.github.io/rust-138143-tmp-output/exhaust/index.html">exhaust</a></li>
<li><a href="https://kpreid.github.io/rust-138143-tmp-output/exhaust_macros/index.html">exhaust_macros</a></li>
</ul>
<h4>Dependencies</h4>
<ul>
<li><a href="https://kpreid.github.io/rust-138143-tmp-output/either/index.html">either</a></li>
<li><a href="https://kpreid.github.io/rust-138143-tmp-output/itertools/index.html">itertools</a></li>
<li><a href="https://kpreid.github.io/rust-138143-tmp-output/mutants/index.html">mutants</a></li>
<li><a href="https://kpreid.github.io/rust-138143-tmp-output/proc_macro2/index.html">proc_macro2</a></li>
<li><a href="https://kpreid.github.io/rust-138143-tmp-output/quote/index.html">quote</a></li>
<li><a href="https://kpreid.github.io/rust-138143-tmp-output/syn/index.html">syn</a></li>
<li><a href="https://kpreid.github.io/rust-138143-tmp-output/unicode_ident/index.html">unicode_ident</a></li>
</ul>

</td></tr></tbody></table>

When you build a documentation site using Rustdoc's CLI,
you can provide `--crate-list-heading="WHATEVER"` to put the crate under a subheading.
For example (including [RFC 3662][]):

[RFC 3662]: https://rust-lang.github.io/rfcs/3662-mergeable-rustdoc-cross-crate-info.html

```console
# Project crates
$ rustdoc --write-doc-meta=doc.meta --crate-name=exhaust exhaust/src/lib.rs -o doc
$ rustdoc --write-doc-meta=doc.meta --crate-name=exhaust_macros exhaust_macros/src/lib.rs -o doc
# Dependency crates
$ rustdoc --write-doc-meta=doc.meta --crate-name=either third_party/either/src/lib.rs --crate-list-heading="Dependencies" -o doc
$ rustdoc --write-doc-meta=doc.meta --crate-name=itertools third_party/itertools/src/lib.rs --crate-list-heading="Dependencies" -o doc
$ rustdoc --write-doc-meta=doc.meta --crate-name=mutants third_party/mutants/src/lib.rs --crate-list-heading="Dependencies" -o doc
$ rustdoc --write-doc-meta=doc.meta --crate-name=proc_macro2 third_party/proc_macro2/src/lib.rs --crate-list-heading="Dependencies" -o doc
$ rustdoc --write-doc-meta=doc.meta --crate-name=quote third_party/quote/src/lib.rs --crate-list-heading="Dependencies" -o doc
$ rustdoc --write-doc-meta=doc.meta --crate-name=syn third_party/syn/src/lib.rs --crate-list-heading="Dependencies" -o doc
$ rustdoc --write-doc-meta=doc.meta --crate-name=unicode_ident third_party/unicode_ident/src/lib.rs --crate-list-heading="Dependencies" -o doc
# Finalize
$ rustdoc --read-doc-meta=doc.meta -o doc
```

This option provides one level of organization.
It currently affects the sidebar, but Rustdoc reserves the right to use it for more
or ignore it entirely.

This option is orthogonal to [RFC 3243][]. Because `--crate-list-heading` is a CLI option,
it provides different classification depending on the build environment,
while RFC 3243 namespaces provide a way for crates to organize themselves
independently of when they're built.

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

### `--crate-list-heading`: organize this crate under a heading

Using this flag looks like this:

```console
$ rustdoc src/lib.rs --crate-name mycrate --crate-list-heading="Dependencies"
```

When this option is used, a sub-section underneath the "Crates" heading is added
if it doesn't already exist, and the crate is listed there.
By default, the crate appears in the list directly under the "Crates" heading.

## Drawbacks
[drawbacks]: #drawbacks

- The categories set by this option are ad-hoc.
  That means, if we decide to significantly change the UI,
  we might have to ignore this option entirely and design a new CLI for the new system.
- Because these options are build system controlled,
  the user who builds (and maybe publishes) might not actually want them.
  It's up to the build system to provide a good way to control this.
- Only one level of hierarchy is supported.
- Because the build system drives it,
  we're outsourcing a lot of design onto the build system.

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

### Why not an explicit `--is-dependency` parameter? Or a way to pass the entire dependency graph?

Trying not to bake too many Cargo-isms into Rustdoc.
They're designed together, and this feature is designed so Cargo can use it,
but you should be able to use it in build systems that don't match Cargo perfectly.

### Why no hierarchy?

This feature is meant to reflect build system controlled categories.
There's already [RFC 3243][] (crates as namespaces) that can handle more complex categories,
controlled by the crate author instead of the builder.

## Prior art
[prior-art]: #prior-art

### ExDoc (Erlang and Elixir doc tool)

Roughly speaking, a crate is called an Application over there.
It's not a perfect analogy, but it's good enough for this.

The [ExDoc CLI][] supports umbrella projects by passing compiled metadata
all at once as command-line parameters. It groups by app when doing this.
The docs describe a `groups_per_modules` option, but I can't find specific docs
about what it's supposed to do.

[ExDoc CLI]: https://github.com/elixir-lang/ex_doc#cli

## Unresolved questions
[unresolved-questions]: #unresolved-questions

Does `--crate-list-heading=""` work? I suppose it should act as a way to manually specify
the same behavior as not having the option at all.

What happens when you compile the same doc twice, once with a heading and once without?
I suppose it should move the crate.

## Future possibilities
[future-possibilities]: #future-possibilities

A well-designed feature for umbrella projects should have a way to write a top-level "readme"
site that isn't connected to a crate. There's already `--enable-index-page`, which is fine,
but would be better if you could put text documentation on top of it.

Besides headers, we could add a way to manually control the order of the sidebar entries.
We could also add a way to manually control the order of the headers,
perhaps by adding a CLI option to manually specify the sort key.
