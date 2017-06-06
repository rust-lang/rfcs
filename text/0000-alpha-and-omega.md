- Feature Name: Higher and Lower Types
- Start Date: 2017-04-01 
- RFC PR: 
- Rust Issue: 

# Summary
[summary]: #summary

This proposal enables programmers to continuously navigate the performance-expressiveness curve by providing a mechanism for explicitly working with higher and lower types ("Omega Types").

This allows Rust programmers to take advantage of type-level monads (meganads), and negative-cost abstractions (NCAs).




# Motivation
[motivation]: #motivation

Rust currently lives in an awkward middle-ground: it's currently incredibly inexpressive, making it undesirable for the mainstream programming community which is used to languages like Idris. At the same time, Rust is also incredibly inefficient, making it undesirable for hardcore systems programmers who are used to the blazing performance of Haskell.

Both of these problems can be reduced to a single issue: Rust's type system has a significant impedance mismatch with how machines work. This is why user code needs to pass through so many intermediate representations: 

```
Text -> AST -> HIR -> MIR -> LLVM-IR -> LLVM-MIR -> ASM -> Machine Code.
```

At each step the compiler is forced to try to synthesize standard machine abstractions like monads and lenses, given only sloppy imperative Rust code. This incredible journey is why introducing higher-kinded types to the language has been such a struggle: lowering the types would be dangerous at such heights. In particular, due to the Turing Completeness of Rust's type system, it's undecidable if a given lowering will reach *terminating velocity*.





# Detailed design
[design]: #detailed-design

Our solution to this problem is simple: expose the compiler's pipeline to developers, so they can place types at whatever height they want, and manage the abstraction velocity themselves.

Under this system, types may be made *higher* and *lower*, using the theory of *Omega Types*. The syntax for this, is simple and natural: types may be raised with the HTML `<sup>` tag, and lowered with the HTML `<sub>` tag. For example:

```
i32                             // A regular type (coexistential type)
Vec<i32>                        // A generic type (Van Emde Boas type)
Vec<sup>i32</sup>               // A higher type (Omega type)
Vec<sub>i32</sub>               // A lower type (Co-Omega type)
Vec<sup>Vec<sub>i32</sub></sup> // A lower higher type (Re-Omega type)
```

This syntax is admittedly a bit cumbersome to read, but this is easily solved by basic IDE support. As we all know, the Rust community has completely migrated to either Atom or Visual Studio Code. Since both of these IDEs are based on embedding webkit, they can easily support rendering HTML! As such, programmers should actually see:

i32, Vec&lt;i32&gt;, Vec<sup>i32</sup>, Vec<sub>i32</sub>, Vec<sup>Vec<sub>i32</sub></sup>

Which is completely clear and obvious!




## Semantics

The semantics of higher and lower types are straightforward. First and foremost, the height of a type corresponds to where in the compiler pipeline they'll be introduced. Higher types will arrive in stages like parsing (using a system similar to C's preprocessor), while lower types will arrive in later stages like llvm's semantic analysis (by dynamically linking them in).

Higher types have a damage and range advantage over lower types, but lower types will take less fall damage. Fall damage can be prevented with judicious use of trampoline functions, at the cost of compile time. Conversely, lower types improve compilation time, since they have less of a distance to travel.




## Additional Benefits

A natural consequence of this ability to lower types are so-called "underground types", which have been lowered so far down that they actually arrive during the execution of the program. This provides a free JIT implementation for all Rust programs.

Because Rust has so many strings, it will be trivial to "hang" types at a particular height, providing a high quality implementation of incremental compilation. Resuming compilation will be a simple process of applying the min-cut algorithm.

If we instead pull on these strings, we can reverse the compilation process, making it easy to debug the execution and compilation of a program -- meta rr.






# How We Teach This
[how-we-teach-this]: #how-we-teach-this

Teaching materials largely won't be necessary, as this system follows so naturally from the Erik Demaine's seminal paper on Oragami Typing. In fact, most Rust programmers could trivially design this system given only the description of it as "a minimal system for forming an oragami crane in the type system".




# Drawbacks
[drawbacks]: #drawbacks

Omega types are obviously incompatible with legacy platforms like x86 and ARM. However, given the significant benefits of this proposal, this is an obviously correct trade-off to make. Abandoning these platforms would also give us greater resources to focus on our core platforms like the DEC Alpha. 





# Alternatives
[alternatives]: #alternatives

Abandoning Rust is the only alternative. This proposal is the only way forward for our language in an existential crisis.



# Unresolved questions
[unresolved]: #unresolved-questions

How will this proposal interact with ongoing efforts to support SIMD address spaces?



