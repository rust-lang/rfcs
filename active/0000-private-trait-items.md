- Start Date: 2014-04-18
- RFC PR #:
- Rust Issue #:

# Summary

I propose the ability to set trait items (i.e. just methods currently) private as well as public in order to expand the scope of possible use cases of provided methods (i.e. default trait method implementations). I also propose that trait items should be private by default.

# Motivation

Sometimes a trait may be able to provide a default implementation for a method iff it can use a certain method which only the type that implements the trait is in a position to provide a definition for. Often times such a feedback method is supposed to be only a tool for the trait to be able to define provided methods with, and as such, not supposed to become a part of the public interface of the trait or any type which implements the trait. Therefore such a feedback method should be made private. Trait items should be private by default so that we don't have the need to reintroduce the 'priv' keyword. If in future we get the ability to specify that a certain provided method in a trait is 'final' (i.e. not overridable by the type which implements the trait), then, together with private trait methods, we can use the Non-Virtual Interface (NVI) idiom coined and described here by Herb Sutter: http://www.gotw.ca/publications/mill18.htm

# Detailed design

One way of looking at private trait methods (or any private trait items) is to see them as private dialog between a trait and a type which implements the trait. This view could lead to a design where no-one else except the trait and the type which implements it is allowed access to such private feedback item. But given how Rust privacy rules work at module boundaries (and also extend access to submodules), it would make sense that access to a private trait item extended from just the trait or the type which implements it to the enclosing module and its submodules. By this logic I suggest the following privacy rules for private trait items:

Given that:  
1) A trait ```Tr``` specifies a private item ```priv_item``` and is defined in module ```mod_tr```  
3) A type ```Foo``` implements ```Tr``` and is defined in module ```mod_foo```  
3) A type ```Bar``` implements ```Tr``` and is defined in module ```mod_bar_and_baz```  
4) A type ```Baz``` implements ```Tr``` and is defined in module ```mod_bar_and_baz```

It follows that:  
1) ```priv_item``` is accessible from ```mod_tr``` and all its submodules.  
2) ```priv_item``` is accessible from ```mod_foo``` and all its submodules iff it is certain at compile-time that it refers to the ```Foo```'s implementation ```priv_item```  
3) ```priv_item``` is accessible from ```mod_bar_and_baz``` and all its submodules iff it is certain at compile-time that it refers to either the ```Bar```'s or ```Baz```'s implementation of ```priv_item```  

And ```priv_item``` is not accessible from anywhere else.

Example:
```
// in mod_tr.rs
pub trait Tr {
   priv fn priv_item(&self);

   pub fn do_stuff(&self) {
       self.priv_item(); // OK
   }
}

pub fn do_other_stuff<A:Tr>(a: &A) {
   a.priv_item(); // OK
}

// in mod_foo.rs
use mod_tr::Tr;

pub struct Foo;

impl Tr for Foo {
   priv fn priv_item(&self) {}
}

pub fn do_foo_stuff(foo: &Foo) {
   foo.priv_item(); // OK
}

pub fn do_incorrect_stuff<A:Tr>(a: &A) {
   a.priv_item(); // ERROR: "A private trait item Tr::priv_item not accessible from mod_foo"
}
```

# Alternatives



# Unresolved questions

