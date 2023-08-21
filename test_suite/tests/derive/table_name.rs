#![allow(dead_code)]

use ensemble::Model;
use serde::{Deserialize, Serialize};

#[test]
fn derives_table_name_from_model_name() {
    #[derive(Model, Serialize, Deserialize)]
    struct User {
        id: u8,
    }

    #[derive(Model, Serialize, Deserialize)]
    struct Music {
        id: u8,
    }

    #[derive(Model, Serialize, Deserialize)]
    struct Index {
        id: u8,
    }

    #[derive(Model, Serialize, Deserialize)]
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
    #[derive(Model, Serialize, Deserialize)]
    #[ensemble(table = "custom_table")]
    struct ModelWithCustomTableName {
        id: u8,
    }

    assert_eq!(ModelWithCustomTableName::TABLE_NAME, "custom_table");
}
