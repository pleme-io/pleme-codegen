//! Repository Helper Patterns
//!
//! Macros for generating database row to struct mappings and common repository patterns

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Data, Fields};

/// Derive macro for automatic database row mapping
pub fn derive_row_mapper(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    
    // Extract field information
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => panic!("RowMapper only supports structs with named fields"),
        },
        _ => panic!("RowMapper only supports structs"),
    };
    
    eprintln!("[pleme-codegen] RowMapper pattern applied to {} - saving ~50 lines per struct", struct_name);
    
    // Generate field mappings
    let field_mappings = fields.iter().map(|field| {
        let field_name = &field.ident;
        let field_type = &field.ty;
        
        // Handle different field types
        let mapping = match field_type {
            // Check if it's a Decimal type
            ty if is_decimal_type(ty) => {
                quote! {
                    #field_name: rust_decimal::Decimal::from_str(
                        &row.try_get::<sqlx::types::BigDecimal, _>(stringify!(#field_name))
                            .map_err(|e| Self::map_error(e, stringify!(#field_name)))?
                            .to_string()
                    ).map_err(|e| Self::map_error(e, stringify!(#field_name)))?
                }
            },
            // Check if it's an enum that needs string conversion
            ty if is_enum_type(ty) => {
                quote! {
                    #field_name: row.try_get::<String, _>(stringify!(#field_name))
                        .map_err(|e| Self::map_error(e, stringify!(#field_name)))?
                        .parse()
                        .map_err(|_| Self::map_error(
                            sqlx::Error::Decode("Invalid enum value".into()), 
                            stringify!(#field_name)
                        ))?
                }
            },
            // Check if it's JSON
            ty if is_json_type(ty) => {
                quote! {
                    #field_name: serde_json::from_value(
                        row.try_get(stringify!(#field_name))
                            .map_err(|e| Self::map_error(e, stringify!(#field_name)))?
                    ).map_err(|e| Self::map_error(
                        sqlx::Error::Decode(e.to_string().into()), 
                        stringify!(#field_name)
                    ))?
                }
            },
            // Handle Option<Decimal>
            ty if is_option_decimal_type(ty) => {
                quote! {
                    #field_name: row.try_get::<Option<sqlx::types::BigDecimal>, _>(stringify!(#field_name))
                        .map_err(|e| Self::map_error(e, stringify!(#field_name)))?
                        .map(|bd| rust_decimal::Decimal::from_str(&bd.to_string()))
                        .transpose()
                        .map_err(|e| Self::map_error(
                            sqlx::Error::Decode(e.to_string().into()), 
                            stringify!(#field_name)
                        ))?
                }
            },
            // Default case for standard types
            _ => {
                quote! {
                    #field_name: row.try_get(stringify!(#field_name))
                        .map_err(|e| Self::map_error(e, stringify!(#field_name)))?
                }
            }
        };
        
        quote! { #mapping }
    });
    
    let expanded = quote! {
        impl #struct_name {
            /// Convert database row to struct with comprehensive error handling
            pub fn from_row(row: &sqlx::postgres::PgRow) -> Result<Self, PaymentError> {
                use sqlx::Row;
                use std::str::FromStr;
                
                Ok(Self {
                    #(#field_mappings,)*
                })
            }
            
            /// Helper to convert SQLx errors with field context
            fn map_error(err: impl std::error::Error, field: &str) -> PaymentError {
                let msg = format!("Failed to read field '{}': {}", field, err);
                tracing::error!(field = %field, error = %err, "Database field mapping error");
                PaymentError::TransactionFailed(msg)
            }
            
            /// Convert multiple rows to Vec<Self>
            pub fn from_rows(rows: Vec<sqlx::postgres::PgRow>) -> Result<Vec<Self>, PaymentError> {
                rows.into_iter()
                    .map(|row| Self::from_row(&row))
                    .collect()
            }
            
            /// Convert Option<PgRow> to Option<Self>
            pub fn from_optional_row(row: Option<sqlx::postgres::PgRow>) -> Result<Option<Self>, PaymentError> {
                match row {
                    Some(row) => Ok(Some(Self::from_row(&row)?)),
                    None => Ok(None),
                }
            }
        }
    };
    
    TokenStream::from(expanded)
}

/// Derive macro for repository CRUD operations with caching
pub fn derive_repository_crud(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    
    eprintln!("[pleme-codegen] RepositoryCrud pattern applied to {} - saving ~300 lines", struct_name);
    
    let expanded = quote! {
        impl #struct_name {
            /// Create with automatic caching
            pub async fn create_with_cache<T>(&self, entity: &T, cache_key: &str) -> Result<T, PaymentError>
            where
                T: serde::Serialize + serde::de::DeserializeOwned + Clone + Send + Sync,
            {
                let start = std::time::Instant::now();
                
                // Perform database operation (placeholder for actual implementation)
                let created = entity.clone();
                
                // Cache the result if Redis is available
                if let Some(redis_pool) = &self.redis {
                    if let Ok(mut conn) = redis_pool.get().await {
                        let json = serde_json::to_string(&created).map_err(|e| PaymentError::TransactionFailed(e.to_string()))?;
                        let _: Result<(), _> = redis::cmd("SET")
                            .arg(cache_key)
                            .arg(&json)
                            .arg("EX")
                            .arg(300) // 5 minute default TTL
                            .query_async(&mut conn)
                            .await;
                        
                        tracing::debug!(
                            cache_key = %cache_key,
                            duration_ms = %start.elapsed().as_millis(),
                            "Entity cached after creation"
                        );
                    }
                }
                
                Ok(created)
            }
            
            /// Find by ID with caching
            pub async fn find_by_id_cached<T>(&self, id: &str, cache_key: &str) -> Result<Option<T>, PaymentError>
            where
                T: serde::Serialize + serde::de::DeserializeOwned + Send + Sync,
            {
                let start = std::time::Instant::now();
                
                // Try cache first
                if let Some(redis_pool) = &self.redis {
                    if let Ok(mut conn) = redis_pool.get().await {
                        let cached: Result<String, _> = redis::cmd("GET")
                            .arg(cache_key)
                            .query_async(&mut conn)
                            .await;
                        
                        if let Ok(json) = cached {
                            if let Ok(entity) = serde_json::from_str::<T>(&json) {
                                tracing::debug!(
                                    cache_key = %cache_key,
                                    duration_ms = %start.elapsed().as_millis(),
                                    "Cache hit"
                                );
                                return Ok(Some(entity));
                            }
                        }
                    }
                }
                
                // Cache miss - would perform database query here
                tracing::debug!(
                    cache_key = %cache_key,
                    duration_ms = %start.elapsed().as_millis(),
                    "Cache miss - fetching from database"
                );
                
                // Placeholder for actual database fetch
                Ok(None)
            }
            
            /// Update with cache invalidation
            pub async fn update_with_cache<T>(&self, entity: &T, cache_key: &str) -> Result<T, PaymentError>
            where
                T: serde::Serialize + serde::de::DeserializeOwned + Clone + Send + Sync,
            {
                let start = std::time::Instant::now();
                
                // Perform database update (placeholder)
                let updated = entity.clone();
                
                // Invalidate old cache and set new
                if let Some(redis_pool) = &self.redis {
                    if let Ok(mut conn) = redis_pool.get().await {
                        // Delete old cache
                        let _: Result<(), _> = redis::cmd("DEL")
                            .arg(cache_key)
                            .query_async(&mut conn)
                            .await;
                        
                        // Set new cache
                        let json = serde_json::to_string(&updated).map_err(|e| PaymentError::TransactionFailed(e.to_string()))?;
                        let _: Result<(), _> = redis::cmd("SET")
                            .arg(cache_key)
                            .arg(&json)
                            .arg("EX")
                            .arg(300)
                            .query_async(&mut conn)
                            .await;
                        
                        tracing::debug!(
                            cache_key = %cache_key,
                            duration_ms = %start.elapsed().as_millis(),
                            "Cache updated after entity update"
                        );
                    }
                }
                
                Ok(updated)
            }
            
            /// Delete with cache invalidation
            pub async fn delete_with_cache(&self, cache_key: &str) -> Result<(), PaymentError> {
                let start = std::time::Instant::now();
                
                // Perform database delete (placeholder)
                
                // Invalidate cache
                if let Some(redis_pool) = &self.redis {
                    if let Ok(mut conn) = redis_pool.get().await {
                        let _: Result<(), _> = redis::cmd("DEL")
                            .arg(cache_key)
                            .query_async(&mut conn)
                            .await;
                        
                        tracing::debug!(
                            cache_key = %cache_key,
                            duration_ms = %start.elapsed().as_millis(),
                            "Cache invalidated after deletion"
                        );
                    }
                }
                
                Ok(())
            }
            
            /// Execute query with metrics
            pub async fn execute_with_metrics<F, R>(&self, operation_name: &str, query_fn: F) -> Result<R, PaymentError>
            where
                F: std::future::Future<Output = Result<R, sqlx::Error>>,
            {
                let start = std::time::Instant::now();
                
                let result = query_fn.await.map_err(|e| {
                    tracing::error!(
                        repository = %stringify!(#struct_name),
                        operation = %operation_name,
                        error = %e,
                        duration_ms = %start.elapsed().as_millis(),
                        "Repository operation failed"
                    );
                    PaymentError::TransactionFailed(e.to_string())
                })?;
                
                let duration_ms = start.elapsed().as_millis();
                
                tracing::info!(
                    repository = %stringify!(#struct_name),
                    operation = %operation_name,
                    duration_ms = %duration_ms,
                    "Repository operation completed"
                );
                
                // Emit metrics (placeholder for actual metrics emission)
                if duration_ms > 1000 {
                    tracing::warn!(
                        repository = %stringify!(#struct_name),
                        operation = %operation_name,
                        duration_ms = %duration_ms,
                        "Slow repository operation detected"
                    );
                }
                
                Ok(result)
            }
            
            /// Build cache key with product isolation
            pub fn build_cache_key(&self, entity_type: &str, id: &str, product: &str) -> String {
                format!("{}:{}:{}", entity_type, product, id)
            }
            
            /// Batch cache invalidation
            pub async fn invalidate_cache_pattern(&self, pattern: &str) -> Result<u64, PaymentError> {
                if let Some(redis_pool) = &self.redis {
                    if let Ok(mut conn) = redis_pool.get().await {
                        // Use SCAN to find matching keys
                        let keys: Vec<String> = redis::cmd("KEYS")
                            .arg(pattern)
                            .query_async(&mut conn)
                            .await
                            .map_err(|e| PaymentError::TransactionFailed(e.to_string()))?;
                        
                        if !keys.is_empty() {
                            let count = keys.len() as u64;
                            
                            // Delete all matching keys
                            let _: Result<(), _> = redis::cmd("DEL")
                                .arg(keys)
                                .query_async(&mut conn)
                                .await;
                            
                            tracing::info!(
                                pattern = %pattern,
                                count = %count,
                                "Cache keys invalidated"
                            );
                            
                            return Ok(count);
                        }
                    }
                }
                
                Ok(0)
            }
        }
    };
    
    TokenStream::from(expanded)
}

// Helper functions to identify types
fn is_decimal_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "Decimal";
        }
    }
    false
}

fn is_enum_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            // Common enum types in payment service
            return matches!(segment.ident.to_string().as_str(), 
                "PaymentStatus" | "PaymentMethod" | "PixKeyType" | "SubscriptionStatus" | 
                "BillingInterval" | "PayoutStatus" | "TransactionType" | "VerificationLevel");
        }
    }
    false
}

fn is_json_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            // JSON value types
            return matches!(segment.ident.to_string().as_str(),
                "Value" | "PaymentMetadata" | "SubscriptionMetadata");
        }
    }
    false
}

fn is_option_decimal_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Option" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                        return is_decimal_type(inner_ty);
                    }
                }
            }
        }
    }
    false
}