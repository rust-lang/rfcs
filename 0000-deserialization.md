- Start Date: 2014-03-26
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

This is an RFC to convert Rust's deserialization framework to produce a stream of
tagged values.

# Motivation

While Rust's deserialization is very flexible and can deserialize a `JSON`
string like this:

```json
{
    "s": "Hello World",
    "n": 5
}
```

Into a structure like this:

```rust
struct Foo {
    s: ~str,
    n: int,
}
```

It cannot handle deserializing into a generic enum like this:

```rust
enum Json {
    Number(f64),
    String(~str),
    Boolean(bool),
    List(Vec<Json>),
    Object(HashMap<~str, Json>),
    Null,
}
```

The reason why is our current `Decoder`/`Decodable` implementation provides no
form of lookahead. In this case, a `Decodable` for `Foo` asks a `Decoder` for:

 * an object named `"Foo"`
 * a field named `"s"`
 * a `~str` value
 * a field named `"n"`
 * an `int` type

Any deviation from this is a desesrialization error. A `Decodable`
implementation for an enum like `Json` however, needs to be able to ask a
`Decoder` what is the type of the next value in order to pick which variant to
return.


# Detailed design

This RFC proposes that a `Decoder` should return a stream of tagged values.
This allows a `Decodable` to optionally buffer up values if it needs some form
of lookahead. The traits would be changed to:

```rust
pub trait Decoder<E> {
    fn decode(&mut self) -> Result<Value, E>;

    // helper methods...
    fn decode_int(&mut self) -> Result<int, E> {
        match self.decode() {
            Int(v) => Ok(v),
            I8(v) => Ok(v.to_int().unwrap()), // need proper error handling...
            I16(v) => Ok(v.to_int().unwrap()),
            ...
        }
    }
    ...
}

pub trait Decodable<E> {
    fn decode<D: Decoder<E>>(decoder: &mut D) -> Result<Value, E>;
}
```

The enum `Value` would represent all of Rust's primitive and compound values:

```rust
pub enum Value {
    Nil,
    Uint(uint),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    Int(int),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    Bool(bool),
    Char(char),
    Str(~str),

    EnumStart(~str),
    EnumEnd,

    StructStart(~str, uint),
    StructElt(~str, uint),
    StructEnd,

    StructTupleStart(~str, uint),
    StructTupleElt,
    StructTupleEnd,

    TupleStart(uint),
    TupleElt,
    TupleEnd,

    OptionSome,
    OptionNone,

    SeqStart(uint, Option<uint>),
    SeqElt,
    SeqEnd,

    MapStart(uint, Option<uint>),
    MapElt,
    MapEnd,
}
```

Decoding a primitive can be done pretty simply with some macros:

```rust
macro_rules! decode_primitive(
    ($T:ty, $method:ident) => {
        impl<E> Decodable<E> for $T {
            fn decode<D: Decoder<E>>(decoder: &mut D) -> Result<$T, E> {
                decoder.$method()
            }
        }
    }
)

decode_primitive!((),   decode_nil)
decode_primitive!(bool, decode_bool)
decode_primitive!(~str, decode_str)
...
```

Compound values have a similar complexity to the current approach by building a
simple state machine to parse the stream:

```rust
impl<E, T: Decodable<E>> Decodable<E> for Vec<T> {
    fn decode<D: Decoder<E>>(decoder: &mut D) -> Result<Vec<T>, E> {
        match try!(decoder.decode()) {
            SeqStart(min_size, _) => {
                let mut v = Vec::with_capacity(min_size);

                loop {
                    match decoder.decode() {
                        SeqEnd => { return Ok(v); }
                        SeqElt => { v.push(decoder.decode()); }
                        _ => { ... handle error ... }
                    }
                }
            }
            _ => { ... handle error ... }
        }
    }
}
```

Unfortunately there are some downsides to this approach. First, this approach
will add many more branches to the deserialization pipeline, which resulted in
a 10-15% slowdown in some prototype benchmarks. Second, it is possible to
accidentally forget to handle one of the compound value states, which would be
a source of errors with this approach.

# Alternatives

 * The current design of `Decoder` and `Decodable` allows a `Decodable` to be
	 implemented against a specific `Decodable`. For example `impl
	 Decodable<json::Decoder> for json::Json { ... }`. This implementation would
   then be able to access an alternative lookahead API on the `Decoder`.
   Unfortunately, this is running in some variance issues that may need higher
   order kinds to fix.
 * Rust could preserve the current approach and *add* a separate approach that
   supports lookahead. This however would add a substantial amount of code
   duplication.

# Unresolved questions

 * The current implementation of `Encoder` and `Encodable` supports serializing
   all types. Is it worthwhile converting them to a stream-style approach to be
   consistent? Or should it preserve the current approach for performance issues?
 * Should we rename `Encod[er,able]`/`Decod[er,able]` to
   `Serializ[er,able]/Deserializ[er,able]`? While the shorter names are nice,
   it does feel inconsistent.
 * What is the best way to handle errors?
 * Is there a better state machine design that prevents a user from forgetting
   to handle a state?
