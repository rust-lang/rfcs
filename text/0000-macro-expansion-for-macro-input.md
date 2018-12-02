- Feature Name: Macro expansion for macro input
- Start Date: 2018-01-26
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Add an API for procedural macros to expand macro calls in token streams. This will allow proc macros to handle unexpanded macro calls that are passed as inputs, as well as allow proc macros to access the results of macro calls that they construct themselves.

# Motivation

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
    pub fn my_proc_macro(tokens: TokenStream) -> TokenStream {
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

## Recursion in procedural macros

We're going to discuss a technique that doesn't get mentioned a lot when discussing procedural and attribute macros, which is _recursively calling macros_. If you've ever had a look at a fairly complicated declarative macro (otherwise known as "macros by example" that are defined with the `macro` keyword or `macro_rules!` special syntax), or had to implement one yourself, then you've probably encountered something like the recursion in [lazy-static](https://github.com/rust-lang-nursery/lazy-static.rs/blob/master/src/lib.rs). If you look at the implementation of the `lazy_static!` macro, you can see that it calls `__lazy_static_internal!`, which sometimes calls itself _and_ `lazy_static!`.

But recursion isn't just for declarative macros! Rust's macros are designed to be as flexible as possible for macro authors, which means the macro API is always pretty abstract: you take some tokens in, you put some tokens out. Sometimes, the easiest implementation of a procedural macro isn't to do all the work at once, but to do some of it now and the rest in another call to the same macro, after letting the compiler look at your intermediate tokens.

We're going to look at using recursive expansion to solve an issue you might encounter when you're writing a procedural macro: expanding macro calls in your input.

## Macro calls in macro input

When implementing a procedural or attribute macro you should account for the possibility that a user might provide a macro call in their input. As an example of where this might trip you up when writing a procedural macro, here's a silly one that evaluates to the length of the string literal passed in:

```rust
extern crate syn;
#[macro_use]
extern crate quote;

#[proc_macro]
pub fn string_length(tokens: TokenStream) -> TokenStream {
    let lit: syn::LitStr = syn::parse(tokens).unwrap();
    let len = str_lit.value().len();
    
    quote!(#len)
}
```

If you call `string_length!` with something obviously wrong, like `string_length!(struct X)`, you'll get a parser error when `unwrap` gets called, which is expected. But what do you think happens if you call `string_length!(stringify!(struct X))`?

It's reasonable to expect that `stringify!(struct X)` gets expanded and turned into a string literal `"struct X"`, before being passed to `string_length`. However, in order to give the most control to proc macro authors, Rust usually doesn't touch any of the ingoing tokens passed to a procedural macro.

A similar issue happens with attribute macros, but in this case there are two places you have to watch out: the attribute arguments, as well as the body. Consider this:

```rust
#[my_attr_macro(value = concat!("Hello, ", "world!"))]
mod whatever {
    procedural_macro_that_defines_a_struct! {
        ...
    }
}
```

If `#[my_attr_macro]` is expecting to see a struct inside of `mod whatever`, it's going to run into trouble when it sees that macro instead. The same happens with `concat!` in the attribute arguments: Rust doesn't look at the input tokens, so it doesn't even know there's a macro to expand!

## Macro expansion and eager marking

Similar to hygiene scopes and spans, a token also has an expansion scope. When a macro finishes expanding, if some of the produced tokens are marked for eager expansion, they get put in a new child expansion scope; any macros in the child scope will be expanded before any parent macros are.

Here's how we would use this to fix `string_length`:

```rust
#[proc_macro]
pub fn string_length(tokens: TokenStream) -> TokenStream {
    if let Ok(_) = syn::parse::<syn::Macro>(tokens) {
       let eager_tokens = syn::mark_eager(tokens);
       return quote!(string_length!(#eager_tokens));
    }

    // Carry on as before.
    let lit: syn::LitStr = ...;
}
```

Every token starts off in the 'top-level' expansion scope, which we'll call `S0`. After `string_length!(stringify!(struct X))` expands, the scopes look like this:

```rust
// Still in scope S0.
// vvvvvvvvvvvvvvv--------------------vv
   string_length!(stringify!(struct X));
//                ^^^^^^^^^^^^^^^^^^^^
// In a new child scope, S1.
```

Since the new recursive call to `string_length!` is wrapping a macro call in a child scope - the call to `stringify!` - the compiler will expand the child macro before expanding `string_length!` again. Success!

## Expanding expressions in an item

Importantly, the way that the compiler expands eager macro calls is by pretending that the surrounding macro call _doesn't exist_. This becomes relevant when we try and do the above trick for attribute macro arguments. Imagine we have:

```rust
#[my_attr_macro!(concat!("a", "b"))]
struct X;
```
Since the attribute and the body are all part of the macro call to `my_attr_macro!`, if `my_attr_macro!` marks `concat!` for eager expansion then the compiler will ignore everything else and try and expand this:

```rust
concat!("a", "b")
```

And will complain (rightly!) that `concat!` doesn't produce a valid top-level item declaration here. Since we know our attribute is wrapping an item, we can change what we eagerly expand to something like:

```rust
fn tmp() { concat!("a", "b"); }
```

This means that when `my_attr_macro!` is  expanded again, it'll see `fn tmp() { "ab"; }` and know to extract the `"ab"` to figure out what the macro expanded as. Having to handle this sort of thing gets annoying rather quickly, so `syn` provides eager-expanding utility macros like `expand_as_item!` which do this wrapping-expanding-extracting work for you.

# Reference-level explanation

Currently, the compiler performs the following process when doing macro expansion:
1. Collect calls and definitions.
2. For each call, if it can be uniquely resolved to a definition, expand it.
    * If no call can be expanded, report an error that the definitions can't be found.
    * Otherwise, go to step 1.

To adjust this process to allow opt-in eager expansion while handling issues like path resolution, it is sufficient to add a concept of an 'expansion scope', which does two things:
* It prevents a macro from expanding if it depends on another macro having finished expanding.
* It restricts access to potentially-temporary macro definitions.

### Scope creation

Every token starts off in a root expansion scope `S0`. When a macro expands in some scope `P`, if any of the output tokens are marked for eager expansion they are moved to a fresh scope `C`, which is a 'child' of `P`. See the `string_length` example above. 

### Expansion eligibility

We modify the compilers' call search to only include macro calls for which all of the following hold:
* All of the tokens of the call are in some scope `S`. As a consequence none of the tokens are in a child scope, since a token is only ever in a single scope.
* The tokens aren't surrounded by another macro call in `S`. This rules out 'inner' eager expansion, like here:
    ```rust
    // These tokens are all in scope `S`.
    //
    // `a!` is eligible, because it is entirely
    // in scope `S`.
    //
    // `b!` isn't eligible, because it is surrounded
    // by `a!`.
    a! {
        b! {}
    }
    ```

### Expansion and interpolation

When a macro in a child scope `C` is being expanded, any surrounding macro call syntax in the parent scope `P` is ignored. For an attribute macro, this includes the attribute syntax `#[name(...)]` or `#[name = ...]`, as well as any unmarked tokens in the body.

When a child scope `C` has no more expansions, the resulting tokens are interpolated to the parent scope `P`, tracking spans.

This means the following is weird, but works:

```rust
macro m() {
    struct X;
}

expands_some_input! {      //   Marked for expansion.
                           //   |
    foo! {                 // --|-+- Not marked
        mod a {            // <-+ |  for expansion.
            custom marker: // --|-+
            m!();          // <-+ |
        }                  // <-+ |
    }                      // ----+
}
```

During expansion, the compiler sees the following:

```rust
mod a {
    m!();
}
```

Which is successfully expanded. Since the compiler has tracked the span of the original call to `m!` within `expands_some_input`, once `m!` is expanded it can interpolate all the resulting macros back, and so after eager expansion the code looks like:

```rust
expands_some_input! {
    foo! {
        mod a {
            custom marker;
            struct X;
        }
    }
}
```

And `expands_some_input` is ready to be expanded again with its new arguments.

### Scopes and name resolution

When resolving macro definitions, we adjust which definitions can be used to resolve which macro calls. If a call is in scope `S`, then only definitions in `S` or a (potentially transitive) parent scope of `S` can be used to resolve the call. To see why this is necessary, consider:

```rust
b!();

expands_then_discards! {
    macro b() {}
}
```

After expansion, the call to `b!` remains in scope `S0` (the root scope), whereas the definition of `b!` is in a fresh child scope `S1`. Since `expands_then_discards!` won't keep the definition in its final expansion (or might _change_ the definition), letting the call resolve to the definition could result in unexpected behaviour.

The parent-scope resolution rule also allows more sophisticated 'temporary' resolution, like when a parent eager macro provides definitions for a child one:

```rust
eager_1! {
    mod a {
        pub macro m() {}
    }

    eager_2! {
        mod b {
            super::a::m!();
        }
    }
}
```

The definition of `m!` will be in a child scope `S1` of the root scope `S2`. The call of `m!` will be in a child scope `S2` of `S1`. Although the definition of `m!` might not be maintained once `eager_1!` finishes expanding, it _will_ be maintained _during_ its expansion - more specifically, for the duration of the expansion of `eager_2!`.

### Delayed resolution

In the current macro expansion process, unresolved macro calls get added to a 'waiting' queue. When a new macro definition is encountered, if it resolves an unresolved macro call then the call is moved to the _actual_ queue, where it will eventually be expanded.

We extend this concept to eager macros in the natural way, by keeping an unresolved waiting queue for each scope. A definition encountered in a scope `P` is eligible to resolve any calls in `P` or a (possibly transitive) child of `P`. Consider this:

```rust
eager_1! {
    non_eager! {
        macro m() {}
    }
    eager_2! {
        m!();
    }
}
```

Once `eager_2!` expands, `non_eager!` will be eligible to be expanded in scope `S1` and `m!` will be eligible to be expanded in `S2`. Since `m!` is currently unresolvable, it gets put on the `S2` waiting queue and `non_eager!` will be expanded instead. This provides the definition of `m!` in `S1`, which resolves the call in `S2`, and the expansion continues.

### Handling non-expanding attributes

Built-in attributes and custom derive attributes usually don't have expansion defintions. A macro author should be guaranteed that once an eager macro expansion step has completed, any attributes present are non-expanding.

# Drawbacks

This proposal:

* Commits the compiler to a particular (but loose) macro expansion order, as well as a (limited) way for users to position themselves within that order. What future plans does this interfere with? What potentially unintuitive expansion-order effects might this expose?
    * Parallel expansion has been brought up as a future improvement. The above specified expansion order blocks macro expansion on the expansion of any 'inner' marked macros, but doesn't specify any other orderings. Is this flexible enough?

# Rationale and alternatives

The primary rationale is to make procedural and attribute macros work more smoothly with other features of Rust - mainly other macros.

Recalling the examples listed in the [motivation](#motivation) above, a few but not all situations of proc macros receiving unexpanded macro calls could be avoided by changing the general 'hands off' attitude towards proc macros and attribute macros, and more aggressively parse and expand their inputs. This effectively bans macro calls as part of the input grammar, which seems drastic, and wouldn't handle cases of indirection via token tree (`$x:tt`) parameters.

We could encourage the creation of a 'macros for macro authors' crate with implementations of common macros - for instance, those in the standard library - and make it clear that macro support isn't guaranteed for arbitrary macro calls passed in to proc macros. This feels unsatisfying, since it fractures the macro ecosystem and leads to very indirect unexpected behaviour (for instance, one proc macro may use a different macro expansion library than another, and they might return different results). This also doesn't help address macro calls in built-in attributes.

# Unresolved questions

* How does this proposal affect expansion within the _body_ of an attribute macro call? Currently builtin macros like `#[cfg]` are special-cased to expand before things like `#[derive]`; can we unify this behaviour under the new system?

* This proposal tries to be as orthogonal as possible to questions about macro _hygiene_, but does the addition of expansion scopes add any issues?

* This proposal requires that some tokens contain extra semantic information similar to the existing `Span` API. Since that API (and its existence) is in a state of flux, details on what this 'I am a macro call that you need to expand!' idea may need to wait until those have settled.

* It isn't clear how to make the 'non-item macro being expanded by a macro in item position' situation ergonomic. We need to specify how a hypothetical proc macro utility like `expand_as_item!` would actually work, in particular how it gets the resulting tokens back to the author.
    * One possibility would be to allow macros to _anti-mark_ their output so that it gets lifted into the parent scope (and hence is ineligible for future expansion). Similar to other proposals to lift macro _hygiene_ scopes.

# Appendix A: Corner cases

Some fun examples, plus how this proposal would handle them.

### Paths from inside a macro to outside

Compiles: the call to `m!` is in a child scope to the definition.
```rust
macro m() {}

expands_input! {
    mod a {
        super::m!();
    }
}
```

### Paths within a macro

Compiles: the definitions and calls are in the same scope and resolvable in that scope.
```rust
expands_input! {
    mod a {
        pub macro ma() {}
        super::b::mb!();
    };

    mod b {
        pub macro mb() {}
        super::a::ma!();
    };
}
```

### Non-contiguous marked tokens

These both compile: the marked tokens are a syntactically valid item when the unmarked tokens are filtered out.
```rust
expands_untagged_input! {
    mod a {
        super::b::m!();
    }
    dont expand: foo bar;
    mod b {
        pub macro m() {};
    }
}
```
```rust
expands_untagged_input! {
    mod a {
        dont expand: m1!();
        m2!();
    }
}
```

### Paths within nested macros

Compiles: see [scopes and name resolution](#scopes-and-name-resolution) above.
```rust
expands_input! {
    mod a {
        pub macro x() {}
    }

    expands_input! {
        mod b {
            super::a::x!();
        }
    }
}
```
```rust
macro x{}

#[expands_body]
mod a {
    macro x() {}

    #[expands_body]
    mod b {
        super::x!();
    }
}
```

### Paths that disappear during expansion

Does not compile: see [scopes and name resolution](#scopes-and-name-resolution) above. 
```rust
#[deletes_everything]
macro m() {}

m!();
```

### Mutually-dependent expansions

Does not compile: each expansion will be in a distinct child scope of the root scope, so the mutually-dependent definitions won't resolve.
```rust
#[expands_body]
mod a {
    pub macro ma() {}
    super::b::mb!();
}

#[expands_body]
mod b {
    pub macro mb() {}
    super::a::ma!();
}
```

Does not compile: the definition will be ignored because it isn't marked by the attribute macro (and hence won't be included in the same scope as the call).
```rust
#[expands_args(m!())]
macro m() {}
```

Compiles: the definition and call will be in the same scope. TODO: is this unexpected or undesirable?
```rust
#[expands_args_and_body(m!())]
macro m() {}
```

### Delayed definitions

Compiles: see [delayed resolution](#delayed-resolution) above.
```rust
macro make($name:ident) {
    macro $name() {}
}

expands_input! {
    x!();
}

expands_input! {
    make!(x);
}
```

### Non-items at top level

Does not compile: the intermediate expansion is syntactically invalid, even though it _will_ be wrapped in an item syntax.
```rust
mod a {
    expands_input_but_then_wraps_it_in_an_item! {
        let x = "a";
    }
}
```
