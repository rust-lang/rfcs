- Feature Name: `rust_lambdas`
- Start Date: 2023-09-24)
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC proposes the introduction of lambda expressions, also known as anonymous functions or lambdas, to Rust using a concise `(arg1, arg2, ..., argn) -> { // code block }` syntax. Lambda expressions provide a way to define small, inline functions that can capture variables from their surrounding scope.

# Motivation
[motivation]: #motivation

Lambda expressions in Rust address several pain points and limitations present in the existing RFC for [`closure_to_fn_coercion` (RFC 1558)](https://rust-lang.github.io/rfcs/1558-closure-to-fn-coercion.html). While RFC 1558 focused on improving the coercion of closures to function pointers (fn), it didn't provide a concise and intuitive syntax for defining anonymous functions. The introduction of this RFC complements RFC 1558 by offering a more **expressive** and **readable** way to work with anonymous functions.

# Improvements Over RFC 1558
[improvements-over-rfc-1558]: #improvements-over-rfc-1558

1. **Concise Syntax**: RFC 1558 primarily dealt with improving coercion mechanisms, which did not address the verbosity and complexity of closure syntax in Rust. Lambda expressions provide a concise and clear (arg1, arg2, ..., argn) -> { // code block } syntax for defining anonymous functions, making the code more readable and approachable.

1. **Improved Code Expressiveness**: Lambda expressions enhance code expressiveness by allowing developers to define functions in a more straightforward manner. This directly benefits the readability of Rust codebases and reduces the need for boilerplate code.

1. **Familiarity with Other Languages**: Lambda expressions are a feature commonly found in many modern programming languages, such as JavaScript, Java, and Python. Introducing lambda expressions in Rust aligns its syntax with these languages, making it more accessible to developers with experience in those ecosystems.

1. **Better Integration with Closures**: While RFC 1558 focused on coercion, it did not directly address the convenience of working with closures themselves. Lambda expressions improve the overall experience of working with closures, offering a more natural way to define and use them.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Lambda expressions in Rust provide a concise syntax for defining anonymous functions directly within your code. This feature is commonly found in modern programming languages and offers improved code readability and expressiveness, especially for short, one-time-use functions.

Here's an example of how lambda expressions might look in Rust:

```rust
fn main() {
    let add = (x, y) -> {x + y};
    let result = add(6, 3);
    println!("Result: {}", result); // Outputs "Result: 9"
}
```

In this example, `(x, y) -> {x + y}` is a lambda expression that defines a function taking two parameters, `x` and `y`, and returning their sum. This concise notation simplifies the creation of small, on-the-fly functions.

Functional programming often involves passing functions as arguments to higher-order functions. Lambda expressions make it straightforward to create and pass anonymous functions. For instance:

```rust
fn apply_operation(operands: Vec<i32>, operation: impl Fn(i32, i32) -> i32) -> i32 {
    operands.iter().fold(0, (acc, &x) -> operation(acc, x))
}

fn main() {
    let numbers = vec![1, 2, 3, 4, 5];

    // Using lambda expressions to define custom operations
    let addition = (x, y) -> { x + y };
    let multiplication = (x, y) -> { x * y };

    let result_add = apply_operation(numbers.clone(), addition);
    println!("Addition Result: {}", result_add); // Outputs "Addition Result: 15"

    let result_mul = apply_operation(numbers, multiplication);
    println!("Multiplication Result: {}", result_mul); // Outputs "Multiplication Result: 120"
}
```

In this extended example, lambda expressions enable the definition of custom operations on the fly, simplifying the use of higher-order functions. This feature enhances code modularity and readability, making Rust even more versatile for various programming paradigms.

# Comparison to Other Languages
[comparison-to-other-languages]: #comparison-to-other-languages

Lambda expressions are commonly found in other programming languages, providing a convenient way to define short, anonymous functions. Let's compare how lambda expressions in Rust using the `(x, y) -> {x + y}` notation stack up against their counterparts in other languages:

In JavaScript, lambda expressions are widely used, and they offer a concise way to define anonymous functions. Here's how you might write a similar lambda expression in JavaScript:

```javascript
const add = (x, y) => x + y;
```

The `(x, y) -> {x + y}` notation for lambda expressions in Rust bears a resemblance to lambda expressions in languages like Java:

```java
Function<Integer, Integer> add = (x, y) -> x + y;
```

In this Java example, `(x, y) -> x + y` defines a lambda expression that takes two integers and returns their sum.

Python developers often use lambda expressions in conjunction with functions like `map`, `filter`, and `reduce` to perform concise operations on collections. Here's an example in Python:

```python
numbers = [1, 2, 3, 4, 5]
squared = list(map(lambda x: x ** 2, numbers))
```

In this Python code, `lambda x: x ** 2` is a lambda expression employed with the `map` function to square each element in the `numbers` list.

This comparison demonstrates that Rust's lambda expressions, with their `(x, y) -> {x + y}` notation, align with the concise and expressive capabilities seen in other languages, making Rust more accessible and versatile for developers coming from diverse programming backgrounds.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Implementing lambda expressions in Rust requires adjustments to the Rust parser and compiler to recognize and effectively handle the lambda syntax. Additionally, it involves establishing clear rules and semantics governing variable capture, lifetime management, and other aspects specific to lambda expressions.

These technical details ensure that lambda expressions are integrated seamlessly into the Rust language, maintaining consistency and coherence with existing Rust features while enhancing the expressiveness and flexibility of the language.

# Drawbacks
[drawbacks]: #drawbacks

As with any language feature, introducing lambda expressions has potential drawbacks:

- **Learning Curve**: While lambda expressions are common in many programming languages, they may be new to some Rust developers. It could introduce a learning curve for those unfamiliar with this syntax.

- **Integration Challenges**: Lambda expressions need to be well-integrated with Rust's existing closure functionality. Ensuring a smooth transition between closures and lambda expressions is essential.

- **Code Consistency**: The introduction of lambda expressions alongside closures could lead to codebase inconsistencies if developers use both features interchangeably without clear guidelines.

# Rationale and Alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Rationale

By introducing lambda expressions, Rust aims to align itself with modern programming languages that have embraced this feature, making the language more accessible to developers coming from those backgrounds. This addition does not replace existing closure functionality but complements it by offering a more convenient alternative for certain use cases.

## Alternatives

While closures are powerful and versatile, which involve using the `|args| { code }` notation. they may appear less intuitive, especially to developers accustomed to languages with lambda expressions. Lambda expressions provide a more concise and readable way to define short, anonymous functions, which can lead to improved code maintainability and understandability, especially in situations where functions are used as arguments or returned values.

It's important to note that introducing lambda expressions in Rust does not replace closures but offers an additional tool in the developer's toolbox, providing more flexibility and choice when writing Rust code. Developers can choose the approach that best suits their specific use case and coding style.

# Prior Art
[prior-art]: #prior-art

The experience from the programming languages mentioned above demonstrates that lambda expressions can significantly enhance code readability and maintainability, especially for short, one-off functions and functional programming tasks. Their concise syntax and ability to capture variables from the surrounding scope make them valuable tools in a developer's toolkit.

Additionally, the introduction of lambda expressions in Rust aligns with the trend of incorporating functional programming features into modern programming languages, enhancing Rust's versatility and making it more accessible to developers from diverse language backgrounds. However, Rust's unique approach to ownership and borrowing may require careful consideration when implementing lambda expressions to maintain the language's safety and performance guarantees.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- What would be the specific syntax for lambda expressions in Rust?
- How would lambda expressions interact with Rust's ownership and borrowing system?
- What impact might lambda expressions have on code readability and maintainability in the Rust ecosystem?
- Should there be any differences in variable capture behavior between lambda expressions and closures?

# Implementation Plan
[implementation-plan]: #implementation-plan

To bring lambda expressions into the Rust ecosystem, the following steps would be taken:

1. Specification: Establish a formal syntax for lambda expressions within the Rust language specification.

1. Compiler Implementation: Modify the Rust compiler to incorporate lambda expression recognition and parsing, using the `(arg1, arg2, ..., argn) -> { // code block }` notation.

1. Semantic Framework: Define the semantics governing lambda expressions, encompassing rules for variable capture and lifetime management.

1. Documentation Enhancement: Update the Rust documentation, enriching it with illustrative examples and educational content to familiarize developers with lambda expressions and their usage.

# Conclusion
[conclusion]: #conclusion

In conclusion, the incorporation of lambda expressions into Rust has the potential to enhance the language's accessibility and expressiveness, particularly for developers experienced with analogous constructs in other programming languages. This RFC advocates for the integration of lambda expressions and welcomes feedback from the Rust community to facilitate their adoption and implementation.

# Future possibilities
[future-possibilities]: #future-possibilities

If lambda expressions are well-received and adopted by the Rust community, this could open the door to further enhancements, such as support for more advanced lambda features, including closures with custom capture modes and traits.

