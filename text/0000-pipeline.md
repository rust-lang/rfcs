- Feature Name: pipeline_operator
- Start Date: 2019-03-06
- RFC PR: 
- Rust Issue: 

# Summary
[summary]: #summary

The pipeline operator provides a simpliest way to write chained function or method calls.

# Motivation
[motivation]: #motivation

The pipeline operator is a famous feature of elixir programming language which greatly solves the
problem of chained function or method calls, providing a good, readable, maintainable and quickly
understandable program work flow. Every program is nothing more than a chain of function calls at
some period of time. To be more functional and precise at what the code writer or program author
is going to do, we can use pipeline.

Having the pipeline operator implemented gives the Rust language an ability to be written in more
functional and state-less manner. The main outcome is the understandable and easy-to-maintain code.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The pipeline operator is a sequence of symbols (it is a token), `=>` which uses left-side expression as
a last argument (by default) to the function (any callable) which name is written to the right of it,
as in this example:

```rust
fn adder(x: u64, y: u64) -> u64 {
    x + y
}

fn multiplier(x: u64, y: u64) -> u64 {
    x * y
}

fn printer(string: String) -> bool {
    println!("{}", string);
    true
}

let is_printed = 1 => adder(3) => multiplier(2) => _.to_string => printer(); // is_printed == true
```

Here, we pass `1` as a second argument to `adder` function, then, the result of `adder` function call to
the `multiplier` function, then we perform a method call `to_string` on the result of `multiplier` function
call, and then we pass it to the `printer` function which prints the value and returns `true`.

Here, `_` is a placeholder for the intermediate value which is got from previous function in the pipeline (chain), `multiplier`.
We can also use a function call or any sort of expression instead of immediate one, `1` in the beginning:

```rust
fn random_number() -> u64 {
    let num = ...;// calculate random number
    num
}

let is_printed = random_number() => adder(3) => multiplier(2) => _.to_string => printer(); // is_printed == true;
```

The result of the last function can also be a callable function (as usual, nothing new), for example:

```rust
fn first_printer(num: u64) {
    println!("Printing using first printer: {}", num);
}

fn second_printer(num: u64) {
    println!("Printing using second printer: {}", num);
}

fn generate_printer(num: u64) -> fn(u64) {
    if num % 2 == 0 {
        first_printer
    } else {
        second_printer
    }
}

let printer = random_number() => adder(3) => multiplier(2) => generate_printer() => _(random_number());
```

Let's see what the example above does:

1. Calculates random number.
2. Passes it to `adder` function as second argument, while first argument is set to `3`.
3. Passes result of `adder(3, random_number())` function call to `multiplier` with first argument set to `2`.
4. Passes result of `multiplier(2, adder(3, random_number()))` to `generate_printer` function.
5. Calls result of `generate_printer(multiplier(2, adder(3, random_number())))` with `random_number()` as argument.

For better understanding, let us simply use set already rules and rewrite the following pipeline in more explicit manner, by
adding `_` to each part of pipeline as a last argument:

```rust
let printer = random_number() => adder(3, _) => multiplier(2, _) => generate_printer(_) => _(random_number());
```

As we see, in case we use `_` as a function, we don't (we can't) use it anywhere else, like
`_(random_number(), _)` or `_(random_number(_))` as it had already been used before.

So, we have added two named concepts: pipeline intermediate value placeholder (`_`) and pipeline operator (`=>`).
We have chosen `_` and `=>` tokens as Rust language is already familiar with them and they are used
in similar contexts, like `_` as unused argument (placeholder) and `=>` in `match` branches (code branching).

A more complex example can be a handler of http request. Let's use the `rocket` framework and
try to write a handler using `pipeline` operator:

```rust
#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;

fn delete_model_with_name(model_name: &str) -> bool {
    true
}

fn mark_sold_model_with_name(model_name: &str) -> bool {
    true
}

fn do_action_with_model_name(action: &str, model_name: &str) -> bool {
    match action {
        "delete" => delete_model_with_name(model_name),
        "mark_sold" => mark_sold_model_with_name(model_name),
        _ => false,
    }
}

fn parse_action_result(success: bool) -> String {
    if result {
        "Success".to_string()
    } else {
        "Failure".to_string()
    }
}

fn perform_action(vendor_id: u64, model_id: u64, action: &str) -> Option<String> {
    Some(vendor_id
    => find_vendor_id_in_database()?
    => _.get_model_name(model_id)?
    => do_action_with_model_name(action)
    => parse_action_result())
}

#[get("/<vendor_id>/<model_id>/<action>")]
fn do(vendor_id: u64, model_id: u64, action: &str) -> String {
    perform_action(vendor_id, model_id, action).unwrap()
}
```

Speaking of complex examples, if we have multiple calls in a chain like this:


```rust
1 => adder(multiplier(3))
```

(this code is not written fully and is incorrect), we put placeholder into the first, top-most
function, `adder` and not into `multiplier`, so the valid code would be:

```rust
1 => adder(multiplier(3, 5), _)
```

There is also a possibility to include pipelines into another pipelines. For example,
these two pipelines can be mixed together:

```rust
let value1 = 1 => adder(3);
let value2 = 2 => multiplier(4);
let value = value1 => adder(value2); // evaluates to 12
```

to:

```rust
let value = 1 => adder(3) => adder(2 => multiplier(4)); // also evaluates to 12
```

or, with proper formatting, to:

```rust
let value = 1
          => adder(3)
          => adder(2 => multiplier(4));
```
You decide how to use it better for you.

As a particular case, if a pipeline does not need to pass `_` to the next function in it, we omit `()`:

```rust
fn print_and_take_by_move(string: String) {
    println!("String: {}", string);
}

let random = 1 => adder(3) => _.to_string() => print_and_take_by_move() => random_number;
```

Using this feature allows us to:
1. Clearly see the path the code goes, from the beginning, till the end.
2. The way the code is written is purely functional, state-less style, which is
also a style rust code is usually written in.
3. As this code is purely functional and state-less, we can't see any concurrency issues within this code, but
it still can be if we use it with methods, but here we again work with just functions and methods, just in
slightly different calling style, so the rustc compiler will perform its checks anyway.

So, basically, it is just another way of writing the same code, just more functional one.

The placeholder (`_`) can be placed in arbitrary place in the callable, explicitly, but the compiler,
if it was not specified, places it implicitly as a last argument:

```rust
fn adder(x: u64, y: u64, z: u64) -> u64 {
    x + y + z
}

let value1 = 1 => adder(3, 4); // value1 == 8
let value2 = 1 => adder(3, 4, _); // value2 == 8, the same as above
let value3 = 1 => adder(3, _, 4); // value3 == 8, the placeholder is explicitly specified to be second argument.
```

By now, we may write the code in two similar ways: sequential method calls and pipelines:

```rust
fn parse_name(html: &str) -> Option<String> {
    Some(
        select::document::Document::from(html)
            .find(select::predicate::Class("name"))
            .next()?
            .text()
            .trim()
            .to_owned(),
    )
}
```

```rust
fn parse_name(html: &str) -> Option<String> {
    Some(
        select::document::Document::from(html)
        => _.find(select::predicate::Class("name"))
        => _.next()?
        => _.text()
        => _.trim()
        => _.to_owned()
    )
}
```

Which is just the same thing and there is no any difference, but with pipelines
we may also use non-member functions and it is not necessary to use `self`
(objects with methods) at all, and also we can mix them as how we want:

```rust
fn find_by_name(document: &select::Document) -> Option<&str> {
    Some(document.find(select::predicate::Class("name"))
            .next()?
            .text())
}

fn trimmed_and_owned(s: &str) -> Option<String> {
    s.trim().to_owned()
}

fn parse_name(html: &str) -> Option<String> {
    select::document::Document::from(html)
    => find_by_name()?
    => trimmed_and_owned()
}
```

Which is again, much similar and slightly different at the same time. Pipelines are
just a bit more subtle and may allow more things than just OOP-style. Using pipelines,
we can mix calling functions and calling objects methods in any possible way.

The error messages are the same as for everything else (lifetimes, types and everything else),
except these things:

1. Compiler checks that placeholders `_` have an acceptable type to be the last (or specified) argument of a function or a method.
2. The first value can be an immediate value or and expression, but all next expressions must be callable.
3. We may advice code like `f1(f2(f3()))` to migrate to the one using pipepeline, inversing the order:

```rust
f3() => f2() => f1()
```

One more time: though, the effect of pipeline can be achieved using traits and OOP, pipeline can also use
functions and perform sort of placeholder magic, like using intermediate value as the last argument for next
function in pipeline or not and specify it position explicitly.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

We do not need to add any new entities into the parser, we just need to modify `=>` and `_` parser
logic so that it becomes possible to use them in another context: we need to add a few
context-dependent rules to the rust grammar which will allow us to use these tokens in another context.

```
pipeline_callable = function_name | function
pipeline = evaluatable_expression + pipeline_callable.*
```

Pipeline must be evaluated from the beginning till the end, passing the return values to each
consecutive item in the pipeline.

We should be careful when parsing pipelines inside `match` statements:

```rust
let value = match string_ref {
    "a" => _.trim().to_owned(),
};
```

```rust
let value = match string_ref {
    "a" => => String::trim() => String::to_owned(),
};
```

Either attempts to implementing pipeline inside the `match` statement will be difficult
or it could be not so readable (arguably), this is a topic for discussion.

```
f3() => f2() => f1()
```

This, according to the grammar above, can be simply encoded into existing AST:

1. Call to `f3` is performed without arguments, as usual.
2. Call to `f2` is made by setting the argument to be result of `f3` on step 1.
3. Do 2. with `f1`, last function in the pipeline, returning its value to a holder,
which can be either a `let` binding or a `return` statement, according to existing
rust rules.

We can use libsyntax/ast.rs/ExprKind::{Call, MethodCall} for the implementation.

# Drawbacks
[drawbacks]: #drawbacks

Having pipelines may confuse people a little bit, mainly with making them asking
questions like "why do we need this if we have OOP and methods", however, this has
already been answered above. Usage of `=>` and `_` in different contexts will seem
unusual and also may confuse people, as after having this implemented, they will may
have to know knew grammar rules (depends on the implementation), but it is supposed to
be fully backward-compatible.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

For the chaining methods, the only alternative is to create traits and implement them
everywhere you need and/or want, however, chaining functions in more or less readable way
is impossible right now, as well as state-less and functional style using pipelines,
and placing placeholders into arbitraty positions. The design must be somewhat similar to what
[`pipeline`](https://crates.io/crates/pipeline) and Elixir language provide, also
taking advantage of rust AST and access to the whole internal rust parser.

If we do not implement this, people who is really interested in this, will continue to
use `pipeline` macros from the equally-named crate which is not really convenient and
it can't implement all the features discussed in this RFC, such as placeholders, at the
very least.

People from Elixir community are really proud of having pipe functionality in the language.
Having pipelines implemented in Rust, will give us more subtle ways for chaining and
writing the code, as well as it can lead to more readable, clean and understandable code.

# Prior art
[prior-art]: #prior-art

This feature exists in [Elixir](https://elixir-lang.org/getting-started/enumerables-and-streams.html#the-pipe-operator)
and [F#](https://theburningmonk.com/2011/09/fsharp-pipe-forward-and-pipe-backward/) languages.
The main inspiration for this RFC was taken from Elixir. Somewhat similar also exists in
[Clojure](https://clojuredocs.org/clojure.core/-%3E%3E), which can also be found
[here](https://clojure.org/guides/threading_macros).

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Which token to use for the pipeline operator? `=>`, `->`, `|>` are seem to be most reliable,
but as `=>` is already used in Rust, I thought that it can be used here as well.
- Which token to use for the placeholder? `_` is mainly used in C++ and Rust languages, so I think,
it is already chosen to mean a placeholder, we can just use it for intermediate value placeholder.
- Do we allow using `pipeline` inside `match`?
- Do we need to be able to pass intermediate value (`_`) to arbitrary positions? It sounds a good
idea to me, but it may be not that easy to implement (I don't know rustc). It could also be
used for something else in the language, for example in bindings:

```rust
fn x_more(x: u64, y: u64) -> u64 {
    x * y
}

fn twice_more(x: &u64) -> u64 {
    x_more(2, *x)
}

fn main() {
    let v = vec![1, 2, 3];

    println!("Value: {:?}", v.iter().map(twice_more).collect::<Vec<u64>>());
    println!("Value: {:?}", v.iter().map(|x| x_more(2, *x)).collect::<Vec<u64>>());
}
```

Last `println!` could make usage of new placeholder semantics and could be rewritten as:

```rust
println!("Value: {:?}", v.iter().map(x_more(2, _)).collect::<Vec<u64>>());
```

without involving writing lambda for intermediate calculation which is just passing through.

# Future possibilities
[future-possibilities]: #future-possibilities

As already discussed, placeholder semantics could be extended for other
places in the code with some grammar changes.
