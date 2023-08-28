#![allow(dead_code)]

use ensemble::Model;

#[test]
fn derives_table_name_from_model_name() {
    #[derive(Debug, Model)]
    struct User {
        id: u8,
    }

    #[derive(Debug, Model)]
    struct Music {
        id: u8,
    }

    #[derive(Debug, Model)]
    struct Index {
        id: u8,
    }

    #[derive(Debug, Model)]
    struct AirTrafficController {
        id: u8,
    }

    assert_eq!(User::TABLE_NAME, "users");
    assert_eq!(Music::TABLE_NAME, "music");
    assert_eq!(Index::TABLE_NAME, "indices");
    assert_eq!(AirTrafficController::TABLE_NAME, "air_traffic_controllers");
}

#[test]
fn derived_table_name_can_be_overriden_with_attribute() {
    #[derive(Debug, Model)]
    #[ensemble(table = "custom_table")]
    struct ModelWithCustomTableName {
        id: u8,
    }

    assert_eq!(ModelWithCustomTableName::TABLE_NAME, "custom_table");
}
