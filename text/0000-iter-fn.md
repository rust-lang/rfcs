- Feature Name: iter-fn
- Start Date: 2018-04-17
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

Make trivial to have an iterator by just writing a function/closure.
# Motivation
[motivation]: #motivation

We are doing this in order to make trivial having an iterator by just writing a function/closure.
In other words, the objective of this RFC is to reduce the boilerplate when writing simple iterators.
We get something similar from python's or javascript's generators, but instead of having `yield`
and `return`, we have `Some(foo)` and `None` (both always return). Please note that it is possible
for a user to implement this feature by himself, but, if different crates implement this in different ways,
it gets inconsistent.

Generally, when one writes an iterator, one codes a structure containing all the stateful variables
and implement manually an iterator around this state. With closures, one may only set up some local
variables and capture them into the closure. It is expected that writing iterators become less painful.

One example is writing an iterator that produces `Vec<u8>` from another iterator of `u8`. But
it will not collect all the `u8` into a single vec; it will fragment it into arbitrary-sized vecs.
Another example is an infinite fibonacci iterator.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Sometimes when writing iterators we may realise: what if any type implementing `FnMut() -> Option<T>` could also give us
an iterator? One way of doing this is with `std::iter::IterFn<F>`. Passing the closure to `IterFn<F>::new` method will
gives an iterator which calls the closure when `next` is called. The `Iterator<Item = T>` trait is implemented for
`IterFn<F>` when `F` implements `FnMut() -> Option<T>`, so primitive function types also fits into `IterFn`.

## Examples

### 01 - Fibonacci
```rust
fn gen_fib() -> impl Iterator<Item = u64> {
    let mut a = 0;
    let mut b = 1;
    IterFn::new(move || {
        let tmp = b;
        b += a;
        a = tmp;
        Some(a)
    })
}

assert_eq!(
    gen_fib().take(6).collect::<Vec<_>>(),
    vec![1, 1, 2, 3, 5, 8]
);

for n in gen_fib() {
    // infinite loop
}
```

### 02 - Fibonacci And Mapping
```rust
fn gen_fib_succ() -> impl Iterator<Item = u64> {
    let mut fib = {
        let mut a = 0;
        let mub b = 1;
        move || {
            let tmp = b;
            b += a;
            a = tmp;
            Some(a)
        }
    };
    IterFn::new(fib).map(|x| x + 1)
}

assert_eq!(
    gen_fib_succ().take(6).collect::<Vec<_>>(),
    vec![2, 2, 3, 4, 6, 9]
);
```

### 03 - Tokenizer
```rust
fn tokenize<I>(iter: I) -> impl Iterator<Item = String>
where
    I: Iterator<Item = char>
{
    IterFn::new(move || {
        let mut string = String::new();
        let mut started = false;
        loop {
            match iter.next() {
                Some(' ') | Some('\n') | Some('\r') => if started {
                    break string
                },
                Some(ch) => {
                    started = true;
                    string.push(ch)
                },
                _ => if started {
                    break None
                } else {
                    break Some(string)
                }
            }
        }
    })
}

assert_eq!(
    tokenize("Hey You".chars()).collect::<Vec<_>>(),
    vec!["Hey".to_string(), "You".to_string()]
);

```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Add a struct named `IterFn<F>` (stands for Iterator Function) into `std::iter` (its path and name are under discussion).
The structure should contain only one field, a value of type `F`. The definition of `IterFn<F>` does not constraint `F`,
but the implementation of the `Iterator` trait for `IterFn<F>` does. The `impl` of `IterFn` needs, in theory, just one method:
`new`, which accepts a function of type `F` and puts it into an `IterFn<F>`. The implementation of `Iterator` constraints `F`
to `FnMut() -> Option<T>`, has an `Item` assigned to `T`, and `next` just calls the function of type `F` inside the struct.

The struct looks like this:
```rust
pub struct IterFn<F> {
    inner: F,
}

impl<F> IterFn<F> {

    pub fn new(fun: F) -> Self {
        Self {inner: fun}
    }

}

impl<T, F: FnMut() -> Option<T>> Iterator for IterFn<F> {

    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        (self.inner)()
    }

}

```
The heart of this is the next method wrapping an inner function call.
Note that in previous examples (01, 02, 03) we called `IterFn::new` passing a closure
in order to get an `IterFn<F>` with a closure in it. Because `IterFn<F>` implements
`Iterator` we were able to use the iterator's methods. Another trait implementations
for the struct are under discussion.

Note that this struct does not need to be a lang item; it is fine being a library
implementation.

# Drawbacks
[drawbacks]: #drawbacks

* Having to write `IterFn::new(f)` is not so trivial as just using the function
  (but still more trivial then writing an iterator by hand).

# Rationale and alternatives
[alternatives]: #alternatives

Another alternative is to implement `Iterator<Item = T>` for every `FnMut() -> Option<T>`
automatically. But this would conflict with some arbitrary user's `FnMut` type which already
implements `Iterator`. Also, the `for` loop could get a bit confusing; see:
```rust
for n in gen_fib() {
}
```
or
```
for n in gen_fib {
}
```
?

# Prior art
[prior-art]: #prior-art

Some languagens (e.g. Python and JavaScript) have a similar feature named "Generators".
Fibonacci example in JavaScript:
```javascript

function* fib() {

    let tmp, a = 0, b = 1

    for (;;) {
        tmp = b
        b += a
        a = tmp
        yield a
    }

}


for (let n of fib()) {
    // infinite loop...
}

```

Note that generators suspend the function, while this RFC's feature calls a stateful function multiple times.
Also note that, as said before, one can implement this by hand, but it becomes inconsistent if multiple
crates implement it (and also alarms us that this is a needed feature).

# Unresolved questions
[unresolved]: #unresolved-questions

- What traits should `IterFn` implement, besides `Iterator`? Should we implement `Debug`? Should we add more functionalities?
- Is `std::iter::IterFn` a good path and name for our struct?
- Should `IterFn` be at `std` or `core`?
- Should we implement generators instead?
