- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Imageine a tool for making compile time queries (inspired by haskell holes): Add a 'placeholder ident' to the AST; this passes through type inference; it does not resolve to an actual symbol, but instead assumes it is 'whatever needed' to fit the demands around it, and allows the rest of the program to contiue compiling (resolving/typechecking). 

Compiling a program with placeholders will fail, but gives extra informative error messages reporting (a) the types at that point, and (b) potential symbols that fit.

# Motivation

This is to assist navigation and discovery whilst programming, doing a similar job to IDE 'dot-autocomplete', but leveraging  Rust's more advanced 2-way type inference. Already people use the trick of 'let _:()=expr_'  to query the type as an error, but these 'deliberate' queries would allow the inference engine to work harder.

Improving compile time feedback pushes the community forward whilst we wait for IDE tools (with more complex integration) to stabilize and refine.

# Detailed design

see earlier forum post,
https://internals.rust-lang.org/t/ask-the-compiler-syntax-e-g--/700

The placeholder could be configurable (by commandline option or lang item?) to avoid clash, but a sane default would be a single underscore in a function,trait,or variable spot, because no one would use that for a real symbol. By making it a configurable legal ident, it will not take any syntax space. This is distinct to the underscore character in the type context (where it already performs a different function).

The query could be used in the following ways (I note that as of 2017, the rust compiler already gives a bit of assistance in some of these cases)

fn foo<T:_>( a:T) { a.do_something() } // ask what traits have a method 'do_something()'

fn foo(a:&X)->Y { _(a) } // ask for functions close to fitting the signature ( &X)->Y 
fn foo(a:&X)->Z { a._()._()} // ask for any functions X->Y , Y->Z and possible 'Y'

_::bar(x,y,z) // ask for full paths of any functions bar(..)

foo._.bar - ... what member has a sub-member .bar (maybe make it also show,

foo._._.bar foo._._._.bar, i.e. search the object graph.. this is a big deal with complex data structures. )

fn foo(a,b,c)->_ {... do stuff...} .. Do full inference from calls to 'foo', and report what signature this function needs to fit it's uses.. the scenario when you factor code out.

{ ... foo(_) ... } report the arguments 'foo' should take


If the compiler output was formatted nicely (I dont know the full plan with the RLS) perhaps this could be used directly for IDE/editor integration; imagine if output of underscore queries could be collected by an IDE and placed in dropbox menus under the text; this could yield a unique editing experience beyond existing autocomplete IDEs?.




# Drawbacks



# Alternatives

Putting all similar effort into IDE integration.
it might be possible to treat *all* unresolved symbols this way - the compiler already does make a lot of suggestions - but the idea is for a deliberate query that can invoke heavier work.
I note that since I made this suggestion a while back significant progress has been made the with "RLS", but perhaps there is synergy with the other peices IDE support needs.

# Unresolved questions

What parts of the design are still TBD?
