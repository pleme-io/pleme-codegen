//! Repository pattern macro implementation
//!
//! Generates standard repository patterns with:
//! - PostgreSQL implementations
//! - Redis caching integration
//! - CRUD operations
//! - Query builders
//! - Multi-tenant support

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

use crate::utils::*;

/// Implementation of the Repository derive macro
pub fn derive_repository(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    let struct_name_str = struct_name.to_string();
    
    // Extract attributes
    let cache_ttl = get_attribute_int(&input.attrs, "repository", "cache_ttl").unwrap_or(300);
    let soft_delete = has_attribute_flag(&input.attrs, "repository", "soft_delete");
    
    // Generate repository trait
    let repository_trait = generate_repository_trait(struct_name);
    
    // Generate repository implementation
    let repository_impl = generate_repository_implementation(struct_name, cache_ttl, soft_delete);
    
    // Generate cache service integration
    let cache_integration = generate_cache_integration(struct_name, cache_ttl);
    
    let expanded = quote! {
        #repository_trait
        #repository_impl
        #cache_integration
    };
    
    TokenStream::from(expanded)
}

/// Generate repository trait definition
fn generate_repository_trait(struct_name: &syn::Ident) -> TokenStream2 {
    let trait_name = syn::Ident::new(&format!("{}RepositoryTrait", struct_name), proc_macro2::Span::call_site());
    let result_type = syn::Ident::new(&format!("{}Result", struct_name), proc_macro2::Span::call_site());
    
    quote! {
        /// Repository trait for #struct_name
        #[async_trait::async_trait]
        pub trait #trait_name: Send + Sync {
            /// Create a new entity
            async fn create(&self, entity: &#struct_name) -> #result_type<#struct_name>;
            
            /// Find entity by ID and product
            async fn find_by_id(&self, id: uuid::Uuid, product: &str) -> #result_type<Option<#struct_name>>;
            
            /// Update an existing entity
            async fn update(&self, entity: &#struct_name) -> #result_type<#struct_name>;
            
            /// Delete entity by ID and product
            async fn delete(&self, id: uuid::Uuid, product: &str) -> #result_type<bool>;
            
            /// List entities for a product with pagination
            async fn list_by_product(&self, product: &str, limit: i64, offset: i64) -> #result_type<Vec<#struct_name>>;
            
            /// Count entities for a product
            async fn count_by_product(&self, product: &str) -> #result_type<i64>;
            
            /// Find entities by field value
            async fn find_by_field(&self, field: &str, value: &str, product: &str) -> #result_type<Vec<#struct_name>>;
            
            /// Check if entity exists
            async fn exists(&self, id: uuid::Uuid, product: &str) -> #result_type<bool>;
            
            /// Bulk create entities
            async fn bulk_create(&self, entities: &[#struct_name]) -> #result_type<Vec<#struct_name>>;
            
            /// Clear cache for product
            async fn clear_cache(&self, product: &str) -> #result_type<()>;
        }
    }
}

/// Generate repository implementation
fn generate_repository_implementation(
    struct_name: &syn::Ident, 
    cache_ttl: u64, 
    soft_delete: bool
) -> TokenStream2 {
    let repository_name = syn::Ident::new(&format!("{}Repository", struct_name), proc_macro2::Span::call_site());
    let trait_name = syn::Ident::new(&format!("{}RepositoryTrait", struct_name), proc_macro2::Span::call_site());
    let result_type = syn::Ident::new(&format!("{}Result", struct_name), proc_macro2::Span::call_site());
    let error_type = syn::Ident::new(&format!("{}Error", struct_name), proc_macro2::Span::call_site());
    
    let delete_impl = if soft_delete {
        quote! {
            let query = format!("UPDATE {} SET deleted_at = $1, updated_at = $2 WHERE id = $3 AND product = $4", 
                #struct_name::TABLE_NAME);
            let now = chrono::Utc::now();
            
            let result = sqlx::query(&query)
                .bind(now)
                .bind(now)
                .bind(id)
                .bind(product)
                .execute(&self.pool)
                .await
                .map_err(#error_type::Database)?;
                
            let deleted = result.rows_affected() > 0;
            if deleted {
                // Clear from cache
                let cache_key = #struct_name::cache_key_for(product, id);
                if let Err(e) = self.cache.delete(&cache_key).await {
                    tracing::warn!("Failed to delete from cache: {}", e);
                }
            }
            
            Ok(deleted)
        }
    } else {
        quote! {
            let query = format!("DELETE FROM {} WHERE id = $1 AND product = $2", 
                #struct_name::TABLE_NAME);
            
            let result = sqlx::query(&query)
                .bind(id)
                .bind(product)
                .execute(&self.pool)
                .await
                .map_err(#error_type::Database)?;
                
            let deleted = result.rows_affected() > 0;
            if deleted {
                // Clear from cache
                let cache_key = #struct_name::cache_key_for(product, id);
                if let Err(e) = self.cache.delete(&cache_key).await {
                    tracing::warn!("Failed to delete from cache: {}", e);
                }
            }
            
            Ok(deleted)
        }
    };
    
    quote! {
        /// Repository implementation for #struct_name
        pub struct #repository_name {
            pool: sqlx::PgPool,
            cache: std::sync::Arc<dyn CacheServiceTrait>,
        }
        
        impl #repository_name {
            /// Create a new repository instance
            pub fn new(
                pool: sqlx::PgPool,
                cache: std::sync::Arc<dyn CacheServiceTrait>,
            ) -> Self {
                Self { pool, cache }
            }
        }
        
        #[async_trait::async_trait]
        impl #trait_name for #repository_name {
            async fn create(&self, entity: &#struct_name) -> #result_type<#struct_name> {
                // Insert into database
                let query = format!("INSERT INTO {} (id, product, created_at, updated_at, {}) VALUES ($1, $2, $3, $4, {})",
                    #struct_name::TABLE_NAME, "/* field names */", "/* field placeholders */");
                
                let result = sqlx::query_as::<_, #struct_name>(&query)
                    .bind(&entity.id)
                    .bind(&entity.product)
                    .bind(&entity.created_at)
                    .bind(&entity.updated_at)
                    // Add other field bindings here
                    .fetch_one(&self.pool)
                    .await
                    .map_err(#error_type::Database)?;
                
                // Cache the result
                let cache_key = result.cache_key();
                if let Err(e) = self.cache.set(&cache_key, &result, #cache_ttl).await {
                    tracing::warn!("Failed to cache entity: {}", e);
                }
                
                Ok(result)
            }
            
            async fn find_by_id(&self, id: uuid::Uuid, product: &str) -> #result_type<Option<#struct_name>> {
                // Try cache first
                let cache_key = #struct_name::cache_key_for(product, id);
                if let Ok(Some(cached)) = self.cache.get::<#struct_name>(&cache_key).await {
                    return Ok(Some(cached));
                }
                
                // Query database
                let query = #struct_name::select_by_id_query();
                let result = sqlx::query_as::<_, #struct_name>(&query)
                    .bind(id)
                    .bind(product)
                    .fetch_optional(&self.pool)
                    .await
                    .map_err(#error_type::Database)?;
                
                // Cache if found
                if let Some(ref entity) = result {
                    if let Err(e) = self.cache.set(&cache_key, entity, #cache_ttl).await {
                        tracing::warn!("Failed to cache entity: {}", e);
                    }
                }
                
                Ok(result)
            }
            
            async fn update(&self, entity: &#struct_name) -> #result_type<#struct_name> {
                let query = format!("UPDATE {} SET updated_at = $1 WHERE id = $2 AND product = $3",
                    #struct_name::TABLE_NAME);
                
                let mut updated_entity = entity.clone();
                updated_entity.touch();
                
                let result = sqlx::query_as::<_, #struct_name>(&query)
                    .bind(&updated_entity.updated_at)
                    .bind(&updated_entity.id)
                    .bind(&updated_entity.product)
                    .fetch_one(&self.pool)
                    .await
                    .map_err(#error_type::Database)?;
                
                // Update cache
                let cache_key = result.cache_key();
                if let Err(e) = self.cache.set(&cache_key, &result, #cache_ttl).await {
                    tracing::warn!("Failed to update cache: {}", e);
                }
                
                Ok(result)
            }
            
            async fn delete(&self, id: uuid::Uuid, product: &str) -> #result_type<bool> {
                #delete_impl
            }
            
            async fn list_by_product(&self, product: &str, limit: i64, offset: i64) -> #result_type<Vec<#struct_name>> {
                let query = format!("SELECT * FROM {} WHERE product = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
                    #struct_name::TABLE_NAME);
                
                let results = sqlx::query_as::<_, #struct_name>(&query)
                    .bind(product)
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(&self.pool)
                    .await
                    .map_err(#error_type::Database)?;
                
                Ok(results)
            }
            
            async fn count_by_product(&self, product: &str) -> #result_type<i64> {
                let query = #struct_name::count_by_product_query();
                
                let result: (i64,) = sqlx::query_as(&query)
                    .bind(product)
                    .fetch_one(&self.pool)
                    .await
                    .map_err(#error_type::Database)?;
                
                Ok(result.0)
            }
            
            async fn find_by_field(&self, field: &str, value: &str, product: &str) -> #result_type<Vec<#struct_name>> {
                let query = format!("SELECT * FROM {} WHERE {} = $1 AND product = $2",
                    #struct_name::TABLE_NAME, field);
                
                let results = sqlx::query_as::<_, #struct_name>(&query)
                    .bind(value)
                    .bind(product)
                    .fetch_all(&self.pool)
                    .await
                    .map_err(#error_type::Database)?;
                
                Ok(results)
            }
            
            async fn exists(&self, id: uuid::Uuid, product: &str) -> #result_type<bool> {
                let query = format!("SELECT EXISTS(SELECT 1 FROM {} WHERE id = $1 AND product = $2)",
                    #struct_name::TABLE_NAME);
                
                let result: (bool,) = sqlx::query_as(&query)
                    .bind(id)
                    .bind(product)
                    .fetch_one(&self.pool)
                    .await
                    .map_err(#error_type::Database)?;
                
                Ok(result.0)
            }
            
            async fn bulk_create(&self, entities: &[#struct_name]) -> #result_type<Vec<#struct_name>> {
                if entities.is_empty() {
                    return Ok(vec![]);
                }
                
                // Use PostgreSQL batch insert
                // This is a simplified version - real implementation would be more complex
                let mut results = Vec::new();
                for entity in entities {
                    let result = self.create(entity).await?;
                    results.push(result);
                }
                
                Ok(results)
            }
            
            async fn clear_cache(&self, product: &str) -> #result_type<()> {
                let pattern = #struct_name::cache_pattern(product);
                if let Err(e) = self.cache.delete_pattern(&pattern).await {
                    tracing::warn!("Failed to clear cache pattern {}: {}", pattern, e);
                }
                Ok(())
            }
        }
    }
}

/// Generate cache service integration
fn generate_cache_integration(struct_name: &syn::Ident, cache_ttl: u64) -> TokenStream2 {
    quote! {
        /// Cache service trait (simplified version)
        #[async_trait::async_trait]
        pub trait CacheServiceTrait: Send + Sync {
            async fn get<T>(&self, key: &str) -> Result<Option<T>, Box<dyn std::error::Error + Send + Sync>>
            where
                T: serde::de::DeserializeOwned;
                
            async fn set<T>(&self, key: &str, value: &T, ttl: u64) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
            where
                T: serde::Serialize;
                
            async fn delete(&self, key: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
            
            async fn delete_pattern(&self, pattern: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
        }
    }
}