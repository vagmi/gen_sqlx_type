# gen-sqlx-type

A proc-macro to generate named, reusable Rust structs from SQL queries at compile time using SQLx.

## Why?

SQLx's `query!` and `query_file!` macros are excellent for compile-time safety, but they generate anonymous types that are restricted to the scope where they are defined. This makes it impossible to return these types from functions or pass them between modules without manually defining equivalent structs.

`gen_sqlx_type!` solves this by generating a **named struct** based on your database schema at compile time. This allows you to:

* **Define once, reuse everywhere:** Generate a struct that can be passed across function boundaries.
* **Seamless Integration:** Use the generated struct directly with `sqlx::query_as!` or `sqlx::query_file_as!`.
* **Complex Queries:** Keep your complex SQL in separate `.sql` files and still have a strongly-typed, nameable result struct.

## Usage

```rust
use gen_sqlx_type::gen_sqlx_type;

// 1. Generate from an inline string
gen_sqlx_type!(User, source = "SELECT id, name, email FROM users");

// 2. Generate from a .sql file (relative to cargo root)
gen_sqlx_type!(ComplexReport, file = "queries/report.sql");

// 3. Customize derives (Debug, sqlx::FromRow, Serialize, Deserialize, Clone are default)
gen_sqlx_type!(InternalTask, source = "SELECT * FROM tasks", serde = false, clone = false);

// 4. Use with sqlx macros
async fn get_users(pool: &sqlx::PgPool) -> anyhow::Result<Vec<User>> {
    let users = sqlx::query_as!(User, "SELECT id, name, email FROM users")
        .fetch_all(pool)
        .await?;
    Ok(users)
}
```

## Features

- **Multi-Database Support:** Robust implementations for **PostgreSQL** and **SQLite**.
- **Rich Type Mapping:** 
    - Automatic mapping for `JSON`/`JSONB` to `serde_json::Value` (Postgres).
    - Support for `UUID`, `Chrono` (DateTime, Date, Time), and `BigDecimal`.
    - Support for PostgreSQL recursive arrays (`Vec<T>`).
- **Offline Mode:** Fully supports `SQLX_OFFLINE` and `SQLX_OFFLINE_DIR` using the same `.sqlx` metadata cache as SQLx.
- **Configurable Derives:** Default derivation of `Serialize`, `Deserialize`, and `Clone` can be toggled via macro flags.

## Setup

Add the following to your `Cargo.toml`:

```toml
[dependencies]
gen_sqlx_type = { git="https://github.com/vagmi/gen_sqlx_type.git", tag="v0.1.0", features = ["postgres"] } # or "sqlite"
sqlx = { version = "0.8", features = ["runtime-tokio", "postgres", "macros", "chrono", "json", "uuid", "bigdecimal"] }
serde = { version = "1.0", features = ["derive"] }
chrono = { version = "0.4", features = ["serde"] }
```

