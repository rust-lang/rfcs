- Start Date: 2014-05-28
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Unboxed closures should be implemented with three traits (`Fn`, `FnMut`, and `FnOnce`), and there should be a leading sigil (`&:`/`&mut:`/`:`) before the argument list so the programmer can describe which one is meant.

# Motivation

This RFC simply addresses some points that were not ironed out in the previous unboxed closure RFC.

# Detailed design

This builds on RFC #77 "unboxed closures"; see the design for that.

There should be three traits as lang items:

    #[lang="fn"]
    pub trait Fn<A,R> {
        fn call_fn(&self, args: A) -> R;
    }
    
    #[lang="fn_mut"]
    pub trait FnMut<A,R> {
        fn call(&mut self, args: A) -> R;
    }
    
    #[lang="fn_once"]
    pub trait FnOnce<A,R> {
        fn call_once(self, args: A) -> R;
    }

The unboxed closure literal form `|a, b| a + b` creates an anonymous structure implementing one of the above three traits. Accordingly, we introduce new syntaxes for unboxed closures to correspond to the three traits above:

    let f: |&: a, b| a + b;    // implements `Fn`
    let g: |&mut: a, b| a + b; // implements `FnMut`
    let h: |: a, b| a + b;     // implements `FnOnce`

Once boxed closures are removed, the regular `|a, b| a + b` syntax will be an alias for `|&mut: a, b| a + b`, since that is the commonest trait to implement.

The idea behind the syntax is that what goes before the `:` mirrors what goes before `self` in the `call`/`call_fn`/`call_once` function signature. This syntax avoids introducing any new keywords to the language.

The call operator `x(y, z)` will desugar to one of `x.Fn::call_fn((y, z))`, `x.FnMut::call((y, z))`, and `x.FnOnce::call_once((y, z))`, depending on the trait that `x` implements. If `x` implements more than one of `Fn`/`FnMut`/`FnOnce`, then the compiler reports an error and the `x(y, z)` form cannot be used.

We will remove `proc(A...) -> R` and replace with `Box<FnOnce<(A...),R>>`.

# Drawbacks

* The syntax may be ugly.

* It may be that `Fn` and `FnOnce` are too much complexity.

* Tupling the arguments may have ABI impacts, although I researched this on ARM-EABI and x86 and did not find any.

* Because of argument tupling, we lose the ability to pass DSTs by value, which has been proposed in the past.

# Alternatives

The impact of not doing this at all is that the precise trait that unboxed closures implement will be undefined, and we will continue to have `proc`.

An alternative to tupling arguments is to introduce variadic generics, but that seems like a lot of complexity.

# Unresolved questions

It remains to be seen how this interacts with not being able to use "for-all" quantifiers in trait objects. This will break some code until/unless we introduce this capability. How much is unknown.

ABI issues relating to tupling struct arguments on uncommon architectures like MIPS and non-EABI ARM have been inadequately explored.
