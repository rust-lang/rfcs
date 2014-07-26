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
// We want to do generic manipulations
fn calc_area<T: Mul<T, T>>(w: T, h: T) -> T { w * h }

struct Inch(uint);
struct Cm(uint);

let inch = Inch(10);
let cm = Inch(3);

// We must explicitly destruct to reach the values
let Inch(inch_val) = inch;
let Cm(cm_val) = vm;

let inch_area = Inch(calc_area(inch, inch));
let cm_area = Cm(calc_area(cm, cm)); // type Cm
println!("area: {} and {}", inch_area, cm_area);
```

This is verbose, but at least the types don't mix.
We could explicitly define traits for the types, but that's duplication
if want the same capabilities as the underlying type.

Another option is to use the `type` keyword, but then we loose type safety:

```
type Inch = int;
type Cm = int;

let inch: Inch = 10;
let cm: Cm = 2;

let oops = inch + cm; // not safe!
```


# Detailed design

Introduce a new keyword: `newtype`. It introduces a new type, with the same
capabilities as the underlying type, but keeping the types separate.

```
fn calc_area<T: Mul<T, T>>(w: T, h: T) -> T { w * h }

newtype Inch = uint;
newtype Cm = uint;

let inch = Inch(10);
let cm = Inch(3);

let inch_area = calc_area(inch, inch); // type Inch
let cm_area = calc_area(cm, cm); // type Cm
println!("area: {} and {}", inch_area, cm_area);

let not_allowed = inch + cm; // Compile error
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
    let area = Inch(calc_area(v as int, v as int));
    ```


# Unresolved questions

None yet

