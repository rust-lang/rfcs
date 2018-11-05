- Feature Name: `anonymous_variants`
- Start Date: 2018-11-02
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Add anonymous variant types, a natural anonymous parallel to enums much like tuples are an anonymous parallel to structs. 

This RFC is intentionally minimal to simplify implementation and reasoning about interactions, while remaining amenable to extensions through the ecosystem or through future proposals. 

# Motivation
[motivation]: #motivation

Much like tuples are an ad-hoc analog to structs, a number of proposals have been made for an ad-hoc analog to enums. Such types would be a boon to ergonomics, reducing the need to generate new single-purpose enums just to encompass the possible return types of functions, and would reduce duplication across the ecosystem by supplying a canonical sum type for general usage and for the ecosystem to add extension traits to. Some proposals would also allow for these types to be implicitly generated, for similar such types to be unified together, or for such types to automatically implement traits all of their constituent types do. 

However, with these extras comes complexity. RFCs of this kind generally describe their proposed types with extra features which would supply tangible improvements to Rust should they be implemented. However, none of these RFCs have actually been approved for implementation, and arguments against their approval include the complexity associated with these extra features as well as possible ambiguities and undesired interactions of the type, features and all, with the rest of the language. The general idea of anonymous enum-like types people can seem to get behind, but the extras are what get most proposals of this kind. 

This RFC differs from other RFCs in the same vein by deemphasizing ergonomic features, instead focusing on a simple base instantiation of the general idea of anonymous sum types which can be implemented and processed with relative ease, and which library writers and future RFCs can build on top of with relative ease. The RFC itself may be long, but most of this extra length is not used for more detail, but to explain the decisions behind the RFC and how it may be executed and utilized. 

Not to say that this feature won't be useful by itself. Even as described and without ecosystem extras, this feature can still be used reasonably nicely. Here it is used to combine the possible error types of a function with less boilerplate than a single-shot error enum, a commonly cited potential use of ad-hoc sum and algebraic union types: 
```
use std::rc::Weak;
use std::option::NoneError;
use std::num::ParseIntError;

fn multiple_errors(val: Weak<str>) -> Result<i64, (NoneError|ParseIntError)> {

    // If None, a Result error holding an anonymous variant for NoneError is
    // returned early
    let strref = Weak::upgrade(val).ok_or_else(|| <_>::0(NoneError))? 
    
    // If Err, a Result error holding an anonymous variant for ParseIntError
    // is returned
    let num = i64::from_str_radix(strref, 10_u32).or_else(<_>::1)?
    
    Result::Ok(num)
}
```

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Concise guide
------
Anonymous variant types are anonymous sum types, which mean, like enums, their possible values are several different variants. They are declared like tuples, but with vertical bars instead of commas. The variants are referred to for construction and matching by the anonymous variant type name or a placeholder for one, followed by two colons, followed by a (zero-indexed) number for the variant. Like enums, variant order in an anonymous variant type matters, and variants are not automatically combined together or rearranged. 
```
// Declare an anonymous variant type
// Associated items need to be wrapped in angle brackets, hence all the angle
// brackets around the type placeholders
let x = <(i32 | &str)>::0(1_i32);

// And then match on it 
match x {
    <(_ | _)>::0(val) => assert_eq!(val, 1_i32),
    <_>::1(_) => unreachable!("Value was set to the first variant")
};
```

Detailed user guide
------
Anonymous variant types are used in Rust as an ad-hoc type for a value which can have multiple variants. Much like enums are a named type for values which can take on many variants, each associated with an identifier, anonymous variant types are a type for values which can take on many variants, each identified by their position in an anonymous variant type. These variants are called anonymous variants, as they are identified by their position on an anonymous variant type. 

An anonymous variant type name consists of a parentheses-enclosed list of type names separated by vertical bars, and optionally followed by a trailing vertical bar. Anonymous variant type placeholders are named similarly, but using type placeholders as well as type names. Because of syntax restrictions and to make generic implementations over a practical subset of anonymous variant types feasible, there must be at least one variant, and each variant is associated with exactly one type. Use the never type `!` for a type with zero variants, the unit type `()` for the type of a variant that does not need its single field, and tuples for the type of a variant that wants to hold more than one field. These restrictions may be relaxed by future RFCs. 
```
// An anonymous variant type with two variants, one of f32 type, the other of
// i32 type. 
(f32 | i32)

// An anonymous variant type with two variants, one of unit type, the other of
// the (f32, f64) tuple type. 
(() | (f32, f64))

// An anonymous variant type with four variants. 
// The first variant is of 164 type. 
// The second variant is of unit type. 
// The third variant is of i64 type, and is distinct from the first variant. 
// The fourth and last variant is of (i64, f64) type. 
(i64 | () | i64 | (i64, f64))
```
The usage of separating and trailing commas within a matching pair of parentheses indicates a tuple. The usage of separating and trailing vertical bars within a matching pair of parentheses indicates an anonymous variant type. Having neither indicates that the type is merely enclosed in parentheses, while mixing the two is syntactically invalid (though may be made valid and assigned a meaning in the future). 
```
(f32) // f32
(f32,) // tuple consisting only of an f32
(f32|) // anonymous variant type whose only variant is of type f32
(f32,|) // syntax error
(f32|,) // syntax error
```
Much like an enum type's variants are associated with identifiers, an anonymous variant type's variants are associated with numbers in ascending order from zero. Variants of such a type are numbered in the order they were declared in the type, and each variant has a single field having the type declared in the variant. Variant construction and pattern matching on anonymous variant types acts just like it does on named enum types. The angle brackets are not new syntax, but signify that the type is used in an associated item path, in the same way that `(f32,)::clone((1.0_f32,))` is not well-formed and needs to be written `<(f32,)>::clone((1.0_f32,))`
```
let foo = <(i64 | () | i64 | (i64, f64))>::0(4_i64);
let bar = <(i64 | () | i64 | (i64, f64))>::3((-3_i64, 0.0_f64));
assert!(if let <(i64 | () | i64 | (i64, f64))>::0(k) = foo { 
    k == 4_i64 
} else { 
    false 
});
assert!(match bar {
    <(i64 | () | i64 | (i64, f64))>::3((a, b)) => a == -3_i64 && b == 0.0_f64,
    <(_ | _ | _ | _)>::2(_) => false, 
    <_>::1(_) => false, 
    _ => false
});
```
As a safeguard against confusion and ambiguity, however, anonymous variants cannot be represented by numerals alone, and must be path-specced by the anonymous variant type they are a variant of, or by a placeholder that can be inferred to unambiguously correspond to exactly one such type. The concrete type of any anonymous variant type value used must be unambiguously inferrable, just like with enums. 
```
// Will error as an invalid operation on a numeric type, for good reason
let _ = 0(4_i64);

// Can't infer number of variants or type of the variants except for variant 0
let _ = <_>::0(4_i64);

// Can't infer the type of variant 1
let _ = <(_ | _)>::0(4_i64);

// Can't infer the type of variant 1
let _: (i64 | _) = <(_ | _)>::0(4_i64);

// Variant 1 is of type i32
let _: (i64 | _) = <(_ | i32)>::0(4_i64);

// Variant 1 is of type i32
let _: (i64 | _) = <(_ | _)>::1(2_i32);
```
The behavior of anonymous variant types mirrors that of similarly defined enums in every semantic respect. 
```
// Their variants are fully formed functions. 
let _: fn((i64, f64)) -> ((i64, f64) | &str) = <((i64, f64) | &str)>::0;
let _: fn(&str) -> ((i64, f64) | &str) = ((i64, f64) | &str)::1;

// They share memory layout optimizations with enums. 
use std::mem::size_of;

assert_eq!(size_of::<Option<&str>>(), size_of::<(()|&str)>());

// They have discriminants which can be compared. 
use std::mem::discriminant;

let vdisc_a = discriminant(<(() | (i64, i64))>::1((3, 6)));
let vdisc_b = discriminant(<(() | (i64, i64))>::1((0, 0)));
assert_eq!(vdisc_a, vdisc_b);
```
All automatically implemented traits (both Rust-intrinsic and defined in source) are implemented for anonymous variant types if all variants of that type are of types that implement them. The following traits (all of which have opt-in derives for enums in the standard library) are also implemented in the standard library for anonymous variant types with up to 12 variants whose variant types all implement the respective trait (this may be extended to anonymous variant types with even more variants in the future): 
* Copy
* Clone
* Debug
* Hash
* PartialEq
* Eq
* PartialOrd
* Ord

There are no blanket implementations for other traits. It is up to crate developers to decide whether to implement their traits for anonymous variant types. Though it may seem limiting to give up the opportunity to create blanket impls for these types, it is not entirely clear how impls for other traits should work (unlike for the above listed traits, which have familiar automatic derives), and the functionality can be decided on and added later through methods associated with the anonymous variant types. 

Detailed developer guide
------

Given a list of types of length at least one, the name of the anonymous variant type of all of these types in order can be generated by outputting "(", then for each type in the list, outputting that type's name followed by "|", and then after all the types and vertical bars, outputting ")". If the list of types might be of zero length, finishing by outputting "!|)" instead of ")" will guarantee that the output type is essentially the same except for an extra, uninhabitable variant, which will guarantee that the type is well-formed even if an empty list of types is used as the input for generation of the name of the anonymous variant type. 

The variants of the type can similarly be automatically determined. The list of types from before can be reused, and indexed to determine the type of each variant to work with. The type's variants themselves correspond to numbers from 0 up to but not including the length of the list of types. To generate the variant name itself, one can output "<", then the full anonymous variant type name (or a placeholder), then ">::", then the variant number. 

Parsing such a type is similarly easy. An anonymous variant type names consists of a parentheses-enclosed vertical bar separated list of types, optionally with a trailing vertical bar. At the level of a token stream, which will group any nested anonymous variant types into a single token tree, the stream can be divided into potential type names at the vertical bars, but not including the bars themselves, the last one removed if it consists of exactly zero tokens (to account for trailing vertical bars), and the remaining token stream excerpts either subsequently parsed to see if each one corresponds to a type, or just kept as is if semantic analysis is not desired. 

Here is an example bringing all the above together, showing how one might automatically derive a trait for an anonymous variant type. Note that this is only a demonstration summarizing the above points, and it omits a number of checks and extras that a production-grade procedural macro would have, but it should work if a token group corresponding to a valid name of an anonymous variant type whose variants all implement Debug is passed into `debug_derive`. 
```
fn debug_derive(typename: Group) -> TokenStream {
    // Copy the full type name to a string
    let fulltype = typename.clone();
    // Count the number of token stream parts corresponding to a type
    let mut token_count = 0_u64;
    let mut type_count = 0_u64;
    for piece in typename.stream().into_iter() {
        match piece {
            TokenTree::Punct(p) if p.as_char() == '|' => {
                if token_count > 0 {
                    token_count = 0;
                    type_count += 1;
                } else {
                    panic!("Not a valid anonymous variant type");
                }
            },
            _ => {token_count += 1; }
        }
    }
    if token_count > 0 {
        type_count += 1;
    }
    // Map the valid indices to match arms
    let match_arms = (0..type_count)
        .flat_map(|index| (format!{"<_>:: {} (v) => f.write_fmt(
            format_args!(\"{{}}({{:?}})\", {}, v)
        ),", index, index}).parse::<TokenStream>().unwrap())
        .collect::<TokenStream>();
    // Put it all together
    (format!{"impl ::std::fmt::Debug for {} {{
        fn fmt(&self, f: &mut ::std::fmt::Formatter) -> 
            Result<(), ::std::fmt::Error> 
        {{
            match self {{ {} }}
        }}
    }}", fulltype, match_arms}).parse::<TokenStream>().unwrap()
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

A semi-formal description of the syntax of anonymous variant types is shown below, where "..." represents the current rust grammar, immediately before implementation of this RFC. 
```
; Type names
ty_name ::= ... | anon_varty_name ; 

anon_varty_name : "(" (ty_name "|")+ (ty_name "|"?)? ")" ;

; Type labels
ty_label ::= ... | anon_varty_label ;

anon_varty_label : "(" (ty_label "|")+ (ty_label "|"?)? ")" ;

; Function names
fn_name ::= ... | anon_varty_variant ;

anon_varty_variant : "<" anon_varty_label ">::" number ;

; Match patterns
match_pattern ::= ... | anon_varty_match ;

anon_varty_match : anon_varty_variant "(" match_pattern ")" 
```
The anonymous variant type syntax intentionally mirrors that of tuples, with vertical bars in place of commas. The requirement for a vertical bar in the declaration of an anonymous variant type is to distinguish it from a simple parentheses-enclosed type, and the requirement that commas and vertical bars not to be intermixed as separators within a pair of parentheses is to leave open future extensions that may use intermixing of commas and vertical bars within a pair of parentheses in a type context. For now, a parentheses-enclosed token tree in a type context can be identified eagerly as an anonymous variant type by the vertical bar, and once such a type is identified, recognition does not need to fall back to anything other than a syntax error. The matching angle brackets required in naming the type is not new syntax, but rather keeps consistency with associated item paths, which already require the matching angle brackets. 

The clause specifying that numerals are only recognized as anonymous variants if they are path-specced by a type name or placeholder is to prevent ambiguity between numeric literals and anonymous variants, which would otherwise be possible interpretations of numerals. Because numerals have a very strong association with numeric types, numerals by themselves should always remain numeric literals, rather than allowing the interpretation of numerals by themselves as anonymous variants. In any case, anonymous variant types are likely to be rarer than numbers, and prepending a number with `<_>::` or `<(_|_)>::` helps to clearly indicate that an anonymous variant is in usage. 

This clause also prevent unintuitive type inferences involving anonymous variant types if a program, as written, accidentally performs a function call on a numeric literal or variable assigned with one. In the below snippet, if the `1` could be interpreted as an anonymous variant, then the compiler would infer that `y` is of some anonymous variant type with at least two variants, but without any type information to indicate the types of the anonymous variants. Thus, a compiler diagnostic message would indicate that the below snippet is well-formed but ambiguous, and would indicate that `y` should have its type's constituent variant types named out. If the user simply wanted to perform numeric work and intended for the `1` to be a number, this error message would be terribly unintuitive. With the requirement for the path spec, the compiler could point out that a numeric type does not implement any of the `Fn*` traits, and suggest either modifying the first statement to prepend the numeral with a path spec, or modify the second statement to do a more sensible operation on the number, possibly multiplication. 
```
let y = 1;    // This should be of numeric type
let z = y(3); // This should not be interperable as an anonymous variant call
```
Anonymous variant type names can only be recognised in a type context. The naming of the anonymous variants requires the type to be enclosed by angled brackets and followed by two colons, a pattern which is already used to disambiguate that an item is associated with a type. The only difference here is that the items are numbered instead of named, but there exists no other context in which a number makes sense after a path spec, so ambiguity is avoided here. 

Even when this disambiguation is not available, such as in compiler error information, the use of vertical bars separating types with a pair of parentheses is a distinctive pattern outside of matches, and in match contexts, the user will most likely be trying to match against one of the variants of such a type, and the use of two colons followed by numerals is quite distinctive of the use of anonymous variant types. 

As mentioned above, the behavior of an anonymous variant type mirrors that of a similarly defined enum. This is to allow anonymous variant types to be built on top of existing machinery for enums, and share all the internal optimizations of Rust enums. The only implementation difference anticipated beyond parsing and type checking and inference is that if in the future, anonymous variant types are extended to allow different numbers of fields per variant, the functions for each anonymous variant may need the "rust-call" ABI as a workaround for a lack of variadics in Rust. 

# Drawbacks
[drawbacks]: #drawbacks

The addition of new syntax and semantics is always an additional weight on the compiler, and though the minimal design of this RFC tries to relieve this weight by allowing the reuse of existing parts of the compiler, such weight is a concern inherent in any RFC suggesting user-facing language features. And new features of the language will always be another corner of the language for anyone to learn, and for sufficiently inclined users to use to create convoluted code. In particular, this feature allows for the easy expression of sum types, which is by design, but does allow for hard-to-read and convoluted types, especially when nested. 

The proposed syntax, while systematic, is rigid and does not make normal usage particularly easy. In particular, the usage of any anonymous variant type variant always has a type placeholder surrounded by angle brackets, which in turn is followed by two colons, and the type inferencer expects all the anonymous variants to be fully unambiguous. The described syntax limits the expression of possible types to those that have at least one variant, and exactly one field per variant. 

The anonymous variant types themselves do not have properties that may be desired in a sum or algebraic union type, such as commutativity. Because variants are identified by their position within the anonymous variant type, switching the order of the variants is a breaking change, and numbers themselves aren't particularly indicative of what they represent. 

Because of coherence rules, not providing blanket implementations of user implementable traits on anonymous variant types by their stabilization will forever prevent adding such blanket trait implementations without causing breakage. 

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

What's the method behind the mad syntax? 
------

The syntax of anonymous variant types intentionally follows that of tuples. The name of the type is wrapped in parentheses, and consists of separated type names, just like in a tuple. The only syntactic difference is using separating and trailing vertical bars instead of separating and trailing commas. Vertical bars are associated with alternation in a match context, and I decided it would be fine to maintain this intuition in a type context. 

The use of numbers is similar; just like a tuple's fields are numbered in ascending order from zero, and each field has the type of that ordinal type within the tuple declaration, an anonymous variant type's variants are numbered in ascending order from zero, and each variant has the type of that ordinal type within the anonymous variant type declaration. The wrapping angle brackets and path spec are consistent with the use of an associated item's methods, like how one could refer to `(f32,)`'s clone method with `<(f32,)>::clone`. 

You might think that the syntax is unpleasant, and you'd be right in that regard. This is one of tradeoffs made to make the actual proposal in this RFC as lean as possible. Your local crates vendor should have crates which help with that once this RFC gets implemented. Alternatively, you can file new RFCs which enrich the types in this RFC with ergonomic improvements. 

But why all the sacrifice to be minimal? 
------

As previously mentioned, this RFC is one of many different proposed RFCs suggesting the inclusion of some form of anonymous sum or algebraic union type. Other RFCs are more fully featured, and specify more sugar and conveniences than this one does. However, such conveniences introduce additional complexity, and this complexity has been [cited before](https://github.com/rust-lang/rfcs/pull/1154#issuecomment-126780341) as a reason such RFCs are not approved for implementation. 

The minimization of complexity is the primary motivating factor of the design of this RFC, and the hope is that it will be simple enough to approve, and that the ecosystem and further RFCs will be able to flesh it out into a more fully formed ad-hoc sum type, or perhaps an algebraic union type, after implementation. 

So why the enum-like semantics, rather than something more intuitive? 
------

The choice of anonymous sum type for the proposed type is twofold. First, it allows for almost all the compiler machinery already used for enums to be reused for anonymous variant types. Enums have a whole bunch of compiler machinery dedicated to making them work and optimizing them, and duplicating much of that work just to give different semantics to a new family of types would be quite a bit for a proposal that aims to minimize implementation complexity. 

Second, such types have much simpler interactions with themselves and the rest of the type system. 

It may seem to be intuitive for (T|T) to be equivalent to T or (T|), or to forbid it, but there are a number of ways which a user may unwittingly create such a type, which would have to be treated as a special case. Perhaps the type was actually (U|V), where at one particular point, U and V both had the same type of T for a particular monomorphization. Perhaps the type was generated through codegen, and it happened that the user wanted to combine two errors that happened to have the same type. Perhaps the type is in generic code, written by a programmer expecting that in all cases the second case occurs at some point, so a refactoring which changes types in a seemingly unrelated part of code causes hangs because the first case is now catching all the values. 

(U|V) being equivalent to (V|U) would have similar problems: what if both U and V are T? One could specify that they want the variants the other way around by specifying (V|U), but how would one specify that they wanted the variants the other way around for (T|T)? Clearly, there are a number of details to consider for algebraic union types. 

Algebraic sum types are simple in comparison: (T|T) is separate from (T|) which in turn is separate from T, (U|V) is distinct from (V|U) (but can be converted with a simple shim function that also works for (T|T)), and the variants will stay distinct in generic code no matter which types are used for the variants. 

So why named rather than numbered variants? Aren't numbered variants more brittle and harder to use?
------

This comes down to the purpose of the new types. 

The point of the types is to relieve the boilerplate from writing a whole new enum and to allow the ecosystem to have a canonical family of sum types to focus on rather than having a number of mutually ununifiable ones. Having to type out the field names every time the type is used would defeat the whole point of not defining an enum, and there currently exists no syntax for placeholders for the names of variants, so to do that would impose extra burden on implementation. 

Not only that, because of the nature of Rust's generics, that same extra work for type placeholders would also have to be done on generics to make it possible to write implementations for some practical subset of the types, and not just have everyone write for what they decide on. This would lead to informal standards, which really should be formal, and fragmentation from disagreement about names, which would lead us right back to where we started. 

In contrast, having anonymous variant types be numbered allows for traits over a practical subset of anonymous variant types to be created: just expand a macro that creates blanket implementation up to a large number of variants. This is the same approach currently used to provide blanket implementations for a practical subset of tuples and fixed-sized arrays, and it works out well as a stopgap, if not ideally. 

And why just one field per variant?
------

The decision to restrict the proposed type to one field per variant was for similar reasons. Without it, a giant combinatorial explosion of types with varying numbers of fields per variant would abound, and it would be horribly impractical to implement traits for any more than a tiny fraction of them, meaning that once one had an anonymous variant type with a modest number of fields, they would be left without ecosystem help. As a side effect, this decision also allowed for the commas to be dispensed, which helps make the type easier to parse. However, multiple fields may be reintroduced into anonymous variant types in the future pending prerequisite groundwork, so to keep the addition of commas backwards-compatible, the anonymous variant type must be enclosed in parentheses in all contexts. 

What else could we do?
------

There are a number of other solutions which I found and were brought up to me during discussions, some of which are briefly described below. 

There's the option of doing nothing, a tried-and-true system that is Rust's current solution. However, without a common variant type to refer to for usage, the ecosystem has formed a number of replacement solutions, including some, such as [Either](https://docs.rs/futures/0.2.1/futures/future/enum.Either.html) and [Either](https://docs.rs/either/1.5.0/either/enum.Either.html), that have exactly the same semantics and purpose, but cannot be unified with each other without shims. Standard practice is to create a purpose-built enum for each enum purpose, which, while allowing for the crate maintainer to have full control over their types and not muddle them together, requires more than a bit of boilerplate to maintain. And general ecosystem solutions are heavier weight and have longer typenames than a language-level solution would.  

The next alternative are algebraic union types, which are the primary rival to algebraic sum types I found. Their semantics make it so that the type is implicitly flattened, and types deduplicated. I considered this too, but I decided against it because they would require substantial groundwork on the compiler to create, and would require at least one of type distinctness conditions, the ability to quantify over the constituent types generically, tolerating the potential for compile errors at a distance possibly very far removed from the place the types were declared, or tolerating the potential for confusing runtime semantics resulting from the first match arm of a match statement catching cases intended for other match arms. 

Also mentioned were named, rather than numbered, variants. While they are more ergonomic than numbered variants, the Rust groundwork for name placeholders and generifying over name is lacking, so that would have to be developed. And for a minimal proposal such as this one, it's going to be the ecosystem that's going to help develop the type into something more pleasant, so an eye was kept on how amenable this proposal was to ecosystem extensions when it was developed. And named variants unfortunately wouldn't work particularly well with the ecosystem without extra groundwork. 

During the design of this RFC, the idea of being able to define a type whose variants are a subset of those an another type's came up. This is a neat idea, and nicely parallels similar ideas to be able to have restricted-field views of a struct. However, it _is_ a new idea that came up recently, and it will have to be worked out into a full proposal. Such a proposal is orthogonal in functionality to this one, even if it has similar goals, and would fit alongside it comfortably as well as an alternative to it. 

# Prior art
[prior-art]: #prior-art

Ad-hoc sum and algebraic union types (as opposed to named sum types, which are implemented as enums) are a possible addition to Rust that has been rehashed repeatedly, much like higher-kinded types. And like higher-kinded types, they appear in many other languages, especially functional languages. They are sometimes cited as a possible solution to ergonomic problems involving handling any of a number of types in a relatively uniform manner, especially in error handling. 

Some crates have types which provide the functionality that an ad-hoc sum or algebraic union type would provide. The most fully-featured of these ecosystem solutions are [Frunk's Coproduct](https://docs.rs/frunk_core/0.2.1/frunk_core/coproduct/enum.Coproduct.html). The [Either crate](https://crates.io/crates/either) provides two-variant sum type for generic use, and counts among the top 100 most recently downloads crates as of writing. 

Some of the previous suggestions for ad-hoc sum or algebraic union types include (there are many more to find for those that are inclined to do so): 
* https://github.com/rust-lang/rfcs/pull/1154
* https://github.com/rust-lang/rfcs/issues/294
* https://internals.rust-lang.org/t/pre-rfc-anonymous-enums/5695
* https://github.com/rust-lang/rfcs/pull/402
* https://github.com/rust-lang/rfcs/pull/514
* https://github.com/rust-lang/rfcs/issues/2414

# Unresolved questions
[unresolved-questions]: #unresolved-questions

Though I have put effort into making sound judgments for this RFC, there's a chance that I may have made design mistakes, or that others may prefer different tradeoffs to mine. Some of the details which might change before or during implementation may include: 

* Should there be other blanket trait implementations on anonymous variant types? (In particular, perhaps Error should get a blanket implementation too, as this would aid what could be the anonymous variant type's most prominent use.) 
* Should there be contexts where a numeral can be interpreted as an anonymous variant without a path spec? 
* Should the syntax be more flexible or sugary? 
* Should the syntax use something other than parentheses-enclosed, bar-separated types? 

# Future possibilities
[future-possibilities]: #future-possibilities

This RFC is designed to lay out a groundwork for future additions to be applied later, so extras on top of it and relaxations of some of its restrictions, both immediately after implementation and far off in the future, are to be expected. Anonymous variant types, by virtue of their similar semantics, should also benefit from any features added to enums. In particular, unsizing on enums into a trait object implemented by all of its variant fields will also help anonymous variant types resolve their most noticeable deficiency: the inability to dispatch over an anonymous variant type as a whole. 

Some potential extensions of anonymous variant types themselves include: 

* The ability to specify zero, or more than one, fields in an anonymous variant
* The ability to refer to anonymous variants by name or type rather than by number
* Syntax sugar for anonymous variant usage in special or general cases
* Extra methods which implement more functionality on anonymous variant types
* Incorporation into variadic type proposals
