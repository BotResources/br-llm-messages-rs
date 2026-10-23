use std::collections::HashSet;

use super::ErrorCode;

#[test]
fn given_every_error_code_when_named_then_names_are_unique_and_parse_back() {
    let mut names = HashSet::new();
    for code in ErrorCode::ALL {
        assert!(names.insert(code.as_str()), "{} repeated", code.as_str());
        assert_eq!(ErrorCode::parse(code.as_str()), Some(*code));
    }
}

#[test]
fn given_an_unknown_name_when_parsed_then_no_code() {
    assert_eq!(ErrorCode::parse("not_a_code"), None);
}
