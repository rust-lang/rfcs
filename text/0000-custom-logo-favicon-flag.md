- Feature Name: `custom_logo_favicon_flag`
- Start Date: 2022-01-31
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Add new command-line flags to `rustdoc` to specify a file to be used as the logo and/or favicon. The file(s) will be copied to the generated documentation directory and relative paths will be computed to point to the resource from within the generated HTML.

# Motivation
[motivation]: #motivation

Currently, there are two forms of the `doc` attribute to specify a custom favicon and logo: `html_favicon_url` and `html_logo_url`. However, they take the content as-is into all pages. Thus they effectively require an absolute URL/path. In particular, they do not allow to specify a local file and have the system compute the relative path as needed in sub-pages, like it is done in the non-custom case. Therefore, offline usage or publishing under a given path is harder.

A workaround for this is using the non-custom case and replacing the logo and the paths in every HTML file after generation (because the non-custom case generates relative paths to the official logo/favicon). For the favicon case, it is also needed to remove the `alternate icon` ones. Of course, this sort of hacks can break at any time.

Instead, it would be better to cover this use case properly with a command-line flag to specify a file to the favicon/logo and let `rustdoc` copy the file(s) and compute the paths as needed.

Another downside of the current `html_favicon_url` and `html_logo_url` is that they require changing the source code of the crate. For projects with many small crates (but all built together as part of the same project/workspace), it is boilerplate that needs to be repeated in every crate. Furthermore, projects that have custom builds of the standard library need to modify somehow the source code. Both cases may be worked around via `-Zcrate-attr` or a temporary copy. However, a command-line flag would be ideal, and would also cover the first issue discussed above.

It can also be argued that the logo/favicon URLs do not really belong with the Rust source code itself, but rather that they are out-of-band configuration/metadata for the documentation (similar to e.g. build/formatting options for Cargo/`rustfmt`).

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The following is written as if it belonged in the ["Command-line arguments"](https://doc.rust-lang.org/rustdoc/command-line-arguments.html) section of the `rustdoc` documentation.

## `--html-logo`

Using this flag looks like this:

```txt
$ rustdoc src/lib.rs --html-logo logo.svg
```

This flag takes an image file which will be used as the custom HTML logo in the rendered documentation. The file will be bundled together with the rest of the files in the output path.

## `--html-favicon`

Using this flag looks like this:

```txt
$ rustdoc src/lib.rs --html-favicon favicon.svg
```

This flag takes an image file which will be used as the custom HTML favicon in the rendered documentation. The file will be bundled together with the rest of the files in the output path.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The file taken by the flags shall be copied as-is into the root output path. It is a hard error if it cannot be copied for any reason (e.g. the file does not exist or cannot be read).

The rendered HTML should point to the file via relative paths, similarly to how it is done in the non-custom case.

If both a command-line flag and the related `doc` attribute are specified in the crate, the command-line flag takes precedence and a warning is emitted (users should have the ability to "allow" the warning).

# Drawbacks
[drawbacks]: #drawbacks

It adds complexity to the command-line interface of `rustdoc`.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Not doing this implies users will need to use workarounds like those described in the "Motivation" section. These workarounds are not easy to use, but more importantly, they may break at any point.

An alternative to a command-line flag would be having the functionality as part of the `doc` attributes in some form. However, while that would address the ability to use a local image file, it would not address the boilerplate issue nor the custom standard library builds (as mentioned in the "Motivation" section).

# Prior art
[prior-art]: #prior-art

Sphinx's configuration accepts the [`html_logo`](https://www.sphinx-doc.org/en/master/usage/configuration.html#confval-html_logo) and [`html_favicon`](https://www.sphinx-doc.org/en/master/usage/configuration.html#confval-html_favicon) options. Both options accept either a URL or a local file (which gets copied into the static files of the generated docs).

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None.

# Future possibilities
[future-possibilities]: #future-possibilities

The command-line interface could handle the URL case too, possibly via independent flags (e.g. `--html-logo-url` and `--html-favicon-url`) in order to avoid the issue of distinguishing URLs from files.

Perhaps this sort of documentation configuration/metadata could be specified in another place, e.g. a configuration file (similar to e.g. build/formatting options for Cargo/`rustfmt`).
