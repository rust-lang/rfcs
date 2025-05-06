- Feature Name: `register_tool`
- Start Date: 2025-03-22
- RFC PR: [#3803](https://github.com/rust-lang/rfcs/pull/3808)
- Rust Issue: [rust-lang/rust#66079](https://github.com/rust-lang/rust/issues/66079)

# Summary
[summary]: #summary

This RFC adds two new attributes:
- `#![register_lint_tool(tool_name)]` allows controlling namespaced lints with `#[warn(tool_name::lint_name)]`
- `#![register_attribute_tool(tool_name)]` allows using tool names in [inert attributes][inert] with `#[tool_name::attribute_name(token_tree)]`.

Additionally, to allow attribute tools with the same name as crates, this RFC changes name resolution order to give precedence to attribute tools over `extern` crates.

Note that this does not add any new functionality into the compiler; it only relaxes the current restrictions. While `rustc` verifies that tool attributes and lints are syntactically valid, it does no extra processing.

# Motivation
[motivation]: #motivation

There are [several tools predefined in the tool namespace][builtin-tools]. These tools are hard-coded, and cannot be extended with user-defined tools. There are many external programs that would benefit from being able to annotate specific portions of a crate or register custom lints without the compiler raising an error.

[builtin-tools]: https://doc.rust-lang.org/nightly/reference/attributes.html#tool-attributes

Here is a short summary of the built-in tools:

|Tool|Lints|Attributes|
|-|-|-|
|`clippy`|✅|✅|
|`rustfmt`|❌|✅|
|`miri`|❌|✅|
|`rust_analyzer`|❌|✅|
|`rustdoc`|✅|❌|
|`rustc`|✅ (with `-Z unstable-options`)|❌|
|`diagnostic`|❌|✅|

## Why support custom lints?

There are several crates, such as `bevy` and `regex`, that would benefit from API-specific lints that encourage specific styles or warn against potential footguns. While it is possible to create a custom `rustc` driver that registers these lints, any reference to them in code would cause the default compiler to raise an error.

```rust
// While `bevy_lint` will recognize this, the default `rustc` will not, raising a compile error.
#![warn(bevy::style)]
```

There are currently two solutions to this: [upstream lints directly to Clippy](https://rust-lang.github.io/rust-clippy/master/index.html#invalid_regex) or [use `#[cfg_attr(my_tool, warn(...))]`](https://thebevyflock.github.io/bevy_cli/bevy_lint/index.html#toggling-lints-in-code). The prior solution increases the maintenance burden for Clippy developers, and thus will rarely be accepted. The latter is very verbose and requires adding `unexpected_cfgs = { level = "warn", check-cfg = ["cfg(my_tool)"] }` in `Cargo.toml`.

There are also several linting tools that don't make sense to upstream to Clippy:

- [`cargo-semver-checks`](https://github.com/obi1kenobi/cargo-semver-checks/) (uses its own analysis framework, unrelated to `rustc_driver`)
- [`dylint`](https://github.com/trailofbits/dylint) (custom, user-extensible lints)
- [`marker`](https://github.com/rust-marker/marker) (custom, user-extensible lints, but a different approach)
- [`klint`](https://github.com/Rust-for-Linux/linux/pull/958) (Rust-for-Linux specific linter)

## Why support custom attributes?

There are also some tools that would benefit from using developer-added metadata on portions of source code:

- Formal verification tools, such as [prusti] and [kani], want to mark specific functions for verification. While adding contracts is an [explicit project goal][contracts goal], there are many existing tools for contracts that developers would benefit from in the meantime, and formal verification includes more than just contracts.
- Coverage tools, such as [tarpaulin], allow marking specific functions as skipped.
- Source code translation tools, such as [c2rust], want to mark the origin of generated code. One could imagine this also being useful for any kind of generated code, such as a build script, for recording [Source Map]-like metadata.

[prusti]: https://viperproject.github.io/prusti-dev/user-guide/syntax.html
[kani]: https://github.com/model-checking/kani
[stainless]: https://github.com/epfl-lara/rust-stainless/blob/1e16201c0b63fcc7f8871f0f9e9974b663e0e3eb/demo/examples/type_class_specs.rs#L5-L6
[contracts goal]: https://rust-lang.github.io/rust-project-goals/2024h2/Contracts-and-invariants.html
[tarpaulin]: https://github.com/xd009642/tarpaulin/#ignoring-code-in-files
[c2rust]: https://github.com/immunant/c2rust/blob/d28087df86d7fca8532d8679d35efec66f074f8b/c2rust-refactor/tests/reorganize_definitions/old.rs#L18
[Source Map]: https://web.dev/articles/source-maps

## Why change the name resolution order?

Currently, names in the type namespace are resolved in [the following order](https://github.com/rust-lang/reference/pull/1765):

1. Explicit definitions (including imports)
2. [Extern prelude] (crates injected using either `extern crate` or `--extern`)
3. [Tool prelude][builtin-tools]
4. Standard library prelude
5. Language prelude

[Extern prelude]: https://doc.rust-lang.org/nightly/reference/names/preludes.html#extern-prelude

This RFC switches the order of 3 and 2, such that tools take precedence over crates. This is because currently, loading a crate completely prevents using a tool attribute with that name. Consider this program:

```rust
extern crate rustfmt; // or --extern rustfmt

#[rustfmt::skip]
fn foo ( ) { }
```

Today, this gives an error that the crate `rustfmt` does not have a `skip` item. After this RFC, the attribute would still refer to the `rustfmt` tool. To disambiguate and refer to the crate, developers may write `#[::rustfmt::skip]`, which already forces resolution in the [Extern prelude], or simply `extern crate rustfmt as my_rustfmt` to avoid the ambiguity.

The reason this requires resolution changes is because tool attributes are [inert], so there is no way to disambiguate them.

[inert]: https://rustc-dev-guide.rust-lang.org/attributes.html#builtininert-attributes

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## For users of external tools

Several official tools let you configure their behavior on specific parts of your code. For example, Clippy lets you use `#[warn(clippy::as_ptr_cast_mut)]` to warn on that lint for a single item, and Rustfmt lets you use `#[rustfmt::skip]` to avoid formatting a single item. You can also do this for external tools that are not provided in the Rust toolchain. See the documentation of those tools for the lints and attributes they support.

To tell `rustc` that you are using an external tool, use `#![register_lint_tool(my_tool)]` or `#![register_attribute_tool(my_tool)]` in your crate root.

If a tool name conflicts with a crate names, you can disambiguate the crate with `::my_tool`:

```rust
#![register_attribute_tool(my_tool)]

extern crate my_tool;

// This is the attribute specified in the crate.
#[::my_tool::attribute]
fn foo() {
    // ...
}

// This is the attribute specified by the tool.
#[my_tool::attribute]
fn bar() {
    // ...
}
```

Crate-level lints for external tools can be configured in `Cargo.toml`, like any other crate-level lint:

```toml
[lints.my_tool]
my_lint = "warn"
```
 
[Kani]: https://github.com/model-checking/kani
[`bevy_lint`]: https://thebevyflock.github.io/bevy_cli/bevy_lint/
[Bevy game engine]: https://bevyengine.org/

## For authors of external tools

The Rust language can be extended and analyzed using external tools. If your tool can parse Rust, you may wish to allow configuring it at sub-crate levels (e.g. individual functions, types, and modules). To reuse the same syntax as the official tools, like Clippy and Rustfmt, instruct your users to add `#![register_lint_tool(your_tool)]` (if your tool only adds new lints) or `#![register_attribute_tool(your_tool)]` (if your tool only adds new attributes). If your tool supports both lints and attributes, register both. Then, instruct your users to add either `#[warn(your_tool::your_lint)]` or `#[your_tool::your_attribute(your_tokens)]` as appropriate.

The syntax for external attributes is carefully designed such that you do not need to do name resolution in order to recognize the attributes. As long as `register_attribute_tool(your_tool)` is present at the crate root, `#[your_tool::your_attribute]` will always be an [inert] attribute you can parse directly; it can never be a re-export of a different item, nor a reference to a local item.

Please *do* verify that `register_attribute_tool` is present, and either warn or error otherwise. If you do not do so, you may accidentally interpret a crate or local module as your tool.
We will ensure that `rustdoc --output-format json` includes `register_attribute_tool` so that users of rustdoc json are not required to reimplement a rust parser.

Please do *not* suggest using `#[cfg_attr(your_tool, your_attribute)]`. Doing so runs the risk that the language will add that lint or attribute in a future version. Use tool namespaces instead, as that's what they're for! It's ok to pass a custom `cfg` when your tool runs, but avoid using it to guard tool lints and attributes unless it would break your MSRV (minimum supported Rust version).

Please do *not* use tool attributes for metadata that changes the meaning of the code. At that point you are parsing a dialect of Rust, and there is no indication for your users that their code will be interpreted differently by your tool than by the compiler.  For example, `#[must_use]` and `#[automatically_derived]` would be suitable for tool attributes, but `#[repr]` and `#[panic_handler]` are not, because they change the meaning of the code. For that use case, use proc-macros, generated code, or bare (un-namespaced) attributes instead, all of which will give a hard error if they cannot be understood by the compiler. If absolutely necessary to use bare attributes, use a C-style namespace like `#[rustc_const_stable]`.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The tool prelude is separated into the tool attribute prelude (which is in the type namespace) and the lint prelude (which is only active inside lint controls).

The tool attribute prelude is changed to take precedence over the [Extern prelude]. The tool attribute prelude is only considered when resolving in an attribute macro; types in other positions, such as `fn foo() -> rustfmt::x`, do not consider the tool prelude. This is to avoid unnecessary breakage when the meaning is clear.

In the context of an attribute, ambiguity between a tool name and a local item is always a hard error. For example, this code would error:

```rust
#![register_tool(name)]

mod inner {
    mod name {
        // Import the derive macro.
        use Clone as x;
    }

    #[name::x]        // ERROR: ambiguous
    #[name::y]        // ERROR: ambiguous (even though y is not present)
    #[self::name::x] // OK
    fn f() {}
}
```
This is in order to not require external tools to perform name resolution. This restriction may be relaxed in the future to favor tool names.

The `#![register_attribute_tool(ident)]` crate-level attribute adds a new tool to the tool attribute prelude. The `#![register_lint_tool(ident)]` crate-level attribute adds a new tool to the lint prelude. For both attributes, tools must be a single ident, not a nested path.

`#[no_implicit_prelude]` keeps its current behavior, i.e. it removes all tools that are in the prelude by default. However, those tools can now be explicitly added back with e.g. `#![register_attribute_tool(rustfmt)]`.

Like today, attributes and lints in a tool namespace are always considered used by the compiler. The compiler does not verify the contents of any tool attribute, except to verify that all attributes are syntactically valid [tool attributes].

Unknown tool names in lints remain a hard error until the story for proc-macro lints is resolved (see [Future possibilities](#future-possibilities)).

Modules in the first path of an attribute (e.g. `#[unregistered::name]`) are assumed to be a crate if they cannot be resolved, and therefore give a hard error if not registered.

Registering a tool multiple times is an error. This makes registering a predefined tool (`clippy`, `miri`) using `#![register_*_tool(...)]` an error unless `#![no_implicit_prelude]` is specified.

[tool attributes]: https://doc.rust-lang.org/nightly/reference/attributes.html#tool-attributes
[`unknown_lints`]: https://doc.rust-lang.org/rustc/lints/listing/warn-by-default.html#unknown-lints

# Drawbacks
[drawbacks]: #drawbacks

This makes the rules for name resolution even more complicated.

This runs the risk that external tools will add attributes that change the meaning of the code, such that the behavior is different when the tool is present. There is not much we can do about this other than to ask tool authors not to do that.

This introduces a new "meta-breaking" concern: once this is stabilized, adding a new tool namespace, like we did for `diagnostic` and `rust_analyzer`, becomes a breaking change. I think in practice there will be few enough external tools that this will not be a major concern and we can just pick a name that isn't already in use. Note that, because `![register_*_tool(...)]` for a built-in tool errors, the user will see immediately what went wrong, instead of getting confusing errors from the attributes in the tool namespace.

The lang team [expressed a concern][lang concern] in 2022 that the name `register_*_tool` would mislead users into thinking that this *automatically* runs the tool. I do not think this is likely in practice; if someone adds this attribute, it's because the docs for the external tool told them to do so, and those docs should also say how to run the tool.

[lang concern]: https://github.com/rust-lang/rust/issues/66079#issuecomment-1010266282

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- We could "just not do this". That makes it harder to write external tools, and in practice just means that people use `cfg_attr` instead of a namespace, which seems strictly worse.
- We could relax the constraint that tool names cannot conflict with local items. This requires tools to do name resolution; but in practice I do not think we can expect tools to do this, and we must assume that the tool will behave differently than the compiler (`rustfmt` already does this today).
- We could add a syntax to disambiguate tool names from local items. That would add inconsistency with the existing built-in tool attributes, and requires tool authors to parse both the new and existing syntax.
- We could drop the change to name resolution, such that extern crates continue to take precedence over tools. This allows no way to use a tool and a crate with the same name, unless we add a new syntax.
<!-- We could continue to give a hard error on unknown tool names in lints. This doesn't disambiguate anything, so we would be restricting it for not much benefit.-->
- We could use a CLI argument for register_tool instead of a crate-level attribute. This means that the source code is no longer independent from the way it's built (although this is already true for some existing flags, like `--edition`). This would also be unnecessary if [`--crate-attr`][crate-attr] is merged, since that would allow passing any attribute as a flag.
- We could rename the attribute from `register_*_tool` to something else; perhaps `import_*_tool`, `inject_*_tool`, or `use_*_tool` by analogy with the `use` keyword. However, that makes the semantics unclear, and emphasizes the [lang concern] about it seeming as if the tool is run automatically. `rustc` calls adding new lints "registering" internally, and I think this is a good name for the semantics.
- We could continue using a single `register_tool` attribute instead of splitting it up into `register_lint_tool` and `register_attribute_tool`. This is slightly less complicated to write, but has the drawback that merely adding a lint tool changes name resolution for attribute macros, even if the tool does not define any attribute.
- `register_attribute_tool` could be named `register_metadata_tool` instead. `register_metadata_tool` makes it clear that tool attributes do not change the meaning of code, but `register_attribute_tool` makes it clearer how the tool is intended to be used.

[crate-attr]: https://github.com/rust-lang/rfcs/pull/3791

# Prior art
[prior-art]: #prior-art

- [`clang-tidy`], [`pylint`], [`eslint`], and [`review`] (a racket linter) use inline comments. Whether these count as namespacing is debatable; pylint and eslint include their name in the inline comment and clang-tidy does not. `review` allows both `review: ignore` and `lint: ignore`.
- [Roslyn analyzers] and [gcc] use `#pragma`s. GCC uses `#pragma GCC` and Roslyn uses `#pragma warning`.
- C and C++ use [vendor attributes], which are very similar to tool attributes, including namespacing. They do not have syntactic ambiguity with items in the type namespace and so do not perform any kind of name resolution; in Rust terms, all vendor attributes are [inert]. Like this RFC, and unlike the current language, C++ mandates that tools do not restrict namespaces they don't recognize.
- C# uses [attributes][c-sharp attrs], which are [active], not [inert], i.e. they follow normal name resolution rules. Like attribute macros in Rust, they can be defined in user code and are unrelated to external tools.
- [`Resyntax`] (a racket refactoring tool, like `cargo fix`) does not allow inline configuration; instead it requires you to write an extension to the tool specifying the new behavior in code.

[`clang-tidy`]: https://clang.llvm.org/extra/clang-tidy/#suppressing-undesired-diagnostics
[`pylint`]: https://pylint.pycqa.org/en/latest/user_guide/messages/message_control.html#block-disables
[`eslint`]: https://eslint.org/docs/latest/use/configure/rules#using-configuration-comments-1
[`review`]: https://github.com/Bogdanp/racket-review#usage
[Roslyn analyzers]: https://johnnyreilly.com/eslint-your-csharp-in-vs-code-with-roslyn-analyzers#deactivate-linting-partially
[gcc]: https://gcc.gnu.org/onlinedocs/gcc/Diagnostic-Pragmas.html
[vendor attributes]:  https://en.cppreference.com/w/cpp/language/attributes
[c-sharp attrs]: https://learn.microsoft.com/en-us/dotnet/standard/attributes/applying-attributes
[active]: https://rustc-dev-guide.rust-lang.org/attributes.html#non-builtinactive-attributes
[`Resyntax`]: https://docs.racket-lang.org/resyntax/Refactoring_Rules_and_Suites.html#(part._.Exercising_.Fine_.Control_.Over_.Comments)

# Unresolved questions
[unresolved-questions]: #unresolved-questions

How does this interact with [proc-macro lints][`proc_macro_lint`]?

# Future possibilities
[future-possibilities]: #future-possibilities

- Proc macros wish to register custom lints; see [`proc_macro_lint`]. We would have to establish some mechanism to prevent overlapping namespaces. Perhaps `warn(::project::lint_name)` could refer to the proc macro and `warn(project::lint_name)` would refer to any registered tool (only when a `project` tool is regisetered; in the common case where no tool is registered, `project::` would still refer to the proc macro).
- Projects may wish to have both a proc-macro crate with lints and a CLI with lints. To allow this, we would require `proc_macro_lint` to create an exhaustive list of lints that can be created, such that we can still run `unknown_lints` and do not need to create a new cooperation mechanism between `proc_macro_lint` and `register_lint_tool`, nor to require users of the project to distinguish the two with `::project` (see immediately above). We might still run into difficulty if the proc-macro lint namespace is only active while the proc-macro is expanding; it depends on how `proc_macro_lint` is specified. But I think it's ok to delay that discussion until `proc_macro_lint` gets an RFC.
- We could allow proc-macros to register a scoped tool, such that e.g. `#[serde::flatten]` is valid while the proc-macro is expanding, but not elsewhere in the crate. This is similar to [derive helpers], but namespaced. We would have to take care to avoid ambiguity between the scoped tool and globally registered tools in such a way that external tools still do not need to perform name resolution.
- Once [expression attributes] are stabilized, this would also allow tool attributes on expressions.
- We could support registering tools through `Cargo.toml`, which Cargo would then pass to `rustc` using `--crate-attr` (similar to `--check-cfg`). We would have to consider how this interacts with duplicate registration; we don't want to error if a tool is configured both through Cargo.toml and in source code.

[`proc_macro_lint`]: https://github.com/rust-lang/rust/pull/135432
[derive helpers]: https://doc.rust-lang.org/nightly/reference/procedural-macros.html#derive-macro-helper-attributes
[expression attributes]: https://github.com/rust-lang/rust/issues/15701
