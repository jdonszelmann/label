
# Label

![](https://img.shields.io/crates/v/label)
![](https://docs.rs/label/badge.svg)
![](https://github.com/jonay2000/label/workflows/label/badge.svg)

`label` is a library that can be used to create custom attributes for functions, through which you can list them and perform actions on them.
Label uses no global state during the compilation process, to avoid incremental compilation breaking it.

# Example

```rust

create_label!(fn test() -> ());

#[test::label]
fn my_fn() {
    println!("Test!");
}

fn main() {
    println!("calling all 'test' label");
    // using iter you can go through all functions with this annotation.
    for i in test::iter() {
        i();
    }
}

```

Label also supports labels on `static` and `const` variables, and iterating over the names of labeled items.
For more information about this, visit the [docs](https://docs.rs/label)

## Contributing

Any contributions are welcome. Just make a pull request or issue and I will try to respond as soon as possible.

### License

[MIT](./LICENSE)
