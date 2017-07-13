- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

allow the ommision of function parameter types and the return type in the implementaiton of a trait for a type.

# Motivation

Rust signatures can become quite heavy with nested angle-bracketed types, trait-bounds, lifetimes etc, 
Also,coming from C++, the need to write and reference traits *before* you can 'overload' functions comes as a shock. 

However, if the informtion in the trait declaration was used to avoid repeating information, they would come across as more of a virtue: writing and referenceing them would directly *save* repetition when writing many functions following a similar pattern.

Note that this would not make writing the implementation any harder: Unlike with general purpose whole-program inference , constraining is already implied by the trait itself; the compiler already knows that one must match the other, and when it doesn't it reports an error. Compared to C++, Rusts syntax allows the ommision of types whilst still parsing parameter names in a straightforward manner, creating this opportunity. The lack of overloading *within* a trait/impl means there is no need to write the types to disambiguate the functions; you would be fully leveraging the trait name to do that.

Behaviour of this type can be seen in the Haskell language, e.g

    class FooBar f where                  -- typeclass definition (roughly = Rust trait)
      foo::f->i32->[i32]->String          -- only write function names and function signatures
      bar::f->[String]->String->Maybe String
  
    instance FooBar Apple where     -- typeclass instance (roughly = Rust impl)
      foo s x y = ..1..             -- only write the argument names and function definition
      bar s z w = ..2..
  
    instance FooBar Banana where
      foo s x y = ..3..             -- only write the argument names and function definition
      bar s z w = ..4..

(..1.. etc denote function definition bodies)

In particular with the traits for operator overloads, the type information is more directly specified in the trait itself

# Detailed design

by example: the proposal is to allow the following (..1.. etc denote the function definition bodies roughly equivalent to the pattern above)

    struct Apple(i32);  struct Banana(String)
    trait FooBar {
        fn foo(&self, x:i32, y:Vec<i32>)->i32;
        fn bar(&self, z:&Vec<String>, w:&String)->Option<String>;
    }

    impl FooBar for Apple {
        fn foo(&self, x,y){ ..1.. }   // no need to repeat :i32 :Vec<i32> ->i32
        fn bar(&self, z,w){ ..2.. }   // no need to repeat :&Vec<String> , :String -> Option<String>
    }
    
    impl FooBar for Banana {
        fn foo(&self, x,y){ ..3.. }   // no need to repeat  :i32 ->i32
        fn bar(&self, z,w){ ..4.. }   // no need to repeat :Vec<String> , :String -> Option<String>
    }
    



# Drawbacks


One potential objection is that you can no longer see the types when you read the impl. 

However, whilst engaged in the compile-edit cycle, the compiler can directly report what the types should be if you make an error; also the programmer *must* have the trait documentation or original source at hand (or gain enough guidance from the error message) in order to actually write the implementation in the first place.


# Alternatives

allowing more general whole-program inference;
another way to streamline the number of mental steps when writing trait 'impls' would be to swap the trait/self-type order as this is more coherent with the function declarations themselves (no need to mentally swap them back and forth), as well as 'trait object casting' (type as trait) and disambiguation syntax <X as Trait>::method_name() ; this would be the subject of another RFC. It would be an alternate declaration syntax complemeting the existing one. impl type as trait {..}, or impl type:trait {} It would complement this suggestion.

# Unresolved questions

What parts of the design are still TBD?
