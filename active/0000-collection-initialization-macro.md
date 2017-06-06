- Start Date: 2014-08-20
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Add a `seq!` macro to the standard library. This macro can be used to
initialize most of the collections available in the standard library, and can
be extended (via a trait) to support collections defined in external
libraries.

# Motivation

The standard library only provides a `vec!` macro to create `Vec`tors, but the
community has expressed the desire of having [more macros][0] to create other
types of collections like `HashMap`s. Also, other programming languages like
C++, Go and Python provide syntactic sugar to initialize map-like collections.

# Detailed design

### Usage pattern

The `seq!` macro would have the following usage pattern:

``` rust
let empty: Bitv = seq![];
let v: Vec<int> = seq![1, 2, 3];
let m: HashMap<char, String> = seq!{
    'a' => "apple".to_string(),
    'b' => "banana".to_string(),
};
```

### The `Seq` trait

A `Seq` trait will be introduced to abstract over the idea of a growable
collection:

``` rust
trait Seq<T> {
    /// Creates an empty collection with an initial capacity (if applicable)
    fn with_capacity(n: uint) -> Self;
    /// Add a new element to the collection
    fn add_elem(&mut Self, elem: T);
}
```

### `seq!` definition

The `seq!` macro will use the `Seq` trait to create a generic collection. The
concrete type of the collection will be determined via type inference.

This is the definition of the `seq!` macro:

``` rust
macro_rules! seq {
    // List style: seq![1, 2, 3]
    ($($x:expr),*) => ({
        let mut _temp = Seq::with_capacity(count_args!($($x),*));

        $(Seq::add_elem(&mut _temp, $x);)*

        _temp
    });
    // Map style: seq!{"I" => 1, "II" => 2}
    ($($k:expr => $v:expr),*) => ({
        let mut _temp = Seq::with_capacity(count_args!($(($k, $v)),*));

        $(Seq::add_elem(&mut _temp, ($k, $v));)*

        _temp
    });
    // Trailing commas
    ($($x:expr),+,) => { seq!($($x),+) };
    ($($k:expr => $v:expr),+,) => { seq!($($k => $v),+) };
}
```

(The `count_args!` macro returns the number of arguments that received, and
should be replaced by the `$#($args)` syntax if [its RFC][1] gets accepted)

### Initial capacity optimization

As an optimization the `count_args!` macro will provide the exact capacity that
the collection must have on its creation to avoid reallocations. (This part is
optional, see [Alternatives]).

### Extensibility

The `seq!` macro can be used to initialize any collection that implements the
`Seq` trait. For example, to add support for `HashMap`s, the following
implementation must exist in the standard library:

``` rust
impl<K, V> Seq<(K, V)> for HashMap<K, V> where K: Eq + Hash {
    fn with_capacity(n: uint) -> HashMap<K, V> {
        HashMap::with_capacity(n)
    }

    fn add_elem(m: &mut HashMap<K, V>, (key, value): (K, V)) {
        m.insert(key, value);
    }
}
```

This also means that third-party collections can be used with `seq!` if they
implement the `Seq` trait in their respective libraries.

### (Why `add_elem` is defined as a static method?)

The compiler [can't infer][2] the type of `_temp` if `_temp.add_elem($x)` is
used to insert elements into the collection. To workaround this problem,
`add_elem` must be defined as a static method.

We may be able to remove the workaround (i.e. `add_elem` would become a normal
method) under any of these two scenarios:

- The inference engine is extended to support this particular case.
- We get UFCS that will let us call a normal method (`self.method(args)`) as
  `Trait::method(self, args)`.

# Drawbacks

- Yet another trait/macro in the standard library.

# Alternatives

### Don't do this

Keep supporting only the `vec!` macro, and let the community create their own
macros to deal with the creation of standard/custom collections.

### Do this, but without the initial capacity optimization

Remove the `with_capacity` function from the `Seq` trait and use the `Default`
trait to create the initial collection in the `seq!` macro.

``` rust
trait Seq<T>: Default {
    fn new() -> Self {
        Default::default()
    }

    fn add_elem(&mut Self, T);
}
```

The rationale is that the concept of capacity doesn't apply to all the
collections (like `TreeMap`).

The downside of this alternative is that some (the most common?) collections:
`Vec` and `HashMap` will/may incur in reallocations during their
initialization.

The upside is that we won't have to add the `count_args!` macro (or wait for
the `#$($args)` syntax to land).

# Unresolved questions

- Should we deprecate the `vec!` macro in favor of the `seq!` macro?

- Should the map style version of `seq!` use the `a => b` notation or the
  `a: b` notation.

- `seq!`/`Seq` may not be the best name for the macro/trait, because
  sequence means "ordered list", and in some collections (like `HashMap`) the
  order of the arguments is lost. (Let the bikeshed begin!)

[0]: https://github.com/rust-lang/rust/issues/14726
[1]: https://github.com/rust-lang/rfcs/pull/88
[2]: https://github.com/rust-lang/rust/issues/14726#issuecomment-45457987
