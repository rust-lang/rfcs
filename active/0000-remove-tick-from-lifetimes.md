- Start Date: 2014-06-23
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Remove the `'` from lifetime parameters. Instead, infer whether each generic parameter is a lifetime or a type.

# Motivation

As punctuation, the `'` notation is visually noisy, and it turns out to be unnecessary for both the compiler and the reader. The compiler can infer whether each generic parameter is a lifetime or a type parameter from its use, and a reader can infer the identity of a generic parameter from its case.

`'` as a generic parameter comes from the ML syntactic tradition, which is quite different from that of C. In particular, the overloading of `'` to mean both characters and lifetimes can be jarring for anyone not used to OCaml or Perl 4. And mixing of ML-style `'` generics and C++-style `<>` generics is entirely without precedent.

This has been presented (by some members of the Chromium team) as a particularly egregious example:

    fn get_mut<'a>(&'a mut self) -> &'a mut T;

With the `'` dropped, this becomes significantly less noisy:

    fn get_mut<a>(&a mut self) -> &a mut T;

# Detailed design

We replace the `LIFETIME` token with the `IDENT` token in all productions in the grammar. This will affect type parameters, reference types, and labels.

When determining the semantics of a type declaration, the compiler looks for uses of each generic parameter within `<>` to ascertain its identity (type parameter or lifetime):

* If the generic parameter is bounded, it is obviously a type parameter.

* If the generic parameter is used after an `&`, it is a lifetime.

* If the generic parameter is used as a type, it is a type parameter.

* If the generic parameter is used in a lifetime position in a type's generic parameter, it is a lifetime parameter.

* Otherwise, the generic parameter is a type parameter. (Note that these rules mean that phantom—unused—parameters become type parameters.)

The labeled block syntax is unambiguous because we have solved the ambiguity between structure literals and labeled statements by syntactically restricting the positions in which structure literal expressions may be used per Rust PR #14885.

A lint pass will be added that warns if lifetimes do not begin with a lowercase letter or if type parameters do begin with a lowercase letter. This is important because code that does not maintain this style may mislead readers (though not the compiler).

# Drawbacks

* Syntax highlighters will have a more difficult time determining the difference between lifetimes and type parameters, because lifetimes will no longer constitute a distinct lexical class.

* It may be more difficult for casual readers to tell the difference between lifetimes and type parameters at a glance, because case is not as lexically distinctive as punctuation.

* Phantom lifetimes will no longer work, though they will continue to work via marker types.

* Loop labels will shadow variable names, and vice versa. This is fallout from the way the macro hygiene algorithm works.

# Alternatives

The impact of not doing this is that the drawbacks noted in "Motivation" above will persist.

# Unresolved questions

None.
