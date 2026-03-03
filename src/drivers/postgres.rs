use super::{Driver, QueryInfo};
use crate::common::block_on;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use sqlx::{Column, Connection, Executor, TypeInfo, Either};
use sqlx::postgres::{PgTypeInfo, PgTypeKind, Postgres};
use sqlx_core::describe::Describe;

pub struct PostgresDriver;

impl Driver for PostgresDriver {
    fn name(&self) -> &'static str {
        "PostgreSQL"
    }

    fn database_type(&self) -> TokenStream {
        quote!(::sqlx::Postgres)
    }

    fn url_schemes(&self) -> &'static [&'static str] {
        &["postgres", "postgresql"]
    }

    fn describe_query(
        &self,
        database_url: &str,
        sql: &str,
    ) -> Result<QueryInfo, String> {
        block_on(async {
            let mut conn = sqlx::postgres::PgConnection::connect(database_url)
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
        let describe: Describe<Postgres> = serde_json::from_value(describe_json)
            .map_err(|e| format!("Failed to deserialize Postgres describe: {}", e))?;
        Ok(QueryInfo {
            fields: gen_fields(&describe),
            params: gen_params(&describe),
        })
    }
}

fn gen_params(describe: &Describe<Postgres>) -> Vec<TokenStream> {
    let mut params = Vec::new();
    if let Some(Either::Left(param_types)) = &describe.parameters() {
        for (i, type_info) in param_types.iter().enumerate() {
            let field_name = format_ident!("p{}", i + 1);
            let type_tokens = map_pg_type(type_info);
            params.push(quote! {
                pub #field_name: #type_tokens
            });
        }
    }
    params
}

fn gen_fields(describe: &Describe<Postgres>) -> Vec<TokenStream> {
    let mut fields = Vec::new();
    for (i, column) in describe.columns().iter().enumerate() {
        let name = column.name();
        let field_name = format_ident!("{}", name);
        let is_nullable = describe.nullable(i).unwrap_or(true);
        let type_info = column.type_info();
        
        let type_tokens = map_pg_type(type_info);

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

fn map_pg_type(ty: &PgTypeInfo) -> TokenStream {
    match ty.kind() {
        PgTypeKind::Array(element) => {
            let elem_tokens = map_pg_type(element);
            quote!(Vec<#elem_tokens>)
        }
        _ => {
            let name = ty.name().to_uppercase();
            match name.as_str() {
                "BOOL" | "BOOLEAN" => quote!(bool),
                "BYTEA" => quote!(Vec<u8>),
                "CHAR" | "\"CHAR\"" => quote!(i8),
                "INT2" | "SMALLINT" | "SMALLSERIAL" => quote!(i16),
                "INT4" | "INTEGER" | "SERIAL" => quote!(i32),
                "INT8" | "BIGINT" | "BIGSERIAL" => quote!(i64),
                "FLOAT4" | "REAL" => quote!(f32),
                "FLOAT8" | "DOUBLE PRECISION" => quote!(f64),
                "TEXT" | "VARCHAR" | "BPCHAR" | "NAME" | "UNKNOWN" => quote!(String),
                "OID" => quote!(u32),
                "JSON" | "JSONB" => quote!(serde_json::Value),
                "UUID" => quote!(sqlx::types::uuid::Uuid),
                "DATE" => quote!(sqlx::types::chrono::NaiveDate),
                "TIME" | "TIME WITHOUT TIME ZONE" => quote!(sqlx::types::chrono::NaiveTime),
                "TIMESTAMP" | "TIMESTAMP WITHOUT TIME ZONE" => quote!(sqlx::types::chrono::NaiveDateTime),
                "TIMESTAMPTZ" | "TIMESTAMP WITH TIME ZONE" => quote!(sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>),
                "NUMERIC" | "DECIMAL" => quote!(sqlx::types::BigDecimal),
                "MONEY" => quote!(f64), // Or special type
                "VOID" => quote!(()),
                _ => {
                    // Handle schema-qualified names or unknown types
                    quote!(String)
                }
            }
        }
    }
}
