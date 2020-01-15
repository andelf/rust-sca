//! Test vectors are from GM/T 0004-2012
use sm3::Sm3;
use digest::Digest;

#[test]
fn sm3_example_1() {
    let string = "abc".to_owned();
    let s = string.as_bytes();

    let mut hasher = Sm3::new();
    hasher.input(s);
    assert_eq!(
        format!("{:x}", hasher.result()),
        "66c7f0f462eeedd9d1f2d46bdc10e4e24167c4875cf2f7a2297da02b8f4ba8e0"
    );
}

#[test]
fn sm3_example_2() {
    let string = "abcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcd".to_owned();
    let s = string.as_bytes();

    let mut hasher = Sm3::new();
    hasher.input(s);
    assert_eq!(
        format!("{:x}", hasher.result()),
        "debe9ff92275b8a138604889c18e5a4d6fdb70e5387e5765293dcba39c0c5732"
    );
}
