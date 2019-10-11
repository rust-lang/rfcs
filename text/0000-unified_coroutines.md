- Feature Name: `unified_coroutines`
- Start Date: 2019-10-09
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Unify the `Generator` traits with the `Fn*` family of traits. Add a way to pass new arguments upon each resuming of the generator.

# Motivation
[motivation]: #motivation

The generators/coroutines are extremely powerful concept, but its implementation in Rust is severely limited, and  its usage requires workarounds to achieve useful patterns. The current view of generators is also extremely disconnected from similar concepts, which already exist in the language and the standard library;


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Consider a function. In the olden days, also called a subroutine. This concept is at the core of every programming language, since it is extremely useful. The subroutine has a single entrypoint and a single exit point. Both of these points provide 
an interface, which is used to transfer data. Upon entering, caller can pass data into the subroutine and upon exiting, the subroutine can return data back to the caller.

Many programming languages(including Rust) also adopted more general concept, callled coroutine. The coroutine , or `Generator` in Rust terms differs from function/subroutine in a single way. It allows the coroutine to `suspend` itself, by storing its state, and `yield`ing control back to the caller, along with data. The caller can then repeatedly pass more data back into the coroutine, and `resume` it. This back-and forth communication is an extremely useful tool for solving a wide class of problems.

Except this is not the truth for Rust's coroutines. The `Generator`, as it  was introduced in order to provide a tool for implementing `async-await`, does not provide this functionality. In order to implement async-await feature, the generators were implemented in their most basic form. And in this form, Generators can't accept arguments, and are  connected to closures only in their syntax, and not their representation in the type system.

These issues severely lessen the usability of the generator feature, and are not difficult to solve.

Current generators take the form of a closure, which contains at least a single yield statement.

```rust
let name = "World";
let done = "Done";
let gen = || {
    yield "Hello";
    yield name;
    return done;
};
```
And are used by calling the `resume` method on the generator.
```rust
println!("{}", gen.resume());
println!("{}", gen.resume());
println!("{}", gen.resume());
```

Which results in 
```rust
Yielded("Hello")
Yielded("World")
Finished("Done")
```

This RFC proposes the ability of a generator to take arguments with a syntax used by closures.
```rust
let gen = |name: &'static str| {
    yield "Hello";                
    yield name;
    return name;
}
```

Then, we propose a way to pass the arguments to the generator in the form of a tuple.
```rust 
println!("{}", gen.resume(("Not used")));
println!("{}", gen.resume(("World")));
println!("{}", gen.resume(("Done")));
```
Which would also result in:
```rust
Yielded("Hello")
Yielded("World")
Finished("Done")
```
Or expanded with values in between:
```rust
let gen = |name: &'static str| {
    // name = "Not used"
    yield "Hello";      
    // name = "World
    yield name;
    // name = "Done"
    return name;
}
```
Notice that in this example the argument to first resume call was unused, and the generator yielded only the value which was passed to the second resume. This behavior is radically different from the first example, in which the name variable from outer scope was captured by generator and yielded with the second `yield` statment;

The value of `name` in previous example is `"Hello"` between its start, and first `yield`, and `"World"` between first and second `yield`. And assumes the value of `"Done"` between second yield and a the `return` statement

The behavior, in which a generator consumes a different value upon each resume is currently not possible without introducing some kind of side channel, like storing the expected value in thread local storage, which is what the current implementation of async-await does.

There are possible other syntaxes to denote the fact that the value assigned to `name` is different after each `yield`, but we believe that the simplest syntax, which is used in the example above, is in this case the best.

### Alternative syntaxes
There are several other possible syntaxes, but each one has its problems. Several are outlined here

1. Assigning into the name to denote that the value is changed
```rust
let gen = |name :&'static str| {
    name = yield "hello";
    name = yield name;
}
```
We are unable to denote assignment to multiple values at the same time, and would therefore have to revert to using a tuple and possibly some kind of destructuring assignment. The problem is the non-coherence between receiving arguments for the first time,
and upon multiple resumes. This however is only a syntactic inconvenience, and as such we think that this approach is a very good possible choice.

If we could perform tuple destructuring when assigning:
```rust
let gen = |name :&'static str, val : i32| {
    name, val = yield "hello";
    or
    (name, val, ) = yield name;
}
```
Or if we could 'pack' the arguments into tuple:
```rust
let gen = |..args| {
    args = yield "hello";
    args = yield args.name;
}
```
This syntactic choice would probably be the better one. Making the change of the `name` explicit. However, we do not want to introduce a behavior, which would further separate generators from closures.

2. Creating a new binding upon each yield
```rust
let gen = |name :&'static str| {

    let (name, ) = yield "hello";
    let (name, ) = yield name;
}
```
We are creating a new binding upon each yield point, and therefore are shadowing earlier bindings. This would mean that by default, the generator stores all of the arguments passed into it through the resumes. Another issue with these approaches is, that they require progmmer, to write additional code to get the default behavior. In other words: What happens when user does not perform the required assignment ? Simply said, this code is permitted, but nonsensical:
```rust
let gen = |name: &static str| {
    yield "hello";
    let (name, ) = yield name;
}
```
Another issue is, how does this work with loops ? What is the value assigned to `third` in following example ?
```rust
let gen = |a| {
    loop {
        println!("a : {:?}", a);
        let (a,) = yield 1;
        println!("b : {:?}", a);
        let (a,) = yield 2;
    }        
}
let first = gen.resume(("0"));
let sec = gen.resume(("1"));
let third = gen.resume(("2"));
```


3. Introducing a 'parametrized' yield;
```rust
let gen = | name: &'static str| {
    yield(name) "hello";
    yield(name) name;
}
```
Introduces a new concept of a parametrized statement, which is not used anywhere else in the language, and makes the default behavior store the passed argument inside the generator, making the easiest choice the wrong one on many cases.


The design we propose, in which the generator arguments are mentioned only at the start of the generator most closely resembles what is hapenning. And the user can't make a mistake by not assigning to the argument bindings from the yield statement. Only drawback of this approach is, the 'magic'. Since the value of the `name` is magically changed after each `yield`. But we pose that this is very similar to a closure being 'magically' transformed into a generator if it contains a `yield` statement.

![magic](https://media2.giphy.com/media/12NUbkX6p4xOO4/giphy.gif)

But, like shia himself, this point is controversial, and is the main issue that prevented us from adding generator arguments to the language in the first place.

The introduction of this implicit behavior will require additional cognitive load for new users when learning this feature. However, the behavior of Generators without arguments is unchanged, and therefore this change does not impose this cost upfront, making it possible to introduce the more complex behavior in progressively more complex examples.

Another issue posed by our approach is lifetimes of the generator arguments.
```rust
let gen = |a| {
    loop {
        println!("a : {:?}", a);
        yield 1;
        println!("b : {:?}", a);
        yield 2;
    }        
}
let first = gen.resume(("0"));
let sec = gen.resume(("1"));
let third gen.resume(("2"));
```
In the loop example, the lifetime of `a` is different upon each resuming of the generator, and in the case of generator resuming from the second yield point, the lifetime starts at the end of the generator, and ends at the beginning, which is not expected.
However, if we take into consideration the form generators take when they are transformed into MIR, in this representation the lifetimes the arguments are no different than they would be in a manual `match` based implementation [See addendum](addendum-samples)

### Standard library changes

This change would result in following generator trait.

```rust
pub trait Generator<Args> {
    type Yield;
    type Return;
    fn resume(self: Pin<&mut Self>, args: Args) -> GeneratorState<Self::Yield, Self::Return>;
}
```

While the RFC does not deal with the lifetimes of the arguments, the similarity of the modified `Generator` trait with the existing `Fn*` traits suggests that rules which currently apply to closures will also apply to generators. [More info later](theoretical-basis).

### Use cases:
1. Futures generated by async-await. The current implementation of async futures requires the use of thread-local storage in order to  pass the `task::Context` argument into underlying futures. This imposes small, but not zero overhead, which would be removed by this RFC.

2. Protocol state machines - When a user wants to implement a state machine in order to correctly represent a network protocol, 
the ususal approach is to create a `State` enum, and upon every state machine transition `mem::replace`the current state with a default one, and perform a match, to possibly generate a new state, which is then stored into the place of original state.

Example:
Consider an implementation of following state machine:
```
     a                 b
  +-----+           +----+
  |     |           |    |
  |  +--v--+  b  +-----+ |
  |  |     +----->     | |
  +--+  F  |     |  S  <-+
     |     <-----+     |
     +-----+  a  +-----+

```
How the similar state machines are implemented today:
```rust
enum State { Empty, First, Second }

enum Event { A, B }

fn machine(state: &mut State, event: Event) -> &'static str {
    match (mem::replace(state, State::Empty), event) {
        (State::First, Event::A) => {
            *state = State::First;
            return "Action First(A)";
        }
        (State::First, Event::B) => {
            *state = State::Second;
            return "Action First(B)";
        }
        (State::Second, Event::A) => {
            *state = State::First;
            return "Action Second(A)";
        }
        (State::Second, Event::B) => {
            *state = State::Second;
            return "Action Second(B)";
        }
    }
}
```
How we could implement similar state machines after this RFC is accepted:
```rust
let machine = |action| {
    loop {
        // First state
        while action == Action::A {
            yield "Action First(A)";
        }
        // Second state
        yield "Action First(B)";
        while action == Action::B {
            yield "Action Second(B)";
        }
        yield "Action Second(A)";
    }
};
  ```
To see how would the generated code change, check out [This sample](addendum-samples)

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation
The proposed design modifies the Generator trait, and the MIR which is generated for generators.

The proposed changes to generator trait are pretty straightforward and do not change its properties significantly. However, they allow us to unify the language view of a closure with a that of a generator.

The implementation of MIR generation will be more complex, and the author of this RFC is unable to properly gauge the amount of work that will be required.

### Theoretical basis
[theoretical-basis]: #theoretical-basis
The goal of this RFC is to unify the Rust's implementation of Generators, and the theoretical concept of 'Coroutine' as a generalization of the 'Subroutine/Function', and with this unification also comes the unification of rusts Generators and Functions for free.

The current implementation serves its purpose (at least for async-await), but is sevely limited and disjointed from the rest of the language. By introducing the arguments in the same way that they are represented in proposed `Fn` traits, we can bring these 2 concepts more closely together. Additinal info about this unification can be found in [future-possibilities], but if the generator arguments are introduced in proposed form, the future modifications will be just syntax improvements, and will not change the semantics in a significant way.

Example of current `Fn*` traits:
```rust
pub trait FnOnce<Args> {
    type Output;
    fn call_once(self, args: Args) -> Self::Output;
}
pub trait FnMut<Args> : FnOnce<Args> {
    fn call_mut(&mut self, args: Args) -> Self::Output;
}
```
And a proposed `Generator` trait:
```rust
pub trait Generator<Args> {
    type Yield;
    type Return;
    
    fn resume(self: Pin<&mut Self>, args: Args) -> GeneratorState<Self::Yield, Self::Return>;
}
```
Considering the closeness of these 2 concepts, it might be beneficial to utilize a different form of the `Generator` trait.
```rust
pub trait FnGen<Args> : FnOnce<Args, Output=GeneratorState<Self::Yield,Self::Return>> {
    type Yield;
    type Return;
    
    fn call_resume(self: Pin<&mut Self>, args: Args) -> Self::Output;
}
```
It might also be beneficial to introduce a new trait for denoting closures which must be pinned between invocations:
```rust
pub trait FnPin<Args> : FnOnce<Args> {
    fn call_pin(self: Pin<&mut Self>, args: Args) -> Self::Output;
}
```
And either utilize the `FnPin` as a `FnGen/Generator` supertrait, or disregard the `FnGen/Generator` trait completely 
and utlize generators as a trait alias for a `FnPin<Args, Output = GeneratorState<Self::Yield, Self::Return>>`

# Drawbacks
[drawbacks]: #drawbacks

1. Increased complexity of implementation of the Generator feature. 

2. If we only implement necessary parts of this RFC, users will need to pass empty tuple into the `resume` function for most common case, which could be solved by introducing a trivial trait
```rust
trait NoArgGenerator : Generator<()> {
    fn resume(self: Pin<&mut Self>) ->  GeneratorState<Self::Yield, Self::Return> {
        self.resume_with_args(())
    }
}
```
If we introduce this trait, and rename the original `Generator::resume` method to `Generator::resume_with_args`, the existing behavior will not change. But we think this approach is **WRONG**, and the Generator trait should stay with the changes proposed above (`resume`/`call_resume` accepting a tuple). The rationale for this decision is provided in [future-possibilities] section.

3. Need to teach the special interaction between generator arguments and the yield statement.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Rationale:
- This introduces the missing piece into the generators, rounding out an unfinished feature.
- Unifies 2 parts of the language, improving laungage coherence
- Allows implementations of new patterns, such as complex state machines, which are an extremely useful tool in several areas, eg. network protocol implementations.

Alternatives:
- Only current alternative is storing required information inside side channels such as `Rc<RefCell<Args>>` 
or a thread local storage, which introduces runtime overhead and requires `std`. Proposed design is `no_std` compatible.
- The proposed syntax could be changed, but from the explored options, we pose that the simplest syntax is best, even though it introduces new semantics.
- Implement radically new syntax just for generators
- Leave the generator as is, leaving it disconnected from the rest of the language.
- Remove generators completely 

# Prior art
[prior-art]: #prior-art

- Pre-RFC published by different author on rust-internals forum [link](https://internals.rust-lang.org/t/pre-rfc-generator-resume-args/10011/5)
   
  Explored the design space and proposed a basic design of the modified `Generator trait`. The described approach solved only the 'Generator resume arguments' part of this RFC, and did not attempt to unify generators with closures, which resulted in unnecessarily complex design, which kept the generators as a separate concept, and even introduced another layer of complexity in form of a trait alias for generators with/without arguments. But nonetheless, reading this discussion was an invaluable source of ideas in this design space.
  
- Work on implementing futures which wouldn't require TLS: [link](https://github.com/rust-lang/rust/issues/62918)

- Python & Lua coroutines - They can be resumed with arguments, with yield expression returning these values [usage](https://www.tutorialspoint.com/lua/lua_coroutines.htm). 
  
  These are interesting, since they both adopt a syntax, in which the yield expression returns values passed to resume. We think that this approach is the right one for dynamic languages like Python or lua but the wrong one for Rust. The reason is, these languages are dynamically typed, and allow passing of multiple values into the coroutine. The design proposed here is static, and allows passing only a single argument into the coroutine, a tuple. The argument tuple is treated the same way as in the `Fn*` family of traits. 

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Proposed syntax: Do we somehow require assignemnt from yield expression(As outlined by [different pre-rfc](https://internals.rust-lang.org/t/pre-rfc-generator-resume-args/10011)), or we do we specify arguments only at the start of the coroutine, and require 
explanation of the different behavior in combination with the `yield` keyword explanation?

- Do we unpack the coroutine arguments, unifying the behavior with closures, or do we force only a single argument and encourage the use of tuples?

- Do we allow non `'static` coroutine arguments? How would they interact with the lifetime of the generator, if the generator moved the values passed into `resume` into its local state?

- Do we adopt the `FnGen` form of the generator trait and include it into the `Fn*` trait hierarchy making it first class citizen in the type system of closures?

- Do we introduce the `FnPin` trait into the `Fn*` hierarchy and make `FnGen/Generator` just an alias?

# Future possibilities
[future-possibilities]: #future-possibilities
One of the areas of improvement, is interaction with generators. Currently the generator takes the form of a closure, but the resume method is called like a trait method. One of the future improvements would be making generators callable like closures, since this RFC recomments their unification with closures.

So, current syntax with 2 arguments looks like:
```rust
let mut gen = Pin::box(|name| {
    yield ("Hello", name);
    yield ("Bye", name);
});
gen.resume(("unused", "arg"));
gen.resume(("world", "!"));
```
Would become 
```rust
let mut gen = Pin::box(|name| {
    yield ("Hello", name);
    yield ("Bye", name);
});
let _ = gen("unused", "arg");
let second = gen("world", "!");
```

And this is would fully unify the interface provided by generators with the one provided by closures, but is intertwined with other issues, like [Fn traits](https://github.com/rust-lang/rust/issues/29625) and thus would block the current RFC. Therefore we propose generator syntax accepting the multiple arguments:
```rust
let gen = |a,b,c| {
  yield a;
  yield b;
  yield c;
}
```
But the `Generator::resume` method accepting tuple of arguments, which are unpacked by the compiler.
```rust
let a = gen.resume(("Why", "Hello", "There !"));
let b = gen.resume(("Why", "Hello", "There !"));
let c = gen.resume(("Why", "Hello", "There !"));
```
Since this approach most closely resembles current approach to Function traits, and since we consider the concept of a Coroutine/Generator a generalization of the Function concept we aim to unify these 2 concepts. Then, the further work that deals with closures could be transparently applied to generators.

However, the main goal of this RFC is to provide a basis for these decisions and discussions after the `FnGen/Generator` trait is introduced, and the ability of generators to accept arguments is implemented.

# Addendum: samples
[addendum-samples]: #addendum-samples

The Generator concept is transformed into a state machine on the MIR level, which is contained inside a single function. The current implementation is transformed to something like this:

```rust
let captured_string = "Hello";
let mut generator = {
    enum __Generator {
        Start(&'static str),
        Yield1(&'static str),
        Done,
    }

    impl Generator for __Generator {
        type Yield = i32;
        type Return = &'static str;

        fn resume(mut self: Pin<&mut Self>) -> GeneratorState<i32, &'static str> {
            use std::mem;
            match mem::replace(&mut *self, __Generator::Done) {
                __Generator::Start(s) => {
                    *self = __Generator::Yield1(s);
                    GeneratorState::Yielded("Hello")
                }

                __Generator::Yield1(s) => {
                    *self = __Generator::Done;
                    GeneratorState::Complete(s)
                }

                __Generator::Done => {
                    panic!("generator resumed after completion")
                }
            }
        }
    }

    __Generator::Start(captured_string)
};
```

After implementing the changes in this RFC, the generated code could be approximated by this:

```rust
let captured_string = "Hello"
let mut generator = {
    enum __Generator {
        Start(&'static str),
        Yield1(&'static str),
        Done,
    }

    impl Generator<(&'static str,)> for __Generator {
        type Yield = i32;
        type Return = &'static str;

        fn resume(mut self: Pin<&mut Self>, (name,) : (&'static str,)) -> GeneratorState<i32, &'static str> {
            use std::mem;
            match mem::replace(&mut *self, __Generator::Done) {
                __Generator::Start(s) => {
                    *self = __Generator::Yield1(s);
                    GeneratorState::Yielded("Hello")
                }

                __Generator::Yield1(s) => {
                    *self = __Generator::Done;
                    GeneratorState::Complete(name)
                }

                __Generator::Done => {
                    panic!("generator resumed after completion")
                }
            }
        }
    }

    __Generator::Start(captured_string)
};
```
