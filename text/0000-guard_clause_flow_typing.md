- guard_clause_flow_typing
- Start Date: 2017-11-19
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Having _Flow Typing_ for _Guard Clause_ scenarios will allow explicit use of pattern matching assignment
and remove the need for `match`, `map_err()?`, and `unwrap` in any situation where an explicit return is
used for one of the given enum types as a _Guard Clause_.  Using an `if let None = thing {return Error;}`
is one example of a `Guard Clause` handling the `None` path for the item `thing`.  With that path handled
we should then be able to simply use pattern matching for assigning out the value from thing with
`let Some(value) = thing;` and not need to use any of the aforementioned match/map/unwrap.

# Motivation
[motivation]: #motivation

The biggest motivations for this is for providing simplicity and clarity in our code base.  When teaching
some one how to program if I only need to show a person how to pattern match to get the values they want
then the learning overhead is minimal.  Even as an experienced developer having so much extra syntax
required for your day to day situations will cause us to need more time to take in the situation of what's
going on in the code base.  This is especially true when dealing with many nested conditional blocks through
either `if` or `match`.  With using a _Guard Clause_ and being able to pattern match assignment, you
can avoid many scopes of indentation and the extra lines of outer scope variable creation for return values.
Using a _Guard Clause_ allows you to inline scope of the code and provide maximum legibility.


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

A `Guard Clause` is to be used whenever you have a need to explicitly return from one of the paths of
any given Enum.  For the `Option` type if you want to return a default value, or an error when it's `None`,
then you may use an `if let` assignment check to explicitly return the value you want for this path.  You
may then use pattern matching to extract the value from `Some` and continue your code within the same scope
and block providing maximum code readability.

Lets look at three examples of what not to do before showing how to correctly use a _Guard Clause_.  The
examples will be return a `Result` type that will have it's own custom `Error` type used internally.  _This
is a simplified configuration file parsing example._

### Wrong (A)
```rust
fn read_config1() -> Result<Config, Error> {
  let file = File::open("program.cfg");
  if let Ok(f) = file {
    let mut buf_reader = BufReader::new(f);
    let mut contents = String::new();

    if buf_reader.read_to_string(&mut contents).is_ok() {
      let mut data: Vec<u8> = vec![];

      for item in contents.
          split("\n").
          map(|s| s.to_string()).
          filter(|s| !s.is_empty()).
          collect::<Vec<String>>() {

        let num = item.parse::<u8>();

        if let Ok(conf) = num {
          data.push(conf);
        } else {
          return Err(Error::ConfigParseFail);
        }
      }

      Ok( Config { data: data } )
    } else {
      Err(Error::ConfigLoadFail)
    }
  } else {
    Err(Error::ConfigLoadFail)
  }
}
```
### Wrong (B)
```rust
fn read_config2() -> Result<Config, Error> {
  let file = File::open("program.cfg");
  match file {
    Ok(f) => {
      let mut buf_reader = BufReader::new(f);
      let mut contents = String::new();

      match buf_reader.read_to_string(&mut contents) {
        Ok(_) => {
          let mut data: Vec<u8> = vec![];

          for item in contents.
              split("\n").
              map(|s| s.to_string()).
              filter(|s| !s.is_empty()).
              collect::<Vec<String>>() {

            let num = item.parse::<u8>();

            match num {
              Ok(conf) => data.push(conf),
              _ => { return Err(Error::ConfigParseFail); },
            }
          }

          Ok( Config { data: data } )
        },
        _ => { Err(Error::ConfigLoadFail) }
      }
    },
    _ => { Err(Error::ConfigLoadFail) }
  }
}
```
### Wrong (C)
```rust
fn read_config3() -> Result<Config, Error> {
  let file = File::open("program.cfg");

  if let Ok(f) = file {
    let mut buf_reader = BufReader::new(f);
    let mut contents = String::new();

    if buf_reader.read_to_string(&mut contents).is_ok() {
      let mut data: Vec<u8> = vec![];

      for item in contents.
          split("\n").
          map(|s| s.to_string()).
          filter(|s| !s.is_empty()).
          collect::<Vec<String>>() {

        let num = item.parse::<u8>();

        if let Ok(conf) = num {
          data.push(conf);
        } else {
          return Err(Error::ConfigParseFail);
        }
      }

      return Ok( Config { data: data } );
    }
  }

  Err(Error::ConfigLoadFail)
}
```

And here is the correct usage of a _Guard Clause_ which allows us to avoid deeply nested logic.

### Correct
```rust
fn read_config4() -> Result<Config, Error> {
  let file = File::open("program.cfg");

  // Correct use of Guard Clause
  if let Err(_) = file { return Err(Error::ConfigLoadFail); }
  
  // Safe use of pattern matching assignment after Guard Clause
  let Ok(f) = file;

  let mut buf_reader = BufReader::new(f);
  let mut contents = String::new();

  // Correct use of Guard Clause
  if let Err(_) = buf_reader.read_to_string(&mut contents) {
    return Err(Error::ConfigLoadFail);
  }
  
  let mut data: Vec<u8> = vec![];

  for item in contents.
      split("\n").
      map(|s| s.to_string()).
      filter(|s| !s.is_empty()).
      collect::<Vec<String>>() {

    let num = item.parse::<u8>();

    // When all paths only have very short code blocks
    // a Guard Clause isn't necessary as the readability
    // of the code is no burden in this context.
    match num {
      Ok(conf) => data.push(conf),
      Err(_) => { return Err(Error::ConfigParseFail); }
    }
  }

  Ok( Config { data: data } )
}
```

Using a _Guard Clause_ should be thought of as a best practice for explicit return scenarios whenever it
provides better clarity by avoiding nested additional scopes.  This will likely be in the majority of
cases where an explicit return is used.  Exceptions to this is may be when working with enums that have
more than two paths to work with and the other paths require more involved code blocks… in that case how
you implement it would make more sense with all the paths following the same form factor like you can
have with `match`.

With the acceptance of this RFC you can replace your usages of `unwrap` everywhere with pattern matching
assignment.  This should improve readability.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The _Guard Clause_ will only be explicitly returned during an `if let`.  The compiler will take into
account that one of the paths have been accounted for in the _Guard Clause_ and assignment with pattern
matching will work for the rest of the scope _(that the item is owned within)_ rather than using `unwrap`,
`ok`, or any other extracting methods for the item.

Combining use of a _Guard Clause_ with a `match` to follow should be taboo _in my opinion_ as it's
using two separate systems for implementing the same logic.  So raising an explanatory error on why the
_Guard Clause_ is not to be intermixed with a `match` should appear.  The `match` not allowing a
preceding _Guard Clause_ should be considered a style warning and not an incompatible use of code.
To get rid of that warning they should use something like `#[allow(match_after_guard)]`.  I believe
it's fine to have `match` still require the path be handled even when a _Guard Clause_ and `allow` have
been used.

An equivalent from the earlier example which does not use pattern matching is:

```rust
// if let Err(_) = file { return Err(Error::ConfigLoadFail); }
// let f = file.unwrap();
let f = file.map_err(|_| Error::ConfigLoadFail)?;
```
but this isn't something a new programmer could understand as easily which is why using pattern matching is
preferable.

Because `if let` doesn't allow other conditionals there isn't any compounded complexity in evaluating the
paths which have been followed for any given scope.  The compiler can simply keep a marker for items with
_Guard Clause_ paths taken and not need to raise the following error any longer:

```
error[E0005]: refutable pattern in local binding: `Err(_)` not covered
   --> src/lib.rs:166:7
    |
166 |   let Ok(f) = file;
    |       ^^^^^ pattern `Err(_)` not covered

error: aborting due to previous error
```

One area that might be a little more complex to account for is if some one were to try putting further
conditional logic inside the _Guard Clause_ code block.

```rust
if let Err(_) = file { 
  if random_value > 5 { return Err(Error::ConfigLoadFail); }
}
```

The compiler should be able to not allow the user this behavior by simply treating it as not being a
_Guard Clause_ so any assignment use of it later will require the `Err` path to still be accounted for.

These are the edge cases I can think of.
# Drawbacks
[drawbacks]: #drawbacks

None (TBD)

# Rationale and alternatives
[alternatives]: #alternatives

Using Rust's existing pattern matching and furthering its ability will reduce overall complexity. This
feature will help the Rust community grow more rapidly as it helps alleviate the barrier to learning
that we currently have with the heavy use of our syntax.

Alternatives: [`let … else {}` RFC #1303](https://github.com/rust-lang/rfcs/pull/1303)

# Unresolved questions
[unresolved]: #unresolved-questions

- What parts of the design do you expect to resolve through the RFC process before this gets merged?

(TBD)

- What parts of the design do you expect to resolve through the implementation of this feature before stabilization?

(TBD)

- What related issues do you consider out of scope for this RFC that could be addressed in the future independently of the solution that comes out of this RFC?

During discussion in the [Pre-RFC](https://internals.rust-lang.org/t/pre-rfc-allow-pattern-matching-after-guard-clause/6238)
a few people were more interested in reviving [`let … else {}`](https://github.com/rust-lang/rfcs/pull/1303) rfc rather
than discussing the merits of this pattern matching approach.
