use ensemble::migrations::{Error, Migration, Schema};

#[derive(Debug, Default)]
pub struct CreateUsersTable;

#[ensemble::async_trait]
impl Migration for CreateUsersTable {
    async fn up(&self) -> Result<(), Error> {
        Schema::create("users", |table| {
            table.id();
            table.string("name");
            table.string("email").unique(true);
            table.string("password");
            table.timestamps();
        })
        .await
    }

    async fn down(&self) -> Result<(), Error> {
        Schema::drop("users").await
    }
}

#[derive(Debug, Default)]
pub struct CreatePostsTable;

#[ensemble::async_trait]
impl Migration for CreatePostsTable {
    async fn up(&self) -> Result<(), Error> {
        Schema::create("posts", |table| {
            table.id();
            table.string("title");
            table.text("content");
            table.timestamps();

            table.foreign_id("user_id").on_delete("cascade");
        })
        .await
    }

    async fn down(&self) -> Result<(), Error> {
        Schema::drop("posts").await
    }
}
