- Start Date: 2014-07-26
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)


# Summary

Introduce a `newtype` construction allowing newtypes to use the
capabilities of the underlying type while keeping type safety.


# Motivation

Consider the situation where we want to create separate primitive
types. For example we want to introduce an `Inch` and a `Cm`. These
could be modelled with `uint`, but we don't want to accidentally
mix the types.

With the current newtypes:

```
struct Inch(uint);
struct Cm(uint);

// We want to do generic manipulations
fn calc_distance<T: Sub<T, T>>(start: T, end: T) -> T {
    end - start
}

let (start_inch, end_inch) = (Inch(10), Inch(18));
let (start_cm, end_cm) = (Cm(2), Cm(5));

// We must explicitly destruct to reach the values
let (Inch(start), Inch(end)) = (start_inch, end_inch);
let inch_dist = Inch(calc_distance(start, end));

let (Cm(start), Cm(end)) = (start_cm, end_cm);
let cm_dist = Cm(calc_distance(start, end));

let (Inch(inch_val), Cm(cm_val)) = (inch_dist, cm_dist);
println!("dist: {} and {}", inch_val, cm_val);

// Disallowed compile time
let not_allowed = calc_distance(start_inch, end_cm);
```

This is verbose, but at least the types don't mix.
We could explicitly define traits for the types, but that's duplication
if want the same capabilities as the underlying type.

Another option is to use the `type` keyword, but then we loose type safety:

```
type Inch = uint;
type Cm = uint;

let inch: Inch = 10;
let cm: Cm = 2;

let oops = inch + cm; // not safe!
```


# Detailed design

Introduce a new keyword: `newtype`. It introduces a new type, with the same
capabilities as the underlying type, but keeping the types separate.

```
newtype Inch = uint;
newtype Cm = uint;

// We want to do generic manipulations
fn calc_distance<T: Sub<T, T>>(start: T, end: T) -> T {
    end - start
}

let (start_inch, end_inch) = (Inch(10), Inch(18));
let (start_cm, end_cm) = (Cm(2), Cm(5));

let inch_dist = calc_distance(start_inch, end_inch);
let cm_dist = calc_distance(start_cm, end_cm);

println!("dist: {} and {}", inch_dist, cm_dist);

// Disallowed compile time
let not_allowed = calc_distance(start_inch, end_cm);
```

The grammar rules will be the same as for `type`.
It would also allow generics, like `type`:

```
struct A<N, M> { n: N, m: M }
newtype B<T> = A<uint, T>;
```

Just like `type` does.


# Drawbacks

It adds a new keyword to the language and increases the language complexity.


# Alternatives

* Keep it the same

    It works, but life could be simpler.

* `as` could be used to convert from a newtype to the underlying value:

    ```
    struct Inch(int);

    let v: Inch = Inch(10);
    println!("inches: {}", v as int);
    ```

    But we still need to explicitly cast when using generic functions:

    ```
    let dist = Inch(calc_distance(start_inch as int, end_inch as int));
    ```

    It also loses type safety.


# Unresolved questions

None yet

