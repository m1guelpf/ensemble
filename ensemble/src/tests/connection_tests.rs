use tokio_test::block_on;
use rbatis::RBatis;
use ensemble::connection::{setup, get};
use ensemble::Model;

#[test]
fn setup_test() {
    let database_url = "postgres://username:password@localhost/database";
    let role = "test_role";

    let result = block_on(setup(database_url, Some(role)));

    assert!(result.is_ok());
    assert!(RBatis::is_role_assigned("test_role"));
}

#[test]
fn get_test() {
    let result = block_on(get());

    assert!(result.is_ok());
    let connection = result.unwrap();
    assert_eq!(connection.current_role(), Some("test_role"));
}

#[test]
fn assume_role_test() {
    struct MockModel;
impl Model for MockModel {
    type PrimaryKey = i32; // Assuming PrimaryKey is of type i32
    // Implement any other required methods for the Model trait here
}

    let role = "test_role";
    let result = block_on(mock_model.assume_role(role));

    assert!(result.is_ok());
    let assumed_role = MockModel::assume_role(role).await;
assert!(assumed_role.is_ok());
// Assuming we have a way to extract the current role from MockModel (e.g., method `current_role`)
assert_eq!(MockModel::current_role(), Some(role));
}

