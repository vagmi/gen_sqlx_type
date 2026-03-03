use proc_macro2::TokenStream;
use serde_json::Value;

pub struct QueryInfo {
    pub fields: Vec<TokenStream>,
    pub params: Vec<TokenStream>,
}

pub trait Driver {
    fn name(&self) -> &'static str;
    fn database_type(&self) -> TokenStream;
    fn url_schemes(&self) -> &'static [&'static str];
    fn describe_query(
        &self,
        database_url: &str,
        sql: &str,
    ) -> Result<QueryInfo, String>;
    fn describe_query_offline(
        &self,
        describe_json: Value,
    ) -> Result<QueryInfo, String>;
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
