//! TransactionalRepository derive macro implementation
//!
//! Generates transactional database operations with proper locking order to prevent deadlocks.
//! Handles complex multi-step operations common in financial systems.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, format_ident};
use syn::{parse_macro_input, DeriveInput, Data, Fields, Field, Attribute, Meta, NestedMeta, Lit};

/// Transaction configuration extracted from attributes
#[derive(Default)]
struct TransactionConfig {
    pool_field: String,
    error_type: String,
    lock_timeout: Option<u32>,
    isolation_level: Option<String>,
}

impl TransactionConfig {
    fn from_attrs(attrs: &[Attribute]) -> Self {
        let mut config = TransactionConfig {
            pool_field: "pool".to_string(),
            error_type: "PaymentError".to_string(),
            lock_timeout: Some(30),
            isolation_level: Some("ReadCommitted".to_string()),
        };
        
        for attr in attrs {
            if attr.path.is_ident("transactional") {
                if let Ok(Meta::List(meta_list)) = attr.parse_meta() {
                    for nested_meta in meta_list.nested {
                        if let NestedMeta::Meta(Meta::NameValue(name_value)) = nested_meta {
                            match name_value.path.get_ident().map(|i| i.to_string()).as_deref() {
                                Some("pool_field") => {
                                    if let Lit::Str(lit_str) = name_value.lit {
                                        config.pool_field = lit_str.value();
                                    }
                                }
                                Some("error_type") => {
                                    if let Lit::Str(lit_str) = name_value.lit {
                                        config.error_type = lit_str.value();
                                    }
                                }
                                Some("isolation_level") => {
                                    if let Lit::Str(lit_str) = name_value.lit {
                                        config.isolation_level = Some(lit_str.value());
                                    }
                                }
                                Some("lock_timeout") => {
                                    if let Lit::Int(lit_int) = name_value.lit {
                                        config.lock_timeout = lit_int.base10_parse().ok();
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
        
        config
    }
}

pub fn derive_transactional_repository(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    let config = TransactionConfig::from_attrs(&input.attrs);
    
    let pool_field = format_ident!("{}", config.pool_field);
    let error_type = format_ident!("{}", config.error_type);
    let lock_timeout = config.lock_timeout.unwrap_or(30);
    let isolation_level = config.isolation_level.unwrap_or_else(|| "ReadCommitted".to_string());
    
    let expanded = quote! {
        impl #struct_name {
            /// Execute operations within a database transaction with automatic rollback on error
            pub async fn with_transaction<F, R>(&self, operation: F) -> Result<R, crate::models::#error_type>
            where
                F: for<'t> FnOnce(&mut sqlx::Transaction<'t, sqlx::Postgres>) -> 
                   std::pin::Pin<Box<dyn std::future::Future<Output = Result<R, crate::models::#error_type>> + Send + 't>>,
                R: Send + 'static,
            {
                let mut tx = self.#pool_field.begin().await
                    .map_err(|e| crate::models::#error_type::TransactionFailed(
                        format!("Failed to begin transaction: {}", e)
                    ))?;
                
                // Set lock timeout to prevent hanging transactions
                sqlx::query(&format!("SET LOCAL lock_timeout = '{}s'", #lock_timeout))
                    .execute(&mut tx)
                    .await
                    .map_err(|e| crate::models::#error_type::TransactionFailed(
                        format!("Failed to set lock timeout: {}", e)
                    ))?;
                
                tracing::debug!(
                    repository = %stringify!(#struct_name),
                    lock_timeout = %#lock_timeout,
                    isolation_level = %#isolation_level,
                    "Transaction started"
                );
                
                let start = std::time::Instant::now();
                let result = operation(&mut tx).await;
                
                match result {
                    Ok(value) => {
                        tx.commit().await
                            .map_err(|e| crate::models::#error_type::TransactionFailed(
                                format!("Failed to commit transaction: {}", e)
                            ))?;
                        
                        let duration = start.elapsed();
                        tracing::info!(
                            repository = %stringify!(#struct_name),
                            duration_ms = %duration.as_millis(),
                            "Transaction committed successfully"
                        );
                        
                        Ok(value)
                    }
                    Err(e) => {
                        if let Err(rollback_err) = tx.rollback().await {
                            tracing::error!(
                                repository = %stringify!(#struct_name),
                                rollback_error = %rollback_err,
                                "Failed to rollback transaction"
                            );
                        }
                        
                        let duration = start.elapsed();
                        tracing::warn!(
                            repository = %stringify!(#struct_name),
                            duration_ms = %duration.as_millis(),
                            error = %e,
                            "Transaction rolled back due to error"
                        );
                        
                        Err(e)
                    }
                }
            }
            
            /// Execute operations with row-level locking in deterministic order to prevent deadlocks
            pub async fn with_ordered_locks<F, R>(
                &self, 
                mut entity_ids: Vec<uuid::Uuid>,
                operation: F
            ) -> Result<R, crate::models::#error_type>
            where
                F: for<'t> FnOnce(&mut sqlx::Transaction<'t, sqlx::Postgres>, Vec<uuid::Uuid>) -> 
                   std::pin::Pin<Box<dyn std::future::Future<Output = Result<R, crate::models::#error_type>> + Send + 't>>,
                R: Send + 'static,
            {
                // Sort IDs to ensure consistent locking order across all transactions
                entity_ids.sort();
                
                self.with_transaction(|tx| {
                    Box::pin(async move {
                        // Acquire locks in sorted order
                        for id in &entity_ids {
                            sqlx::query("SELECT pg_advisory_xact_lock($1)")
                                .bind(id.as_u128() as i64) // Convert UUID to i64 for advisory lock
                                .execute(&mut *tx)
                                .await
                                .map_err(|e| crate::models::#error_type::TransactionFailed(
                                    format!("Failed to acquire advisory lock for {}: {}", id, e)
                                ))?;
                        }
                        
                        tracing::debug!(
                            repository = %stringify!(#struct_name),
                            locked_entities = %entity_ids.len(),
                            "Advisory locks acquired in order"
                        );
                        
                        operation(tx, entity_ids).await
                    })
                }).await
            }
            
            /// Transfer operation with balance validation and atomic updates
            pub async fn atomic_transfer<T>(
                &self,
                from_id: uuid::Uuid,
                to_id: uuid::Uuid,
                amount: rust_decimal::Decimal,
                validator: impl Fn(&T, rust_decimal::Decimal) -> Result<(), crate::models::#error_type>,
                updater: impl for<'t> Fn(&mut sqlx::Transaction<'t, sqlx::Postgres>, uuid::Uuid, rust_decimal::Decimal, bool) -> 
                         std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, crate::models::#error_type>> + Send + 't>>,
            ) -> Result<(T, T), crate::models::#error_type>
            where
                T: Send + 'static + Clone,
            {
                if amount <= rust_decimal::Decimal::ZERO {
                    return Err(crate::models::#error_type::ValidationFailed(
                        "Transfer amount must be positive".to_string()
                    ));
                }
                
                if from_id == to_id {
                    return Err(crate::models::#error_type::ValidationFailed(
                        "Cannot transfer to the same account".to_string()
                    ));
                }
                
                let entity_ids = vec![from_id, to_id];
                
                self.with_ordered_locks(entity_ids, |tx, sorted_ids| {
                    Box::pin(async move {
                        let from_id = sorted_ids[0];
                        let to_id = sorted_ids[1];
                        
                        // Get current balances with SELECT FOR UPDATE
                        let from_entity = updater(tx, from_id, rust_decimal::Decimal::ZERO, false).await?;
                        validator(&from_entity, amount)?;
                        
                        let to_entity = updater(tx, to_id, rust_decimal::Decimal::ZERO, false).await?;
                        
                        // Perform the transfer
                        let updated_from = updater(tx, from_id, -amount, true).await?;
                        let updated_to = updater(tx, to_id, amount, true).await?;
                        
                        tracing::info!(
                            repository = %stringify!(#struct_name),
                            from_id = %from_id,
                            to_id = %to_id,
                            amount = %amount,
                            "Atomic transfer completed"
                        );
                        
                        Ok((updated_from, updated_to))
                    })
                }).await
            }
            
            /// Batch operation with transaction batching for performance
            pub async fn batch_operation<T, F>(
                &self,
                items: Vec<T>,
                batch_size: usize,
                operation: F,
            ) -> Result<Vec<T>, crate::models::#error_type>
            where
                T: Send + 'static + Clone,
                F: Clone + for<'t> Fn(&mut sqlx::Transaction<'t, sqlx::Postgres>, Vec<T>) -> 
                   std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<T>, crate::models::#error_type>> + Send + 't>>,
            {
                let mut results = Vec::with_capacity(items.len());
                let chunks: Vec<Vec<T>> = items.chunks(batch_size).map(|chunk| chunk.to_vec()).collect();
                
                tracing::info!(
                    repository = %stringify!(#struct_name),
                    total_items = %items.len(),
                    batch_count = %chunks.len(),
                    batch_size = %batch_size,
                    "Starting batch operation"
                );
                
                for (batch_index, batch) in chunks.into_iter().enumerate() {
                    let batch_result = self.with_transaction({
                        let operation = operation.clone();
                        |tx| {
                            Box::pin(async move {
                                operation(tx, batch).await
                            })
                        }
                    }).await?;
                    
                    results.extend(batch_result);
                    
                    tracing::debug!(
                        repository = %stringify!(#struct_name),
                        batch_index = %batch_index,
                        processed_count = %results.len(),
                        "Batch completed"
                    );
                }
                
                tracing::info!(
                    repository = %stringify!(#struct_name),
                    total_processed = %results.len(),
                    "Batch operation completed successfully"
                );
                
                Ok(results)
            }
            
            /// Retry transaction operation with exponential backoff for deadlock handling
            pub async fn retry_transaction<F, R>(
                &self,
                max_retries: u32,
                base_delay_ms: u64,
                operation: F,
            ) -> Result<R, crate::models::#error_type>
            where
                F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<R, crate::models::#error_type>> + Send>>,
                R: Send + 'static,
            {
                let mut attempt = 0;
                
                loop {
                    match operation().await {
                        Ok(result) => return Ok(result),
                        Err(e) => {
                            attempt += 1;
                            
                            // Check if it's a retryable error (deadlock, serialization failure)
                            let is_retryable = match &e {
                                crate::models::#error_type::TransactionFailed(msg) => {
                                    msg.contains("deadlock") || 
                                    msg.contains("serialization") ||
                                    msg.contains("could not serialize")
                                }
                                _ => false,
                            };
                            
                            if !is_retryable || attempt >= max_retries {
                                tracing::error!(
                                    repository = %stringify!(#struct_name),
                                    attempt = %attempt,
                                    max_retries = %max_retries,
                                    is_retryable = %is_retryable,
                                    error = %e,
                                    "Transaction failed after retries"
                                );
                                return Err(e);
                            }
                            
                            // Exponential backoff with jitter
                            let delay = base_delay_ms * 2_u64.pow(attempt - 1);
                            let jitter = rand::random::<u64>() % (delay / 4 + 1);
                            let total_delay = delay + jitter;
                            
                            tracing::warn!(
                                repository = %stringify!(#struct_name),
                                attempt = %attempt,
                                delay_ms = %total_delay,
                                error = %e,
                                "Transaction failed, retrying with backoff"
                            );
                            
                            tokio::time::sleep(tokio::time::Duration::from_millis(total_delay)).await;
                        }
                    }
                }
            }
            
            /// Get transaction statistics for monitoring
            pub async fn get_transaction_stats(&self) -> Result<std::collections::HashMap<String, i64>, crate::models::#error_type> {
                let mut stats = std::collections::HashMap::new();
                
                // Get active transaction count
                let active_tx_result = sqlx::query_scalar!(
                    "SELECT COUNT(*) FROM pg_stat_activity WHERE state = 'active' AND backend_type = 'client backend'"
                )
                .fetch_one(&self.#pool_field)
                .await
                .map_err(|e| crate::models::#error_type::TransactionFailed(
                    format!("Failed to get active transactions: {}", e)
                ))?;
                
                stats.insert("active_transactions".to_string(), active_tx_result.unwrap_or(0));
                
                // Get lock statistics
                let locks_result = sqlx::query_scalar!(
                    "SELECT COUNT(*) FROM pg_locks WHERE locktype = 'advisory'"
                )
                .fetch_one(&self.#pool_field)
                .await
                .map_err(|e| crate::models::#error_type::TransactionFailed(
                    format!("Failed to get lock count: {}", e)
                ))?;
                
                stats.insert("advisory_locks".to_string(), locks_result.unwrap_or(0));
                
                tracing::debug!(
                    repository = %stringify!(#struct_name),
                    stats = ?stats,
                    "Transaction statistics retrieved"
                );
                
                Ok(stats)
            }
        }
    };
    
    eprintln!("[pleme-codegen] TransactionalRepository pattern applied to {}", struct_name);
    TokenStream::from(expanded)
}