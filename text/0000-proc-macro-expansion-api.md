- Feature Name: Macro Expansion API for Proc Macros
- Start Date: 2018-01-26
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Add an API for procedural macros to expand macro calls in token streams. This will allow proc macros to handle unexpanded macro calls that are passed as inputs, as well as allow proc macros to access the results of macro calls that they construct themselves.

# Motivation
[motivation]: #motivation

There are a few places where proc macros may encounter unexpanded macros in their input even after [rust/pull/41029](https://github.com/rust-lang/rust/pull/41029) is merged:

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

It's reasonable to expect that `stringify!(struct X)` gets expanded and turned into a string literal `"struct X"`, before being passed to `string_length`. However, in order to give the most control to proc macro authors, Rust doesn't touch any of the ingoing tokens passed to a proc macro (**Note:** this doesn't strictly hold true for [proc _attribute_ macros](#macro-calls-in-attribute-macros)).

Thankfully, there's an easy solution: the proc macro API offered by the compiler has methods for constructing and expanding macro calls. The `syn` crate uses these methods to provide an alternative to `parse`, called `parse_expand`. As the name suggests, `parse_expand` parses the input token stream while expanding and parsing any encountered macro calls. Indeed, replacing `parse` with `parse_expand` in our definition of `string_length` means it will handle input like `stringify!(struct X)` exactly as expected.

As a utility, `parse_expand` uses sane expansion options for the most common case of macro calls in token stream inputs. It assumes:

* The called macro, as well as any identifiers in its arguments, is in scope at the macro call site.
* The called macro should behave as though it were expanded in the source location.

To understand what these assumptions mean, or how to expand a macro differently, check out the section on how [macro hygiene works](#spans-and-scopes) as well as the detailed [API overview](#api-overview).

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

**Note:** Because the built-in attribute 'macro' `#[cfg]` is expanded and evaluated before body tokens are sent to an attribute macro, the compiler will also expand any other macros before then too for consistency. For instance, here `my_attr_macro!` will see `field: u32` instead of a call to `type_macro!`:

```rust
macro_rules! type_macro {
    () => { u32 };
}

#[my_attr_macro(...)]
struct X {
    field: type_macro!(),
}
```

## Spans and Scopes
[guide-sshm]: guide-sshm

**Note:** This isn't part of the proposed changes, but is useful for setting up the language for understanding proc macro expansion.

If you're not familiar with how spans are used in token streams to track both line/column data and name resolution scopes, here is a refresher. Consider the following proc macro:

```rust
#[macro_use]
extern crate quote;

#[proc_macro]
fn my_hygienic_macro(tokens: TokenStream) -> TokenStream {
    quote! {            
        let mut x = 0;  // [Def]
        #tokens         // [Call]
        x += 1;         // [Def]
    }
}
```

Each token in a `TokenStream` has a span, and that span tracks where the token is treated as being created - you'll see why we keep on saying "treated as being created" rather than just "created" [later](#unhygienic-scopes)!

In the above code sample:

* The tokens in lines marked `[Def]` have spans with scopes that indicate they should be treated as though they were defined here in the definition of `my_hygienic_macro`.
* The tokens in lines marked with `[Call]` keep their original spans and scopes, which in this case indicate they should be treated as though they were defined at the macro call site, wherever that is.

Now let's see what happens when we use `my_hygienic_macro`:

```rust
fn main() {
    my_hygienic_macro! {
        let mut x = 1;
        x += 2;
    };
    println!(x);
}
```

After the call to `my_hygienic_macro!` in `main` is expanded, `main` looks something like this:

```rust
fn main() {
    let mut x = 0; // 1. [Def]
    let mut x = 1; // 2. [Call]
    x += 2;        // 3. [Call]
    x += 1;        // 4. [Def]
    println!(x);   // 5. [Call]
}
```

As you can see, the macro expansion has interleaved tokens provided by the caller (marked with `[Call]`) and tokens provided by the macro definition (marked with `[Def]`). 

Scopes are used to _resolve_ names. For example, in lines 3 and 5 the variable `x` is in the `[Call]` scope, and so will resolve to the variable declared in line 2. Similarly, in line 4 the variable `x` is in the `[Def]` scope, and so will resolve to the variable declared in line 1. Since the names in different _scopes_ resolve to different _variables_, this means mutating a variable in one scope doesn't mutate the variables in another, or shadow them, or interfere with name resolution. This is how Rust achieves macro hygiene!

This doesn't just stop at variable names. The above principles apply to mods, structs, trait definition, trait method calls, macros - anything with a name which needs to be looked up.

### Unhygienic Scopes

Importantly, macro hygiene is _optional_: since we can manipulate the spans on tokens, we can change how a variable is resolved. For example:

```rust
extern crate proc_macro;
#[macro_use]
extern crate quote;

use proc_macro::Span;

#[proc_macro]
fn my_unhygienic_macro(tokens: TokenStream) -> TokenStream {
    let hygienic = quote_spanned! { Span::def_site(),
        let mut x = 0; // [Def]
    };
    let unhygienic = quote_spanned! { Span::call_site(),
        x += 1;        // [Call]
    };
    quote! {
        #hygienic      // [Def]
        #tokens        // [Call]
        #unhygienic    // [Call]
    }
}
```

If we call `my_unhygienic_macro` instead of `my_hygienic_macro` in `main` as before, the result is:

```rust
fn main() {
    let mut x = 0; // 1. [Def]
    let mut x = 1; // 2. [Call], from main
    x += 2;        // 3. [Call], from main
    x += 1;        // 4. [Call], from my_unhygienic_macro
    println!(x);   // 5. [Call]
}
```

By changing the scope of the span of the tokens on line 4 (using `quote_spanned` instead of `quote`), that instance of `x` will resolve to the one defined on line 2 instead of line 1. In fact, the variable actually declared by our macro on line 1 is never used.

This trick has a few uses, such as 'exporting' a name to the caller of the macro. If hygiene was not optional, any new functions or modules you created in a macro would only be resolvable in the same macro.

There are also some interesting [examples](https://github.com/dtolnay/syn/blob/030787c71b4cfb2764bccbbd2bf0e8d8497d46ef/examples/heapsize2/heapsize_derive/src/lib.rs#L65) of how this gets used to resolve method calls on traits declared in `[Def]`, but called with variables from `[Call]`.

## API Overview

The full API provided by `proc_macro` and used by `syn` is more flexible than suggested by the use of `parse_expand` and `parse_meta_expand` above. To begin, `proc_macro` defines a struct, `MacroCall`, with the following interface:

```rust
struct MacroCall {...};

impl MacroCall {
    fn new_proc(path: TokenStream, args: TokenStream) -> Self;
    
    fn new_attr(path: TokenStream, args: TokenStream, body: TokenStream) -> Self;
    
    fn call_from(self, from: Span) -> Self;
    
    fn expand(self) -> Result<TokenStream, Diagnostic>;
}
```

The functions `new_proc` and `new_attr` create a procedural macro call and an attribute macro call, respectively. Both expect `path` to parse as a [path](https://docs.rs/syn/0.12/syn/struct.Path.html) like `println` or `::std::println`. The scope of the spans of `path` are used to resolve the macro definition. This is unlikely to work unless all the tokens have the same scope.

The `args` tokens are passed as the main input to proc macros, and as the attribute input to attribute macros (the `things` in `#[my_attr_macro(things)]`). The `body` tokens are passed as the body input to attribute macros (the `struct Foo {...}` in `#[attr] struct Foo {...}`). Remember that the body of an attribute macro usually has any macro calls inside it expanded _before_ being passed to the attribute macro itself.

The method `call_from` is a builder-pattern method to set what the calling scope is for the macro.

The method `expand` consumes the macro call, resolves the definition, applies it to the provided input in the configured expansion setting, and returns the resulting token tree or a failure diagnostic. For resolution:

* If the scope of `path` is anywhere other than that of `Span::def_site()`, then the macro definition is resolved in that scope.
* If the scope of `path` is that of `Span::def_site()`, then the macro definition is resolved in the crate defining the current macro (as opposed to being resolved using the imports in the token stream _produced by_ the current macro). This allows proc macros to expand macros from crates that aren't available to or provided by the caller.

### Calling Scopes

The method `call_from` sets the calling scope for the macro. What does this mean?

Say we are defining a macro `my_proc!` and want to use another macro `helper!` as part of `my_proc!`. If `helper!` is hygienic, then all of its new variables and modules and whatever will live in its own `[Def]` scope independent the `[Def]` scope of `my_proc!`.

If `helper!` is _unhygienic_ then any unhygienic declarations will live in the `[Call]` scope of `helper!` - but which scope is that? Assume that `helper!` expands to something like this:

```rust
struct S; // [Def]
struct T; // [Call]

// [Call]
//   v
impl T {
    // These implementation functions can refer to S because
    // they're in the same scope
    ... // [Def]
}
```

* If the `[Call]` scope of `helper!` is the `[Def]` scope of `my_proc!`, then `helper!` will 'export' or 'expose' the declaration of `T` to `my_proc!`, which lets `my_proc!` refer to `T`. This lets us delegate part of the implementation of `my_proc!` to other proc and decl macros (perhaps from other crates).

* If instead the `[Call]` scope of `helper!` is the `[Call]` scope of `my_proc!`, then `helper!` will export the declarations to the caller of `my_proc!` instead of `my_proc!`. If we don't need access to `T` and just want to export it straight to the caller of `my_proc!` (or if `helper!` is actually just part of the caller's input to `my_proc!`, like `my_proc!(helper!(...))`) then this is what we want.

Since both of these are legitimate use cases, `MacroCall` provides `call_from` to set what the `[Call]` scope of the macro call will be.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The proposed additions to the proc macro API in `proc_macro` are outlined above in the [API overview](#api-overview). Here we focus on technical challenges.

When a source file is parsed any `macro_rules!` and `macro` definitions get added to a definition map long before the first macro is expanded. Procedural macros currently need to live in a separate crate, and it seems they will for a while. This means that _in principle_ any macro call that would resolve in the caller's scope should be available to resolve at the time the proc macro is expanded.

Built-in macros already look more and more like proc macros (or at the very least could be massaged into acting like them), and so they can also be added to the definition map.

Since proc macros and `macro` definitions are relative-path-addressable, the proc macro call context needs to keep track of what the path was at the call site. I'm not sure if this information is available at expansion time, but are there any issues getting it?

# Drawbacks
[drawbacks]: #drawbacks

This proposal:

* Increases the API surface of `proc_macro` and any crate trying to emulate it. In fact, since it requires actually evaluating macro calls it isn't clear how a third-party crate like `proc_macro2` could even try to emulate it.

* Greatly increases the potential for hairy interactions between macro calls. This opens up more of the implementation to be buggy (that is, by restricting how macros can be expanded, we might keep implementation complexity in check).

* Relies on proc macros being in a separate crate, as discussed in the reference level explanation [above](#reference-level-explanation). This makes it harder to implement any future plans of letting proc macros be defined and used in the same crate.

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

* Can't handle macros that are defined in the input, such as:

    ```rust
    foo! {
        macro bar!(...);
        bar!(hello, world!);
    }
    ```

    Handling this would require adding more machinery to `proc_macro`, something along the lines of `add_definition(scope, path, tokens)`. Is this necessary for a minimum viable proposal? 

# Rationale and alternatives
[alternatives]: #alternatives

The primary rationale is to make proc macros work more smoothly with other features of Rust - mainly other macros.

Recalling the examples listed in [Motivation](#motivation) above, a few but not all situations of proc macros receiving unexpanded macro calls could be avoided by changing the general 'hands off' attitude towards proc macros and attribute macros, and more aggressively parse and expand their inputs. This effectively bans macro calls as part of the input grammar, which seems drastic, and wouldn't handle cases of indirection via token tree (`$x:tt`) parameters.

We could encourage the creation of a 'macros for macro authors' crate with implementations of common macros - for instance, those in the standard library - and make it clear that macro support isn't guaranteed for arbitrary macro calls passed in to proc macros. This feels unsatisfying, since it fractures the macro ecosystem and leads to very indirect unexpected behaviour (for instance, if one proc macro uses a different macro expansion library than another, and they return different results). This also doesn't help address macro calls in built-in attributes.

# Unresolved questions
[unresolved]: #unresolved-questions

The details of the `MacroCall` API need more thought and discussion:

* Do we need a separate configurable `Context` argument that specifies how scopes are resolved, combined with a `resolve_in(self, ctx: Context)` method?

* Is `call_from` necessary? Are there any known uses, or could it be emulated by patching the spans of the called macro result? Would this be better served with a more flexible API around getting and setting span parents?

* This API allows for a first-pass solution to the problems listed in [Motivation](#motivation). Does it interfere with any known uses of proc macros? Does it prevent any existing techniques from working or cut off potential future ones?

* Are there any reasonable cases where someone can call a macro, but the resolution of that macro's path isn't possible until after expansion?
