use proc_macro2::TokenStream;

pub trait Driver {
    fn name(&self) -> &'static str;
    fn url_schemes(&self) -> &'static [&'static str];
    fn describe_query(
        &self,
        database_url: &str,
        sql: &str,
    ) -> Result<Vec<TokenStream>, String>;
    fn describe_query_offline(
        &self,
        describe_json: serde_json::Value,
    ) -> Result<Vec<TokenStream>, String>;
}

#[cfg(feature = "sqlite")]
pub mod sqlite;

#[cfg(feature = "postgres")]
pub mod postgres;

pub const DRIVERS: &[&dyn Driver] = &[
    #[cfg(feature = "sqlite")]
    &sqlite::SqliteDriver,
    #[cfg(feature = "postgres")]
    &postgres::PostgresDriver,
];
