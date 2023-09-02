#![allow(dead_code)]

use ensemble::rbs::{self, value_map};
use ensemble::value::to_value;
use ensemble::Model;
use serde_json::json;

#[test]
fn properly_serializes_model_to_json() {
    #[derive(Debug, Model)]
    struct MyModel {
        id: u8,
        name: String,
    }

    let model = MyModel {
        id: 123,
        name: "JSON Test".to_string(),
    };

    assert_eq!(model.json(), json!({ "id": 123, "name": "JSON Test" }));
}

#[test]
fn hides_marked_properties_on_returned_json() {
    #[derive(Debug, Model)]
    struct MyModel {
        id: u8,
        #[model(hide)]
        name: String,
    }

    let model = MyModel {
        id: 123,
        name: "JSON Test".to_string(),
    };

    assert_eq!(model.json(), json!({ "id": 123 }));
}

#[test]
fn automatically_hides_password_field() {
    #[derive(Debug, Model)]
    struct MyModel {
        id: u8,
        password: String,
    }

    let model = MyModel {
        id: 123,
        password: "JSON Test".to_string(),
    };

    assert_eq!(model.json(), json!({ "id": 123 }));
}

#[test]
fn hidden_fields_are_still_preserved_on_database() {
    #[derive(Debug, Model)]
    struct MyModel {
        id: u8,
        #[model(hide)]
        name: String,
    }

    let model = MyModel {
        id: 123,
        name: "JSON Test".to_string(),
    };

    assert_eq!(
        to_value(model),
        rbs::Value::Map(value_map! {
            "id" : 123u32,
            "name" : "JSON Test",
        })
    );
}

#[test]
fn hidden_fields_hidden_when_serializing_to_json() {
    #[derive(Debug, Model)]
    struct MyModel {
        id: u8,
        #[model(hide)]
        name: String,
    }

    let model = MyModel {
        id: 123,
        name: "JSON Test".to_string(),
    };

    assert_eq!(serde_json::to_value(model).unwrap(), json!({ "id": 123 }));
}
