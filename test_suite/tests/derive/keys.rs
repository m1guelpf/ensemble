#![allow(dead_code)]

use ensemble::Model;

#[test]
fn extracts_the_model_fields() {
    #[derive(Debug, Model)]
    struct MyModel {
        id: u8,
        name: String,
        email: String,
    }

    assert_eq!(MyModel::keys(), vec!["id", "name", "email"]);
}
