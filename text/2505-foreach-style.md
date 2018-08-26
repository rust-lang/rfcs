- Start Date: 2018-08-26

# Summary
Add new foreach system.

# Motivation
In your code you need to add many times somes foreach but actually it's long to writte so this need to change

Ancient rust:
```rust
for i in 0..list.len() {
  let x = list[i];
}
```

My proposition:

```rust
for w in list {

}
```

OR

```rust
for w,x in map {

}
```

# How to create this

During the compilation
if an object iterable is found in for
build this like:
for <name> in <iterable>

```rust
for <name>_count in 0..<iterable>.len() {
  let <name> = list[<name>_count];
}
```

The mut word can be used
