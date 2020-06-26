
# Label

`label` is a library that can be used to create custom attributes for functions, through which you can list them and perform actions on them.

# Example

```rust

create_label!(fn test() -> (););

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

## Contributing

Any contributions are welcome. Just make a pull request or issue and I will try to respond as soon as possible.

### License

[MIT](./LICENSE)
