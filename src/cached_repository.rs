//! CachedRepository derive macro implementation
//!
//! Generates Redis caching patterns for repository structs, eliminating ~180 lines
//! of boilerplate code per repository.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, format_ident};
use syn::{parse_macro_input, DeriveInput, Data, Fields, Field, Attribute, Meta, NestedMeta, Lit};

/// Configuration extracted from attributes
#[derive(Default)]
struct CacheConfig {
    entity: Option<String>,
    key_pattern: Option<String>,
    ttl: Option<u32>,
    pool_field: Option<String>,
}

impl CacheConfig {
    fn from_attrs(attrs: &[Attribute]) -> Self {
        let mut config = CacheConfig::default();
        
        for attr in attrs {
            if attr.path.is_ident("cached") {
                if let Ok(Meta::List(meta_list)) = attr.parse_meta() {
                    for nested_meta in meta_list.nested {
                        if let NestedMeta::Meta(Meta::NameValue(name_value)) = nested_meta {
                            if let Lit::Str(lit_str) = name_value.lit {
                                match name_value.path.get_ident().map(|i| i.to_string()).as_deref() {
                                    Some("entity") => config.entity = Some(lit_str.value()),
                                    Some("key_pattern") => config.key_pattern = Some(lit_str.value()),
                                    Some("pool_field") => config.pool_field = Some(lit_str.value()),
                                    _ => {}
                                }
                            } else if let Lit::Int(lit_int) = name_value.lit {
                                if name_value.path.is_ident("ttl") {
                                    config.ttl = lit_int.base10_parse().ok();
                                }
                            }
                        }
                    }
                }
            }
        }
        
        config
    }
}

pub fn derive_cached_repository(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    let config = CacheConfig::from_attrs(&input.attrs);
    
    // Set defaults
    let entity = config.entity.unwrap_or_else(|| {
        struct_name.to_string()
            .replace("Postgres", "")
            .replace("Repository", "")
            .to_lowercase()
    });
    
    let key_pattern = config.key_pattern.unwrap_or_else(|| 
        format!("{}:{{product}}:{{id}}", entity)
    );
    
    let ttl = config.ttl.unwrap_or(300);
    let pool_field = config.pool_field.unwrap_or_else(|| "redis".to_string());
    
    // Generate method names
    let cache_method = format_ident!("cache_{}", entity);
    let get_cached_method = format_ident!("get_cached_{}", entity);
    let invalidate_cache_method = format_ident!("invalidate_{}_cache", entity);
    let invalidate_pattern_method = format_ident!("invalidate_cache_by_pattern");
    
    // Generate the entity type name (capitalize first letter)
    let entity_type = format_ident!("{}", 
        entity.chars()
            .enumerate()
            .map(|(i, c)| if i == 0 { c.to_uppercase().collect::<String>() } else { c.to_string() })
            .collect::<String>()
    );
    
    let expanded = quote! {
        impl #struct_name {
            /// Cache entity in Redis with configured TTL
            pub async fn #cache_method(&self, entity: &#entity_type) -> Result<(), crate::models::PaymentError> {
                if let Some(redis_pool) = &self.#pool_field {
                    let mut conn = redis_pool.get().await
                        .map_err(|e| crate::models::PaymentError::TransactionFailed(format!("Redis error: {}", e)))?;
                    
                    // Extract key components based on pattern
                    let key = if #key_pattern.contains("{product}") && #key_pattern.contains("{id}") {
                        format!(#key_pattern, 
                            product = &entity.product,
                            id = &entity.id
                        )
                    } else if #key_pattern.contains("{product}") {
                        format!(#key_pattern, product = &entity.product)
                    } else {
                        format!(#key_pattern, id = &entity.id)
                    };
                    
                    let json = serde_json::to_string(entity)
                        .map_err(|e| crate::models::PaymentError::TransactionFailed(
                            format!("Serialization error for {}: {}", stringify!(#entity_type), e)
                        ))?;
                    
                    let _: () = redis::AsyncCommands::set_ex(&mut conn, &key, json, #ttl).await
                        .map_err(|e| crate::models::PaymentError::TransactionFailed(
                            format!("Redis set error for key {}: {}", key, e)
                        ))?;
                    
                    tracing::debug!(
                        entity = %stringify!(#entity_type),
                        cache_key = %key,
                        ttl = %#ttl,
                        "Entity cached successfully"
                    );
                }
                Ok(())
            }
            
            /// Retrieve cached entity from Redis
            pub async fn #get_cached_method(&self, id: uuid::Uuid, product: &str) -> Result<Option<#entity_type>, crate::models::PaymentError> {
                if let Some(redis_pool) = &self.#pool_field {
                    let mut conn = redis_pool.get().await
                        .map_err(|e| crate::models::PaymentError::TransactionFailed(format!("Redis error: {}", e)))?;
                    
                    let key = if #key_pattern.contains("{product}") && #key_pattern.contains("{id}") {
                        format!(#key_pattern, product = product, id = id)
                    } else if #key_pattern.contains("{product}") {
                        format!(#key_pattern, product = product)
                    } else {
                        format!(#key_pattern, id = id)
                    };
                    
                    let json: Option<String> = redis::AsyncCommands::get(&mut conn, &key).await
                        .map_err(|e| crate::models::PaymentError::TransactionFailed(
                            format!("Redis get error for key {}: {}", key, e)
                        ))?;
                    
                    if let Some(json) = json {
                        let entity = serde_json::from_str(&json)
                            .map_err(|e| crate::models::PaymentError::TransactionFailed(
                                format!("Deserialization error for {}: {}", stringify!(#entity_type), e)
                            ))?;
                        
                        tracing::debug!(
                            entity = %stringify!(#entity_type),
                            cache_key = %key,
                            "Cache hit"
                        );
                        
                        return Ok(Some(entity));
                    } else {
                        tracing::debug!(
                            entity = %stringify!(#entity_type),
                            cache_key = %key,
                            "Cache miss"
                        );
                    }
                }
                Ok(None)
            }
            
            /// Invalidate specific entity cache
            pub async fn #invalidate_cache_method(&self, id: uuid::Uuid, product: &str) -> Result<(), crate::models::PaymentError> {
                if let Some(redis_pool) = &self.#pool_field {
                    let mut conn = redis_pool.get().await
                        .map_err(|e| crate::models::PaymentError::TransactionFailed(format!("Redis error: {}", e)))?;
                    
                    let key = if #key_pattern.contains("{product}") && #key_pattern.contains("{id}") {
                        format!(#key_pattern, product = product, id = id)
                    } else if #key_pattern.contains("{product}") {
                        format!(#key_pattern, product = product)
                    } else {
                        format!(#key_pattern, id = id)
                    };
                    
                    let _: () = redis::AsyncCommands::del(&mut conn, &key).await
                        .map_err(|e| crate::models::PaymentError::TransactionFailed(
                            format!("Redis del error for key {}: {}", key, e)
                        ))?;
                    
                    tracing::debug!(
                        entity = %stringify!(#entity_type),
                        cache_key = %key,
                        "Cache invalidated"
                    );
                }
                Ok(())
            }
            
            /// Invalidate cache entries matching a pattern
            pub async fn #invalidate_pattern_method(&self, pattern: &str) -> Result<u32, crate::models::PaymentError> {
                if let Some(redis_pool) = &self.#pool_field {
                    let mut conn = redis_pool.get().await
                        .map_err(|e| crate::models::PaymentError::TransactionFailed(format!("Redis error: {}", e)))?;
                    
                    // Get all keys matching the pattern
                    let keys: Vec<String> = redis::AsyncCommands::keys(&mut conn, pattern).await
                        .map_err(|e| crate::models::PaymentError::TransactionFailed(
                            format!("Redis keys error for pattern {}: {}", pattern, e)
                        ))?;
                    
                    let count = keys.len() as u32;
                    
                    if !keys.is_empty() {
                        let _: () = redis::AsyncCommands::del(&mut conn, keys).await
                            .map_err(|e| crate::models::PaymentError::TransactionFailed(
                                format!("Redis batch del error: {}", e)
                            ))?;
                    }
                    
                    tracing::debug!(
                        entity = %stringify!(#entity_type),
                        pattern = %pattern,
                        invalidated_count = %count,
                        "Pattern-based cache invalidation completed"
                    );
                    
                    return Ok(count);
                }
                Ok(0)
            }
            
            /// Get cache statistics for this repository
            pub async fn get_cache_stats(&self) -> Result<std::collections::HashMap<String, u64>, crate::models::PaymentError> {
                let mut stats = std::collections::HashMap::new();
                
                if let Some(redis_pool) = &self.#pool_field {
                    let mut conn = redis_pool.get().await
                        .map_err(|e| crate::models::PaymentError::TransactionFailed(format!("Redis error: {}", e)))?;
                    
                    let pattern = format!("{}:*", #entity);
                    let keys: Vec<String> = redis::AsyncCommands::keys(&mut conn, &pattern).await
                        .map_err(|e| crate::models::PaymentError::TransactionFailed(
                            format!("Redis keys error: {}", e)
                        ))?;
                    
                    stats.insert("total_cached_entries".to_string(), keys.len() as u64);
                    stats.insert("cache_ttl_seconds".to_string(), #ttl as u64);
                    
                    tracing::debug!(
                        entity = %stringify!(#entity_type),
                        total_entries = %keys.len(),
                        "Cache statistics retrieved"
                    );
                }
                
                Ok(stats)
            }
            
            /// Warm up cache for frequently accessed entities
            pub async fn warm_cache<F, Fut>(&self, ids: Vec<uuid::Uuid>, product: &str, fetcher: F) -> Result<u32, crate::models::PaymentError>
            where
                F: Fn(uuid::Uuid) -> Fut,
                Fut: std::future::Future<Output = Result<Option<#entity_type>, crate::models::PaymentError>>,
            {
                let mut warmed = 0u32;
                
                for id in ids {
                    // Check if already cached
                    if self.#get_cached_method(id, product).await?.is_none() {
                        // Not in cache, fetch and cache it
                        if let Some(entity) = fetcher(id).await? {
                            self.#cache_method(&entity).await?;
                            warmed += 1;
                        }
                    }
                }
                
                tracing::info!(
                    entity = %stringify!(#entity_type),
                    warmed_count = %warmed,
                    "Cache warming completed"
                );
                
                Ok(warmed)
            }
        }
    };
    
    eprintln!("[pleme-codegen] CachedRepository pattern applied to {}", struct_name);
    TokenStream::from(expanded)
}