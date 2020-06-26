use annotations::{create_annotatation};

// TODO: allow for creating multiple label in one create_annotation! macro.
// Create two label.
create_annotatation!(fn test() -> (););
create_annotatation!(fn test2(usize) -> usize;);

pub mod  child {
    // annotate a function by giving the path to the annotation and postfixing ::annotate.
    #[super::test::annotate]
    fn my_fn() {
        println!("Test2!");
    }
}

pub mod folder {
    // multiple label living in any submodule or supermodule are possible.
    #[crate::test::annotate]
    #[child::test1::annotate]
    fn my_fn() {
        println!("Test4!");
    }

    pub mod  child {
        use annotations::create_annotatation;

        #[super::super::test::annotate]
        fn my_fn() {
            println!("Test3!");
        }

        create_annotatation!(
            fn test1() -> ();
        );
    }
}

#[test::annotate]
#[folder::child::test1::annotate]
fn my_fn() {
    println!("Test1!");
}

#[test2::annotate]
// label are typed, so functions annotated with test2 must take a usize and return one.
fn my_usize_fn(x: usize) -> usize {
    println!("my usize: {}", x);
    x + 1
}

fn main() {
    println!("calling all 'test' label");
    // using iter you can go through all functions with this annotation.
    for i in test::iter() {
        i();
    }

    println!("calling all 'test1' label");
    for i in folder::child::test1::iter() {
        i();
    }

    println!("calling all 'usize' label");
    for i in test2::iter() {
        println!("{}", i(3));
    }
}

