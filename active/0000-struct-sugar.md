- Start Date: 2014-09-30
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Features:

* A sugar for structs with optional arguments
* A sugar for declaring all members of a struct public
* Language change: Function call syntax for named arguments

Goals:

* Refactor friendly syntax
* Error safe function call syntax
* Improve readability and ergonomics

Remove redundant typing when ...

* ... returning a struct from a function or expression block
* ... initializing collections, for example a vector
* ... handling optional function arguments
* ... destructuring a struct with optional values

# Motivation

Improve ergnomics for structs, add named argument syntax and optional arguments.

The idea can be illustrated with a toy example:

```Rust
// `pub:` makes all members public.
pub: struct Character<'a> {
    first_name: &'a str,
    last_name: &'a str,
    age: u16,
    father: Option<&'a str>,
    mother: Option<&'a str>,
    children: Option<&'a [&'a str]>,
    home_town: Option<&'a str>,
    favourite_food: Option<&'a str>,
    partner: Option<&'a str>,
    abilities: Option<&'a [&'a str]>
}

impl World {
    // World has different representation of a character,
    // therefore the arguments are pointers and not values.
    fn add_character(&mut self, Character {
        first_name,
        last_name,
        age,
        father,
        mother,
        children = [].as_slice(),
        home_town,
        favourite_food = "pizza",
        partner,
        abilities = [].as_slice()
    }) {
        ...   
    }
}

let last_name = "Brown";
world.add_character(
    first_name: "Mike", last_name, age: 56, children: vec!["Julia"].as_slice(), ..
);
world.add_character(
    first_name: "Julia", last_name, age: 15, father: "Mike", ..
);
```

This gets desugared into:

```Rust
pub struct Character<'a> {
    pub first_name: &'a str,
    pub last_name: &'a str,
    pub age: u16,
    pub father: Option<&'a str>,
    pub mother: Option<&'a str>,
    pub children: Option<&'a [&'a str]>,
    pub home_town: Option<&'a str>,
    pub favourite_food: Option<&'a str>,
    pub partner: Option<&'a str>,
    pub abilities: Option<&'a [&'a str]>
}

pub struct World;

impl World {
    // World has different representation of a character,
    // therefore the arguments are pointers and not values.
    fn add_character(&mut self, Character {
        first_name,
        last_name,
        age,
        father,
        mother,
        children,
        home_town,
        favourite_food,
        partner,
        abilities
    }: Character) {
        let children = match children.to_option() {
                None => { [].as_slice() },
                Some(val) => val
            };
        let favourite_food = match favourite_food.to_option() {
                None => "pizza",
                Some(val) => val
            };
        let abilities = match abilities.to_option() {
                None => { [].as_slice() },
                Some(val) => val
            };
        ...
    }
}

let last_name = "Brown";
world.add_character(Character {
    first_name: "Mike",
    last_name: "Brown",
    age: 56,
    father: Optional::none(),
    mother: Optional::none(),
    children: Optional::some(vec!["Julia"].as_slice())),
    home_town: Optional::none(),
    favourite_food: Optional::none(),
    partner: Optional::none(),
    abilities: Optional::none()
});
world.add_character(Character {
    first_name: "Julia",
    last_name: "Brown",
    age: 15,
    father: Optional::some("Mike"),
    mother: Optional::none(),
    children: Optional::none(),
    home_town: Optional::none(),
    favourite_food: Optional::none(),
    partner: Optional::none(),
    abilities: Optional::none()
});
```

In the example above, the sugar requires 53% of the characters, even there are only two characters involved.
With a similar named syntax for functions, the sugar requires 42% of the characters.

# Detailed design

Add a new `Optional` trait to the standard library:

```Rust
/// Implemented by types used for optional arguments.
pub trait Optional<T> {
    /// Called by `..` for optional arguments, for example `foo(x, ..)`.
    fn none() -> Self,
    /// Called by optional arguments that pass a value.
    fn some(T) -> Self,
    /// Called when unwrapping an optional argument.
    fn to_option(self) -> Option<T>
}
```

The `Optional` trait should be implemented by `Option`:

```Rust
impl<T> Optional<T> for Option<T> {
    fn none() -> Option<T> { None }
    fn some(val: T) -> Option<T> { Some(val) }
    fn to_option(self) -> Option<T> { self }
}
```

`Option` is likely to be the most common type for default arguments.

### Syntax for marking members of a struct public

A colon `pub: struct` makes all members in struct public.
This makes it possible to refactor arguments from a function,
or move a local defined struct to global scope, with just a few keystrokes.

Example:

```Rust
pub: struct Character<'a> {
    first_name: &'a str,
    last_name: &'a str,
    age: u16,
    father: Option<&'a str>,
    mother: Option<&'a str>,
    children: Option<&'a [&'a str]>,
    home_town: Option<&'a str>,
    favourite_food: Option<&'a str>,
    partner: Option<&'a str>,
    abilities: Option<&'a [&'a str]>
}
```

### Sugar for initializing structs

Desugar `x: 1, y` into `Foo { x: 1, y: y }` when `Foo` is expected.
At least one named argument is required, and unnamed arguments must match the member name.
This reduces typing while preventing error under refactoring where swapping is intended.
A `..` with no following value fills in with `Optional::none()` for all optional arguments.
Values to optional arguments are desugared to `Optional::some(val)` when
the expected type is not the same as the value.

Example:

```Rust
let point: Point = x: 20.0, y: 10;
let point2 = x: 30.0, ..point;

let last_name = "Brown";
let mike = first_name: "Mike", last_name, ..;
let julia = first_name: "Julia", last_name, ..;
world.add_character(mike); // expects Character
world.add_character(julia);
```

There is no "unnamed struct", the syntax desugars to the expected type.

This sugar reduces redudant typing when returning from a function:

```Rust
fn x() -> Point { x: 1.0, y: 0.0 }
```

While this looks like a struct initialization, it is a normal expression block.

Another example, where this encourages efficient immutable updates:

```Rust
pub: struct Window {
    title: Option<String>,
}

impl Window {
    pub fn new() -> Window { .. }

    #[inline(always)]
    pub fn title(&self, title: String) -> Window { title, ..*self }
}

let mut window = Window::new().title("Hello world!".to_string());
let window2 = window.title("I am a copy".to_string());
```

If the title should be required, this can easily be refactored to:

```Rust
pub: Window {
    title: String,
}

impl Window {
    pub fn new(title: String) -> Window { title, .. }

    #[inline(always)]
    pub fn title(&self, title: String) -> Window { title, ..*self }
}

let mut window = Window::new("Hello world!".to_string());
let window2 = window.title("I am a copy".to_string());
```

Notice that struct members can be made optional/required, without breaking more code than necessary.

### Named argument syntax for functions

When there is a single argument to a function, or, a single argument besides `self` in a method,
the struct sugar looks like a named parameter syntax:

```Rust
world.add_character(first_name: "Julia", last_name: "Brown", ..);
```

In fact, it is possible to use this syntax without introducing ambiguity:

```Rust
fn add_character(
    &mut self,
    first_name: &str,
    last_name: &str,
    age,
    father: Option<&str>,
    mother: Option<&str>,
    children: Option<&[&str]> = [].as_slice(),
    home_town: Option<&str>,
    favourite_food: Option<&str> = "pizza",
    partner: Option<&str>,
    abilities: Option<&[&str]> = [].as_slice()
) {
    ...
}
```

This is equivalent to:

```Rust
fn add_character(
    &mut self,
    first_name: &str,
    last_name: &str,
    age,
    father: Option<&str>,
    mother: Option<&str>,
    children: Option<&[&str]>,
    home_town: Option<&str>,
    favourite_food: Option<&str>,
    partner: Option<&str>,
    abilities: Option<&[&str]>
) {
    let children = match children.to_option() {
        None => { [].as_slice() }
        Some(val) => val
    };
    let favourite_food = match favourite_food.to_option() {
        None => { "pizza" }
        Some(val) => val
    };
    let abilities = match abilities.to_option() {
        None => { [].as_slice() }
        Some(val) => val
    };
    ...
}
```

In the example above, the sugar requires 53% of the characters.
The reduced number of characters when calling the function is not included.

### Destructure pattern

The same desugar as for optional arguments is used in destructuring.

Assume the following:

```Rust
fn send_form(
    first_name: Option<&str> = return Err("first name is missing"),
    last_name: Option<&str> = return Err("last name is missing"),
    age: Option<uint> = return Err("age is missing")
) -> Result<(), &'static str> {
    ...
}
```

The programmer's task is to refactor the arguments to a `Form` struct.
This is done by copying the arguments, remove default arguments for the struct,
add a lifetime paramter,
and remove the type annotation for the function:

```Rust
pub: struct Form<'a> {
    first_name: Option<&'a str>,
    last_name: Option<&'a str>,
    age: Option<uint>
}

fn send_form(Form {
    first_name = return Err("first name is missing"),
    last_name = return Err("last name is missing"),
    age = return Err("age is missing")
}: Form) -> Result<(), &'static str> {
    ...
}
```

This change can be done without breaking existing code,
where the named argument syntax is used.

```Rust
// Same code before and after refactoring, thanks to struct sugar.
send_form(first_name: "Mike", last_name: "Brown", age: 56);
```

Later, the programmer's task is to create a method:

```Rust
impl Form {
    fn send_form(self) -> Result<(), &'static str> {
        Form {
            first_name = return Err("first name is missing"),
            last_name = return Err("last name is missing"),
            age = return Err("age is missing")
        } = self;
        ...
    }
}
```

This breaks the code, but the fix is not far away:

```Rust
Form { first_name: "Mike", last_name: "Brown", age: 56 }.send_form();
```

### Other benefits

When initializing a vector, the struct sugar reduces the amount of changes required to make the code work.
There is only one change required:

```Rust
// let levels: Vec<Level> = vec![
let levels: Vec<Scene> = vec![
    { narrative: "...", .. },
    { narrative: "...", .. },
    { mini_game: true, .. },
];
```

The `Optional` trait can be implemented by any type, to do custom actions with a different semantics.
Assume there is a syntax extension `optional(DefaultVariant)`:

```Rust
#[optional(JustGiveMeSomething)]
pub enum MovieRequest
    JustGiveMeSomething,
    Comedy(uint),
    Drama(uint),
    Action(uint),
    Thriller(uint),
    Romance(uint),
    Family(uint),
    SciFi(uint),
    Recommended(uint),
}
```

This generates the code:

```Rust
impl Optional<MovieRequest> for MovieRequest {
    fn none() -> MovieRequest { JustGiveMeSomething }
    fn some(val: MovieRequest) -> MovieRequest { val }
    fn to_option(self) -> Option<uint> {
        match self {
            JustGiveMeSomething => None,
            x => Some(x)
        }
    }
}
```

This can be used to set user opt-in defaults that are specific to the request.

```Rust
impl MovieDatabase {
    // If default filter is specified, show top 5 recommended movies.
    fn get_best_movies(
        &self, 
        request: MovieRequest = Recommended(5)
    ) -> Vec<Movie> {
        ...
    }
    
    // If default filter is specified, pick 5 from the user's favourite category.
    fn get_random_movies(
        &self, 
        user: User,
        request: MovieRequest = user.favourite_category(5)
    ) -> Vec<Movie> {
        ...
    }
}

let best_recommended_movies = db.get_best_movies(..);

let top_ten_comedy = db.get_best_movies(Comedy(10));

let random_in_favourite_category = db.get_random_movies(user: log_in(), ..);
```

The `Optional` trait can also be used with generics:

```Rust
fn foo<T: Optional<U>, U: Default>(
    bar: T = default::Default(), 
    baz: T = default::Default()
) {
    // bar has type U.
    ...
}

// `Some` is used to wrap it in an optional type.
foo(bar: Some(10u), ..);
foo(baz: Some("hello"), ..);
```

### Corner cases

When a function takes a closure, it may apply named for some arguments but not others.
All arguments must named to use named syntax.

```Rust
fn call_me(f: |num: u32, u32|) {
    f(num: 2, 5); // ERROR: All arguments must be named to use named syntax.
}

call_me(|num, x| println!("{} {}", num, x));
```

The name does not have to match with the name in the callback.
Closures do not have to carry name information for their arguments.

A closure taking a single struct argument can not be casted to a closure with multiple arguments.
When refactoring, the callback and the closure must be changed.

```Rust
pub: Callback {
    num: u32,
    x: u32
}

fn call_me(f: |Callback|) {
    f(num: 2, x: 5);
}

call_me(|num, x| println!("{} {}", num, x)); // ERROR: Expected `Callback`
call_me(|Callback { num, x }| println!("{} {}", num, x)); // fix
```

When refactoring function arguments into a struct,
all code will break that uses unnamed syntax.
This is because the named syntax is designed to not break reordering.
At least one named argument is required to avoid error when swapping is intended:

```Rust
fn foo(x: uint, y: uint) { ... }

foo(y, x); // swaps x and y
```

When refactored, the code will break:

```Rust
pub: struct Foo {
    x: uint,
    y: uint
}

fn foo(Foo { x, y }) { .. }

foo(y, x); // ERROR: expected `Foo`
foo(x: y, y: x); // fix
```

Code also breaks when function is changed to take a borrowed struct:

```Rust
fn foo(&Foo) { ... }

foo(x: 0, y: 1); // ERROR: Expected `&Foo` but found `Foo`
foo(&{ x: 0, y: 1 }); // fix
```

When nesting a destructured pattern, the order determine dependency between the variables:

```Rust
let Foo { bar: Bar { baz = x.len() }, x } = foo; // ERROR: `x` is not defined

let Foo { x, bar: Bar { baz = x.len() } } = foo; // fix
```

# Drawbacks

There is a performance overhead by calling methods on `Optional` and then matching `Option`.
This is the price to get non-static expressions for cases where an argument is not given.
However, there is no performance or safety choice to be made for the user that calls the function.

```Rust
fn foo(bar: Option<uint> = 3) {
    println!("{}", bar);
}

foo(Some(5)); // There is no performance gain by requiring explicit wrapping.

foo(5); // The performance is the same, but now with less line noise.
```

On the contrary, it is easier to remove optional arguments to increase performance,
when the programmer discover later that the optional argument was not needed.
The existing code will compile unless the argument is desugared with `..`.

Because there is a performance penalty, Rust core libraries will likely not use optional arguments.
If the standard library should benefit from optional arguments, it would have to consider an alternative.
Named syntax can still be used on functions and methods in the standard library,
if this becomes a part of the language.

Because the struct sugar is unproven design, it will take some time to mature.
No changes are required to existing code, meaning it can be added post-1.0.

# Alternatives

### Add struct sugar, drop named syntax

It can be added as pure sugar, but without the named syntax for normal functions.

### Make optional arguments static expressions directly on structs

This will bind the semantics of default values to the type,
which also will complicate the interface between libraries.

With struct sugar it requires little effort to write a constructor.
It also encourages efficient immutable builder patterns,
and refactors better with natural concepts that emerges from working on the code.

For example:

```Rust
// This code is likely to change semantics multiple times,
// while another instance of a character, Mike, undergoes refactoring.
let julia = Character { first_name: "Julia", last_name: "Brown", .. };

// TODO: Create a chief constructor.
let chief = Character::new("Mike", "Brown", ..);

// TODO: This needs more clearity.
let chief = Character::chief("of security", "Mike", "Brown", ..);

// TODO: Should there be a Role struct?
let chief = Character::chief(
    desc: "of security", 
    first_name: "Mike", 
    last_name: "Brown",
    ..
);

let chief = Character::new(
    role: Role::chief("of security"), 
    first_name: "Mike", 
    last_name: "Brown",
    ..
);
```

If default values were bound to the type,
the programmer would have to change all the code that dependend on these default values.
There is no way to know exactly how that influences the existing code.

With struct sugar, the semantics of the construction of `julia` is non-ambigious,
and does not break by refactoring somewhere else.

When there are obvious choices for default, bounded to a type, the `Default` trait should be used.
A syntax sugar for `Default` might be added, compatible with struct sugar, but this belongs in another RFC.

```Rust
let julia = Character { first_name: "Julia", last_name: "Brown", ..Default::default() };

// Sugar for Default?
let julia = Character { first_name: "Julia", last_name: "Brown", ..* };
```

### Make optional arguments static expressions directly on functions

This requires compiler semantics on the call side,
which also complicates the interface between libraries.

It does not provide any benefit over struct sugar except for a marginal performance gain.
Worse, it requires a static model of semantics, which complicates the modelling process.
A static model of semantics is much less friendly toward refactoring,
because it introduces dependencies on concepts that must be known upfront for all use cases.

Neither does it play well with `Option<T>`:

```Rust
fn foo(bar: Option<uint> = Some(3)) {
    let bar = bar.unwrap();
}

foo(..); // works
foo(None); // task failure
```

### Make default arguments part of the core language

If this is not designed properly, Rust has to live with regrets.
The benefit of a sugar means it can be swapped with a better one.

### Special-case on `Option`

The first idea behind optional arguments was to use `Option` with an implicit type annotation:

```Rust
/// `bar` has type `Option<uint>` outside, but `uint` inside.
fn foo(bar: uint = 2) {
   ...
}
```

This idea was rejected after early feedback from the Rust community.
The reason is to keep the language small and not treat any type
as "special" unless it is necessary.
By using the `Optional` trait, other semantics is allowed,
and the ergonomic benefits of the sugar is more obvious.

### Generalize to struct tuples

This introduces another axis of refactoring which confuses the user of the library.
Struct tuples also removes name information from arguments, preventing named syntax,
which will break reordering of arguments where name syntax is used before refactoring.

### Implicit cast from tuples

A way to make structs easier to initialize in vectors,
is to allow implicit cast from tuples where the order and types are the same.
This does not allow optional arguments, and any reordering will break the code.
However, it will make code shorter when long member names are repeated:

```Rust
// Implicit casting from tuples
let people: Vec<Person> = vec![
    ("Mike", "Brown"),
    ("Julia", "Brown")
];

// Struct sugar
let people: Vec<Person> = vec![
    { first_name: "Mike", last_name: "Brown" },
    { first_name: "Julia", last_name: "Brown" }
];
```

Implicit casting from tuples can be combined with struct sugar,
and be turned on/off with an attribute.

### Monadic macros

The nature of refactoring is lack of prediction when a macro is needed.
Sometimes, it takes longer time to include a macro than typing out the code it generates.
This is in particular true for structs, where the patterns are simple and error prone.
If something goes wrong with a macro, it is harder to debug.
Also, the same sugar can not be replaced by a single macro.

### Higher kinded types, monads etc.

Monads require redundant typing for structs.
It also lacks the refactoring capability that struct sugar has.

For example, with a `do` modad similar to Haskell:

```Haskell
let person = read_person();
do {
    first_name <- person.first_name;
    last_name <- person.last_name;
    age <- person.age;
    println!("{} {} {}", first_name, last_name, age);
};
```

Compared to struct sugar:

```Rust
let Person {
    first_name = return,
    last_name = return,
    age = return,
} = read_person();

println!("{} {} {}", first_name, last_name, age);
```

While both looks nice, there are 36 more characters in the do monad,
which increases linearly with the number of fields.

# Unresolved questions

* Add a `#[optional(DefaultVariant)]` for enums to the standard library?
* Design attributes for turning on/off the sugar, for example for extra performance concerns?
* Make type annotation optional in function arguments where structs are destructured?
* Generalize to overloaded functions using enums that have unique type signatures?
