## Introduction

Database tables are often related to one another. For example, a blog post may have many comments or an order could be related to the user who placed it. Ensemble makes managing and working with these relationships easy, with native support for the three most common:

-   [One To One](#one-to-one)
-   [One To Many](#one-to-many)
-   [Many To Many](#many-to-many-relationships)

## Defining Relationships

Ensemble relationships are defined as fields on your Ensemble model. Ensemble will automatically generate a homonymous method to resolve the relationship, but you can directly access the relationship object to chain additional query constrainsts at runtime:

```rust
let posts = user.posts().await.unwrap();

let active_posts = user.posts.query()
    .r#where("active", '=', 1)
    .get().await.unwrap();
```

But, before diving too deep into using relationships, let's learn how to define each type of relationship supported by Ensemble.

### One To One

A one-to-one relationship is a very basic type of database relationship. For example, a `User` model might be associated with one `Phone` model. To define this relationship, we will place a `phone` field on the User model. The `phone` field should be of type [`HasOne<User, Phone>`], which is available under the `ensemble::relationships` module:

```rust
use ensemble::{Model, relationships::HasOne};

#[derive(Debug, Model)]
struct User {
    id: u64,
    name: String,

    phone: HasOne<User, Phone>
}
```

The [`HasOne`] type expects two generics: the type of the current model, and the type of the related model. Once the relationship is defined, we may retrieve the related record using the dynamic method Ensemble's registers for you, which will bear the same name as the relationship field:

```rust
let user = User::find(1).await.unwrap();

let phone = user.phone().await.unwrap();
```

Ensemble determines the foreign key of the relationship based on the parent model name. In this case, the `Phone` model is automatically assumed to have a `user_id` foreign key. If you wish to override this convention, you can use the `#[model(foreign_key)]` attribute:

```rust
use ensemble::{Model, relationships::HasOne};

#[derive(Debug, Model)]
struct User {
    id: u64,
    name: String,

    #[model(foreign_key = "foreign_key")]
    phone: HasOne<User, Phone>
}
```

Additionally, Ensemble assumes that the foreign key should have a value matching the primary key column of the parent. In other words, Ensemble will look for the value of the user's primary column in the `user_id` column of the `Phone` record. If you would like the relationship to use a separate value from your model's primary key, you may use the `#[model(column)]` attribute:

```rust
use ensemble::{Model, relationships::HasOne};

#[derive(Debug, Model)]
struct User {
    id: u64,
    name: String,

    #[model(column = "local_key")]
    phone: HasOne<User, Phone>
}
```

#### Defining The Inverse Of The Relationship

So, we can access the `Phone` model from our `User` model. Next, let's define a relationship on the `Phone` model that will let us access the user that owns the phone. We can define the inverse of a [`HasOne`] relationship using the [`BelongsTo`] type:

```rust
use ensemble::{Model, relationships::BelongsTo};

#[derive(Debug, Model)]
struct Phone {
    id: u64,
    number: String,

    user: BelongsTo<Phone, User>
}
```

When invoking the `user` method, Ensemble will attempt to find a `User` model that has a primary key which matches the `user_id` column on the `Phone` model.

Ensemble determines the foreign key name by examining the name of the relationship method and suffixing the method name with `_id`. So, in this case, Ensemble assumes that the `Phone` model has a `user_id` column. However, if the foreign key on the `Phone` model is not `user_id`, you may provide a custom key name using the `#[model(foreign_key)]` attribute:

```rust
use ensemble::{Model, relationships::BelongsTo};

#[derive(Debug, Model)]
struct Phone {
    id: u64,
    number: String,

    #[model(foreign_key = "author_id")]
    user: BelongsTo<Phone, User>
}
```

If you wish to find the associated model using a different column than the model's primary key, you may specify the parent table's custom key using the `#[model(column)]` attribute:

```rust
use ensemble::{Model, relationships::BelongsTo};

#[derive(Debug, Model)]
struct Phone {
    id: u64,
    number: String,

    #[model(foreign_key = "author_id", column = "uuid")]
    user: BelongsTo<Phone, User>
}
```

### One To Many

A one-to-many relationship is used to define relationships where a single model is the parent to one or more child models. For example, a blog post may have an infinite number of comments. Like all other Ensemble relationships, one-to-many relationships are defined by defining a field on your Ensemble model:

```rust
use ensemble::{Model, relationships::HasMany};

#[derive(Debug, Model)]
struct Post {
    id: u64,
    title: String,
    content: String,

    comments: HasMany<Post, Comment>
}
```

Remember, Ensemble will automatically determine the proper foreign key column for the `Comment` model. By convention, Ensemble will take the "snake case" name of the parent model and the related model's primary key. So, in this example, Ensemble will assume the foreign key column on the `Comment` model is `post_id`.

Once the relationship method has been defined, we can access the list of related comments by calling the comments function. Remember, since Ensemble automatically registers a function for each relationship, we can access relationship methods as if they were defined as properties on the model:

```rust
use crate::models::Post;

let post = Post::find(1).await.unwrap();

for comment in post.comments().await.unwrap() {
    // ...
}
```

Since all relationships also serve as query builders, you may add further constraints to the relationship query by calling the `query` method on the comments field and continuing to chain conditions onto the query:

```rust
let post = Post::find(1).await.unwrap();

let comment = post.comments.query()
    .r#where("title", '='. "foo")
    .first().await.unwrap();
```

Like the [`HasOne`] relationship, you may also override the foreign and local keys with the `#[model(foreign_key)]` and `#[model(column)]` attributes:

```rust
use ensemble::{Model, relationships::HasMany};

#[derive(Debug, Model)]
struct Post {
    id: u64,
    title: String,
    content: String,

    #[model(foreign_key = "post_title", column = "title")]
    comments: HasMany<Post, Comment>
}
```

### One To Many (Inverse) / Belongs To

Now that we can access all of a post's comments, let's define a relationship to allow a comment to access its parent post. To define the inverse of a [`HasMany`] relationship, define a field on the child model with the [`BelongsTo`] type:

```rust
use ensemble::{Model, relationships::BelongsTo};

#[derive(Debug, Model)]
struct Comment {
    id: u64,
    content: String,

    post: BelongsTo<Comment, Post>
}
```

Once the relationship has been defined, we can retrieve a comment's parent post by accessing the post function:

```rust
use crate::models::Comment;

let comment = Comment::find(1).await.unwrap();

return comment.post().await.unwrap().title;
```

In the example above, Ensemble will attempt to find a `Post` model that has an id which matches the `post_id` column on the `Comment` model.

Ensemble determines the default foreign key name by examining the name of the parent model and suffixing it with a `_` followed by the name of the parent model's primary key column. So, in this example, Ensemble will assume the `Post` model's foreign key on the comments table is `post_id`.

However, if the foreign key for your relationship does not follow these conventions, you may provide a custom foreign key name using the `#[model(foreign_key)]` attribute:

```rust
use ensemble::{Model, relationships::BelongsTo};

#[derive(Debug, Model)]
struct Comment {
    id: u64,
    content: String,

    #[model(foreign_key = "article_id")]
    post: BelongsTo<Comment, Post>
}
```

If you wish to find the associated model using a different column than the model's primary key, you may specify the parent table's custom key using the `#[model(column)]` attribute:

```rust
use ensemble::{Model, relationships::BelongsTo};

#[derive(Debug, Model)]
struct Comment {
    id: u64,
    content: String,

    #[model(column = "uuid", foreign_key = "article_id")]
    post: BelongsTo<Comment, Post>
}
```

## Many To Many Relationships

Many-to-many relations are slightly more complicated than [`HasOne`] and [`HasMany`] relationships. An example of a many-to-many relationship is a user that has many roles and those roles are also shared by other users in the application. For example, a user may be assigned the role of "Author" and "Editor"; however, those roles may also be assigned to other users as well. So, a user has many roles and a role has many users.

#### Table Structure

To define this relationship, three database tables are needed: `users`, `roles`, and `role_user`. The `role_user` table is derived from the alphabetical order of the related model names and contains `user_id` and `role_id` columns. This table is used as an intermediate table linking the users and roles.

Remember, since a role can belong to many users, we cannot simply place a `user_id` column on the roles table. This would mean that a role could only belong to a single user. In order to provide support for roles being assigned to multiple users, the `role_user` table is needed. We can summarize the relationship's table structure like so:

```
users
    id - integer
    name - string

roles
    id - integer
    name - string

role_user
    user_id - integer
    role_id - integer
```

#### Model Structure

Many-to-many relationships are defined by creating a field of type [`BelongsToMany`]. For example, let's define a `roles` field on our `User` model. Like all relationships in Ensemble, the [`BelongsToMany`] type expects two generics: the type of the current model, and the type of the related model.

```rust
use ensemble::{Model, relationships::BelongsToMany};

#[derive(Debug, Model)]
struct User {
    pub id: u64,
    pub name: String,

    roles: BelongsToMany<User, Role>
}
```

Once the relationship is defined, you may access the user's roles using the `roles` dynamic function:

```rust
use crate::models::User;

let user = User::find(1).await.unwrap();

for role in user.roles().await.unwrap() {
    // ...
}
```

Since all relationships also serve as query builders, you may add further constraints to the relationship query by calling the `query` method on the roles property and continuing to chain conditions:

```rust
let user = User::find(1).await.unwrap();

let roles = user.roles.query()
    .order_by("name", "asc")
    .get().await.unwrap();
```

To determine the table name of the relationship's intermediate table, Ensemble will join the two related model names in alphabetical order. However, you are free to override this convention. You may do so using the `#[model(pivot_table)]` attribute:

```rust
use ensemble::{Model, relationships::BelongsToMany};

#[derive(Debug, Model)]
struct User {
    pub id: u64,
    pub name: String,

    #[model(pivot_table = "role_user")]
    roles: BelongsToMany<User, Role>
}
```

In addition to customizing the name of the intermediate table, you may also customize the column names of the keys on the table using the `#[model(local_key)]` attribute for the foreign key name of the model on which you are defining the relationship, and the `#[model(foreign_key)]` attribute for the foreign key name of the model that you are joining to:

```rust
use ensemble::{Model, relationships::BelongsToMany};

#[derive(Debug, Model)]
struct User {
    pub id: u64,
    pub name: String,

    #[model(local_key = "user_id", foreign_key = "role_id")]
    roles: BelongsToMany<User, Role>
}
```

#### Defining The Inverse Of The Relationship

To define the "inverse" of a many-to-many relationship, you should define a field on the related model of type [`BelongsToMany`] as well. To complete our user / role example, let's define the `users` field on the `Role` model:

```rust
use ensemble::{Model, relationships::BelongsToMany};

#[derive(Debug, Model)]
struct Role {
    pub id: u64,
    pub name: String,

    users: BelongsToMany<Role, User>
}
```

As you can see, the relationship is defined exactly the same as its `User` model counterpart with the exception of referencing the `User` model. Since we're reusing the [`BelongsToMany`] type, all of the usual table and key customization options are available when defining the "inverse" of many-to-many relationships.

## Querying Relations

Since all Ensemble relationships are defined via fields, you may access those fields to obtain an instance of the relationship without actually executing a query to load the related models. In addition, all types of Ensemble relationships also serve as query builders, allowing you to continue to chain constraints onto the relationship query before finally executing the SQL query against your database.

For example, imagine a blog application in which a `User` model has many associated `Post` models:

```rust
use ensemble::{Model, relationships::HasMany};

#[derive(Debug, Model)]
struct User {
    pub id: u64,
    pub name: String,

    pub posts: HasMany<User, Post>
}
```

You may query the `posts` relationship and add additional constraints to the relationship like so:

```rust
use crate::models::User;

let user = User::find(1).await.unwrap();

let posts = user.posts.query()
    .r#where("active", '=', 1)
    .get().await.unwrap();
```

#### Chaining `or_where` Clauses After Relationships

As demonstrated in the example above, you are free to add additional constraints to relationships when querying them. However, use caution when chaining [`or_where`](Builder::or_where) clauses onto a relationship, as the [`or_where`](Builder::or_where) clauses will be logically grouped at the same level as the relationship constraint:

```rust
user.posts.query()
    .r#where("active", '=', 1)
    .or_where("votes", ">=", 100)
    .get().await.unwrap();
```

The example above will generate the following SQL. As you can see, the `or` clause instructs the query to return any post with greater than 100 votes. The query is no longer constrained to a specific user:

```sql
select *
from posts
where user_id = ? and active = 1 or votes >= 100
```

In most situations, you should use logical groups to group the conditional checks between parentheses:

```rust
user.posts.query()
    .r#where("active", '=', 1)
    .where_group(|query| {
        query.r#where("active", '=', 1)
            .or_where("votes", ">=", 100);
    })
    .get().await.unwrap();
```

The example above will produce the following SQL. Note that the logical grouping has properly grouped the constraints and the query remains constrained to a specific user:

```sql
select *
from posts
where user_id = ? and (active = 1 or votes >= 100)
```

### Relationship Fields Vs. Dynamic Functions

If you do not need to add additional constraints to an Ensemble relationship query, you may access the relationship as if it were a method. For example, continuing to use our `User` and `Post` example models, we may access all of a user's posts like so:

```rust
use crate::models::User;

let user = User::find(1).await.unwrap();

for post in user.posts().await.unwrap() {
    // ...
}
```

Dynamic relationship functions perform "lazy loading", meaning they will only load their relationship data when you actually access them. Because of this, developers often use [eager loading](#eager-loading) to pre-load relationships they know will be accessed after loading the model. Eager loading provides a significant reduction in SQL queries that must be executed to load a model's relations.

### Counting Related Models

Sometimes you may want to count the number of related models for a given relationship without actually loading the models. To accomplish this, you may use the [`count`](Builder::count) method on the relationship's query builder, like so:

```rust
use crate::models::User;

let user = User::find(1).await.unwrap();

let posts_count = user.posts.query().count().await.unwrap();
```

## Eager Loading

When accessing Ensemble relationships as properties, the related models are "lazy loaded". This means the relationship data is not actually loaded until you first call the function. However, Ensemble can "eager load" relationships at the time you query the parent model. Eager loading alleviates the "N + 1" query problem. To illustrate the N + 1 query problem, consider a `Book` model that "belongs to" to an `Author` model:

```rust
use use ensemble::{Model, relationships::BelongsTo};

#[derive(Debug, Model)]
struct Book {
    pub id: u64,
    pub title: String,

    pub author: BelongsTo<Book, Author>
}
```

Now, let's retrieve all books and their authors:

```rust
use crates::models::Book;

for book in Book::all().await.unwrap() {
    let author = book.author().await.unwrap();

    println!(author.name);
}
```

This loop will execute one query to retrieve all of the books within the database table, then another query for each book in order to retrieve the book's author. So, if we have 25 books, the code above would run 26 queries: one for the original book, and 25 additional queries to retrieve the author of each book.

Thankfully, we can use eager loading to reduce this operation to just two queries. When building a query, you may specify which relationships should be eager loaded using the [`with`](Model::with) method:

```rust
use crates::models::Book;

for book in Book::with("author").get().await.unwrap() {
    let author = book.author().await.unwrap();

    println!(author.name);
}
```

For this operation, only two queries will be executed - one query to retrieve all of the books and one query to retrieve all of the authors for all of the books:

```sql
select * from books

select * from authors where id in (1, 2, 3, 4, 5, ...)
```

#### Eager Loading Multiple Relationships

Sometimes you may need to eager load several different relationships. To do so, just pass an array of relationships to the with method:

```rust
let books = Book::with(&["author", "publisher"]).get().await.unwrap();
```

### Lazy Eager Loading

Sometimes you may need to eager load a relationship after the parent model has already been retrieved. For example, this may be useful if you need to dynamically decide whether to load related models:

```rust
use crate::models::Book;

let mut books = Book::all().await.unwrap();

if someCondition {
    books.load(&["author", "publisher"]);
}
```
