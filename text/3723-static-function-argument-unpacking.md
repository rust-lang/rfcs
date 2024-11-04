- Feature Name: `static_function_argument_unpacking`
- Start Date: 2024-10-30
- RFC PR: [rust-lang/rfcs#3723](https://github.com/rust-lang/rfcs/pull/3723)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC adds call-site unpacking of tuples, tuple structs, and fixed-size arrays, using `...expr` within the function call's parentheses as a shorthand for passing arguments. The full contents of these collections with known sizes are unpacked directly as the next arguments of a function call, desugaring into the corresponding element accesses during compilation.

# Motivation
[motivation]: #motivation

Argument unpacking reduces the verbosity and increases the ergonomics of Rust, it:

- Improves code writing ergonomics by removing the need for repetitive, unneeded intermediate steps.
- Allows more concise code in terms of number of lines and, occasionally, line length.
- Allows reducing the number of named local variables in scope.
- Is intuitive for developers accustomed to argument unpacking from other programming languages.
- Adds a missing piece to the family of certain kind of syntactic sugar already in Rust, with features such as *struct update syntax* and *destructuring assignment*.

Furthermore, argument unpacking provides groundwork for both the syntax and its intended use for possible next steps and related proposals: As long as compatibility is sufficiently considered, the proposed feature could also reduce the workload and scope of more general and ambitious initiatives, e.g. *variadic generics*, by iterating towards them in smaller steps. This may be a double-edged sword, however, as argued under [Drawbacks](#drawbacks).

# Guide-Level Explanation
[guide-level-explanation]: #guide-level-explanation

Instead of first taking items out of a collection and then immediately passing them on, one by one, to a function using the items, *argument unpacking* streamlines this operation by allowing the developer to directly forward the items to where they are used.

Consider ellipsis, i.e. the three dots, `...`, as a machine-readable shorthand for telling the compiler where to get the rest of the stuff from. The *where* part – written immediately after the ellipsis operator – is an expression that has a collection type with a size known during program compilation. This `...expr` is used within the parentheses of a function call to enter the items in the collection as arguments to the function being called.

The collection types that can be unpacked as arguments this way are *tuples*, *tuple structs*, and *fixed-size arrays*. Argument unpacking works on function, method, and closure calls, but not on macro invocations.

The types of the elements in the collection being unpacked, in the order in which they are in the collection, must be compatible with the next of the remaining function parameters being filled, i.e. function parameters that don't have an argument yet. The number of the elements may not exceed the number of the remaining unfilled parameter slots.

**As a rule of thumb: Argument unpacking would be valid, if, for a collection with length *n*, inside a function call's parentheses, all the collection's fields could currently be accessed manually, in order, with `.0`, `.1`, …, `.(n-1)` for tuples and tuple structs or with `[0]`, `[1]`, …, `[n-1]` for fixed-size arrays, entering each of the fields/elements using that syntax as arguments to consecutive parameter slots.**

Consider code with the following functions defined:
```rust
fn print_rgb(r: u8, g: u8, b: u8) {
    println!("r: {r}, g: {g}, b: {b}");
}

fn hex2rgb(hexcode: &str) -> (u8, u8, u8) {
    let r = u8::from_str_radix(&hexcode[1..3], 16).unwrap();
    let g = u8::from_str_radix(&hexcode[3..5], 16).unwrap();
    let b = u8::from_str_radix(&hexcode[5..7], 16).unwrap();
    (r, g, b)
}
```

Currently, to pass the output of `hex2rgb` to `print_rgb`, the return value of `hex2rgb` needs to be first stored as a local variable:
```rust
fn main() {
    // Store the result of `hex2rgb` and pass it to `print_rgb`:
    let rgb = hex2rgb("#123456");
    print_rgb(rgb.0, rgb.1, rgb.2);
}
```

or local variables:
```rust
fn main() {
    // Store the results of `hex2rgb` and pass them to `print_rgb`:
    let (r, g, b) = hex2rgb("#123456");
    print_rgb(r, g, b);
}
```

Whereas with *argument unpacking*, the intermediate step is skipped:
```rust
fn main() {
    // Unpack the expression into function arguments:
    print_rgb(...hex2rgb("#123456"));
}
```

# Reference-Level Explanation
[reference-level-explanation]: #reference-level-explanation

Syntactic sugar commonly known as argument unpacking is a zero-cost abstraction that shortens code related to passing of arguments in function, method, and closure calling.

## Scope of Planned Use

Argument unpacking works when calling **any** functions, methods, and closures that accept arguments. This is in contrast to some other programming languages that only allow unpacking arguments when the parameters of the function being called are named, variadic, positioned at the end of parameter list, or have default values. As tuple struct and tuple-like enum variant instantiations use the call expression, argument unpacking works on them too.

The scope of argument unpacking, for now, is restricted to compile-time context during which the number, types, and order of appearance of the unpacked arguments are known. To all intents and purposes, the proposed form of argument unpacking is infallible at run time. Infallibility is not a part of the specification – rather, it's a consequence of the restricted scope of this proposal; errors are prevented by the compiler rejecting scenarios that would not work.

This version of argument unpacking only affects:

- **Functions.** Only function, method, and closure calls are affected. Tuple struct and tuple-like enum variant instantiations are counted amongst them. Macro invocations are out of scope.
- **Call-site.** The feature is only about *argument* unpacking, not *parameter* packing or variadic functions.
- **Compile-time context.** Hence the word static in the RFC name. The feature is not about run-time behavior.
- **Provably successful situations.** The collection types usable for the feature are selected to make the use of the proposed feature infallible.

### Collections That Can Be Unpacked

Tuples, tuple structs, and fixed-size arrays can be unpacked. These collection types have a size known at compile time, and their elements have an unambiguous order, which allows the compiler to determine success. Other collections are out of the scope.

Structs with named fields also have a size known at compile time, but instead of an unambiguous order, they have unambiguously named fields. A design allowing unpacking of structs that, for example, matches these field names with parameter names is left under [Future Possibilities](#future-possibilities) due to difficult to solve questions.

## Syntax

Argument unpacking is syntactic sugar, causing it to expand to comma-separated field accesses exhaustively for the collection being unpacked. The order in which a function call's argument unpackings are desugared does not matter, since the same result follows from unpacking in any order.

Unary prefix ellipsis `...` symbol, i.e. three consecutive ASCII dot characters, followed by an expression is selected as the syntax for argument unpacking. The unpacking operator `...` has a high precedence, forcing the developer to explicitly use parentheses or braces with any complicated expressions following it. The syntax is limited to be used only within the function call parentheses.

In the function call, argument unpacking can occur at any comma-separated location in place of a conventional argument and the arguments coming after that until the collection has been completely unpacked. Argument unpacking can occur arbitrarily many times as well, as long as there are corresponding valid parameter slots left to pass the next arguments into.

For example, if a function is defined with three parameters and it is called with argument unpacking of a 2-tuple and one conventional argument, the unpacked 2-tuple at the first parameter slot consumes the first and the second slots, and the conventional argument goes to the third slot.

### Implementation of the Syntax Change

The function call syntax is modified to allow a comma-separated list of (argument unpacking OR expression) choices instead of expressions. Specifically, *CallParams* in the [call expressions](https://doc.rust-lang.org/reference/expressions/call-expr.html) and the [method-call expressions](https://doc.rust-lang.org/reference/expressions/method-call-expr.html) are changed in the following way:
- *CallParams* is defined as: *(Expression | `...`Expression) (`,` (Expression | `...`Expression))\*`,`?*.

## Unpacking Rules

Whether unpacking is successful is checked during compilation, and unsuccessful programs are rejected by the compiler, having the side effect that argument unpacking is infallible during run-time. Simply put, argument unpacking happens when it is desugared, as it gets replaced by the corresponding consecutive field accesses.

Nevertheless, some corollaries follow:

1. The order of the items in the collection is the same as the order in which they are unpacked.
2. Each item inside the collection is passed as an argument matching one parameter.
3. The types of the items in the collection must be compatible with the corresponding parameters.
4. All of the items inside the collection are unpacked.
    - For example, attempting to unpack a thousand-element array just to pass the first two elements as arguments to a function taking two parameters seems like a mistake that should be explicitly prevented.
    - Consequently, there must be at least as many unfilled parameter slots left in the function call as there are items inside the collection being unpacked. If there are *N* items in the collection being unpacked, the immediately next *N* parameter slots in the function call are filled with the collection's items as the arguments.

## Examples

The term *parameter slots* refers to the individual comma-separated places in a function call that each need to be filled by an argument for the function call to proceed.

As long as there are parameter slots remaining unfilled by arguments, filling them with arguments or by argument unpacking is allowed.

Assume the following definitions:
```rust
// Function `takes_five` has five parameter slots p0–p4.
fn takes_five(p0: u8, p1: u8, p2: u8, p3: u8, p4: u8) {}

fn main() {
    let tup1 = (2,);
    let tup2a = (0, 1);
    let tup2b = (3, 4);
    let tup3 = (1, 2, 3);
    let tup4 = (0, 1, 2, 3);
    let tup5 = (0, 1, 2, 3, 4);
    
    // ...
}
```

Provided that ultimately all parameter slots are filled with arguments, the function having more parameters than are being unpacked is a valid use-case, since unpacking always unpacks **everything** from the collection and fills the **next** of the remaining parameters. Unpacking can occur multiple times within the same function call as well. The example below demonstrates how unpacking arguments from collections of sizes *1–5* fills in the next *1–5* parameter slots of a function call. In the example, each of the function calls occurring in `main` lead to the same result.
```rust
// Parameter slots p0–p4 filled with values from `tup5`
takes_five(...tup5);

// p0–p3 filled with values from `tup4`, p4 filled with literal 4
takes_five(...tup4, 4);

// p0 and p4 filled with literals 0 and 4, respectively, and
// p1–p3 filled with values from `tup3`
takes_five(0, ...tup3, 4);

// p2 filled with literal 2; p0–p1 and p3–p4, respectively,
// filled with values from `tup2a` and `tup2b`
takes_five(...tup2a, 2, ...tup2b);

// p2 filled with the value from `tup1`; p0–p1 and p3–p4,
// respectively, filled with values from `tup2a` and `tup2b`
takes_five(...tup2a, ...tup1, ...tup2b);
```

Unpacking a collection of three items fills in as many consecutive parameter slots in a function call, starting from the slot it was defined in.
```rust
// At call-site, there seem to be three comma-separated places,
// while the function has five parameter slots
takes_five(0, ...tup3, 4)
    
// Desugared, the call above looks like this:
takes_five(0, tup3.0, tup3.1, tup3.2, 4);
//            ^^^^^^^^^^^^^^^^^^^^^^^
// The whole tuple of three fields was unpacked into the
// underlined parameter slots.
}
```

## Non-Trivial Cases

If, inside the call parentheses, the collection's fields can currently be accessed manually, in order, with `.idx`/`[idx]`, entering each as arguments to consecutive parameter slots, unpacking is valid.

### Empty Collections

When unpacking a unit tuple struct, the unit type, or an empty array, desugaring simply emits no field accesses. Essentially, the result is the same as if no arguments were passed at the point where a zero-length collection was unpacked.

Since this is dead code, a lint is emitted.

### Generic Parameters

When function parameters are generic, using `<T>`, `impl` or `dyn`, exactly the same should happen as when the arguments are passed by hand. I.e., the argument's type must be *compatible* with the parameter's type. Just as when entering that argument manually with a field access expression.

### References and Mutability in Function Parameters

Rust's current syntax for calling functions that take one parameter by value, by reference, or by mutable reference requires the developer to be explicit about what they want:
```rust
fn ret_one_arg() -> u8 {
    123
}

fn use_one_arg(p: u8) {
    println!("{p}");
}

fn use_one_refarg(p: &u8) {
    println!("{p}");
}

fn use_one_refmutarg(p: &mut u8) {
    println!("{p}");
}

fn main() {
    use_one_arg(ret_one_arg());
    use_one_refarg(&ret_one_arg()); // `&` needed to compile!
    use_one_refmutarg(&mut ret_one_arg()); // `&mut` needed to compile!
}
```

Rust also requires the developer to explicitly dereference references:
```rust
const CONST_NUMBER: u8 = 42;

fn ret_one_refarg() -> &'static u8 {
    &CONST_NUMBER
}

fn use_one_arg(p: u8) {
    println!("{p}");
}

fn use_one_refarg(p: &u8) {
    println!("{p}");
}

fn main() {
    use_one_arg(*ret_one_refarg()); // `*` needed to compile!
    use_one_refarg(ret_one_refarg());
}
```

Explicitly indicating varying degrees of (de)reference status or mutability on arguments being unpacked does not follow from the proposed syntax in any straightforward way. Thus, although it limits the usefulness of the feature, the design for such possibility is left out of the scope of this proposal. Consequently, the code will only compile if passing the arguments one by one with the corresponding field access expressions would compile.

### Type Coercions of Collections

If the collection being unpacked is a reference for the collection type, whether argument unpacking works, depends on if accessing it directly with the field access expression (`.idx`, or `[idx]`) would work at compile time. If it does, then argument unpacking works. (For the reference, see [`std::ops::Deref`](https://doc.rust-lang.org/std/ops/trait.Deref.html) and [type coercions](https://doc.rust-lang.org/reference/type-coercions.html).)

For example, the following will work, since the alternative works:
```rust
fn consume(a: u8, b: u8, c: u8) {
    println!("{a}, {b}, {c}");
}

fn main() {
    let tup = &(1, 2, 3);
    consume(...tup);
    // Alternative: consume(tup.0, tup.1, tup.2);
}
```

## Diagnostics

- Error: Attempt to pass the expression itself as an argument without unpacking it, if and only if the conditions that would allow argument unpacking are fulfilled.
    - Suggest refactor: Did you mean (same but with the unpacking syntax)?

- Error: Attempt to unpack an expression where a specific element/field is incorrect (e.g. has the wrong type).
    - Point out the incorrect field by underlining it, telling what it incorrectly is, and what is expected instead.

- Error: Attempt to unpack a slice, trait object, iterator, vector, or HashMap.
    - Note that fallible unpacking of Dynamically Sized Types is not supported.
    
- Error: Attempt to unpack a struct instance whose fields are visible at call-site.
    - Note that structs cannot be unpacked.

- Error: Attempt to unpack any other unpackable type.
    - Note that unpacking this type is not supported.

- Lint: When unnecessarily unpacking a collection that has zero items.
    - Note that unpacking collections with no items is dead code.
    - Suggest refactor: Remove the code.

- Lint: When unnecessarily unpacking a collection that has one item.
    - Suggest refactor: Pass the only value in the collection using the more explicit `.0`/`[0]` instead.

- Lint: When directly unpacking arguments from an expression could be done instead of exhaustively using temporary variables that are not used elsewhere or accessing the elements/fields by hand.
    - Suggest refactor: Use unpacking instead.

- Lint: When unnecessarily building a collection and unpacking that, e.g. passing `...(1, 2, 3)` instead of `1, 2, 3`.
    - Suggest refactor: Pass the arguments one by one instead of unpacking.

## Guide/Documentation Changes

The Rust Reference:

- The likely place to document argument unpacking would be under its own subheading in [Call expressions](https://doc.rust-lang.org/reference/expressions/call-expr.html). (For the reference, the similar [Functional update syntax](https://doc.rust-lang.org/reference/expressions/struct-expr.html#functional-update-syntax) is documented under *Struct expressions*.)

Standard library documentation that may benefit from mentioning the new syntax:

- Tuple structs: [stdlib struct keyword](https://doc.rust-lang.org/std/keyword.struct.html).
- Tuples: [stdlib tuple primitive](https://doc.rust-lang.org/std/primitive.tuple.html).
- Arrays: [stdlib array primitive](https://doc.rust-lang.org/std/primitive.array.html).

Various Rust books may want to mention or teach the feature. For example, The Rust Programming Language book's [Appendix B: Operators and Symbols](https://doc.rust-lang.org/book/appendix-02-operators.html) could include the syntax.

# Drawbacks
[drawbacks]: #drawbacks

Functions that accept many parameters may already be a code smell, and the proposed change would likely help calling such functions the most, becoming an enabler for anti-patterns. At the same time, unpacking three of four arguments by hand is not much work, which means that the impact of the change in normal code is not huge.

A sufficiently smart language server could automate argument unpacking, slightly decreasing the usefulness of having the feature in language itself when writing new code. However, there are many scenarios where a language server doesn't help, such as code examples in books.

Although the proposed syntax is familiar from other contexts, e.g. from other programming languages, it still burdens developers with additional syntax to understand. Possibly, depending on how intuitive the syntax is or how familiar the developer is with similar features from other programming languages, this may or may not imply an additional mental overhead when working with Rust code. However, as the new syntax comes in the form of syntactic sugar, this shouldn't be so bad: no-one is forced to use this even though they may be forced to understand this when reading code written by others. Additionally, it could be reasonably argued that the proposed change makes the language a bit more consistent, since a similar feature for struct instantiation already exists. Anecdotally, the author of this RFC tried to use the syntax for the proposed feature only to notice it doesn't exist yet.

Any initiatives for the distinct features of *named parameters*, *optional/variadic parameters*, *parameters with default values* or combinations thereof will need to consider the corresponding proposals' interactions with *argument unpacking*. The selected syntax of `...` will also be cemented to specific uses, possibly denying its use in some other contexts.

Ecosystem churn with MSRV *(Minimal Supported Rust Version)* bumps may be expected as some crate authors may decide to use argument unpacking in places where a workaround was previously used.

The ellipsis symbol composed from three consecutive ASCII dot characters is used in the "et cetera" or "and so on" sense in many design documents and code examples. Giving it an actual syntactical meaning could lead to some confusion or readability issues. Preferring `…`, i.e. the Unicode character U+2026, Horizontal Ellipsis, in those places could help.

# Rationale and Alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The supported use cases are limited to avoid problems that come with large scope; however, to help avoid metaphorical cul-de-sacs, i.e. incompatibilities with future features, some out-of-scope expansions are laid out under [Future Possibilities](future-possibilities).

Guiding principles in this design are:

- Familiarity of syntax.
- Compatibility with other features.
- Zero-cost – this is just syntactic sugar for passing the arguments by hand.
- Intuitiveness of use and the principle of least astonishment.
- Avoiding ambiguity with simple rules and by requiring explicit control by the user (developer).

Some programming languages such as JavaScript and PHP use an ellipsis prefix, `...`, as the syntax for a similar feature. Using this same syntax benefits inter-language consistency and familiarity for new users of Rust. There's an ongoing effort on [variadic generics](https://internals.rust-lang.org/t/variadic-generics-design-sketch/18974) proposing a `...` operator for unpacking in a compatible but wider setting than in this RFC.

Commonly, in other programming languages, the order in which the tokens appear is that inside the parentheses of a function call syntax, the collection to unpack the arguments from is **prefixed** by the symbol that is used for unpacking (e.g. `...` or `*`). Thus, the same prefix order has been selected in this RFC. One notable exception to this is Julia, in which argument unpacking – known as [splatting](https://docs.julialang.org/en/v1/manual/functions/#Varargs-Functions) – is performed with `f(args...)`.

## Other Terms

Of the known term alternatives, *argument unpacking* can be hypothesized as being a strong contender in intuitiveness for general programmer audience: the name makes it clear that the feature relates to *arguments*, and *unpacking* seems a somewhat typical operation that can be performed on a collection in a neat and orderly fashion.

Several names amongst programming languages and programmer lingo refer to argument unpacking or to a similar feature. Various terms include, in alphabetical order:
- *deconstruction*,
- *destructuring*,
- *expanding*,
- *exploding*,
- *scattering*,
- *splatting*,
- *spreading*,
- *unpacking*.

Probably, developers experienced in a specific programming language are most familiar with the term used for the feature in that programming language.

People sometimes mistakenly conflate arguments and parameters. Selecting a term that is unlikely to feed into that confusion is a plus.

It is also worth pointing out that many Rust users have a non-English background. Thus, consulting the dictionary entries for the term alternatives before committing to a specific selection may be prudent, if an understandability of the term is a priority.

## Different Syntax

Some programming languages (e.g. Python and Ruby) use the asterisk `*` character in place of the proposed `...`. In Rust, such syntax would be confusing, since it's already used for dereferencing. Table 1 collects alternatives for the symbol and its place in the syntax.

Table 1. Operator symbol alternatives.

|  Place | Operator | Notes                                                                                                |
|  :---: |   :---:  | ---------------------------------------------------------------------------------------------------- |
| Prefix | `...`    | **The proposed syntax.** Used already for [C-variadic functions](https://doc.rust-lang.org/beta/unstable-book/language-features/c-variadic.html). [Used](https://github.com/rust-lang/rfcs/pull/1192) in place of `..=` for inclusive ranges [previously](https://github.com/rust-lang/rust/issues/28237). Used in JavaScript and PHP for argument unpacking. |
| Suffix | `...`    | Used in Julia for argument unpacking.                                                                |
| Prefix | `...?`   | Used in Dart as null-aware spread operator.                                                          |
| Prefix | `..`     | Used already for *Functional Record Updates*. Clashes with `RangeTo<T>`.                             |
| Prefix | `*`      | Used in Python and Ruby for argument unpacking, in Kotlin for spreading. Clashes with dereferencing. |
| Prefix | `**`     | Used in Python for unpacking dictionaries into named keyword parameters.                             |
| Prefix | `@`      | At. Emphasis on _where_ the arguments come from. Used in PowerShell for splatting.                   |
| Prefix | `^`      | Connotation of "up and out". Used for `XOR` in binary contexts.                                      |
| Prefix | `~`      | Connotation of "inverting the collection inside-out", or C++ destructors.                            |

It is worth mentioning, that *variadic functions*, i.e. functions having a variable number of parameters, is a kind of mirror image of *argument unpacking* in the sense that it works on the opposite side of the function call: at the function definition. Often, programming languages that have both of these features also use the same symbol for both.

### Alternative Syntax of `..` Prefix

*Functional Record Updates* (i.e., *Struct Update Syntax*) already allow automatically filling fields when instantiating structs. This RFC recognizes the likeness of this feature with the proposed argument unpacking and treats them as belonging to the same family of syntactic sugar. Using similar syntax is tempting as well. However, adopting its current syntax of a `..` prefix would clash with `RangeTo<T>`'s syntactic sugar.

Inside struct instantiation braces is a parsing context with a comma-separated list of *{ field_name: expr, field_name: expr, ... }*, where **instead** of *field_name: expr*, the alternative *..other_struct* is allowed, i.e. `..expr` by itself is not a valid item, mitigating the clash with `RangeTo<T>` in *Functional Record Updates*. However, inside function call parentheses is a parsing context of *( expr, expr, ... )* – a comma-separated list of expressions (which this RFC proposes to change). As `..expr` is already itself a valid expression, producing a `RangeTo<T>`, some additional workaround would need to be designed to overcome the possible breakage from a syntax change.

For example, `..(1, 2, 3)` is valid syntax producing a `RangeTo<(u8, u8, u8)>`. More generally, `..expr` works for any type *T* emitted by `expr`, producing a `RangeTo<T>`.

For consistency, if `..expr` were selected for argument unpacking, the argument unpacking syntax could be favored and take precedence instead of the `RangeTo<T>` sugar. Conversely, if a developer actually wants to pass a constructed-on-the-fly-using-syntactic-sugar `RangeTo<T>` argument, it could be wrapped inside braces or parentheses: i.e. `{..expr}` or `(..expr)`. As this change in syntax would be a breaking change, it could be stabilized in the next edition.

## Implement a Smaller Subset

Aside from not implementing argument unpacking as proposed at all, some smaller but still useful subset of it could be implemented instead. For instance, the scope could be further restricted by only allowing unpacking
- of tuples,
- once in the function call at the end of the argument list,
- once in the function call without additional conventional arguments, or
- when the function has variadic parameters, if such feature is implemented.

### Limit to Unpacking Tuples

Tuples already look very much like argument lists, being a comma-separated list of items in parentheses. Special treatment of tuples as the sole arguments replacing the whole argument list has been requested a few times.

### Limit to Unpacking at End of Argument List

Other programming languages where argument unpacking is restricted such that it is only allowed at the end of argument list seem to do it for reasons connected with variadic/optional, named, or default-valued parameters.

### Limit to Unpacking Xor Conventional Arguments

Limiting function calls to have either use of conventional arguments or argument unpacking seems, superficially, a bit arbitrary. It is hard to come up with a good technical explanation it, other than perhaps it being simpler to implement.

### Limit to Unpacking into Variadic Parameter Slots

In other programming languages, limiting unpacking to only work with variadic parameter slots may be a natural or incidental consequence of a more variadic parameter centric approach, with less thought put into argument unpacking.

## Disallowing Unpacking Empty Collections

Disallowing unpacking of empty collections altogether could be argued for. In trivial cases it seems like an obvious mistake, for example when writing code such as:
```rust
fn foo(p: u8) {}

fn main() {
    let empty = ();
    foo(1, ...empty);
}
```

However, disallowing would also cause bad interactions with *const generics* and possible future implementations of *variadic generics*. For example, unpacking the output of function `f` below as the arguments for a variadic function with `0..n` parameters would unnecessarily cause an error, if unpacking of empty collections was disallowed:
```rust
const fn f<const S: usize>() -> [u8; S] {
    [0; S]
}
```

## General-Purpose Unpacking Operator

Instead of changing call expressions to accept choices of expressions OR `...`expressions, the operator `...` could be implemented generally, such that `...expr` as a whole becomes an expression. This could have more far-reaching consequences and interactions with other features, but could also help implement some future possibilities, such as unpacking in fixed-size arrays and tuple definitions.

## Give Macros More Control over Arguments

Empowering macros with new features might avoid new syntax.

Argument unpacking would follow naturally as a part of a more ambitious initiative of treating function argument places as distinct tokens accessible by macros. In Lisps, `apply`, and in Lua, the function `table.unpack` seem to superficially lead to the same result. An example of how this might look like in Rust:
```rust
fn main() {
    // Turns (u8, u8, u8) into three u8 arguments in the function call
    print_rgb(to_args!(hex2rgb("#123456")));
}
```

If [postfix macros](https://github.com/rust-lang/rfcs/pull/2442) are implemented, the following could be done instead:
```rust
fn main() {
    // Turns (u8, u8, u8) into three u8 arguments in the function call
    print_rgb.call_with_args!(hex2rgb("#123456"));
}
```

How, or if, multiple collections could be unpacked, possibly along with other, conventional arguments, would need further design.

One obvious downside with these approaches would be including another macro in `std`; including the macro in a separate external crate via the ecosystem could be done as a workaround, but the cost-to-benefit ratio of including another dependency may not make it worth it for some users.

## Capturing Arguments from the Variables in the Current Scope

A somewhat different design, allowing the use of bare `...` as a shorthand for passing variables in the current scope as arguments in the function call, would still make code shorter. Technically, this wouldn't conflict with the design proposed in this RFC. However, having two different but syntactically similar shorthands for functionality resembling each other might be confusing, which may be a reason to only commit to one or the other. Closures already capture the environment, and this would be similar in that the parameters with matching names would be filled with arguments. A downside in this approach would be that tracking what actually goes into the function becomes harder, and changes to variable names within the calling function's scope could make this approach error-prone. This approach would also have the problems related to exposing parameter names in the public API as described below for the future possibility of [unpacking structs](#unpacking-structs).

## Workarounds If RFC Is Not Implemented

### Unpack Tuples into Arguments with `fn_traits`

Instead of changing the language to include the syntactic sugar, a standard library method from the [`fn_traits`](https://doc.rust-lang.org/beta/unstable-book/library-features/fn-traits.html) feature could be used. A slightly more verbose example:
```rust
fn main() {
    std::ops::Fn::call(&print_rgb, hex2rgb("#123456"));
}
```
One downside of this is that the syntax diverges from a normal function call, i.e. superficially, the code seems to be calling `call`, with the actual function to be called being just one of the arguments. Given the verbosity and unfamiliar syntax compared to argument unpacking in other programming languages, this option also doesn't increase ergonomics that much. Relying on this might also confuse language servers when trying to locate uses of the called function. Directly unpacking tuple structs or fixed-size arrays isn't supported either, although `.into()` can be called on the latter. This does not work directly with methods either – extra work is required to call the *associated function* using fully qualified syntax and add `&self` to the beginning of the tuple, which is impractical if the tuple comes from a return value as in the provided example.

### Refactor the Callee

Another simple way to avoid the verbosity of having to pass the arguments in a collection by hand is to change the type signature of the function being called to accept the tuple/tuple struct/array instead. In some cases, defining the [function parameters as patterns](https://doc.rust-lang.org/stable/book/ch18-01-all-the-places-for-patterns.html#function-parameters) can be useful. For example, the callee can be refactored to accept a tuple instead:
```rust
fn print_rgb((r, g, b): (u8, u8, u8)) {
    println!("r: {r}, g: {g}, b: {b}");
}
```
and calling it simply becomes:
```rust
print_rgb(hex2rgb("#123456"));
```
There is no flexibility in accepting arguments one by one, but instead, a tuple must be constructed at the call site if passing single arguments is needed. Refactoring also is not always possible, for example if the function is in a 3rd party crate. In these cases it's possible to manually implement a wrapper for the 3rd party function.

# Prior Art
[prior-art]: #prior-art

## Argument Unpacking in Different Programming Languages

Python has [*argument unpacking*](https://docs.python.org/3.13/tutorial/controlflow.html#unpacking-argument-lists), (also see [subchapter Calls under Expressions](https://docs.python.org/3.13/reference/expressions.html#calls) in Python Language Reference) which allows using the `*` or `**` operators at call site to, respectively, extract values from tuples or dictionaries into distinct arguments.

The same example as in [Guide-Level Explanation](#guide-level-explanation), but implemented in Python:
```python
def print_rgb(r: int, g: int, b: int) -> None:
    print(f"r: {r}, g: {g}, b: {b}")

def hex2rgb(hexcode: str) -> tuple[int, int, int]:
    r = int(hexcode[1:3], 16)
    g = int(hexcode[3:5], 16)
    b = int(hexcode[5:7], 16)
    return r, g, b

if __name__ == "__main__":
    print_rgb(*hex2rgb("#123456"))
```

A related Python feature, packing of the parameters, is unrelated to this proposal and connected to the distinct concept of *variadic functions*. However, as it uses the same syntax in different context (function definition) as it uses for argument unpacking, it's worth mentioning as an example of how different programming languages may reuse the same syntax with argument unpacking and variadic functions.

Table 2 below summarizes argument unpacking in some other programming languages. Assume that in the examples of the syntax column, following that language's conventions:
- `sum_four` is a function accepting **four** distinct integer parameters to return their sum, and
- `nums` is a list- or tuple-like collection that contains **four** integer numbers.

Table 2. Non-exhaustive summary of argument unpacking in different programming languages.

| Language   | Term               | Syntax of Argument Unpacking   | Source(s)                                    |
| ---------- | ------------------ | ------------------------------ | -------------------------------------------- |
| Crystal    | Splatting          | `sum_four *nums`               | [Language reference][crystal]                |
| Dart       | Spread collections | N/A¹                           | [Feature specification][dart]                |
| JavaScript | Spread syntax      | `sum_four(...nums)`            | [MDN JavaScript Reference][js]               |
| Julia      | Splat              | `sum_four(nums...)`            | [Manual][julia]                              |
| Kotlin     | Spread operator    | `sum_four(*nums)`              | [Documentation][kotlin]                      |
| Lisp       | `apply`            | `(apply 'sum_four nums)`       | [Clojure], [Common Lisp][com-lisp], [Elisp], [Racket], [Scheme] |
| Lua        | `table.unpack`     | `sum_four(table.unpack(nums))` | [Manual][lua]                                |
| PHP        | Argument unpacking | `sum_four(...$nums)`           | [RFC][php-rfc], [mailing list][php-mail]     |
| PowerShell | Splatting          | `sum_four @nums`               | [Reference][powershell]                      |
| Python     | Argument unpacking | `sum_four(*nums)`              | [Tutorial][py-tut], [reference][py-ref]      |
| Ruby       | Splat operator     | `sum_four(*nums)`              | [Syntax documentation][ruby]                 |

¹ Dart uses the feature for inserting multiple elements into a collection.

[crystal]: <https://crystal-lang.org/reference/1.14/syntax_and_semantics/splats_and_tuples.html#splatting-a-tuple> "Crystal Language Formal Specification: Splatting a Tuple"
[dart]: <https://github.com/dart-lang/language/blob/9dc3737010f3ccac5ef54bf63b402d8e86b9115c/accepted/2.3/spread-collections/feature-specification.md> "Dart Feature Specification for Spread Collections"
[js]: <https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Operators/Spread_syntax> "MDN JavaScript Reference: Spread Syntax (...)"
[julia]: <https://docs.julialang.org/en/v1/manual/functions/#Varargs-Functions> "Julia Manual: Varargs Functions"
[kotlin]: <https://kotlinlang.org/docs/functions.html#variable-number-of-arguments-varargs> "Kotlin Documentation: Functions, Variable Number of Arguments (varargs)"
[Clojure]: <https://clojuredocs.org/clojure.core/apply> "Clojure Documentation: apply - clojure.core"
[com-lisp]: <https://lisp-docs.github.io/cl-language-reference/chap-5/f-d-dictionary/apply_function> "Common Lisp Docs - apply"
[Elisp]: <https://www.gnu.org/software/emacs/manual/html_node/elisp/Calling-Functions.html#index-apply> "GNU Emacs Lisp Reference Manual: Calling Functions - apply"
[Racket]: <https://docs.racket-lang.org/guide/application.html#%28part._apply%29> "The Racket Guide: Function Calls - The apply Function"
[Scheme]: <https://docs.scheme.org/schintro/schintro_69.html)> "An Introduction to Scheme and its Implementation - apply"
[lua]: <https://www.lua.org/manual/5.4/manual.html#pdf-table.unpack> "Lua Reference Manual - table.unpack"
[php-rfc]: <https://wiki.php.net/rfc/argument_unpacking> "PHP RFC: Argument Unpacking"
[php-mail]: <https://marc.info/?l=php-internals&m=137787624231187)> "php-internals Mailing List Discussion: \[RFC\] Argument Unpacking"
[powershell]: <https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_splatting> "PowerShell Reference: Splatting"
[py-tut]: <https://docs.python.org/3.13/tutorial/controlflow.html#unpacking-argument-lists> "The Python Tutorial: Unpacking Argument Lists"
[py-ref]: <https://docs.python.org/3.13/reference/expressions.html#calls> "The Python Language Reference: Expressions - Calls"
[ruby]: <https://docs.ruby-lang.org/en/3.3/syntax/calling_methods_rdoc.html#label-Array+to+Arguments+Conversion> "Ruby Syntax Documentation: Calling Methods - Array to Argument Conversion"

Haskell has no separate syntactic sugar for argument unpacking, but various `uncurryN` functions can be implemented, where `N` is the number of items in a tuple, e.g.:
```haskell
uncurry3 :: (a -> b -> c -> d) -> (a, b, c) -> d
uncurry3 f (a, b, c) = f a b c
```

### Notable Differences to Existing Implementations

For example, in Python, *fallible* unpacking occurs *dynamically*, at run time. Use cases, such as unpacking data structures created at run time with varying number of elements, are supported. On the other hand, whether unpacking can happen at all is not known until it is attempted during program execution, resulting to errors such as the following, when attempting something that wouldn't work:
```
TypeError: print_rgb() takes 3 positional arguments but 4 were given
```

The proposed feature in this RFC is different, only allowing unpacking when it is proven to succeed during compilation, marking it static. Consequently, it also makes the feature infallible.

### Ellipsis in Rust

The three ASCII dots syntax is already used for [C-variadic functions](https://doc.rust-lang.org/beta/unstable-book/language-features/c-variadic.html).

[Previously](https://github.com/rust-lang/rust/issues/28237), ellipsis [was](https://github.com/rust-lang/rfcs/pull/1192) used as syntax for inclusive ranges, i.e. in place of `..=`.

### Use of Ellipsis in Different Programming Languages

The ellipsis syntax is also used for features other than argument unpacking. E.g., C++ uses ellipsis suffix for [Pack expansion](https://en.cppreference.com/w/cpp/language/parameter_pack#Pack_expansion).

## Existing Rust Work on Subject

### Specifically on Argument Unpacking

Rust Internals:
- [Pre-RFC v2: Static Function Argument Unpacking](https://internals.rust-lang.org/t/pre-rfc-v2-static-function-argument-unpacking/21732), the Pre-RFC for this RFC.
- [Pre-RFC: Static Function Argument Unpacking](https://internals.rust-lang.org/t/pre-rfc-static-function-argument-unpacking/20770), an older version of the above Pre-RFC.
- [Tuple Unpacking](https://internals.rust-lang.org/t/tuple-unpacking/14688)

Stack Overflow questions:
- [Is it possible to unpack a tuple into function arguments?](https://stackoverflow.com/questions/39878382/is-it-possible-to-unpack-a-tuple-into-function-arguments)
- [Is it possible to unpack a tuple into method arguments?](https://stackoverflow.com/questions/60381690/is-it-possible-to-unpack-a-tuple-into-method-arguments)

### Unpacking Arrays

Rust Internals:
- [Pre-RFC: Array expansion syntax](https://internals.rust-lang.org/t/pre-rfc-array-expansion-syntax/13490)

### Using Tuples in Place of Argument Lists

Rust GitHub:
- [Language feature: flat tuple as function arguments](https://github.com/rust-lang/rfcs/issues/2667)

Rust Zulip t-lang:
- [t-lang \> Function call tuple destructuring](https://rust-lang.zulipchat.com/#narrow/channel/213817-t-lang/topic/Function.20call.20tuple.20destructuring)

Rust Users Forum:
- [Why can’t we map tuples on arguments in Rust?](https://users.rust-lang.org/t/why-cant-we-map-tuples-on-arguments-in-rust/15628)

### Unpacking Structs

Rust Internals:
- [Exploding structs](https://internals.rust-lang.org/t/exploding-structs/13884)

Rust Users Forum:
- [Unpacking struct members simultaneously](https://users.rust-lang.org/t/unpacking-struct-members-simultenously/8736)

### Other Related

On variadic generics in general: This might in the end solve the same problem along with many others as well. Variadic generics is a more ambitious feature with a large design space, and the design progress seems to have been ongoing since 2014. Meanwhile, argument unpacking essentially provides a subset of the consequences of variadic generics designs seen so far.

Rust GitHub:
- [Draft RFC: variadic generics](https://github.com/rust-lang/rfcs/issues/376)
- [RFC 2909: destructuring_assignment](https://github.com/rust-lang/rfcs/blob/master/text/2909-destructuring-assignment.md)

Rust Internals:
- [Variadic generics design sketch](https://internals.rust-lang.org/t/variadic-generics-design-sketch/18974)

# Unresolved Questions
[unresolved-questions]: #unresolved-questions

- Should argument unpacking desugar into Alternative A or Alternative B below, or does it make any difference?
    
    Alternative A:
    ```rust
    let tup = (1, 2, 3);
    
    // callee_fn(...tup); desugars into:
    {
        let _tmp0 = tup.0;
        let _tmp1 = tup.1;
        let _tmp2 = tup.2;
        callee_fn(_tmp0, _tmp1, _tmp2);
    }
    ```
    
    Alternative B:
    ```rust
    let tup = (1, 2, 3);
    
    // callee_fn(...tup); desugars into:
    callee_fn(tup.0, tup.1, tup.2);
    ```

- What happens when unpacking an empty collection as the "arguments" for a function that doesn't define parameters?
    - Probably nothing, at least if functions without parameters are not special-cased. Experimenting with this will be easier with a proof-of-concept implementation.

# Future Possibilities
[future-possibilities]: #future-possibilities

Future RFCs should freely venture outside the scope of this RFC and complement or build on this limited form of argument unpacking.

## Unpacking in Fixed-Size Array and Tuple Literals

The same ellipsis syntax with a very similar meaning could be adopted to defining fixed-size arrays and tuple literals as well. For example:
```rust
const CDE: [char; 3] = ['C', 'D', 'E'];
const ABCDEFG1: [char; 7] = ['A', 'B', ...CDE, 'F', 'G'];
const ABCDEFG2: [char; 7] = ['A', 'B', CDE[0], CDE[1], CDE[2], 'F', 'G'];

assert_eq(ABCDEFG1, ABCDEFG2);
```

At least Python and JavaScript have this feature.

Notably, tuple structs can already be built with the adoption of argument unpacking in general, since their constructors use the call expression that is modified by the addition of argument unpacking.

Prior work on designing such feature exists at least in the Rust Internals thread [Pre-RFC: Array expansion syntax](https://internals.rust-lang.org/t/pre-rfc-array-expansion-syntax/13490).

## Assigning Automatic Reference Status or Mutability

Technically, during argument unpacking, it's possible to automatically assign varying degrees of (de)reference status or mutability such that code compiles. The following trivial code could most likely be inferred by the compiler in a way that it would compile successfully, for example:
```rust
fn create_collection() -> (u8, u8, u8) {
    (1, 2, 3)
}

fn consume(a: u8, b: &u8, c: &mut u8) {
    *c = a + b;
}

fn main() {
    // consume(...create_collection()); desugars to:
    {
        let (_tmp0, _tmp1, mut _tmp2) = create_collection();
        consume(_tmp0, &_tmp1, &mut _tmp2);
    }
}
```

Further specification of automatically fitting argument unpacking to the reference or mutability status of the parameters in the function being called would merit a separate RFC.

## Unpacking Arguments for Macro Invocations

Macros, callable with the `macro_name!(...)` syntax have been omitted from the scope of this proposal. The only reason for omission is the time concerns related to doing the due diligence researching the differences in design. For example, some macros (e.g. `println!`) accept an indefinite number of arguments. Unpacking tuples, tuple structs, and fixed-size arrays probably would make sense – after all, argument unpacking is only syntactic sugar for something that can be done already in the desugared form. Further design, meriting a separate RFC amending this one, is needed.

## Unpacking Structs with Named Fields as Arguments

Unpacking structs with named fields has been omitted as well. Although a design where the struct's field names are unpacked as the arguments, provided that the types are compatible, to the correspondingly named parameters could be considered, there are major concerns in this design related to API guarantees and the current lack thereof regarding function parameter naming: If a function in crate A is used such that at call-site, in crate B, its parameters are filled with arguments that were unpacked from correspondingly named struct fields, a **major** version bump would be required for crate A to prevent a Semantic Versioning violation in crate B from refusing to compile if parameter names in the function in crate A are changed. Currently, since the parameter names don't have an effect at the user's side, such change can be made with a patch version bump. Essentially, allowing name-based matching of struct fields to parameters would introduce parameter names as part of the public API of libraries.

To support future work on unpacking structs, an opt-in attribute that declares a function's parameter's name as stable could be considered. This could unlock other possibilities related to argument unpacking as well, for example, overriding an argument that was already unpacked by explicitly using a named argument for it, after unpacking.

Another aspect to consider could be introducing a `#[derive]`able trait for structs, allowing them to be unpacked in the field to parameter name fashion.

### Sketch of Unpacking Structs

It may be important to give this some thought before accepting any argument unpacking rules whatsoever. The reason is that *if* the unpacking of structs is seen as an actual future possibility, we wouldn't want to introduce rules that are incompatible. Importantly, the design space has some notable overlap with another future possibility described below: [fallible run-time unpacking](#fallible-runtime-unpacking-of-dynamic-collections) of, e.g. `HashMap`s.

The basic idea in unpacking structs could be to match the struct's field names with the called function's parameter names. Some rules can already be thought of:
- If unpacking a struct with the exactly named fields, the order of the struct's fields vis-à-vis the arguments doesn't matter. Just pass the struct fields as the correspondingly named parameters.
- The struct fields need to be visible at call-site, e.g. `pub` or `pub(crate)`.
- Attempting to unpack a struct with named fields, where the number and types of fields match, but the names are different, is rejected.
    - Technically, under certain circumstances, it would be possible to emit syntactically correct code from the sugar, but the motivation is ambiguous. Therefore, it's better to leave it up to the developer to decide what is it that they want to accomplish.
    - Also, it's difficult to specify what would happen when there are multiple arguments of the same type: What should the order be when the names don't match? What would happen if one of the struct's fields was renamed into one of the parameter names?

However, several unresolved questions when unpacking structs would need to be considered as well:
- What to do when unpacking structs with named fields into macro invocation's arguments?
- How does unpacking more than one struct work?
- How does unpacking structs combine with passing conventional arguments?
- How does unpacking structs combine with unpacking tuples, tuple structs, and fixed-size arrays?
- How to reconcile trait methods having differently named parameters?

## Fallible Runtime Unpacking of Dynamic Collections

The scope of argument unpacking could be expanded to dynamic contexts as well. Runtime unpacking of `dyn Trait` trait objects, slices, `Vec`s, `HashMap`s, iterators in general etc. would be fallible, since the existence of a correct number, order, typing and naming of items to match the parameters can't be guaranteed at compile time. A syntax such as `...expr?` or `...?expr` could be considered to improve ergonomics of argument passing for those cases as well, but that would definitely merit a separate RFC.

Possibly, this would involve an stdlib trait, e.g. `TryArgUnpack`, whose implementation the language would use to get the arguments. This would enable unpacking custom collections as well.

## Syntax Change for Functional Record Updates

Adopting the `...expr` syntax for argument unpacking means that it is now part of the general "take stuff from here and expand it" family of syntactic sugar. As Rust already uses the `..` syntax for *Functional Record Updates*, changing that to use an ellipsis instead would be congruent.

Comments in the first pre-RFC thread suggest that the specific way the *Functional Record Updates* feature is currently implemented, syntax-wise, has a mixed community appraisal.
