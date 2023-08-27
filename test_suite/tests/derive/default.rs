#![allow(dead_code)]

use ensemble::types::DateTime;
use ensemble::Model;
use uuid::Uuid;

#[test]
fn automatically_implements_default_for_all_fields() {
    #[derive(Debug, Model)]
    struct MyModel {
        id: u8,
        uuid: Uuid,
        name: String,
    }

    let model = MyModel::default();

    assert_eq!(model.id, u8::default());
    assert_eq!(model.uuid, Uuid::nil());
    assert_eq!(model.name, String::default());
}

#[test]
fn respects_custom_default_values_via_attributes() {
    #[derive(Debug, Model)]
    struct MyModel {
        #[model(default = 42)]
        id: u8,

        #[model(default = "custom_string".to_string())]
        name: String,
    }

    let model = MyModel::default();

    assert_eq!(model.id, 42);
    assert_eq!(model.name, "custom_string".to_string());
}

#[test]
fn initialises_marked_uuids_automatically() {
    #[derive(Debug, Model)]
    struct MyModel {
        #[model(uuid)]
        id: Uuid,
    }

    let model = MyModel::default();

    assert_ne!(model.id, Uuid::nil());
}

#[test]
fn initialises_created_at_and_updated_at_when_marked() {
    #[derive(Debug, Model)]
    struct MyModel {
        id: u8,

        #[model(created_at)]
        created_at: DateTime,
        #[model(updated_at)]
        updated_at: DateTime,
    }

    let model = MyModel::default();

    assert_ne!(model.created_at, DateTime::from_timestamp(0));
    assert_ne!(model.updated_at, DateTime::from_timestamp(0));
}

#[test]
fn initialises_created_at_and_updated_at_when_named() {
    #[derive(Debug, Model)]
    struct MyModel {
        id: u8,

        created_at: DateTime,
        updated_at: DateTime,
    }

    let model = MyModel::default();

    assert_ne!(model.created_at, DateTime::from_timestamp(0));
    assert_ne!(model.updated_at, DateTime::from_timestamp(0));
}
