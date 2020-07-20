- Feature Name: `rustdoc_json`
- Start Date: 2020-06-26
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

This RFC describes the design of a JSON output for the tool `rustdoc`, to allow tools to
lean on its data collection and refinement but provide a different front-end.

# Motivation
[motivation]: #motivation

The current HTML output of `rustdoc` is often lauded as a key selling point of Rust. It's a
ubiquitous tool, that you can use to easily find nearly anything you need to know about a crate.
However, despite its versatility, its output format has some drawbacks:

- Viewing this output requires a web browser, with (for some features of the output) a JavaScript
  interpreter.
- The HTML output of `rustdoc` is explicitly not stabilized, to allow `rustdoc` developers the
  option to tweak the display of information, add new information, etc. In addition it's not
  generated  with the intent of being scraped by users which makes converting this HTML into a
  different format impractical. People are still able to build [cool stuff](https://crates.io/crates/rocdoc)
  on top of it, but it's unwieldy and limiting to parse the HTML like that. For use cases like
  this, a stable, well documented, easily parsable format with semantic information
  accessible would be far more useful.
- As the HTML is the only available output of `rustdoc`, its integration into centralized,
  multi-language, documentation browsers is difficult.

In addition, `rustdoc` had JSON output in the past, but it failed to keep up with the changing
language and [was taken out][remove-json] in 2016. With `rustdoc` in a more stable position, it's
possible to re-introduce this feature and ensure its stability. This [was brought up in 2018][2018-discussion]
with a positive response and there are [several][2019-interest] [recent][rustdoc-infopages]
discussions indicating that it would be a useful feature.

In [the draft RFC from 2018][previous-rfc] there was some discussion of utilizing `save-analysis`
to provide this information, but with [RLS being replaced by rust-analyzer][RA-RLS] it's possible
that the feature will be eventually removed from the compiler. In addition `save-analysis` output
is just as unstable as the current HTML output of `rustdoc`, so a separate format is preferable.

[remove-json]: https://github.com/rust-lang/rust/pull/32773
[2018-discussion]: https://internals.rust-lang.org/t/design-discussion-json-output-for-rustdoc/8271/6
[2019-interest]: https://github.com/rust-lang/rust/issues/44136#issuecomment-467144974
[rustdoc-infopages]: https://internals.rust-lang.org/t/current-state-of-rustdoc-and-cargo/11721
[previous-rfc]: https://github.com/QuietMisdreavus/rfcs/blob/rustdoc-json/text/0000-rustdoc-json.md#unresolved-questions
[RA-RLS]: https://github.com/rust-lang/rfcs/pull/2912

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

(*Upon successful implementation/stabilization, this documentation should live in The Rustdoc
Book.*)

In addition to generating the regular HTML, `rustdoc` can create a JSON file based on your crate.
These can be used by other tools to take information about your crate and convert it into other
output formats, insert into centralized documentation systems, create language bindings, etc.

To get this output, pass the `--output-format json` flag to `rustdoc`:

```shell
$ rustdoc lib.rs --output-format json
```

This will output a JSON file in the current directory (by default). For example, say you have the
following crate:

```rust
//! Here are some crate-level docs!

/// Here are some docs for `some_fn`!
pub fn some_fn() {}

/// Here are some docs for `SomeStruct`!
pub struct SomeStruct;
```

After running the above command, you should get a `lib.json` file like the following:

```json
{
  "root": "0:0",
  "version": null,
  "includes_private": false,
  "index": {
    "0:3": {
      "crate_num": 0,
      "name": "some_fn",
      "source": {
        "filename": "lib.rs",
        "begin": [4, 0],
        "end": [4, 19]
      },
      "visibility": "public",
      "docs": "Here are some docs for `some_fn`!",
      "attrs": [],
      "kind": "function",
      "inner": {
        "function_item": {
          "decl": {
            "inputs": [],
            "output": null,
            "c_variadic": false
          },
          "generics": {...},
          "header": "",
          "abi": "\"Rust\""
        }
      }
    },
    "0:4": {
      "crate_num": 0,
      "name": "SomeStruct",
      "source": {
        "filename": "lib.rs",
        "begin": [7, 0],
        "end": [7, 22]
      },
      "visibility": "public",
      "docs": "Here are some docs for `SomeStruct`!",
      "attrs": [],
      "kind": "struct",
      "inner": {
        "struct_item": {
          "struct_type": "unit",
          "generics": {...},
          "fields_stripped": false,
          "fields": [],
          "impls": [...]
        }
      }
    },
    "0:0": {
      "crate_num": 0,
      "name": "lib",
      "source": {
        "filename": "lib.rs",
        "begin": [1, 0],
        "end": [7, 22]
      },
      "visibility": "public",
      "docs": "Here are some crate-level docs!",
      "attrs": [],
      "kind": "module",
      "inner": {
        "module_item": {
          "is_crate": true,
          "items": [
            "0:4",
            "0:3"
          ]
        }
      }
    }
  },
  "paths": {
    "0:3": {
      "crate_num": 0,
      "path": ["lib", "some_fn"],
      "kind": "function"
    },
    "0:4": {
      "crate_num": 0,
      "path": ["lib", "SomeStruct"],
      "kind": "struct"
    },
    ...
  }
  "extern_crates": {
    "9": {
      "name": "backtrace",
      "html_root_url": "https://docs.rs/backtrace/"
      },
    "2": {
      "name": "core",
      "html_root_url": "https://doc.rust-lang.org/nightly/"
    },
    "1": {
      "name": "std",
      "html_root_url": "https://doc.rust-lang.org/nightly/"
    },
    ...
  }
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

(*Upon successful implementation/stabilization, this documentation should live in The Rustdoc
Book and/or an external crate's Rustdoc.*)

(*Given that the JSON output will be implemented as a set of Rust types with serde serialization,
the most useful docs for them would be the 40 or so types themselves. By writing docs on those
types the Rustdoc page for that module would become a good reference. It may be helpful to provide
some sort of [schema](http://json-schema.org/) for use with other languages*)

When you request JSON output from `rustdoc`, you're getting a version of the Rust abstract syntax
tree (AST), so you could see anything that you could export from a valid Rust crate. The following
types can appear in the output:

## ID

To provide various maps/references to items, the JSON output uses unique strings as IDs for each
item. They happen to be the compiler internal DefId for that item, but in the JSON blob they should
be treated as opaque as they aren't guaranteed to be stable across compiler invocations. IDs should
NOT be used to link items from external crates (see [the Resolving IDs section](#resolving-ids)).

## Crate

A Crate is the root of the outputted JSON blob. It contains all doc-relevant information about the
local crate, as well as some information about external items that are referred to locally.

Name      | Type    | Description
----------|---------|------------------------------------------------------------------------------
`name`    | String  | The name of the crate. If `--crate-name` is not given, based on the filename.
`version` | String  | (*Optional*) The version string given to `--crate-version`, if any.
`includes_private`  | bool  | Whether or not the output includes private items.
`root`    | [ID](#ID)      | The ID of the root module Item.
`index`   | Map<[ID](#ID), [Item](#Item)> | A collection of all Items in the crate[\*](#resolving-ids).
`paths`   | Map<[ID](#ID), [ItemSummary](#ItemSummary)> | Maps all IDs (even external ones[\*](#resolving-ids)) to a brief description including their name, crate of origin, and kind.
`extern_crates` | Map<int, [ExternalCrate](#ExternalCrate)> | A map of "crate numbers" to metadata about that crate.

### Resolving IDs

The `index` field of [Crate](#Crate) contains _almost_ entirely local items, which includes impls
of external traits on local types or local traits on external types. The exception to this is that
external trait definitions are also included in the `index` to allow users to access information
about their methods and associated types/consts.

This means that many IDs aren't included in the `index`. In these cases the fallback is to look up
the ID in `paths` where it should be present regardless of whether it's local. That gives
[enough information](#ItemSummary) to create cross references or simply provide names without
copying all of the information about external Items into the local crate's JSON output.

### ExternalCrate

Name      | Type    | Description
----------|---------|------------------------------------------------------------------------------
`name`    | String  | The name of the crate.
`html_root_url` | String  | (*Optional*) The`html_root_url` for that crate if they specify one.

### ItemSummary

Name      | Type    | Description
----------|---------|------------------------------------------------------------------------------
`crate_num` | int   | A number corresponding to the crate this Item is from. Used as an key to the `extern_crates` map in [Crate](#Crate). A value of zero represents an Item from the local crate, any other number means that this Item is external.
`path`    | [String] | The fully qualified path (e.g. ["std", "io", "lazy", "Lazy"] for `std::io::lazy::Lazy`) this Item.
`kind`    | String  | What type of Item this is (see [Item](#Item)).

## Item

An Item represents anything that can hold documentation - modules, structs, enums, functions,
traits, type aliases, and more. The Item data type holds fields that can apply to any of these,
and leaves kind-specific details (like function args or enum variants) to the `inner` field.

Name      | Type    | Description
----------|---------|------------------------------------------------------------------------------
`crate_num` | int   | A number corresponding to the crate this Item is from. Used as an key to the `extern_crates` map in [Crate](#Crate). A value of zero represents an Item from the local crate, any other number means that this Item is external.
`name`    | String  | The name of the Item, if present. Some Items, like impl blocks, do not have names.
`span`    | [Span](#Span) | The source location of this Item.
`visibility` | String | `"default"`, `"public"`, `"crate"`, or `"restricted"` (`pub(path)`). TODO: show how the restricted path info is represented.
`docs`    | String  | The extracted documentation text from the Item.
`attrs`   | [String]| The attributes (other than doc comments) on the Item, rendered as strings.
`kind`    | String  | The kind of Item this is. Determines what fields are in `inner`.
`inner`   | Object  | The type-specific fields describing this Item. Check the `kind` field to determine what's available.

### `kind == "module"`

Name     | Type   | Description
---------|--------|--------------------------------------------------------------------------------
`items`  | [[ID](#ID)] | The list of Items contained within this module. The order of definitions is preserved.

### `kind == "function" || "foreign_function"`

Foreign functions are `fn`s from an `extern` block

Name       | Type     | Description
-----------|----------|----------------------------------------------------------------------------
`decl`     | [FnDecl](#FnDecl) | Information about the function signature, or declaration.
`generics` | [Generics](#Generics) | Information about the function's type parameters and `where` clauses.
`header`   | String   | `"const"`, `"async"`, `"unsafe"`, or a space separated combination of those
modifiers.
`abi`      | String   | The ABI string on the function. Non-`extern` functions have a `"Rust"` ABI, whereas `extern` functions without an explicit ABI are `"C"`. See [the reference](https://doc.rust-lang.org/reference/items/external-blocks.html#abi) for more details.

### `kind == "struct" || "union"`

Name          | Type     | Description
--------------|----------|-------------------------------------------------------------------------
`struct_type` | String   | Either `"plain"` for braced structs, `"tuple"` for tuple structs, or `"unit"` for unit structs.
`generics`    | [Generics](#Generics) | Information about the struct's type parameters and `where` clauses.
`fields_stripped` | bool | Whether any fields have been removed from the result, due to being private or hidden.
`fields`      | [[ID](#ID)] | The list of fields in the struct. All of the corresponding Items have `kind == "structfield"`.
`impls`       | [[ID](#ID)] | All impls (both trait and inherent) for this type. All of the corresponding Items have `kind = "impl"`

### `kind == "structfield"`

Name          | Type     | Description
--------------|----------|-------------------------------------------------------------------------
`type`        | [Type](#Type) | The type of this field.

### `kind == "enum"`

Name          | Type     | Description
--------------|----------|-------------------------------------------------------------------------
`generics`    | [Generics](#Generics) | Information about the enum's type parameters and `where` clauses.
`fields`      | [[ID](#ID)]     | The list of variants in the enum. All of the corresponding Items have `kind == "variant"`.
`fields_stripped` | bool | Whether any variants have been removed from the result, due to being private or hidden.
`impls`       | [[ID](#ID)] | All impls (both trait and inherent) for this type. All of the corresponding Items have `kind = "impl"`

### `kind == "variant"`

`inner` can be one of the 3 following objects:
- `"plain"` (e.g. `Enum::Variant`)
- `{"tuple": [Type]}` (e.g. `Enum::Variant(u32, String)`)
- `{"struct": Object}` (e.g. `Enum::Variant{foo: u32, bar: String}`) in which case the `Object` is the same as `inner` when `kind == "struct"`.

### `kind == "trait"`

Name          | Type     | Description
--------------|----------|-------------------------------------------------------------------------
`is_auto`     | bool     | Whether this trait is an autotrait like `Sync`.
`is_unsafe`   | bool     | Whether this is an `unsafe trait` such as `GlobalAlloc`.
`items`       | [[ID](#ID)] | The list of method, constant, and typedef items contained in this trait definition.
`generics`    | [Generics](#Generics) | Information about the trait's type parameters and `where` clauses.
`bounds`      | [[GenericBound](#GenericBound)] | Trait bounds for this trait definition (e.g.  `trait Foo: Bar<T> + Clone`).

### `kind == "trait_alias"`

An [unstable feature](https://doc.rust-lang.org/beta/unstable-book/language-features/trait-alias.html)
which allows writing aliases like `trait Foo = std::fmt::Debug + Send` and then using `Foo` in
bounds rather than writing out the individual traits.

Name          | Type     | Description
--------------|----------|-------------------------------------------------------------------------
`generics`    | [Generics](#Generics) | Any type parameters that the trait alias takes.
`bounds`      | [[GenericBound](#GenericBound)] | The list of traits after the equals.

### `kind == "method"`

Name          | Type     | Description
--------------|----------|-------------------------------------------------------------------------
`decl`        | [FnDecl](#FnDecl) | Information about the method signature, or declaration.
`generics`    | [Generics](#Generics) | Information about the method's type parameters and `where` clauses.
`header`   | String   | `"const"`, `"async"`, `"unsafe"`, or a space separated combination of those
modifiers.
`has_body`    | bool     | Whether this is just a method signature (in a trait definition) or a method with an actual body.

### `kind == "impl"`

Name          | Type     | Description
--------------|----------|-------------------------------------------------------------------------
`is_unsafe`   | bool     | Whether this impl is for an unsafe trait.
`generics`    | [Generics](#Generics) | Information about the impl's type parameters and `where` clauses.
`provided_trait_methods` | [String] | The list of names for all provided methods in this impl block. This is provided for ease of access if you don't need more information from the `items` field.
`trait`       | [Type](#Type) | The trait being implemented or `null` if the impl is "inherent".
`for`         | [Type](#Type) | The type that the impl block is for.
`items`       | [[ID](#ID)] | The list of method, constant, and typedef items contained in this impl block.
`negative`    | bool     | Whether this is a negative impl (e.g. `!Sized` or `!Send`).
`synthetic`   | bool     | Whether this is an impl that's implied by the compiler (for autotraits).
`blanket_impl` | [Type](#Type) | TODO

### `kind == "constant"`

Name          | Type     | Description
--------------|----------|-------------------------------------------------------------------------
`type`        | [Type](#Type) | The type of this constant.
`expr`        | String   | The stringified expression of this constant.
`value`       | String   | (*Optional*) The value of the evaluated expression for this constant, which is only computed for numeric types.
`is_literal`  | bool     | Whether this constant is a bool, numeric, string, or char literal.

### `kind == "static"`

Name          | Type     | Description
--------------|----------|-------------------------------------------------------------------------
`type`        | [Type](#Type) | The type of this static.
`expr`        | String   | The stringified expression that this static is assigned to.
`mutable`     | bool     | Whether this static is mutable.

### `kind == "typedef"`

Name          | Type     | Description
--------------|----------|-------------------------------------------------------------------------
`type`        | [Type](#Type) | The type on the right hand side of this definition.
`generics`    | [Generics](#Generics) | Any generic parameters on the left hand side of this definition.

### `kind == "assoc_const"`

These items only show up in trait _definitions_. When looking at a trait impl item, the item where the associated constant is defined is a `"constant"` item.

Name          | Type     | Description
--------------|----------|-------------------------------------------------------------------------
`type`        | [Type](#Type) | The type of this associated const.
`default`     | String | (*Optional*) The stringified expression for the default value, if provided.

### `kind == "assoc_type"`

These items only show up in trait _definitions_. When looking at a trait impl item, the item where the associated type is defined is a `"typedef"` item.

Name          | Type     | Description
--------------|----------|-------------------------------------------------------------------------
`bounds`      | [[GenericBound](#GenericBound)] | The bounds for this associated type.
`default`     | [Type](#Type) | (*Optional*) The default for this type, if provided.

### `kind == "foreign_type"`

Name          | Type     | Description
--------------|----------|-------------------------------------------------------------------------
`bounds`      | [[GenericBound](#GenericBound)] | The bounds for this associated type.
`default`     | [Type](#Type) | (*Optional*) The default for this type, if provided.

### `kind == "extern_crate"`
TODO
### `kind == "import"`
TODO
### `kind == "opaque_ty"`
TODO
### `kind == "macro"`
TODO
### `kind == "proc_attribute"`
TODO
### `kind == "proc_derive"`

## Span

Name       | Type     | Description
-----------|----------|----------------------------------------------------------------------------
`filename` | String   | The path to the source file for this span relative to the crate root.
`begin`    | (int, int) | The zero indexed line and column of the first character in this span.
`begin`    | (int, int) | The zero indexed line and column of the last character in this span.

## FnDecl

Name       | Type     | Description
-----------|----------|----------------------------------------------------------------------------
`inputs`   | [(String, [Type](#Type))] | A list of parameter names and their types.
`output`   | [Type](#Type) | (*Optional*) Output type.
`c_variadic` | bool   | Whether this function uses [an unstable feature](https://doc.rust-lang.org/beta/unstable-book/language-features/c-variadic.html) for variadic FFI functions.

## Generics

Name       | Type     | Description
-----------|----------|----------------------------------------------------------------------------
`params`   | [[GenericParamDef](#GenericParamDef)] | A list of generic parameter definitions (e.g.  `<T: Clone + Hash, U: Copy>`)
`where_predicates` | [[WherePredicate](#WherePredicate)] | A list of where predicates (e.g.  `where T: Iterator, T::Item: Copy`)

### Examples

Here are a few full examples of the Generics fields for different rust code:

#### Lifetime bounds

```rust
pub fn foo<'a, 'b, 'c>(a: &'a str, b: &'b str, c: &'c str)
where
    'a: 'b + 'c, {…}
```

```json
"generics": {
  "params": [
    {
      "name": "'a",
      "kind": "lifetime"
    },
    {
      "name": "'b",
      "kind": "lifetime"
    },
    {
      "name": "'c",
      "kind": "lifetime"
    }
  ],
  "where_predicates": [
    {
      "region_predicate": {
        "lifetime": "'a",
        "bounds": [
          {
            "outlives": "'b"
          },
          {
            "outlives": "'c"
          }
        ]
      }
    }
  ]
```

#### Trait bounds

```rust
pub fn bar<T, U: Clone>(a: T, b: U)
where
    T: Iterator,
    T::Item: Copy,
    U: Iterator<Item=u32>, {…}
```

```json
"generics": {
  "params": [
    {
      "name": "T",
      "kind": {
        "type": {
          "bounds": [],
          "synthetic": false
        }
      }
    },
    {
      "name": "U",
      "kind": {
        "type": {
          "bounds": [
            {
              "trait_bound": {
                "trait": {/* `Type` representation for `Clone`*/},
                "generic_params": [],
                "modifier": "none"
              }
            }
          ],
          "synthetic": false
        }
      }
    }
  ],
  "where_predicates": [
    {
      "bound_predicate": {
        "ty": {
          "generic": "T"
        },
        "bounds": [
          {
            "trait_bound": {
              "trait": {/* `Type` representation for `Iterator`*/},
              "generic_params": [],
              "modifier": "none"
            }
          }
        ]
      }
    },
    {
      "bound_predicate": {
        "ty": {/* `Type` representation for `Iterator::Item`},
        "bounds": [
          {
            "trait_bound": {
              "trait": {/* `Type` representation for `Copy`*/},
              "generic_params": [],
              "modifier": "none"
            }
          }
        ]
      }
    },
    {
      "bound_predicate": {
        "ty": {
          "generic": "U"
        },
        "bounds": [
          {
            "trait_bound": {
              "trait": {/* `Type` representation for `Iterator<Item=u32>`*/},
              "generic_params": [],
              "modifier": "none"
            }
          }
        ]
      }
    }
  ]
}
```

### GenericParamDef

Name       | Type     | Description
-----------|----------|----------------------------------------------------------------------------
`name`     | String   | The name of the type variable of a generic parameter (e.g `T` or `'static`)
`kind`     | Object   | Either `"lifetime"`, `"const": Type`, or `"type: Object"` with the following fields:

Name       | Type     | Description
-----------|----------|----------------------------------------------------------------------------
`bounds`   | [GenericBound](#GenericBound) | The bounds on this parameter.
`default`  | [Type](#Type) | (*Optional*) The default type for this parameter (e.g `PartialEq<Rhs = Self>`).

### WherePredicate

Can be one of the 3 following objects:
- `"bound_predicate": {"ty": Type, "bounds": [GenericBound]}` for `T::Item: Copy + Clone`
- `"region_predicate": {"lifetime": String, "bounds": [GenericBound]}` for `'a: 'b`
- `"eq_predicate": {"lhs": Type, "rhs": Type}`

### GenericBound

Can be either `"trait_bound"` with the following fields:

Name       | Type     | Description
-----------|----------|----------------------------------------------------------------------------
`trait`    | [Type](#Type) | The trait for this bound.
`modifier` | String   | Either `"none"`, `"maybe"`, or `"maybe_const"`
`generic_params` | [[GenericParamDef](#GenericParamDef)] | `for<>` parameters used for [HRTBs](https://doc.rust-lang.org/nomicon/hrtb.html)

## Type

Rustdoc's representation of types is fairly involved. Like Items, they are represented by a
`"kind"` field and an `"inner"` field with the related information. Here are the possible
contents of that inner Object:

### `kind = "resolved_path"`

This is the main kind that represents all user defined types.

Name       | Type     | Description
-----------|----------|----------------------------------------------------------------------------
`name`     | String   | The path of this type as written in the code (`"std::iter::Iterator"`, `"::module::Struct"`, etc.).
`args`     | [GenericArgs](#GenericArgs) | Any arguments on this type such as `Vec<i32>` or `SomeStruct<'a, 5, u8, B: Copy, C = 'static str>`.
`id`       | [ID](#ID) | The ID of the trait/struct/enum/etc. that this type refers to.
`param_names` | [GenericBound](#GenericBound) | (*Optional*) If this type is of the form `dyn Foo + Bar + ...` then this field contains the information about those trait bounds.

#### GenericArgs

Can be either `"angle_bracketed"` with the following fields:

Name       | Type     | Description
-----------|----------|----------------------------------------------------------------------------
`args`     | [[GenericArg](#GenericArg)] | The list of each argument on this type.
`bindings` | [TypeBinding](#TypeBinding) | Associated type or constant bindings (e.g. `Item=i32` or `Item: Clone`) for this type.

or `"parenthesized"` (for `Fn(A, B) -> C` arg syntax) with the following fields:

Name       | Type     | Description
-----------|----------|----------------------------------------------------------------------------
`inputs`   | [[Type](#Type)] | The `Fn`'s parameter types for this argument.
`output`   | [Type](#Type) | (*Optional*) The return type of this argument.

#### GenericArg

Can be one of the 3 following objects:
- `"lifetime": String`
- `"type": Type`
- `"const": Object` where the object is the same as the `inner` field of `Item` when `kind == "constant"`

#### TypeBinding

Name       | Type     | Description
-----------|----------|----------------------------------------------------------------------------
`name`     | String   | The `Fn`'s parameter types for this argument.
`binding`  | Object   | Either `"equality": Type` or `"constraint": [GenericBound]`


### `kind = "generic"`

`"inner"'` is a String which is simply the name of a type parameter.

### `kind = "bare_function_decl"`
TODO

### `kind = "tuple"`

`"inner"` is a single list with the Types of each tuple item.

### `kind = "slice"`

`"inner"` is the Type the elements in the slice.

### `kind = "array"`

Name       | Type     | Description
-----------|----------|----------------------------------------------------------------------------
`type`     | [Type](#Type) | The Type of the elements in the array
`len`      | String   | The length of the array as a stringified expression.

### `kind = "impl_trait"`

`"inner"` is a single list of the [GenericBounds](#GenericBound) for this type.

### `kind = "never"`

Used to represent the `!` type, has no fields.

### `kind = "infer"`

Used to represent `_` in type parameters, has no fields.

### `kind = "raw_pointer"`

Name       | Type     | Description
-----------|----------|----------------------------------------------------------------------------
`mutable`  | bool     | Whether this is a `*mut` or just a `*`.
`type`     | [Type](#Type) | The Type that this pointer points at.

### `kind = "borrowed_ref"`

Name       | Type     | Description
-----------|----------|----------------------------------------------------------------------------
`lifetime` | String   | (*Optional*) The name of the lifetime parameter on this referece, if any.
`mutable`  | bool     | Whether this is a `&mut` or just a `&`.
`type`     | [Type](#Type) | The Type that this reference references.

### `kind = "qualified_path"`

When a type is qualified by a trait (`<Type as Trait>::Name`) or associated type (`T::Item` where `T: Iterator`).

Name       | Type     | Description
-----------|----------|----------------------------------------------------------------------------
`name`     | String   | The name at the end of the path (`"Name"` and `"Item"` in the examples above).
`self_type` | [Type](#Type) | The type being used as a trait (`Type` and `T` in the examples above).
`trait`    | [Type](#Type) | The trait that the path is on (`Trait` and `Iterator` in the examples above).

### Examples

Here are some function signatures with various types and their respective JSON representations:

#### Primitives
```rust
pub fn primitives(a: u32, b: (u32, u32), c: [u32], d: [u32; 5]) -> *mut u32 {}
```

```json
"decl": {
  "inputs": [
    [
      "a",
      {
        "kind": "primitive",
        "inner": "u32"
      }
    ],
    [
      "b",
      {
        "kind": "tuple",
        "inner": [
          {
            "kind": "primitive",
            "inner": "u32"
          },
          {
            "kind": "primitive",
            "inner": "u32"
          }
        ]
      }
    ],
    [
      "c",
      {
        "kind": "slice",
        "inner": {
          "kind": "primitive",
          "inner": "u32"
        }
      }
    ],
    [
      "d",
      {
        "kind": "array",
        "inner": {
          "type": {
            "kind": "primitive",
            "inner": "u32"
          },
          "len": "5"
        }
      }
    ]
  ],
  "output": {
    "kind": "raw_pointer",
    "inner": {
      "mutable": true,
      "type": {
        "kind": "primitive",
        "inner": "u32"
      }
    }
  }
}
```
#### References
```rust
pub fn references<'a>(a: &'a mut str) -> &'static MyType {}
```

```json
"decl": {
  "inputs": [
    [
      "a",
      {
        "kind": "borrowed_ref",
        "inner": {
          "lifetime": "'a",
          "mutable": true,
          "type": {
            "kind": "primitive",
            "inner": "str"
          }
        }
      }
    ]
  ],
  "output": {
    "kind": "borrowed_ref",
    "inner": {
      "lifetime": "'static",
      "mutable": false,
      "type": {
        "kind": "resolved_path",
        "inner": {
          "name": "String",
          "id": "5:4936",
          "args": {
            "angle_bracketed": {
              "args": [],
              "bindings": []
            }
          },
          "param_names": null,
          "is_generic": false
        }
      }
    }
  }
}
```
#### Generics
```rust
pub fn generics<T>(a: T, b: impl Iterator<Item = bool>) -> ! {}
```

```json
"decl": {
  "inputs": [
    [
      "a",
      {
        "kind": "generic",
        "inner": "T"
      }
    ],
    [
      "b",
      {
        "kind": "impl_trait",
        "inner": [
          {
            "trait_bound": {
              "trait": {
                "kind": "resolved_path",
                "inner": {
                  "name": "Iterator",
                  "id": "2:5000",
                  "args": {
                    "angle_bracketed": {
                      "args": [],
                      "bindings": [
                        {
                          "name": "Item",
                          "binding": {
                            "equality": {
                              "kind": "primitive",
                              "inner": "bool"
                            }
                          }
                        }
                      ]
                    }
                  },
                  "param_names": null,
                  "is_generic": false
                }
              },
              "generic_params": [],
              "modifier": "none"
            }
          }
        ]
      }
    ]
  ],
  "output": {
    "kind": "never"
  }
}
```
#### Generic Args
```rust
pub trait MyTrait<'a, T> {
    type Item;
    type Other;
}

pub fn generic_args<'a>(x: impl MyTrait<'a, i32, Item = u8, Other = f32>) {
    unimplemented!()
}
```

```json
"decl": {
  "inputs": [
    [
      "x",
      {
        "kind": "impl_trait",
        "inner": [
          {
            "trait_bound": {
              "trait": {
                "kind": "resolved_path",
                "inner": {
                  "name": "MyTrait",
                  "id": "0:11",
                  "args": {
                    "angle_bracketed": {
                      "args": [
                        {
                          "lifetime": "'a"
                        },
                        {
                          "type": {
                            "kind": "primitive",
                            "inner": "i32"
                          }
                        }
                      ],
                      "bindings": [
                        {
                          "name": "Item",
                          "binding": {
                            "equality": {
                              "kind": "primitive",
                              "inner": "u8"
                            }
                          }
                        },
                        {
                          "name": "Other",
                          "binding": {
                            "equality": {
                              "kind": "primitive",
                              "inner": "f32"
                            }
                          }
                        }
                      ]
                    }
                  },
                  "param_names": null,
                  "is_generic": false
                }
              },
              "generic_params": [],
              "modifier": "none"
            }
          }
        ]
      }
    ]
  ],
  "output": null
}
```

# Drawbacks
[drawbacks]: #drawbacks

- By supporting JSON output for `rustdoc`, we should consider how much it should mirror the
  internal structures used in `rustdoc` and in the compiler. Depending on how much we want to
  stabilize, we could accidentally stabilize the internal structures of `rustdoc`. We have tried
  to avoid this by introducing a mirror of `rustdoc`'s AST types which exposes as few compiler
  internals as possible by stringifying or not including certain fields.
- Adding JSON output adds *another* thing that must be kept up to date with language changes,
  and another thing for compiler contributors to potentially break with their changes.
  Hopefully this friction will be kept to the minimum because the JSON output doesn't need any
  complex rendering logic like the HTML one. All that is required for a new language item is
  adding an additional field to a struct.

# Alternatives
[alternatives]: #alternatives

- **Status quo.** Keep the HTML the way it is, and make users who want a machine-readable version of
  a crate parse it themselves. In the absence of an accepted JSON output, the `--output-format` flag
  in rustdoc remains deprecated and unused.
- **Alternate data format (XML, Bincode, CapnProto, etc).** JSON was selected for its ubiquity in
  available parsers, but selecting a different data format may provide benefits for file size,
  compressibility, speed of conversion, etc. Since the implementation will lean on serde then this
  may be a non-issue as it would be trivial to switch serialization formats.
- **Alternate data structure.** The proposed output very closely mirrors the internal `Clean` AST
  types in rustdoc. This simplifies the implementation but may not be the optimal structure for
  users. If there are significant improvements then a future RFC could provide the necessary
  refinements, potentially as another alternative output format if necessary.

# Prior art
[prior-art]: #prior-art

A handful of other languages and systems have documentation tools that output an intermediate
representation separate from the human-readable outputs:

- [ClangDoc] has the ability to output either rendered HTML, or tool consumable YAML.
- [PureScript] uses an intermediate JSON representation when publishing package information to their
  [Pursuit] directory. It's primarily used to generate documentation, but can also be used to
  generate `etags` files.
- [DartDoc] is in the process of implementing a JSON ouput.
- [Doxygen] has an option to generate an XML file with the code's information.
- [Haskell]'s documentation tool, [Haddock], can generate an intermediate representation used by the
  type search engine [Hoogle] to integrate documentation of several packages.
- [Kythe] is a "(mostly) language-agnostic" system for integrating documentation across several
  langauges. It features its own schema that code information can be translated into, that services
  can use to aggregate information about projects that span multiple languages.
- [GObject Introspection] has an intermediate XML representation called GIR that's used to create
  langauge bindings for GObject-based C libraries. While (at the time of this writing) it's not
  currently used to create documentation, it is a stated goal to use this information to document
  these libraries.

[ClangDoc]: https://clang.llvm.org/extra/clang-doc.html/
[PureScript]: http://www.purescript.org/
[Pursuit]: https://pursuit.purescript.org/
[DartDoc]: https://dart.dev/tools/dartdoc/
[Doxygen]: https://www.doxygen.nl/
[Haskell]: https://www.haskell.org/
[Haddock]: https://www.haskell.org/haddock/
[Hoogle]: https://www.haskell.org/hoogle/
[Kythe]: http://kythe.io/
[GObject Introspection]: https://gi.readthedocs.io/en/latest/

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- What is the stabilization story? As language features are added, this representation will need to
  be extended to accommodate it. As this will change the structure of the data, what does that mean
  for its consumers?
- How will users be able to manipulate the data? Is it a good idea to host a crate outside the
  compiler that contains the struct definitions for all the types that get serialized so that
  people could easily hack on the data without the compiler? Should that crate be the source of
  truth for those types and be depended on by librustdoc, or should it be a mirror that gets
  updated externally to reflect the changes to the copy in the compiler?
- How will intra-doc links be handled?
  - Supporting `struct.SomeStruct.html` style links seems infeasible since it would tie alternative
    front-ends to `rustdoc`'s file/folder format.
  - With the nightly [intra-rustdoc link syntax](https://github.com/rust-lang/rust/pull/47046) it's
    debatable whether we should resolve those to HTML links or leave that up to whatever consumes
    the JSON. Leaving them unresolved seems preferable but it would mean that consumers have to do
    markdown parsing to replace them with actual links.
  - In the case of items from the local crate vs external crates should the behavior be different?
  - If there's an `html_root_url` attribute/argument for an external crate should the behavior be
    different?

## Output structure questions

These aren't essential and could be deferred to a later RFC. The current implementation does
include spans, but doesn't do any of the other things mentioned here.

- Should we store `Span`s in the output even though we're not exporting the source itself like the
  HTML output does? If so is there a simple way to sanitize relative links to the files to avoid
  inconsistent output based on where `rustdoc` is invoked from. For example `rustdoc
  --output-format json /home/user/Downloads/project/crate/src/lib.rs` would include that absolute
  path in the spans, but it's probably preferable to have it just list the filename for single
  files or the path from the crate root for cargo projects.
- The proposed implementation exposes a strict subset of the information available to the HTML,
  backend: the `Clean` types for Items and some mappings from the `Cache`. Are there other
  mappings/info from elsewhere that would be helpful to expose to users?
- There are some items such as attributes that defer to compiler internal symbols in their `Clean`
  representations which would make them problematic to represent faithfully. Is it OK to simply
  stringify these and leave their handling up to the user?
- Should we specially handle `Deref` trait impls to make it easier for a struct to find the methods
  they can access from their deref target?
- Should we specially handle auto-traits? They can be included in the normal set of trait impls
  for each type but it clutters the output. Every time a user goes through the impls for a type
  they need to filter out those synthetic impls.
