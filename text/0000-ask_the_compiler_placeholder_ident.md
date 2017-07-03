- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

This is inspired by haskell holes: Add a 'placeholder identifier' to the AST; when encountered, it passes through type inference; it does not resolve to an actual definition, but instead assumes it is 'whatever needed' to fit the demands around it, and allows the rest of the program to contiue compiling (resolving/typechecking). 

Compiling a program with placeholders will fail, but gives extra informative error messages reporting (a) the types at that point, and (b) potential symbols that fit.

# Motivation

This is to assist navigation and discovery during the compile-edit cycle, doing a similar job to IDE 'dot-autocomplete', but leveraging  Rust's more advanced 2-way type inference by making deliberate queries with more information.

Already people use the trick of 'let _:()=expr_'  to query the type as an error, but these queries would allow the inference engine to work harder.

Improving compile time feedback pushes the community forward whilst we wait for IDE tools (with more complex integration) to stabilize and refine.

# Detailed design

It requires the AST to contain the concept of a query in an ident slot, even though the presence of a query signifies that this program cannot compile.

see earlier forum post,
https://internals.rust-lang.org/t/ask-the-compiler-syntax-e-g--/700

The placeholder would be configurable (by commandline option) to avoid any clash, but a sane default would be a single underscore.
The placeholder parses as any other ident from a syntacital structure POV; it could be found in the location of any function, variable, argument, field, literal struct-name or trait identifier.

By making it a configurable legal ident, it will not take any syntax space.

This is distinct to the underscore character in the type context (which performs a different function, it leaves a gap where the surrounding context is enough to infer *exactly* what is going on).

The query would be used in the following ways:-

- querying functions, based on parameters/return values:-

```{ a._() }     // ask for available member functions of a, similar to classic 'dot-autocomplete'```
```{ a._(b) }     // ask for available member functions of a, taking 'b' - beyond classic autocomplete, it can be guided by more parameters (not just the first)```

```{ a._(b,_) }     // ask for available member functions of a, taking 'b' - beyond classic autocomplete, it can be guided by more parameters (not just the first)```

```fn foo(a:&X)->Y { _(a) } // ask for functions close to fitting the signature ( &X)->Y ```

```{ ... foo(_) ... } report the arguments 'foo' should take```

```fn foo(a:&X)->Z { a._()._()} // ask for any functions X->Y , Y->Z and possible 'Y'```

```_::bar(x,y,z) // ask for full paths of any functions bar(..)```

```fn foo<T:_>( a:T) { a.do_something() } // ask what traits have a method 'do_something()'```

- querying struct fields:-
```foo._.bar - ... what member has a sub-member .bar (maybe make it also show,```

```foo._._.bar foo._._._.bar, i.e. search the object graph.. this is a big deal with complex data structures. )```


- Querying whole program inference:-
```fn foo(a,b,c)->_ {... do stuff...} .. Do full whole-program inference from any calls TO 'foo', and report what signature this function needs to fit it's uses.. the scenario appears when you factor code out.```



If the compiler output was formatted nicely (I dont know the full plan with the RLS) perhaps this could be used directly for IDE/editor integration; imagine if output of underscore queries could be collected by an IDE and placed in dropbox menus under the text; this could yield a unique editing experience beyond existing autocomplete IDEs?.




# Drawbacks



# Alternatives

Putting all similar effort into IDE integration.
it might be possible to treat *all* unresolved symbols this way - the compiler already does make a lot of suggestions - but the idea is for a deliberate query that can invoke heavier work.
I note that since I made this suggestion a while back significant progress has been made the with "RLS", but perhaps there is synergy with the other peices IDE support needs.

# Unresolved questions

What parts of the design are still TBD?
