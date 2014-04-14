- Start Date: 2014-04-14
- RFC PR #:
- Rust Issue #:

# Summary

Current Rust makes no attempt to be safe against integer overflow, and for the most part simply reuses the same integer semantics C has. While this doesn't necessarily cause memory unsafety, it still results in a language such that the programmer cannot be fully confident that programs written in it will work properly.

This RFC discusses a few options, and then proposes one, explaining a new idea to make it workable.

The goal is mostly to present this idea, and not to provide a final design.

Note that some of the formulas may be wrong, as they have been minimally reviewed, but hopefully it's easy to get the intuition behind them.

# Alternatives

## Change nothing

Changing nothing has the issues that the current u32, i32, etc. types are used as if they were integers, but they instead mostly behave like elements of the Z/(2^K)Z finite ring.

Among other issues, the total order (improperly) defined on them is not compatible with arithmetic operations, which results in highly non-intuitive and inappropriate behavior, such as ```a > b``` not implying ```a + k > b + k``` and ```a > 0``` and ```b > 0``` not implying ```a + b > 0``` like a non-programmer would expect.

## Fail or return None on overflow

The simplest solution is to detect overflow and fail, or perhaps return None after changing operations to return Option.

However, this means that the associative property is lost: ```((-BIG) + BIG) + BIG``` has value ```BIG```, but ```(-BIG) + (BIG + BIG)``` fails due to overflow.

Furthermore, the programmer has to manually choose integer types large enough to not overflow without help from the compiler.

Code-generation wise, there are branches everywhere and lots of code to handle errors and unwind the stack.

## Make return types large enough

The only solution that preserves all mathematical properties is to make operations return a type which is large enough to represent all possible results given the input types.

For example, ```a + a``` where ```a``` is an ```u32``` would not have ```u32``` type, but rather ```u33```, or otherwise a large enough type to represent both ```0``` and ```2^33 - 2```.

The immediate issue of this is that, with a naive design, the statement ```a = a + 1``` is no longer valid, because the type of ```a + 1``` is never compatible with the type of ```a```.

In other words, the type system would be incomplete relative to the subtyping relation, because we introduced a subtyping relation with infinitely long chains, but no supremum defined.

The solution to this is to define a supremum, and the easiest option is to introduce and use big integers: whenever the type inferencer sees a cycle like ```a = a + 1```, it deduces ```a``` to be of big integer type, which could be represented with an optimization for inline small values

## Make return types large enough, avoid bigints where possible

The above option results in simple integer-indexed loops requiring a big integer type, which is undesirable.

This can be solved as described later, by observing that the number of instructions ever executed in a program is bounded, and thus that a 64-bit or 128-bit integer is enough to serve as a loop counter which is only increment by 1, and desigining the type system with this assumption in a more general setting.

# Detailed design

## Introduction: range types

Formalizing what we said above, the proposal is to introduce range types ```[a .. b]```, where ```a``` and ```b``` are arbitrarily large signed integer compile-time constants, which represent integers ```x``` such that ```a <= x <= b```. These are represented as N-bit integers, where N is the smallest possible value.

In addition, we introduce big integers, called ```[..]```, and half-range types ```[a..]``` and ```[..b]```, which represent respectively all integers, all integers ```>= a``` and all integers ```<= b```. These are pointer-sized, represented as an enum variant consisting of copy-on-write Arcs of machine word digits, and a variant for small numbers (where the discriminator is stored by stealing the MSB or LSB of the pointer).

Types like ```u8``` become aliases for ```[0 .. 255]``` and so on.

Operations can be defined naturally based on integers:
- ```[a .. b] + [c .. d] -> [a + c .. b + d]```
- ```-[a .. b] -> [-b ... -a]```
- For *, etc.: ```[a .. b] op [c .. d] -> [min(a op c, b op c, a op d, b op d), max(a op c, b op c, a op d, b op d)]```

## Runtime-closed range types

To solve the type supremum issue, we introduce "runtime-closed range types", denoted by ```[a .. b]*```.

The observation is that a program running for ```Y``` years on a machine with ```C``` CPUs operating at ```F``` GHz and capable of issuing ```I``` instructions per cycle can execute at most ```R = C * F * I * Y * 60 * 60 * 24 * 366 * 10^9```  instructions, and each operation requires at least one instruction (note: this might require limits on the optimizer).

Hence, if a variable is initialized to a value of type ```[a .. b]```, and then incremented by values of type ```[a .. b]```, then its value always satisifies ```min(a, a * R) <= x <= max(b, b * R)```.

To denote such variables, we introduce the type ```[a .. b]*```, which unlike normal range integers, is **NOT COPYABLE** and **NOT CLONEABLE**.

which supports the following operations:
- ```new([a .. b]) -> [a .. b]*```
- ```[a .. b]* + [c .. d] -> [min(a, c) .. max(b, d)]*```
- ```[a .. b]* + [c .. d]* -> [min(a, c) .. max(b, d)]*``` as long as operands are distinct, and both operands are destroyed and not reused

It is easy to prove that it is not possible to overflow a ```[min(a, a * R) .. max(b, b * R)]``` representation by repeating those operations up to ```R``` times.

For instance, given a simple loop that increments a variable by 1, then the ```[0 .. 1]*``` is suitable for the variable, and can be stored in a 64-bit or 128-bit integer depending on the value we assign to ```R```

## The value of R

The formula ```R = C * F * I * Y * 60 * 60 * 24 * 366 * 10^9``` can be recast as ```log2(R) = log2(C) + log2(F) + log2(I) + log2(Y) + log2(60 * 60 * 24 * 366 * 10^9)``` where the latter addend has value 54.8117963842085.

A conservative estimate of current and likely future hardware is that we have at most 4096 CPUs operating at 8 GHz and capable of issuing 8 instructions per cycle, with the program running for a million years.

This gives ```log2(R) = 55 + 12 + 3 + 20 = 90```, which means that ```u32*``` can be stored in a 128-bit integer, and ```u64*``` in a 192-bit integer.

A less conservative estimate can give ```log2(R) = 64```, reducing the latter to a 128-bit integer too, and allowing to store ```[0..1]*``` (the "loop counter type") in a 64-bit integer.

## More on runtime-closed range types

First of all, we observe that runtime-closed range types are not closed under multiplication and other operations, and instead have this rule:
- ```[a .. b]* * [c .. d] -> [min(a * c, b * c, a * d, b * d), max(a * c, b * c, a * d, b * d)]*```

In addition, they are not closed under non-destructive self-addition, which follows this rule:
- ```[a .. b]* + [c .. d]* -> [a + c .. b + d]*``` when the operands are reusable

Note that if we didn't have these restrictions, we could use such addition to double the value of a runtime-closed range type, and thus rapidly cause it to overflow by successive doublings.

However, it is possible to close them under addition, by defining "higher runtime-closed range-types" ```[a ..b]**```, ```[a .. b]***```, which can also be denoted by ```[a .. b]^e``` (notation is not final), where of course ```[a .. b]^0``` is the same as ```[a .. b]```. ```[a .. b]^e``` is represented as ```[min(a, a * R^e) .. max(b, b * R^e)]```

This allows to generalize and complete the rules to:
- ```new([a .. b]^e) -> [a .. b]^(e + 1)```
- ```unsafe new([min(a, a * R^e) .. max(b, b * R^e)]^f) -> [a .. b]^(e+f)```
- ```[a .. b]^e + [c .. d]^f -> [min(a, c) .. max(b, d)]^max(e+f)``` where ```e != f```
- ```[a .. b]^e + [c .. d]^e -> [min(a, c) .. max(b, d)]^e``` when ```e > 0``` and as long as operands are distinct, and are both operands are destroyed and not reused
- ```[a .. b]^e + [c .. d]^e -> [min(a, c) .. max(b, d)]^(e+1)``` otherwise
- ```[a .. b]^e * [c .. d]^f -> [min(a * c, b * c, a * d, b * d), max(a * c, b * c, a * d, b * d)]^(e+f)```
- ```sum([a_i .. b_i]^e_i) = [min(a_i)..max(b_i)]^max(e_i)``` where ```a_i``` and ```b_i``` are bounded, ```e_i``` is bounded, only a finite number of ```e_i``` are equal to ```max(e_i)```, and the operands are all distinct and destroyed
- ```sum([a_i .. b_i]^e_i) = [min(a_i)..max(b_i)]^(max(e_i)+1)``` where ```a_i``` and ```b_i``` are bounded, ```e_i``` is bounded, otherwise
- ```sum([a_i .. b_i]^e_i) = [..]``` otherwise
- ```prod([a_i .. b_i]^e_i) = [..]```

Note that we could in principle define "hyper-runtime-closed" integers rather than use big integers as supremums of chains of higher and higher runtime-closed integers, since such chains have, at runtime, length at most R. However, such integers would be able to represent R^R and thus require more than R or 2^64 bits of storage, which is obviously not practical, so big integers are the only practical option.

## Type inference algorithms

It seems it should be possible to infer runtime-closed range types at least in simple cases, and hopefully it is possible to have a proper algorithm worknig on all cases.

## Modular types

In addition to the integer-based types described above, which are subsets of the ring of integers ```Z```, we should also offer types modelling the finite rings ```Z/nZ```.

This is much easier since such rings are finite and thus directly representable, and thus we can just have a ```[ % n]``` type representing ```Z/nZ```; when n is a power of a power of two, this reduces to the familiar 8-bit, 16-bit, 32-bit etc. integers.

They would behave like ```u32``` and so on, except that TotalOrd and comparisons are not provided, since they make no sense; there would be instead non-automatic conversions to the ```[0 .. n]``` range type, the ```[-ceil((n-1)/2) .. floor((n-1)/2)]``` range type (which are equivalent to converting to ```uN``` and ```iN``` where N is a power of two), and ```[k .. k + n - 1]``` range types as well as the obvious homomorphism in the other direction.

# Impact on the language

## By-reference vs by-value add

Runtime-closed range types, which are non-copyable, support addition, but only in-place addition (or out-of-place addition with by-value operands), and adding them to each other requires to take both operands by value.

This unfortunately means that the addition operator needs to be overloaded in those semantics, and 

## Generalizing over range types

This will require integer generic parameters and compile-time function evaluation, plus return type inference limited to integers (ideally one would specify the return type as "int?" and the compiler would infer it).

## Optimizer issues

A too clever optimizer will destroy our assumptions.

For example, consider this code:
```let mut i = 0; while i < (1 << 1024) {++i}```

Without optimization, a 64-bit or 128-bit integer is enough to store ```i```, since the computer is not fast enough to overflow such an integer before it stops operating.

However, if the optimizer replaces that with ```let i = (1 << 1024)``` then this is no longer true.

Hence, this requires to change LLVM so that this doesn't happen (e.g. by adding a marker intrinsic that prevents such loop elimination).

## Can this be done in a library?

Probably not, since it requires special type inference behavior due to the unique subtyping relationship with infinitely long chains.

# Evaluating this design

## Literature?

What I called "runtime-closed range types" looks like a well-known concept, so I'm pretty sure there is a commonly used name in the type theory literature, and it would be nice to use it.

## How to teach newcomers all of this?

The weakest point of the proposal is how to teach all this to programmers, since no popular language has anything like this.

However, this design has the advantage that all integer expressions have the same meaning as they have in mathematics, eliminating the need to think about overflows and representations, and is thus far more intuitive to non-programmers.

On the other hand, the concept of runtime-closed integers will be novel to any programmer, and might require some long help texts in compiler error messages, as well as dedicated tutorials.

Overall, the positive side is that this can be marketed as "Rust provides the most advanced protection from integer overflow bugs without forcing slow bigints" and sold as something awesome.

## Do we really need all this?

As far as I can tell, this is the only design that satisfies these two intuitively obvious properties:
1. The value of expressions is the same as defined by mathematics on integers, and all properties that hold on mathematical integers (i.e. the ring ```Z```) hold on integers in the language
2. It is possible to write a ```for(i = 0; f(i); i += const)``` loop without using special syntax, with the type system preventing overflow and without representing i as a big integer

# Backward compatibility and implementation plan

Unfortunately, this is both not backwards compatible at all and very complex to implement.

If we want this, the best option is to first implement a limited version that omits runtime-closed range integers and instead uses big integers with a small value optimization, and also possibly uses ```uN``` types instead of ranges, by "rounding up" ranges to the next ```uN``` type (where N could also be restricted to powers of two).

Range types, half-range types and runtime-closed range integers can then be added later compatibly.

