use label::create_label;

create_label!(
    static staticname: usize;
    const constname: usize;
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
