- Feature Name: `doc_edit_links`
- Start Date: 2020-09-10
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary
Add an Edit icon next to rustdoc items to link to a relevant online editor.


# Motivation
[motivation]: #motivation

This is the `rustdoc` part of https://github.com/rust-lang/docs.rs/issues/1007.

Currently, rustdoc provides a `[src]` button,
which only links to the corresponding source render from rustdoc.
In case readers of a documentation would like to provide a quick fix
on either the code or a typo in the documentation,
it is necessary to:

1. Find the repository on GitHub (already provided by docs.rs header),
2. Identify the subdirectory in repository for the crate (in case the repository is complex)
3. Identify the corresponding file (by peeking the address in `[src]`)
4. Identify the corresponding line (sometimes same as `[src]`, sometimes moved)
5. Click the `E` hotkey on GitHub, or corresponding hotkey in other git tree browsers

This complexity increases barrier to contribute quick fixes.
This RFC adds the capability to generate links based on source file and line,
which can allow crates.io/docs.rs to elide steps 2, 3, 5 and sometimes 4 as well.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

`rustdoc` accepts an option `--edit-link-format` with value in a `std::fmt`-alike syntax.
If this option is passed, next to each `[src]` button,
a pen icon (alt text "Edit") with a hyperlink to the edit link is inserted.
The value of option is replaced with the following rules:

- `{file}` with the `src` path relative to the crate (using forward slashes even on Windows)
- `{start_line}` with the *starting* line number of the linked source
- `{end_line}` with the *ending* (inclusive) line of the linked source

For example, `libstd` would be documented with the following arguments:

```
--edit-link-format "https://github.com/rust-lang/rust/edit/master/libstd/{file}#L{start_line}-L{end_line}"
```

# Drawbacks
[drawbacks]: #drawbacks

This increases complexity of the documentation page, especially with mobile devices.
Good UX design is required to prevent bloating the content.

# Alternatives
[alternatives]: #alternatives

It is also possible to perform this with JavaScript on behalf of docs.rs only.

Pros:

- *May* support previous crate builds

Cons:

- Does not support self-hosted non-docs.rs builds
- `rustdoc` output is unstable and may not be forward-compatible

# Future possibilities
[future-possibilities]: #future-possibilities

docs.rs could use the new flags in all new generated documentation.
