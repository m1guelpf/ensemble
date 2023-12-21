## Introduction

Migrations are like version control for your database, allowing your team to define and share the application's database schema definition. If you have ever had to tell a teammate to manually add a column to their local database schema after pulling in your changes from source control, you've faced the problem that database migrations solve.

Ensemble's [`Schema`] struct provides database agnostic support for creating and manipulating tables across all of Laravel's supported database systems. Typically, migrations will use this struct to create and modify database tables and columns.

## Creating Migrations

Ensemble migrations consist of a struct implementing the [`Migration`] interface. Typically, these will leave in the `migrations` module of your application. Here's an example migration:

```rust
use ensemble::migrations::{Migration, Error};

#[derive(Debug, Default)]
struct CreateUsersTable;

#[ensemble::async_trait]
impl Migration for CreateUsersTable {
    async fn up(&self) -> Result<(), Error> {
        todo!()
    }

    async fn down(&self) -> Result<(), Error> {
        todo!()
    }
}
```

## Migration Structure

The [`Migration`] trait contains two methods: `up` and `down`. The `up` method is used to add new tables, columns, or indexes to your database, while the `down` method should reverse the operations performed by the `up` method.

Within both of these methods, you may use Ensemble's [schema builder](Schema) to expressively create and modify tables. To learn about all of the methods available on the [`Schema`] builder, check out its documentation. For example, the following migration creates a flights table:

```rust
use ensemble::migrations::Schema;
# use ensemble::migrations::{Migration, Error};

#[derive(Debug, Default)]
struct CreateFlightsTable;

#[ensemble::async_trait]
impl Migration for CreateFlightsTable {
    async fn up(&self) -> Result<(), Error> {
        Schema::create("flights", |table| {
            table.id();
            table.string("name");
            table.string("airline");
            table.timestamps();
        }).await
    }

    async fn down(&self) -> Result<(), Error> {
        Schema::drop("flights").await
    }
}
```

## Running Migrations

To run all of your outstanding migrations, call the [`migrate!`](crate::migrate) macro somewhere within your application's code with a list of your migrations:

```rust no_run
use std::env;
# mod migrations {
# #[derive(Default)]
# pub struct CreateFlightsTable;
# #[ensemble::async_trait]
# impl ensemble::migrations::Migration for CreateFlightsTable {
#     async fn up(&self) -> Result<(), ensemble::migrations::Error> {
#         todo!()
#     }
#
#     async fn down(&self) -> Result<(), ensemble::migrations::Error> {
#         todo!()
#     }
# }
# }

#[tokio::main]
async fn main() {
    // make sure to call ensemble::setup() first!
    ensemble::setup(&env::var("DATABASE_URL").expect("DATABASE_URL must be set"))
        .await
        .expect("Failed to set up database pool.");

    ensemble::migrate!(migrations::CreateFlightsTable)
        .await
        .expect("Failed to run migrations.");
}
```

## Tables

### Creating tables

To create a new database table, use the `create` method on the [`Schema`] struct. The create method accepts two arguments: the first is the name of the table, while the second is a closure which receives a [`Table`](schema::Table) object that may be used to define the new table:

```rust
use ensemble::migrations::Schema;
# use ensemble::migrations::schema::Table;

Schema::create("users", |table: &mut Table| {
    table.id();
    table.string("name");
    table.string("email");
    table.timestamps();
});
```

When creating the table, you may use any of the schema builder's [column methods](schema::Table) to define the table's columns.

#### Table Options

When using MySQL, you may use the `charset` and `collation` properties to specify the character set and collation for the created table when using MySQL:

```rust
# use ensemble::migrations::Schema;
Schema::create("users", |table| {
    table.charset = "utf8mb4";
    table.collation = "utf8mb4_unicode_ci";

    // ...
});
```

### Updating Tables

The `table` method on the [`Schema`] facade may be used to update existing tables. Like the `create` method, the `table` method accepts two arguments: the name of the table and a closure that receives a Table instance you may use to add columns or indexes to the table:

```rust
# use ensemble::migrations::Schema;
Schema::create("users", |table| {
    table.integer("votes");
});
```

### Renaming / Dropping Tables

To rename an existing database table, use the `rename` method:

```rust
# use ensemble::migrations::Schema;
Schema::rename("old_table", "new_table");
```

To drop an existing table, you may use the drop or dropIfExists methods:

```rust
# use ensemble::migrations::Schema;
Schema::drop("users");

Schema::drop_if_exists("users");
```

#### Renaming Tables With Foreign Keys

Before renaming a table, you should verify that any foreign key constraints on the table have an explicit name in your migration files instead of letting Ensemble assign a convention based name. Otherwise, the foreign key constraint name will refer to the old table name.

## Columns

### Creating Columns

The `table` method on the [`Schema`] struct may be used to update existing tables. Like the `create` method, the `table` method accepts two arguments: the name of the table and a closure that receives a [`Table`](schema::Table) instance you may use to add columns to the table:

```rust
# use ensemble::migrations::Schema;
Schema::create("users", |table| {
    table.integer("votes");
});
```

### Available Column Types

The schema builder blueprint offers a variety of methods that correspond to the different types of columns you can add to your database tables. You can see all the listed methods on the [`Table`](schema::Table) struct.

### Column Modifiers

In addition to the column types listed above, there are several column "modifiers" you may use when adding a column to a database table. For example, to make the column "nullable", you may use the nullable method:

```rust
# use ensemble::migrations::Schema;
Schema::table("users", |table| {
    table.string("email").nullable(true);
});
```

The following table contains all of the available column modifiers. This list does not include [index modifiers](#creating-indexes):

| Modifier                           | Description                                                                           |
| ---------------------------------- | ------------------------------------------------------------------------------------- |
| `.after("column")`                 | Place the column "after" another column (MySQL).                                      |
| `.auto_increment(true)`            | Set INTEGER columns as auto-incrementing (requires either a primary or unique index). |
| `.charset("utf8mb4")`              | Specify a character set for the column (MySQL).                                       |
| `.collation("utf8mb4_unicode_ci")` | Specify a collation for the column.                                                   |
| `.comment("my comment")`           | Add a comment to a column.                                                            |
| `.default(value)`                  | Specify a "default" value for the column.                                             |
| `.nullable(true)`                  | Allow NULL values to be inserted into the column.                                     |
| `.unsigned(true)`                  | Set INTEGER columns as UNSIGNED (MySQL).                                              |
| `.use_current(true)`               | Set `TIMESTAMP` columns to use `CURRENT_TIMESTAMP` as default value.                  |
| `use_current_on_update(true)`      | Set `TIMESTAMP` columns to use `CURRENT_TIMESTAMP` when a record is updated (MySQL).  |

### Modifying Columns

The `change` method allows you to modify the type and attributes of existing columns. For example, you may wish to make an existing column nullable. To see the `change` method in action, let's make the `address` column nullable. To accomplish this, we simply define the new state of the column and then call the `change` method:

```rust
# use ensemble::migrations::Schema;
Schema::table("users", |table| {
    table.string("email").nullable(true).change();
});
```

When modifying a column, you must explicitly include all of the modifiers you want to keep on the column definition - any missing attribute will be dropped. For example, to retain the `unsigned`, and `default` attributes, you must call each modifier explicitly when changing the column:

```rust
# use ensemble::migrations::Schema;
Schema::table("users", |table| {
    table.integer("votes").unsigned().default(1).change();
});
```

### Renaming Columns

To rename a column, you may use the `rename_column` method provided by the [schema builder](schema::Table):

```rust
# use ensemble::migrations::Schema;
Schema::table("users", |table| {
    table.rename_column("from", "to");
});
```

Note that, if you're using MySQL, you must be on version `8.0.3` or newer for this to work.

### Dropping Columns

To drop a column, you may use the `drop_column` method on the [schema builder](schema::Table):

```rust
# use ensemble::migrations::Schema;
Schema::table("users", |table| {
    table.drop_column("votes");
});
```

You may drop multiple columns from a table by passing an array of column names to the `drop_column` method:

```rust
# use ensemble::migrations::Schema;
Schema::table("users", |table| {
    table.drop_column(&["votes", "avatar", "location"]);
});
```

## Indexes

### Creating Indexes

The Ensemble schema builder supports several types of indexes. The following example creates a new `email` column and specifies that its values should be unique. To create the index, we can chain the `unique` method onto the column definition:

```rust
# use ensemble::migrations::Schema;
Schema::table("users", |table| {
    table.string("email").unique(true);
});
```

Alternatively, you may create the index after defining the column. To do so, you should call the unique method on the schema builder. This method accepts the name of the column that should receive a unique index:

```rust
# use ensemble::migrations::Schema;
# Schema::table("users", |table| {
table.unique("email");
# });
```

You may even pass an array of columns to an index method to create a compound (or composite) index:

```rust
# use ensemble::migrations::Schema;
# Schema::table("users", |table| {
table.unique(&["account_id", "created_at"]);
# });
```

#### Available Index Types

Ensemble's schema builder class provides methods for creating each type of supported index. The name will be derived from the names of the table and column(s) used for the index, as well as the index type. Each of the available index methods is described in the table below:

| Command                               | Description             |
| ------------------------------------- | ----------------------- |
| `table.primary("id")`                 | Adds a primary key.     |
| `table.primary(&["id", "parent_id"])` | Adds composite keys.    |
| `table.unique("email")`               | Adds a unique index.    |
| `table.index("state")`                | Adds an index.          |
| `table.full_text("body")`             | Adds a full text index. |

```rust
# use ensemble::migrations::Schema;
# Schema::table("users", |table| {
table.primary("id"); // Adds a primary key
table.primary(&["id", "parent_id"]); // Adds composite keys
table.index("state");
table.full_text("body");
# });
```

#### Index Lengths & MySQL

By default, Ensemble uses the `utf8mb4` character set. If you are running a version of MySQL older than the 5.7.7 release, you may need to enable the `innodb_large_prefix` option for your database. Refer to your database's documentation for instructions on how to properly enable this option.

### Renaming Indexes

To rename an index, you may use the `rename_index` method provided by the schema builder. This method accepts the current index name as its first argument and the desired name as its second argument:

```rust
# use ensemble::migrations::Schema;
# Schema::table("users", |table| {
table.rename_index("from", "to");
# });
```

### Dropping Indexes

To drop an index, you must specify the index's name. By default, Ensemble automatically assigns an index name based on the table name, the name of the indexed column, and the index type. Here are some examples:

| Command                                       | Description                                    |
| --------------------------------------------- | ---------------------------------------------- |
| `table.drop_primary("users_id_primary")`      | Drop a primary key from the "users" table.     |
| `table.drop_unique("users_email_unique")`     | Drop a unique index from the "users" table.    |
| `table.drop_index("geo_state_index")`         | Drop a basic index from the "geo" table.       |
| `table.drop_full_text("posts_body_fulltext")` | Drop a full text index from the "posts" table. |

If you pass an array of columns into a method that drops indexes, the conventional index name will be generated based on the table name, columns, and index type:

```rust
# use ensemble::migrations::Schema;
Schema::table("geo", |table| {
    table.drop_index(&["state"]);
});
```

### Foreign Key Constraints

Ensemble also provides support for creating foreign key constraints, which are used to force referential integrity at the database level. For example, let's define a `user_id` column on the `posts` table that references the `id` column on a `users` table:

```rust
# use ensemble::migrations::Schema;
Schema::table("posts", |table| {
    table.integer("user_id");

    table.foreign("user_id").references("id").on("users");
});
```

Since this syntax is rather verbose, Ensemble provides additional, terser methods that use conventions to provide a better developer experience. When using the `foreign_id` method to create your column, the example above can be rewritten like so:

```rust
# use ensemble::migrations::Schema;
Schema::table("posts", |table| {
    table.foreign_id("user_id");
});
```

The `foreign_id` method creates an `UNSIGNED BIGINT` equivalent column, and will use conventions to determine the table and column being referenced. If your table name does not match Ensemble's conventions, you can still use the `references` and `on` methods to configure it.

You may also specify the desired action for the "on delete" and "on update" properties of the constraint:

```rust
# use ensemble::migrations::Schema;
Schema::table("posts", |table| {
    table.foreign_id("user_id")
        .on_delete("cascade")
        .on_update("cascade");
});
```

#### Dropping Foreign Keys

To drop a foreign key, you may use the `drop_foreign` method, passing the name of the foreign key constraint to be deleted as an argument. Foreign key constraints use the same naming convention as indexes. In other words, the foreign key constraint name is based on the name of the table and the columns in the constraint, followed by a "\_foreign" suffix:

```rust
# use ensemble::migrations::Schema;
# Schema::table("posts", |table| {
table.drop_foreign("posts_user_id_foreign");
# });
```

Alternatively, you may pass an array containing the column name that holds the foreign key to the `drop_foreign` method. The array will be converted to a foreign key constraint name using Ensemble's constraint naming conventions:

```rust
# use ensemble::migrations::Schema;
# Schema::table("posts", |table| {
table.drop_foreign(&["user_id"]);
# });
```
