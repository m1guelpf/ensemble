#![allow(dead_code)]

use ensemble::Model;

#[test]
fn returns_labeled_primary_key() {
    #[derive(Debug, Model)]
    struct MyModel {
        #[model(primary)]
        my_primary_key: u8,

        id: u8,
    }

    assert_eq!(MyModel::primary_key(), "my_primary_key");
}

#[test]
fn marks_id_as_primary_key_if_found() {
    #[derive(Debug, Model)]
    struct MyModel {
        id: u8,
    }

    assert_eq!(MyModel::primary_key(), "id");
}
