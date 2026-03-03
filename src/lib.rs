use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Ident, LitStr, Token, LitBool};
use syn::parse::{Parse, ParseStream};
use std::env;
use std::path::PathBuf;
use url::Url;
use std::fs;

mod common;
mod drivers;
use drivers::{DRIVERS, QueryInfo};
use common::{hash_string, resolve_path};

struct MacroInput {
    struct_name: Ident,
    sql: String,
    serde: bool,
    clone: bool,
}

impl Parse for MacroInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let struct_name: Ident = input.parse()?;
        input.parse::<Token![,]>()?;

        let mut sql = None;
        let mut serde = true;
        let mut clone = true;

        while !input.is_empty() {
            let lookahead = input.lookahead1();
            if lookahead.peek(Ident) {
                let key: Ident = input.parse()?;
                input.parse::<Token![=]>()?;
                
                if key == "source" {
                    let value: LitStr = input.parse()?;
                    sql = Some(value.value());
                } else if key == "file" {
                    let value: LitStr = input.parse()?;
                    let path = resolve_path(value.value(), value.span())?;
                    let content = fs::read_to_string(&path).map_err(|e| {
                        syn::Error::new(value.span(), format!("Failed to read query file: {}", e))
                    })?;
                    sql = Some(content);
                } else if key == "serde" {
                    let value: LitBool = input.parse()?;
                    serde = value.value;
                } else if key == "clone" {
                    let value: LitBool = input.parse()?;
                    clone = value.value;
                } else {
                    return Err(syn::Error::new(key.span(), format!("Unexpected key: {}", key)));
                }
            } else if lookahead.peek(LitStr) {
                // Fallback for the old format: gen_sqlx_type!(TypeName, "query")
                let value: LitStr = input.parse()?;
                sql = Some(value.value());
            } else {
                return Err(lookahead.error());
            }

            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        let sql = sql.ok_or_else(|| input.error("Expected 'source' or 'file' parameter"))?;

        Ok(MacroInput {
            struct_name,
            sql,
            serde,
            clone,
        })
    }
}

/// Generates a Rust struct based on the result of a SQL query.
#[proc_macro]
pub fn gen_sqlx_type(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as MacroInput);
    let struct_name = input.struct_name;
    let params_struct_name = quote::format_ident!("{}Params", struct_name);
    let sql = input.sql;
    let derive_serde = input.serde;
    let derive_clone = input.clone;

    let offline = env::var("SQLX_OFFLINE")
        .map(|s| s.eq_ignore_ascii_case("true") || s == "1")
        .unwrap_or(false);
    
    let database_url = env::var("DATABASE_URL").ok();
    let res = if offline || database_url.is_none() {
        get_fields_offline(&sql)
    } else {
        get_fields_online(&sql)
    };

    let (query_info, db_type) = match res {
        Ok(r) => r,
        Err(e) => {
            return TokenStream::from(quote! {
                compile_error!(#e);
            });
        }
    };

    let fields = query_info.fields;
    let params = query_info.params;
    let param_names = (1..=params.len()).map(|i| quote::format_ident!("p{}", i)).collect::<Vec<_>>();

    let mut derives: Vec<proc_macro2::TokenStream> = vec![quote!(Debug), quote!(sqlx::FromRow)];
    if derive_serde {
        derives.push(quote!(serde::Serialize));
        derives.push(quote!(serde::Deserialize));
    }
    if derive_clone {
        derives.push(quote!(Clone));
    }

    let expanded = quote! {
        #[derive(#(#derives),*)]
        pub struct #struct_name {
            #(#fields),*
        }

        #[derive(#(#derives),*)]
        pub struct #params_struct_name {
            #(#params),*
        }

        impl #struct_name {
            pub async fn fetch_all<'a, E>(ex: E, params: #params_struct_name) -> Result<Vec<Self>, ::sqlx::Error>
            where E: 'a + ::sqlx::Executor<'a, Database = #db_type>
            {
                ::sqlx::query_as!(Self, #sql, #(params.#param_names),*).fetch_all(ex).await
            }

            pub async fn fetch_one<'a, E>(ex: E, params: #params_struct_name) -> Result<Self, ::sqlx::Error>
            where E: 'a + ::sqlx::Executor<'a, Database = #db_type>
            {
                ::sqlx::query_as!(Self, #sql, #(params.#param_names),*).fetch_one(ex).await
            }

            pub async fn fetch_optional<'a, E>(ex: E, params: #params_struct_name) -> Result<Option<Self>, ::sqlx::Error>
            where E: 'a + ::sqlx::Executor<'a, Database = #db_type>
            {
                ::sqlx::query_as!(Self, #sql, #(params.#param_names),*).fetch_optional(ex).await
            }
        }
    };

    TokenStream::from(expanded)
}

fn get_fields_online(sql: &str) -> Result<(QueryInfo, proc_macro2::TokenStream), String> {
    let database_url = env::var("DATABASE_URL").map_err(|_| "DATABASE_URL must be set")?;
    let database_url_parsed = Url::parse(&database_url).map_err(|e| format!("Failed to parse DATABASE_URL: {}", e))?;
    let scheme = database_url_parsed.scheme();

    let driver = DRIVERS.iter().find(|d| d.url_schemes().contains(&scheme))
        .ok_or_else(|| format!("No driver found for scheme: {}", scheme))?;

    let info = driver.describe_query(&database_url, sql)
        .map_err(|e| format!("Failed to describe query using {} driver: {}", driver.name(), e))?;

    Ok((info, driver.database_type()))
}

fn get_fields_offline(sql: &str) -> Result<(QueryInfo, proc_macro2::TokenStream), String> {
    let hash = hash_string(sql);
    let filename = format!("query-{}.json", hash);

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").map_err(|_| "CARGO_MANIFEST_DIR must be set")?;
    let manifest_dir_path = PathBuf::from(manifest_dir);

    let offline_dir = env::var("SQLX_OFFLINE_DIR").unwrap_or(".sqlx".to_string());
    
    let mut search_dirs = Vec::new();
    search_dirs.push(manifest_dir_path.join(offline_dir));

    for dir in search_dirs {
        let path = dir.join(&filename);
        if path.exists() {
            let content = fs::read_to_string(&path).map_err(|e| format!("Failed to read cached query file: {}", e))?;
            let data: serde_json::Value = serde_json::from_str(&content).map_err(|e| format!("Failed to parse cached query JSON: {}", e))?;
            
            let db_name = data["db_name"].as_str().ok_or("Missing db_name in cached query data")?;
            let describe = data["describe"].clone();

            let driver = DRIVERS.iter().find(|d| d.name() == db_name)
                .ok_or_else(|| format!("No driver found for database: {}", db_name))?;

            let info = driver.describe_query_offline(describe)?;
            return Ok((info, driver.database_type()));
        }
    }

    Err(format!("No cached data found for query with hash {}. Run 'cargo sqlx prepare' or set DATABASE_URL.", hash))
}
