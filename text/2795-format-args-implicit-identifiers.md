- Feature Name: `format_args_implicits`
- Start Date: 2019-10-27
- RFC PR: [rust-lang/rfcs#2795](https://github.com/rust-lang/rfcs/pull/2795)
- Rust Issue: [rust-lang/rust#67984](https://github.com/rust-lang/rust/issues/67984)


# Summary
[summary]: #summary

Add implicit named arguments to `std::format_args!`, inferred from the format string literal.

This would result in downstream macros based on `format_args!` to accept implicit named arguments, for example:

    let (person, species, name) = ("Charlie Brown", "dog", "Snoopy");

    // implicit named argument `person`
    print!("Hello {person}");

    // implicit named arguments `species` and `name`
    format!("The {species}'s name is {name}.");

Implicit named argument capture only occurs when a corresponding named argument is not provided to the macro invocation. So in the below example, no implicit lookup for `species` is performed:

    // explicit named argument `species`
    // implicit named argument `name`
    format!("The {species}'s name is {name}.", species="cat");

(Downstream macros based on `format_args!` include but are not limited to `format!`, `print!`, `write!`, `panic!`, and macros in the `log` crate.)


# Motivation
[motivation]: #motivation

The macros for formatting text are a core piece of the Rust standard library. They're often one of the first things users new to the language will be exposed to. Making small changes to improve the ergonomics of these macros will improve the language for all - whether new users writing their first lines of Rust, or seasoned developers scattering logging calls throughout their program.

This proposal to introduce implicit named arguments aims to improve ergonomics by reducing the amount of typing needed in typical invocations of these macros, as well as (subjectively) improving readability.


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

If this proposal were captured, the following (currently invalid) macro invocation:

    format_args!("hello {person}")

would become a valid macro invocation, and would be equivalent to a shorthand for the already valid:

    format_args!("hello {person}", person=person)

This identifier `person` would be known as an **implicit named argument** to the macro. `format_args!` would be able to accept any number of such implicit named arguments in this fashion. Each implicit named argument would have to be an identifier which existed in the scope in which the macro is invoked.

Should `person` not exist in the scope, the usual error E0425 would be emitted by the compiler:

    error[E0425]: cannot find value `person` in this scope
     --> .\foo.rs:X:Y
      |
    X |     println!("hello {person}");
      |                     ^^^^^^^^ not found in this scope

As a result of this change, downstream macros based on `format_args!` would also be able to accept implicit named arguments in the same way. This would provide ergonomic benefit to many macros across the ecosystem, including:

 - `format!`
 - `print!` and `println!`
 - `eprint!` and `eprintln!`
 - `write!` and `writeln!`
 - `panic!`, `unreachable!` and `unimplemented!`
 - `assert!` and similar
 - macros in the `log` crate
 - macros in the `failure` crate

(This is not an exhaustive list of the many macros this would affect. In discussion of this RFC if any further commonly-used macros are noted, they may be added to this list.)

## Precedence

Implicit arguments would have lower precedence than the existing named arguments `format_args!` already accepts. For example, in the example below, the `person` named argument is explicit, and so the `person` variable in the same scope would not be captured:

    let person = "Charlie";

    // Person is an explicit named argument, so this
    // expands to "hello Snoopy".
    println!("hello {person}", person="Snoopy");

Indeed, in this example above the `person` variable would be unused, and so in this case the unused variable warning will apply, like the below:

    warning: unused variable: `person`
     --> src/foo.rs:X:Y
      |
    X |     let person = "Charlie";
      |         ^^^^^^ help: consider prefixing with an underscore: `_person`
      |
      = note: `#[warn(unused_variables)]` on by default

Because implicit named arguments would have lower precedence than explicit named arguments, it is anticipated that no breaking changes would occur to existing code by implementing this RFC.

## Generated Format Strings

`format_args!` can accept an expression instead of a string literal as its first argument. `format_args!` will attempt to expand any such expression to a string literal. If successful then the `format_args!` expansion will continue as if the user had passed that string literal verbatim.

No implicit named argument capture will be performed if the format string is generated from an expansion. See the [macro hygiene](#macro-hygiene) discussion for the motivation behind this decision.


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The implementation pathway is directly motivated by the guide level explanation given above:

1. The `format_args!` macro can continue to parse the format string and arguments provided to it in the existing fashion, categorising arguments as either positional or named.

2. In the current implementation of `format_args!`, after parsing has occurred, all the arguments referred to by the format string are validated against the actual arguments provided. If a named argument is referred to in the format string but no corresponding named argument was provided to the macro, then an error is emitted:

        error: there is no argument named `person`
        --> src/foo.rs:X:Y
         |
       X |     println!("hello {person}");
         |                     ^^^^^^^^

   If this RFC were implemented, instead of this resulting in an error, this named argument would be treated as an **implicit named argument** and the final result of the expansion of the `format_args!` macro would be the same as if a named argument, with name equivalent to the identifier, had been provided to the macro invocation.

   Because `person` is only treated as an implicit named argument if no existing named argument can be found, this ensures that implicit named arguments have lower precedence than explicit named arguments.

## Macro Hygiene
[macro-hygiene]: #macro-hygiene


Expanding the macro in this fashion will need to generate an identifier which corresponds to the implicit named argument. The hygiene of this generated identifier would be inherited from the format string, with location information reduced to the section of the format string which contains the implicit named argument.

An interesting case to consider is that `format_args!`-based macros can accept any expression in the format string position. The macro then attempts to expand this expression to a string literal.

This means the below examples of `format!` invocations could compile successfully in stable Rust today:

    format!(include_str!("README.md"), foo=1)
    format!(concat!("hello ", "{bar}")), bar=2)

This RFC argues that `format_args!` should not attempt to expand any implicit named arguments if the macro is provided with an expression instead of a verbatim string literal.

The following are motivations why this RFC argues this case:

* This RFC's motivation for implicit named arguments is to give users a concise syntax for string formatting. When the format string is generated from some other expression this motivation for concise syntax is irrelevant.

* The hygienic context of the string literal generated by the expansion is entirely dependent on the expression. For example, the string literal produced by the `concat!` macro resides in a separate hygienic context. In combination with implicit named arguments using hygiene inherited from the format string, this would lead to puzzling errors like the below:

      error[E0425]: cannot find value `person` in this scope
       --> scratch/test.rs:4:14
        |
        |     let person = "Charlie";
      4 |     println!(concat!("hello {person}"));
        |              ^^^^^^^^^^^^^^^^^^^^^^^^^ not found in this scope

* The expression may expand to a format string which contains new identifiers not written by the users, bypassing macro hygiene in surprising ways. For example, if the `concat!` macro did not have the hygiene issue described above, it could be to "splice together" an implicit named argument like so:

       let person = "Charlie";
       println!(concat!("hello {p", "er", "son", "}"));

   The RFC author argues that it appears highly undesirable that implicit capture of the `person` identifier should occur in this example given above.

* Using the hygienic context of the format string for implicit named arguments can have potentially surprising results even just with `macro_rules!` macros.

  For example, the RFC author found that with a proof-of-concept implementation of implicit named arguments the invocation below would print `"Snoopy"`:

      const PERSON: &'static str = "Charlie";

      fn main() {
          macro_rules! bar {
            () => { "{PERSON}" };
          }

          const PERSON: &'static str = "Snoopy";
          println!(bar!());
      }

  However, by merely changing to `let` bindings and moving the `"Charlie"` declaration three lines down to be inside the `main()` function, as below, the invocation would instead print `"Charlie"`:

      fn main() {
          let person = "Charlie";
          macro_rules! bar {
              () => { "{person}" };
          }

          let person = "Snoopy";
          println!(bar!());
      }

   While it can be argued that this example is very contrived, the RFC author believes that it is undesirable to add such subtle interactions to the `format_args!` family of macros.

These appear to give strong motivation to disable implicit argument capture when `format_args!` expands an expression instead of a verbatim string literal.

# Drawbacks
[drawbacks]: #drawbacks

As the syntax proposed does not currently compile, the author of this RFC does not foresee concerns about this addition creating breaking changes to Rust code already in production.

However, this proposal does increase the complexity of the macros in question, as there would now be three options for how users may provide arguments to the them (positional arguments, named arguments, and the new implicit named arguments).

It would also alter the learning pathway for users as they encounter these macros for the first time. If implicit named arguments prove convenient and popular in the Rust ecosystem, it may be that new users of the language learn how to use the macros in implicit named argument form before they encounter the other two options, and may even not learn about the other two options until some time into their Rust journey.

Furthermore, users familiar with implicit named arguments, but not the other options, may attempt to pass expressions as arguments to format macros. Expressions would not be valid implicit named arguments. For example:

    // get_person() is a function call expression, not an identifier,
    // so could not be accepted as an implicit named argument
    println!("hello {}", get_person());

This is not world-ending, as users who only know about implicit named arguments (and not positional or named arguments) might write something like the following:

    let person = get_person();
    println!("hello {person}");

While two lines rather than one, it is still perfectly readable code.


# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The core macro resonsible for all Rust's string formatting mechanism is `std::format_args!`. It requires a format string, as well as a corresponding number of additional arguments which will be substituted into appropriate locations in the format string.

There are two types of arguments `format_args!` can accept:

1. Positional arguments, which require less typing and so (in the RFC author's experience) are used more frequently:

       format_args!("The {}'s name is {}.", species, name)

2. Named arguments, which require more typing but (in the RFC author's experience) have the upside that the the format string itself is easier to read:

       format_args!(
           "The {species}'s name is {name}",
           species=species,
           name=name
       )

Neither positional or named arguments are restricted to identifiers. They can accept any valid Rust expression, for example:

    format_args!("Hello {}", get_person())
    format_args!("Hello {person}", person=get_person())

However, this RFC author's experience is that a significant majority of arguments to formatting macros are simple identifiers. (It is openly acknowledged that this is a subjective statement.)

Implicit named arguments seek to combine the brevity of positional arguments with the clarity that named arguments provide to the format string:

    format_args!("The {species}'s name is {name}")

## Alternative Implementations and Syntax

Users who wish to use implicit named arguments could make use of a third-party crate, for example the existing [fstrings crate](https://crates.io/crates/fstrings), which was built during early discussion about this proposal. This RFC accepts that deferring to a third-party crate is a reasonable option. It would however miss out on the opportunity to provide a small and straightforward ergnomic boost to many macros which are core to the rust language as well as the ecosytem which is derived from these standard library macros.

For similar reasons this RFC would argue that introducing a new alternative macro to `format_args!` in the standard library would not be a good outcome compared to adding to the existing macro.

An alternative syntax for implicit named arguments is welcomed by this RFC if it can be argued why it is preferable to the RFC's proposed form. The RFC author argues the chosen syntax is the most suitable, because it matches the existing syntax for named arguments.

## Alternative Solution - Interpolation
[interpolation]: #interpolation

Some may argue that if it becomes possible to write identifiers into format strings and have them passed as implicit named arguments to the macro, why not make it possible to do similar with expressions. For example, these macro invocations seem innocent enough, reasonably readable, and are supported in Python 3 and Javascript's string formatting mechanisms:

    println!("hello {get_person()}");  // function call
    println!("hello {self.person}");   // field access

The RFC author anticipates in particular that field access may be requested by many as part of this RFC. After careful consideration this RFC does not propose to go further than the single identifier special case, proposed above as implicit named arguments.

If any expressions beyond identifiers become accepted in format strings, then the RFC author expects that users will inevitably ask "why is *my* particular expression not accepted?". This could lead to feature creep, and before long perhaps the following might become valid Rust:

    println!("hello { if self.foo { &self.person } else { &self.other_person } }");

This no longer seems easily readable to the RFC author.

### Proposed Interpolation Syntax

Early review of this RFC raised an observation that the endpoint of such feature creep would be that eventually Rust would embrace interpolation of any expressions inside these macros.

To keep interpolation isolated from named and positional arguments, as well as for readability and (possibly) to reduce parsing complexity, curly-plus-bracket syntax was proposed for interpolation:

    println!("hello {(get_person())}");
    println!("hello {(self.person)}");

Indeed the RFC's perverse example reads slightly easier with this syntax:

    println!("hello {( if self.foo { &self.person } else { &self.other_person } )}");

Because the interpolation syntax `{(expr)}` is orthogonal to positional `{}` and named `{ident}` argument syntax, and is a superset of the functionality which would be offered by implicit named arguments, the argument was made that we should make the leap directly to interpolation without introducing implicit named arguments so as to avoid complicating the existing cases.

### Argument Against Interpolation

It should first be noted that the interpolation in other languages is often a language feature; if they have string formatting functions they typically do not enjoy syntax-level support. Instead other language formatting functions often behave similarly to Rust's positional and/or named arguments to formatting macros.

For example, Python 3's `.format()` method is on the surface extremely similar to Rust's formatting macros:

    "hello {}".format(person)
    "hello {person}".format(person=person)

However, Python 3 cannot improve the ergonomics of these functions in the same way that this RFC proposes to use implicit named arguments. This is for technical reasons: Python simply does not have a language mechanism which could be used to add implicit named arguments to the `.format()` method. As a result, offering improved ergonomics in Python necessitated the introduction of a language-level interpolation syntax (f-strings, described in the [prior art](#prior-art) section).

(Note, the closest Python 3's `.format()` can get to implicit named arguments is this:

    "hello {person}".format(**locals())

but as noted in [PEP 498](https://www.python.org/dev/peps/pep-0498/#no-use-of-globals-or-locals), the Python language designers had reasons why they wanted to avoid this pattern becoming commonplace in Python code.)

Rust's macros are not constrained by the same technical limitations, being free to introduce syntax as long as it is supported by the macro system and hygiene. The macros can therefore enjoy carefully-designed ergonomic improvements without needing to reach for large extensions such as interpolation.

The RFC author would argue that if named arguments (implicit or regular) become popular as a result of implementation of this RFC, then the following interpolation-free invocations would be easy to read and good style:

    // Just use named arguments in simple cases
    println!("hello {person}", person=get_person());
    println!("hello {person}", person=self.person);

    // For longwinded expressions, create identifiers to pass implicitly
    // so as to keep the macro invocation concise.
    let person = if self.foo { &self.person } else { &self.other_person };
    println!("hello {person}");

Similar to how implicit named arguments can be offered by third-party crates, interpolation macros already exist in the [ifmt crate](https://crates.io/crates/ifmt).

### Interpolation Summary

The overall argument is not to deny that the standard library macros in question would not become more expressive if they were to gain fully interpolation.

However, the RFC author argues that adding interpolation to these macros is less necessary to improve ergonomics when comparing against other languages which chose to introduce language-level interpolation support. Introduction of implicit named arguments will cater for many of the common instances where interpolation would have been desired. The existing positional and named arguments can accept arbitrary expressions, and are not so unergonomic that they feel overly cumbersome when the expression in question is also nontrivial.


# Prior art
[prior-art]: #prior-art

## Field Init Shorthand

Rust already has another case in the language where the single identifier case is special-cased:

    struct Foo { bar: u8 }
    let bar = 1u8;

    let foo = Foo { bar: bar };
    let foo = Foo { bar };        // This shorthand only accepts single identifiers

This syntax is widely used and clear to read. It's [introduced in the Rust Book as one of the first topics in the section on structs](https://doc.rust-lang.org/book/ch05-01-defining-structs.html#using-the-field-init-shorthand-when-variables-and-fields-have-the-same-name). This sets a precedent that the Rust language is prepared to accept special treatment for single identifiers when it keeps syntax concise and clear.

## Other languages

A number of languages support string-interpolation functionality with similar syntax to what Rust's formatting macros. The RFC author's influence comes primarily from Python 3's "f-strings" and JavaScript's backticks.

The following code would be the equivalent way to produce a new string combining a `greeting` and a `person` in a variety of languages:

    // Rust
    format!("{} {}", greeting, person)                                // positional form,
    format!("{greeting} {person}", greeting=greeting, person=person)  // or named form

    # Python 3
    f"{greeting} {person}"

    // Javascript
    `${greeting} ${person}`

    // C# / VB
    $"{greeting} {person}"

    // Swift
    "\(greeting) \(person)"

    // Ruby
    "#{greeting} #{person}"

    // Scala
    s"$greeting $person"

    // Perl and PHP
    "$greeting $person"

It is the RFC author's experience that these interpolating mechanisms read easily from left-to-right and it is clear where each variable is being substituted into the format string.

In the Rust formatting macros as illustrated above, the positional form suffers the drawback of not reading strictly from left to right; the reader of the code must refer back-and-forth between the format string and the argument list to determine where each variable will be substituted. The named form avoids this drawback at the cost of much longer code.

Implementing implicit named arguments in the fashion suggested in this RFC would eliminate the drawbacks of each of the Rust forms and permit new syntax much closer to the other languages:

    // Rust - implicit named arguments
    format!("{greeting} {person}")

It should be noted, however, that other languages' string interpolation mechanisms allow substitution of a wide variety of expressions beyond the simple identifier case that this RFC is focussed on.

Please see the discussion on [interpolation](#interpolation) as an alternative to this RFC.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

## Interaction with `panic!`

The `panic!` macro forwards to `format_args!` for string formatting. For example, the below code compiles on stable Rust today:

    fn main() {
        panic!("Error code: {code}", code=1);
        // thread 'main' panicked at 'Error code: 1' ...
    }

However, in current stable Rust the `panic!` macro does not forward to `format_args!` if there is only a single argument. This would interact poorly with implicit named arguments. In the invocation below, for example, users familiar with implicit named argument capture would expect the panic message to be formatted. Instead, given `panic!`'s current semantics, the panic message would be the unformatted literal:

    fn main() {
        let code = 1;
        panic!("Error code: {code}");
        // thread 'main' panicked at 'Error code: {code}' ...
    }

This semantic of `panic!` has previously been acknowledged as a "papercut", for example in [this Rust issue](https://github.com/rust-lang/rust/issues/22932). However, it has so far been left as-is because changing the design was low priority, and changing it may break existing code.

If this RFC were to be implemented, users will very likely expect invoking `panic!` with only a string literal will capture any implicit named arguments. This semantic would quickly become perceived as a major bug rather than a papercut.

Implementing this RFC therefore would bring strong motivation for making a small breaking change to `panic!`: when a single argument passed to panic is a string literal, instead of the final panic message being that literal (the current behavior), the final panic message will be the formatted literal, substituting any implicit named arguments denoted in the literal.

That is, the desired behavior is as the example below:

    fn main() {
        let code = 1;
        panic!("Error code: {code}");
        // thread 'main' panicked at 'Error code: 1' ...
    }

This change to `panic!` would alter the behavior of existing code (such as the example above). It would also stop some code from being accepted, such as `panic!("{}")`, which is valid code today but would become a compile fail (because this would be a missing positional argument). Crates implementing macros with similar semantics to `panic!` (such as `failure`) may also wish to make changes to their crates in sync with the change to `panic!`. This suggests that this change to `panic!` would perhaps be ideal for release as part of a future Rust edition, say, 2021.

The details of this pathway to change panic are open to discussion. Some possible options:

* `panic!` itself could be made a builtin macro (which would allow its behavior to vary between editions)

* A `$expr:literal` match arm could be added to `panic!`. This arm could forward to a built-in macro which controlled behaviour appropriately.

* A new implementation of `panic!` could be written, and switching between them could be done with a new `std::prelude`.

Whichever route is chosen, it is agreed that this RFC should not be stabilised unless `format!("{foo}")` and `panic!("{foo}")` can be made consistent with respect to implicit named arguments.

## Should implicit named arguments be captured for formatting parameters?

Some of the formatting traits can accept additional formatting parameters to control how the argument is displayed. For example, the precision with which to display a floating-point number:

    println!("{:.5}", x);  // print x to 5 decimal places

It is also possible for the precision to refer to either positional or named arguments using "dollar syntax":

    println!("{:.1$}", x, 5);
    println!("{:.prec$}", x, prec=5);

As a result of this RFC, formatting parameters could potentially also make use implicit named argument capture:

    println!("{x:.precision$}");

The RFC author believes Rust users familiar with implicit named arguments may expect the above to compile (as long as `x` and `precision` were valid identifiers in the scope in question). However, feedback is requested during this RFC process as to whether the this should be indeed become acceptable as part of the RFC.

All such formatting parameters can refer to arguments using dollar syntax, and so this question also applies to them.

## Should we improve the error for invalid expressions in format strings?

Users familiar with implicit named arguments may attempt to write expressions inside format strings, for example a function call:

    println!("hello {get_person()}");

The current error message that would be emitted does not explain that arbitrary expressions are not possible inside format strings:

    error: invalid format string: expected `'}'`, found `'('`
    --> .\foo.rs:X:Y
      |
    3 |     println!("hello {get_person()}");
      |                     -          ^ expected `}` in format string
      |                     |
      |                     because of this opening brace
      |
      = note: if you intended to print `{`, you can escape it using `{{`

An new message which informs the users of alternative possibilities may be helpful:

    error: expressions may not be used inside format strings
    --> .\scratch\test.rs:3:37
      |
    3 |     println!("hello {get_person()}");
      |                     ^^^^^^^^^^^^^^ expression is here
      |
    = note: if you wanted to pass an expression as an argument to a formatting macro,
      try as a positional argument, e.g. println!("hello {}", get_person());
            or as a named argument, e.g. println!("hello {foo}", foo=get_person());

It is not clear how significant a change this might require to `format_args!`'s parsing machinery, or how this error message might scale with the complexity of the format string in question.

# Future possibilities
[future-possibilities]: #future-possibilities

The main alternative raised by this RFC is interpolation, which is a superset of the functionality offered by implicit named arguments. However, for reasons discussed above, interpolation is not the objective of this RFC.

Accepting the addition of implicit named arguments now is not incompatible with adding interpolation at a later date.

Future discussion on this topic may also focus on adding interpolation for just a subset of possible expressions, for example `dotted.paths`. We noted in debate for this RFC that particularly for formatting parameters the existing dollar syntax appears problematic for both parsing and reading, for example `{self.x:self.width$.self.precision$}`.

The conclusion we came to in the RFC discussion is that adding even just interpolations for `dotted.paths` will therefore want a new syntax, which we nominally chose as the `{(expr)}` syntax already suggested in the [interpolation](#interpolation) alternative section of this RFC.

Using this parentheses syntax, for example, we might one day accept `{(self.x):(self.width).(self.precision)}` to support `dotted.paths` and a few other simple expressions. The choice of whether to support an expanded subset, support interpolation of all expressions, or not to add any further complexity to this macro is deferred to the future.

A future proposal for extending interpolation support might wish to explore alternative syntaxes to `{(expr)}` parentheses which can also be parsed and read comfortably.
