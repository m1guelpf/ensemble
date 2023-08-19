#![allow(dead_code)]

use ensemble::Model;
use serde::{Deserialize, Serialize};

#[test]
fn extracts_the_model_fields() {
    #[derive(Debug, Model, Serialize, Deserialize)]
    struct MyModel {
        id: u8,
        name: String,
        email: String,
    }

    assert_eq!(MyModel::keys(), vec!["id", "name", "email"]);
}
