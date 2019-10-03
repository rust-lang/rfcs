- Feature Name: (fill me in with a unique ident, `cargo_plugin_fields`)
- Start Date: (fill me in with today's date, 2019-10-03)
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/cargo#7423](https://github.com/rust-lang/cargo/issues/7423)

# Summary
[summary]: #summary

Allow Cargo.toml to contain arbitrary fields in place, which `cargo` itself is not aware of, and are given semantic meaning by plugins.

# Motivation
[motivation]: #motivation

Plugins sometimes need additional information beyond that specified in Cargo.toml. Existing information like package.metadata is relegated to a side table.  The motivation for this equally, the ability to specify these plugin fields, and specify them _in place_.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

A cargo plugin could add a _plugin field_

- "@pluginname.key" = "value"
- When cargo sees a field beginning with the plugin prefix `@`. Cargo ignores the field, leaving plugins free use it.
- After the `@` symbol is listed the plugin name.
  To ensure that plugins giving different meaning to the same fields is not a problem, the plugin should be registered on crates.io.

### Examples:

Say you wish to write a plugin which classifies crates according to United States Executive Order 13526, and you wish to have this information specified clearly at the top of the crate.

You could create a cargo plugin `cargo-EO13526`

```Cargo.toml
[package]
"@EO13526.clearance" = "unclassified"
name = "foo"
version = "0.0.1"
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This is the technical portion of the RFC. Explain the design in sufficient detail that:

- Cargo currently emits a warning upon encountering such a field.
- This RFC is largely focused on removing that warning.

Removal of the warning is done recursively so
```
"@EO13526.clearance = { classification = "unclasified", history = [...]}
```

So because cargo ignores the top @EO13526 it also ignores classification and history sub-fields.


# Drawbacks
[drawbacks]: #drawbacks

There already exists the package.metadata table, it doesn't allow in place fields, but does allow adding of fields unrecognized by cargo.

And does so without an ugly prefix and key naming convention.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- When a cargo plugin is providing it's own build command, and said build requires additional metadata in order to function.

    It is desirable to have said metadata in place.
- The choice of `@` for the plugin prefix was chosen over `_`, and `.`,

    Dot: being the path separator for toml does not compose well in keypaths, with keys like `package..EO13526`, one of the dots being a separator for a keypath and one the name of the key.

    Underscore: Could work well e.g. `package._EO13526.classification`,
    where "_EO13526.classification" is a key.
    This has the benefit of being similar to rusts identifier naming behavior. 

    The other benefit of this choice is that `_` is not reqired to be quoted by toml.

    At-Sign: Was chosen for the proposal because it stresses that the key is being direct _at_ the plugin, rather than an implementation detail that cargo itself ignores the field.

    


# Prior art
[prior-art]: #prior-art

From build system perspective, Cargo is fairly unique in it's behavior of not allowing arbitrary keys in-place in build configuration files.

- [json-ld](https://www.w3.org/TR/json-ld/): Is somewhat similiar if not more structured, it supports contexts of the style `@prefix:suffix`.


# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Whether we should reserve a prefix at all
- Iff so what is the best prefix to reserve
- Currently does not focus on how Cargo should present the fields to plugins, whether Cargo should filter out keys that do not resolve to the plugins name.  Or if plugins are allowed to implement behaviors based upon the keys of other plugins.



# Future possibilities
[future-possibilities]: #future-possibilities

It would be really nice if in the future there was a `map` behavior

E.g.: 

```
[[map.dependencies]] = {
    keys = { "@EC13526.clearance" = "unclassified", registry = "crates.dhs.gov"}
    to = [{ name = "foo", version = "0.0.1"}]
}
```

Allowing in place plugin fields ensures behavior such as this can be used with plugin fields.
