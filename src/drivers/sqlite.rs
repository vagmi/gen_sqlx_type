use super::{Driver, QueryInfo};
use crate::common::block_on;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use sqlx::{Connection, Executor, Column, Either};
use sqlx::sqlite::Sqlite;
use sqlx_core::describe::Describe;

pub struct SqliteDriver;

impl Driver for SqliteDriver {
    fn name(&self) -> &'static str {
        "SQLite"
    }

    fn database_type(&self) -> TokenStream {
        quote!(::sqlx::Sqlite)
    }

    fn url_schemes(&self) -> &'static [&'static str] {
        &["sqlite"]
    }

    fn describe_query(
        &self,
        database_url: &str,
        sql: &str,
    ) -> Result<QueryInfo, String> {
        block_on(async {
            let mut conn = sqlx::sqlite::SqliteConnection::connect(database_url)
                .await
                .map_err(|e| e.to_string())?;
            let describe = conn.describe(sql)
                .await
                .map_err(|e| e.to_string())?;
            
            Ok(QueryInfo {
                fields: gen_fields(&describe),
                params: gen_params(&describe),
            })
        })
    }

    fn describe_query_offline(
        &self,
        describe_json: serde_json::Value,
    ) -> Result<QueryInfo, String> {
        let describe: Describe<Sqlite> = serde_json::from_value(describe_json)
            .map_err(|e| format!("Failed to deserialize SQLite describe: {}", e))?;
        Ok(QueryInfo {
            fields: gen_fields(&describe),
            params: gen_params(&describe),
        })
    }
}

fn gen_params(describe: &Describe<Sqlite>) -> Vec<TokenStream> {
    let mut params = Vec::new();
    match describe.parameters() {
        Some(Either::Left(param_types)) => {
            for (i, type_info) in param_types.iter().enumerate() {
                let field_name = format_ident!("p{}", i + 1);
                let type_tokens = map_sqlite_type(type_info.to_string().to_ascii_lowercase().as_str());
                params.push(quote! {
                    pub #field_name: #type_tokens
                });
            }
        }
        Some(Either::Right(count)) => {
            for i in 0..count {
                let field_name = format_ident!("p{}", i + 1);
                params.push(quote! {
                    pub #field_name: String
                });
            }
        }
        None => {}
    }
    params
}

fn map_sqlite_type(s: &str) -> TokenStream {
    match s {
        "int4" => quote!(i32),
        "int8" | "bigint" => quote!(i64),
        "boolean" | "bool" => quote!(bool),
        "date" => quote!(sqlx::types::chrono::NaiveDate),
        "time" => quote!(sqlx::types::chrono::NaiveTime),
        "datetime" | "timestamp" => quote!(sqlx::types::chrono::NaiveDateTime),
        _ if s.contains("int") => quote!(i64),
        _ if s.contains("char") || s.contains("clob") || s.contains("text") => quote!(String),
        _ if s.contains("blob") => quote!(Vec<u8>),
        _ if s.contains("real") || s.contains("floa") || s.contains("doub") => quote!(f64),
        _ => quote!(String),
    }
}

fn gen_fields(describe: &Describe<Sqlite>) -> Vec<TokenStream> {
    let mut fields = Vec::new();
    for (i, column) in describe.columns().iter().enumerate() {
        let name = column.name();
        let field_name = format_ident!("{}", name);
        let is_nullable = describe.nullable(i).unwrap_or(true);
        let type_info = column.type_info();
        let s = type_info.to_string().to_ascii_lowercase();
        
        let type_tokens = map_sqlite_type(s.as_str());

        let field_type = if is_nullable {
            quote!(::std::option::Option<#type_tokens>)
        } else {
            type_tokens
        };

        fields.push(quote! {
            pub #field_name: #field_type
        });
    }
    fields
}
