#![allow(dead_code)]

use ensemble::Model;

#[test]
fn derives_table_name_from_model_name() {
    #[derive(Model)]
    struct User {
        id: u8,
    }

    #[derive(Model)]
    struct Music {
        id: u8,
    }

    #[derive(Model)]
    struct Index {
        id: u8,
    }

    assert_eq!(User::table_name(), "users");
    assert_eq!(Music::table_name(), "music");
    assert_eq!(Index::table_name(), "indices");
}

#[test]
fn derived_table_name_can_be_overriden_with_attribute() {
    #[derive(Model)]
    #[ensemble(table_name = "custom_table")]
    struct ModelWithCustomTableName {
        id: u8,
    }

    assert_eq!(ModelWithCustomTableName::table_name(), "custom_table");
}
