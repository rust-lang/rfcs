- Feature Name: i18n_module
- Start Date: 2015-03-20 22:09:16
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Make it possible for translator translate anythings include keywords, standard library, function name.
If possible, translator could adjust the programming language grammer that was write as English.

# Motivation

Q: Why are we doing this?

A: You could be not to do this, I do.
   I do this because it perhaps could terminate or replace the "Easy Programming Language"
   which popular in china. And it could bypass the language barrier, so peoples learning
   this programming language could be easily which not familiar English.

Q: What use cases does it support?

A: That is what I said.
   Make it possible for translator translate anythings include keywords, standard library, function name.
   And rust compiler any outputs like errors, helps.
   If possible, translator could adjust the programming language grammer that was write as English.

Q: What is the expected outcome?

A: I don't know for now.

# Detailed design

Quote:
    This is the bulk of the RFC. Explain the design in enough detail for somebody familiar
    with the language to understand, and for somebody familiar with the compiler to implement.
    This should get into specifics and corner-cases, and include examples of how the feature is used.

``
`fn main() { ... }` are equal to:
  `函数 main(){。。。}`,
  `函数 入口（）{。。。}`

Any translated keywords/macro names/crate names are equal to original what it was.
Any Fullwidth (chars|symbols) could be recognize as Halfwidth.
  http://en.wikipedia.org/wiki/Halfwidth_and_fullwidth_forms
`使用 标准库：：mem;` are equal `use std::mem;`.
`使用 foo：：bar;` are equal `use foo::bar;`.

```rust
fn main() {
    let n = 5;

    if n < 0 {
        print!("{} is negative", n);
    } else if n > 0 {
        print!("{} is positive", n);
    } else {
        print!("{} is zero", n);
    }

    let big_n =
        if n < 10 && n > -10 {
            println!(", and is a small number, increase ten-fold");

            // This expression returns an `i32`.
            10 * n
        } else {
            println!(", and is a big number, reduce by two");

            // This expression must return an `i32` as well.
            n / 2
            // TODO ^ Try suppressing this expression with a semicolon.
        };
    //   ^ Don't forget to put a semicolon here! All `let` bindings need it.

    println!("{} -> {}", n, big_n);
}
```

``` 
函数 入口() {
    变量 n = 5;

    如果 n < 0 {
        打印!（“{} 是负数”， n）；
    } 否则 如果 n > 0 {
        打印!（“{} 是正数”， n）；
    } 否则 {
        // it should be possible for http://en.wikipedia.org/wiki/Halfwidth_and_fullwidth_forms
        打印!("{} 为零", n); 
    }

    变量 big_n =
        如果 n < 10 && n > -10 {
            打印行!(", and is a small number, increase ten-fold");

            // This expression returns an `i32`.
            10 * n
        } 否则 {
            打印行!(", and is a big number, reduce by two");

            // This expression must return an `i32` as well.
            n / 2
            // TODO ^ Try suppressing this expression with a semicolon.
        }；
    //   ^ Don't forget to put a semicolon here! All `let` bindings need it.

    打印行!("{} -> {}", n, big_n)；
}
```

```rust
fn main() {
    // All have type `Option<i32>`
    let number   = Some(7);
    let letter: Option<i32> = None;
    let emoticon: Option<i32> = None;

    // The `if let` construct reads: "if `let` destructures `number` into
    // `Some(i)`, evaluate the block (`{}`). Else do nothing.
    if let Some(i) = number {
        println!("Matched {:?}!", i);
    }

    // If you need to specify a failure, use an else:
    if let Some(i) = letter {
        println!("Matched {:?}!", i);
    } else {
        // Destructure failed. Change the failure case.
        println!("Didn't match a number. Let's go with a letter!");
    };

    // Provide an altered failing condition.
    let i_like_letters = false;

    if let Some(i) = emoticon {
        println!("Matched {:?}!", i);
    // Destructure failed. Evaluated the condition to see if this branch
    // should be taken.
    } else if i_like_letters {
        println!("Didn't match a number. Let's go with a letter!");
    // The condition evaluated false. This branch is the default.
    } else {
        println!("I don't like letters. Let's go with an emoticon :)!");
    };
}
```

```
函数 入口() {
    // All have type `Option<i32>`
    变量 数字 = 有些(7)；
    变量 字母: 选项<i32> = 没有；
    变量 表情符号: 选项<i32> = 没有；

    // The `if let` construct reads: "if `let` destructures `number` into
    // `Some(i)`, evaluate the block (`{}`). Else do nothing.
    如果 变量 有些(甲) = 数字 {
        打印行!("已匹配 {:?}!", 甲);
    }

    // If you need to specify a failure, use an else:
    如果 变量 有些(甲) = 字母 {
        打印行!("已匹配 {:?}!", 甲);
    } 否则 {
        // Destructure failed. Change the failure case.
        打印!("Didn't match a number. Let's go with a letter!");
    };

    // Provide an altered failing condition.
    变量 我喜欢字母 = 假;

    如果 变量 有些(甲) = 符号表情 {
        打印行!("已匹配 {:?}!", 甲);
    // Destructure failed. Evaluated the condition to see if this branch
    // should be taken.
    } 否则 如果 我喜欢字母 {
        打印行!("Didn't match a number. Let's go with a letter!");
    // The condition evaluated false. This branch is the default.
    } 否则 {
        打印行!("I don't like letters. Let's go with an emoticon :)!");
    };
}
```

```rust
macro_rules! create_function {
    // this macro takes an argument of "type" `ident`
    // the `ident` designator is used for variable/function names
    ($func_name:ident) => (
        // this macro creates a function with name `$func_name`
        fn $func_name() {
            // the stringify! macro converts an `ident` into a string
            println!("You called {:?}()",
                     stringify!($func_name))
        }
    )
}

create_function!(foo);
create_function!(bar);

macro_rules! print_result {
    // the `expr` designator is used for expressions
    ($expression:expr) => (
        // stringify! will convert the expression *as it is* into a string
        println!("{:?} = {:?}",
                 stringify!($expression),
                 $expression)
    )
}

fn main() {
    foo();
    bar();

    print_result!(1u32 + 1);

    // remember that blocks are expressions
    print_result!({
        let x = 1u32;

        x * x + 2 * x - 1
    });
}
```

```
宏规则！ 创建函数 {
    // this macro takes an argument of "type" `ident`
    // the `ident` designator is used for variable/function names
    ($函数名:标识) => (
        // this macro creates a function with name `$func_name`
        函数 $函数名() {
            // the stringify! macro converts an `ident` into a string
            打印行!("你召唤了 {:?}()",
                     字符化!($函数名))
        }
    )
}

创建函数!(foo);
创建函数!(bar);

// Yes, this also could be write as English.
macro_rules! print_result {
    // the `expr` designator is used for expressions
    ($expression:expr) => (
        // stringify! will convert the expression *as it is* into a string
        println!("{:?} = {:?}",
                 stringify!($expression),
                 $expression)
    )
}

函数 入口() {
    foo();
    bar();

    print_result!(1u32 + 1);

    // remember that blocks are expressions
    print_result!({
        let x = 1u32;

        x * x + 2 * x - 1
    });
}
```

```rust
// unit-test.rs
// For .powi()
use std::num::Float;


fn distance(a: (f32, f32), b: (f32, f32)) -> f32 {
    (
        (b.0.powi(2) - a.0.powi(2)) +
        (b.1.powi(2) - a.1.powi(2))
    ).sqrt()
}

fn main() {
    println!("If you see this, the tests were not compiled nor ran!");
}

#[test]
fn distance_test() {
    assert!(distance((0f32, 0f32), (1f32, 1f32)) == (2f32).sqrt());
}
```

```
// unit-test.rs
// For .powi()
使用 标准库::数字::浮点数;


函数 距离(a: (f32, f32), b: (f32, f32)) -> f32 {
    (
        (b.0.N次方(2) - a.0.N次方(2)) +
        (b.1.N次方(2) - a.1.N次方(2))
    ).开方()
}

fn main() {
    println!("If you see this, the tests were not compiled nor ran!");
}

#[测试]
fn distance_test() {
    断言!(distance((0f32, 0f32), (1f32, 1f32)) == (2f32).sqrt());
}
```

# Drawbacks

Q: Why should we *not* do this?

A: I said, You could be not to do this, I do.

# Alternatives

Q: What other designs have been considered?

A: Compiler plugin macro.

Q: What is the impact of not doing this?

A: I think it is reinvert the wheel.


# Unresolved questions

Q: What parts of the design are still TBD?

A: TBD? Could you explain it?
