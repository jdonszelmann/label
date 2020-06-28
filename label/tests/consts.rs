use label::create_label;

create_label!(
    static staticname: usize;
    const constname: usize;
    static mut staticmutname: usize;
);

#[staticname::label]
static A: usize = 3;

#[constname::label]
const B: usize = 4;

#[test]
fn test_simple() {
    for i in staticname::iter() {
        assert_eq!(*i, 3);
    }

    for i in constname::iter() {
        assert_eq!(*i, 4);
    }
}

#[test]
fn test_named() {
    for (name, i) in staticname::iter_named() {
        assert_eq!(*i, 3);
        assert_eq!(name, "A")
    }
}