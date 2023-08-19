#![allow(dead_code)]

use ensemble::Model;
use serde::{Deserialize, Serialize};

#[test]
fn returns_labeled_primary_key() {
    #[derive(Debug, Model, Serialize, Deserialize)]
    struct MyModel {
        #[model(primary)]
        my_primary_key: u8,

        id: u8,
    }

    assert_eq!(MyModel::PRIMARY_KEY, "my_primary_key");
}

#[test]
fn marks_id_as_primary_key_if_found() {
    #[derive(Debug, Model, Serialize, Deserialize)]
    struct MyModel {
        id: u8,
    }

    assert_eq!(MyModel::PRIMARY_KEY, "id");
}
