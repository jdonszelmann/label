
# Label functions and iterate over them

`label` is a library that can be used to create custom attributes for functions, through which you can list them and perform actions on them.

# Example

```rust

create_annotatation!(fn test() -> (););

#[test::annotate]
fn my_fn() {
    println!("Test!");
}

fn main() {
    println!("calling all 'test' annotations");
    // using iter you can go through all functions with this annotation.
    for i in test::iter() {
        i();
    }
}

```

### License

[MIT](./LICENSE)
