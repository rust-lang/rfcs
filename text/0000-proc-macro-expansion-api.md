- Feature Name: Macro Expansion API for Proc Macros
- Start Date: 2018-01-26
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Add an API for procedural macros to expand macro calls in token streams. This will allow proc macros to handle unexpanded macro calls that are passed as inputs, as well as allow proc macros to access the results of macro calls that they construct themselves.

# Motivation
[motivation]: #motivation

There are a few places where proc macros may encounter unexpanded macros in their input:

* In attribute and procedural macros:

    ```rust
    #[my_attr_macro(x = a_macro_call!(...))]
    //                  ^^^^^^^^^^^^^^^^^^
    // This call isn't expanded before being passed to `my_attr_macro`, and can't be
    // since attr macros are passed raw token streams by design.
    struct X {...}
    ```

    ```rust
    my_proc_macro!(concat!("hello", "world"));
    //             ^^^^^^^^^^^^^^^^^^^^^^^^^
    // This call isn't expanded before being passed to `my_proc_macro`, and can't be
    // since proc macros are passed raw token streams by design.
    ```

* In proc macros called with metavariables or token streams:

    ```rust
    macro_rules! m {
        ($($x:tt)*) => {
            my_proc_macro!($($x)*);
        },
    }

    m!(concat!("a", "b", "c"));
    // ^^^^^^^^^^^^^^^^^^^^^^
    // This call isn't expanded before being passed to `my_proc_macro`, and can't be
    // because `m!` is declared to take a token tree, not a parsed expression that we know
    // how to expand.
    ```

In these situations, proc macros need to either re-call the input macro call as part of their token output, or simply reject the input. If the proc macro needs to inspect the result of the macro call (for instance, to check or edit it, or to re-export a hygienic symbol defined in it), the author is currently unable to do so. This implies an additional place where a proc macro might encounter an unexpanded macro call, by _constructing_ it:

* In a proc macro definition:

    ```rust
    #[proc_macro]
    fn my_proc_macro(tokens: TokenStream) -> TokenStream {
        let token_args = extract_from(tokens);
    
        // These arguments are a token stream, but they will be passed to `another_macro!`
        // after being parsed as whatever `another_macro!` expects.
        //                                                  vvvvvvvvvv
        let other_tokens = some_other_crate::another_macro!(token_args);
        //                 ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
        // This call gets expanded into whatever `another_macro` expects to be expanded
        // as. There is currently no way to get the resulting tokens without requiring the
        // macro result to compile in the same crate as `my_proc_macro`.
        ...
    }
    ```

Giving proc macro authors the ability to handle these situations will allow proc macros to 'just work' in more contexts, and without surprising users who expect macro calls to interact well with more parts of the language. Additionally, supporting the 'proc macro definition' use case above allows proc macro authors to use macros from other crates _as macros_, rather than as proc macro definition functions.

As a side note, allowing macro calls in built-in attributes would solve a few outstanding issues (see [rust-lang/rust#18849](https://github.com/rust-lang/rust/issues/18849) for an example). 

An older motivation to allow macro calls in attributes was to get `#[doc(include_str!("path/to/doc.txt"))]` working, in order to provide an ergonomic way to keep documentation outside of Rust source files. This was eventually emulated by the accepted [RFC 1990](https://github.com/rust-lang/rfcs/pull/1990), indicating that macros in attributes could be used to solve problems at least important enough to go through the RFC process.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## Macro Calls in Procedural Macros

When implementing procedural macros you should account for the possibility that a user might provide a macro call in their input. For example, here's a silly proc macro that evaluates to the length of the string literal passed in.:

```rust
extern crate syn;
#[macro_use]
extern crate quote;

#[proc_macro]
fn string_length(tokens: TokenStream) -> TokenStream {
    let lit: syn::LitStr = syn::parse(tokens).unwrap();
    let len = str_lit.value().len();
    
    quote!(#len)
}
```

If you call `string_length!` with something obviously wrong, like `string_length!(struct X)`, you'll get a parser error when `unwrap` gets called, which is expected. But what do you think happens if you call `string_length!(stringify!(struct X))`?

It's reasonable to expect that `stringify!(struct X)` gets expanded and turned into a string literal `"struct X"`, before being passed to `string_length`. However, in order to give the most control to proc macro authors, Rust doesn't touch any of the ingoing tokens passed to a proc macro.

Thankfully, there's an easy solution: the proc macro API offered by the compiler has methods for constructing and expanding macro calls. The `syn` crate uses these methods to provide an alternative to `parse`, called `parse_expand`. As the name suggests, `parse_expand` parses the input token stream while expanding and parsing any encountered macro calls. Indeed, replacing `parse` with `parse_expand` in our definition of `string_length` means it will handle input like `stringify!(struct X)` exactly as expected.

## Macro Calls in Attribute Macros

Macro calls also show up in attribute macros. The situation is very similar to that of proc macros: `syn` offers `parse_meta_expand` in addition to `parse_meta`. This can be used to parse the attribute argument tokens, assuming your macro expects a normal meta-item and not some fancy custom token tree. For instance, the following behaves as expected:

```rust
#[proc_macro_attribute]
fn my_attr_macro(attr: TokenStream, body: TokenStream) -> TokenStream {
    let meta: Syn::Meta = syn::parse_meta_expand(attr).unwrap();
    ...
}
```

```rust
// Parses successfully: `my_attr_macro` behaves as though called with
// ``my_attr_macro(value = "Hello, world!")]
// struct X {...}
//                      vvvvvvvvvvvvvvvvvvvvvvvvvvvv
#[my_attr_macro(value = concat!("Hello, ", "world!"))]
struct X {...}

// Parses unsuccessfully: the normal Rust syntax for meta items expects
// a literal, not an expression.
//                      vvvvvvvvvvvvvvvvvvvvvvvvv
#[my_attr_macro(value = println!("Hello, world!"))]
struct Y {...}
```

Of course, even if your attribute macro _does_ use a fancy token syntax, you can still use `parse_expand` to handle any macro calls you encounter.

## API Overview

The full API provided by `proc_macro` defines a struct, `ExpansionBuilder`, with the following interface:

```rust
#[non_exhaustive]
enum ExpansionError {}

struct ExpansionBuilder {...};

impl ExpansionBuilder {
    pub fn new_proc(path: TokenStream, args: TokenStream) -> Self;
    
    pub fn new_attr(path: TokenStream, args: TokenStream, body: TokenStream) -> Self;
    
    pub fn expand(self) -> Result<TokenStream, ExpansionError>;
}
```

The functions `new_proc` and `new_attr` create a procedural macro call and an attribute macro call, respectively. Both expect `path` to parse as a [path](https://docs.rs/syn/0.12/syn/struct.Path.html) like `println` or `::std::println`. The compiler looks up `path` in the caller's scope (in the future, the scope of the spans of `path` will be used to resolve the macro definition, as part of expanding hygiene support).

The `args` tokens are passed as the main input to proc macros, and as the attribute input to attribute macros (the `things` in `#[my_attr_macro(things)]`). The `body` tokens are passed as the body input to attribute macros (the `struct Foo {...}` in `#[attr] struct Foo {...}`). Remember that the body of an attribute macro usually has any macro calls inside it expanded _before_ being passed to the attribute macro itself.

The method `expand` consumes the macro call, resolves the definition, applies it to the provided input in the configured expansion setting, and returns the resulting token tree or a failure diagnostic.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The proposed additions to the proc macro API in `proc_macro` are outlined above in the [API overview](#api-overview). Here we focus on technical challenges.

When a source file is parsed any `macro_rules!` and `macro` definitions get added to a definition map long before the first macro is expanded. Procedural macros currently need to live in a separate crate, and it seems they will for a while. This means that _in principle_ any macro call that would resolve in the caller's scope should be available to resolve at the time the proc macro is expanded.

Built-in macros already look more and more like proc macros (or at the very least could be massaged into acting like them), and so they can also be added to the definition map.

Since proc macros and `macro` definitions are relative-path-addressable, the proc macro call context needs to keep track of what the path was at the call site. I'm not sure if this information is available at expansion time, but are there any issues getting it?

## Future Work: same-crate proc macros

When proc macros are allowed to be defined in the same crate as other items, we should be able to transfer any solution to the problem of internal dependencies over to the expansion API. For example, imagine the following (single) crate:

```rust
fn helper(ts: TokenStream) -> TokenStream { ... }

#[proc_macro]
fn foo(ts: TokenStream) -> TokenStream {
    let helped_ts = helper(ts);
    ...
}

fn main() {
    foo!(bar);
}
```

To get same-crate proc macros working, we need to figure out how (or if) to allow `foo!` to use `helper`. Once we do, we've probably also solved a similar issue with respect to this expansion API:

```rust
#[macro_use]
extern crate cool_library;

#[proc_macro]
fn foo(ts: TokenStream) -> TokenStream { ... }

fn main() {
    cool_library::cool_macro!(foo!(bar));
}
```

Here, we need to solve a similar problem: if `cool_macro!` expands `foo!`, it needs to have access to an executable version of `foo!` despite it being defined in the current crate, similar to how `foo!` needs access to an executable version of `helper` in the previous example.

## Future Work: Hygiene

This iteration of the macro expansion API makes a few concessions to reduce its scope. We completely ignore hygiene for result generation or macro definition lookup. If a proc macro author wants to adjust the scope that a macro's expanded tokens live in, they'll have to do it manually. If an author wants to adjust the scope that a macro definition is resolved in, they're completely out of luck. In short, if `bar!` is part of the input of proc macro `foo!`, then when `foo!` expands `bar!` it will be treated as if it were called in the same context as `foo!` itself.

By keeping macro expansion behind a builder-style API, we hopefully keep open the possibility of adding any future scoping or hygiene related configuration. For instance, a previous version of this RFC discussed an `ExpansionBuilder::call_from(self, Span)` method for adjusting the scope that a macro was expanded in.

## Future Work: Macros Making Macros, Expansion Order

For now, we only guarantee that proc macros can expand macros defined at the top level syntactically (i.e. macros that aren't defined in the expansion of another macro). That is, we don't try to handle things like this:

```rust
macro a() {...}
 
macro b() {
    macro c() {...}
}
b!();
 
// `foo!` is a proc macro
foo! {
    macro bar(...);
    
    // `a!` and `b!` are available since they're defined at the top level.
    // `c!` isn't available since it's only defined in the expansion of another macro.
    // `bar!` isn't available since it's defined in this macro.
}
```

Handling `foo!` calling `c!` would require the `#[proc_macro]` signature to somehow allow a proc macro to "delay" its expansion until the definition of another macro was found (that is, the implementation of `foo!` needs to somehow notify the compiler to retry its expansion if the compiler finds a definitiion of `c!` as a result of another macro expansion). 

Handling `bar!` being expanded in `foo!` would require the ability to register definitions of macros with the compiler.

Both of these issues can be addressed, but would involve a substantial increase in the surface area of the proc macro API that isn't necessary for handling simple but common and useful cases.

# Drawbacks
[drawbacks]: #drawbacks

This proposal:

* Increases the API surface of `proc_macro` and any crate trying to emulate it. In fact, since it requires actually evaluating macro calls it isn't clear how a third-party crate like `proc_macro2` could even try to emulate it.

* Greatly increases the potential for hairy interactions between macro calls. This opens up more of the implementation to be buggy (that is, by restricting how macros can be expanded, we might keep implementation complexity in check).

* Relies on proc macro authors doing macro expansion. This might partition the macro ecosystem into expansion-ignoring (where input macro calls are essentially forbidden for any part of the input that needs to be inspected) and expansion-handling (where they work fine _as long as_ the proc macro author has used the expansion API correctly).

* Leads to frustrating corner-cases involving macro paths. For instance, consider the following:

    ```rust
    macro baz!(...);
    foo! {
        mod b {
            super::baz!();
        }
    }
    ```

    The caller of `foo!` probably imagines that `baz!` will be expanded within `b`, and so prepends the call with `super`. However, if `foo!` naively calls `parse_expand` with this input then `super::baz!` will fail to resolve because macro paths are resolved relative to the location of the call. Handling this would require `parse_expand` to track the path offset of its expansion, which is doable but adds complexity.

* Can't handle macros that are defined in the input, as discussed above.

# Rationale and alternatives
[alternatives]: #alternatives

The primary rationale is to make proc macros work more smoothly with other features of Rust - mainly other macros.

Recalling the examples listed in [Motivation](#motivation) above, a few but not all situations of proc macros receiving unexpanded macro calls could be avoided by changing the general 'hands off' attitude towards proc macros and attribute macros, and more aggressively parse and expand their inputs. This effectively bans macro calls as part of the input grammar, which seems drastic, and wouldn't handle cases of indirection via token tree (`$x:tt`) parameters.

We could encourage the creation of a 'macros for macro authors' crate with implementations of common macros - for instance, those in the standard library - and make it clear that macro support isn't guaranteed for arbitrary macro calls passed in to proc macros. This feels unsatisfying, since it fractures the macro ecosystem and leads to very indirect unexpected behaviour (for instance, if one proc macro uses a different macro expansion library than another, and they return different results). This also doesn't help address macro calls in built-in attributes.

# Unresolved questions
[unresolved]: #unresolved-questions

* Some of the future work discussed above would be more flexible with explicit access to something representing the compilation context, to more finely control what definitions are present or how they get looked up. How do we keep the API forward-compatible?

* This API allows for a first-pass solution to the problems listed in [Motivation](#motivation). Does it interfere with any known uses of proc macros? Does it prevent any existing techniques from working or cut off potential future ones?

* Are there any reasonable cases where someone can call a macro, but the resolution of that macro's path isn't possible until after expansion?
