//! Wallet and Balance Management Patterns
//!
//! Macros for wallet operations with balance tracking and validation

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

/// Derive macro for wallet entities with balance management
pub fn derive_wallet_entity(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    
    eprintln!("[pleme-codegen] WalletEntity pattern applied to {} - saving ~200 lines", struct_name);
    
    let expanded = quote! {
        impl #struct_name {
            /// Get available balance (confirmed funds)
            pub fn available_balance(&self) -> rust_decimal::Decimal {
                self.balance
            }
            
            /// Get total balance (including pending)
            pub fn total_balance(&self) -> rust_decimal::Decimal {
                self.balance + self.pending_balance
            }
            
            /// Add balance with validation and tracking
            pub fn add_balance(&mut self, amount: rust_decimal::Decimal, description: &str) -> Result<(), PaymentError> {
                if amount <= rust_decimal::Decimal::ZERO {
                    return Err(PaymentError::InvalidAmount);
                }
                
                let balance_before = self.balance;
                self.balance += amount;
                self.lifetime_earnings += amount;
                self.updated_at = chrono::Utc::now();
                
                // Track balance change
                tracing::info!(
                    wallet_id = %self.id,
                    user_id = %self.user_id,
                    amount = %amount,
                    balance_before = %balance_before,
                    balance_after = %self.balance,
                    description = %description,
                    "Balance added to wallet"
                );
                
                Ok(())
            }
            
            /// Subtract balance with validation
            pub fn subtract_balance(&mut self, amount: rust_decimal::Decimal, description: &str) -> Result<(), PaymentError> {
                if amount <= rust_decimal::Decimal::ZERO {
                    return Err(PaymentError::InvalidAmount);
                }
                
                if self.balance < amount {
                    return Err(PaymentError::InsufficientFunds);
                }
                
                let balance_before = self.balance;
                self.balance -= amount;
                self.lifetime_spending += amount;
                self.updated_at = chrono::Utc::now();
                
                // Track balance change
                tracing::info!(
                    wallet_id = %self.id,
                    user_id = %self.user_id,
                    amount = %amount,
                    balance_before = %balance_before,
                    balance_after = %self.balance,
                    description = %description,
                    "Balance subtracted from wallet"
                );
                
                Ok(())
            }
            
            /// Add tokens to wallet
            pub fn add_tokens(&mut self, tokens: i64, description: &str) -> Result<(), PaymentError> {
                if tokens < 0i64 {
                    return Err(PaymentError::InvalidAmount);
                }
                
                let tokens_before: i64 = self.tokens;
                self.tokens = tokens_before.saturating_add(tokens);
                self.updated_at = chrono::Utc::now();
                
                tracing::info!(
                    wallet_id = %self.id,
                    user_id = %self.user_id,
                    tokens_added = %tokens,
                    tokens_before = %tokens_before,
                    tokens_after = %self.tokens,
                    description = %description,
                    "Tokens added to wallet"
                );
                
                Ok(())
            }
            
            /// Spend tokens with validation
            pub fn spend_tokens(&mut self, tokens: i64, description: &str) -> Result<(), PaymentError> {
                if tokens < 0i64 {
                    return Err(PaymentError::InvalidAmount);
                }
                
                if self.tokens < tokens {
                    return Err(PaymentError::InsufficientFunds);
                }
                
                let tokens_before: i64 = self.tokens;
                let tokens_to_subtract: i64 = tokens;
                self.tokens = tokens_before - tokens_to_subtract;
                self.updated_at = chrono::Utc::now();
                
                tracing::info!(
                    wallet_id = %self.id,
                    user_id = %self.user_id,
                    tokens_spent = %tokens,
                    tokens_before = %tokens_before,
                    tokens_after = %self.tokens,
                    description = %description,
                    "Tokens spent from wallet"
                );
                
                Ok(())
            }
            
            /// Add pending balance (funds awaiting clearance)
            pub fn add_pending(&mut self, amount: rust_decimal::Decimal, description: &str) -> Result<(), PaymentError> {
                if amount <= rust_decimal::Decimal::ZERO {
                    return Err(PaymentError::InvalidAmount);
                }
                
                self.pending_balance += amount;
                self.updated_at = chrono::Utc::now();
                
                tracing::info!(
                    wallet_id = %self.id,
                    amount = %amount,
                    pending_balance = %self.pending_balance,
                    description = %description,
                    "Pending balance added"
                );
                
                Ok(())
            }
            
            /// Clear pending balance (move to available)
            pub fn clear_pending(&mut self, amount: rust_decimal::Decimal, description: &str) -> Result<(), PaymentError> {
                if amount <= rust_decimal::Decimal::ZERO {
                    return Err(PaymentError::InvalidAmount);
                }
                
                if self.pending_balance < amount {
                    return Err(PaymentError::InvalidAmount);
                }
                
                self.pending_balance -= amount;
                self.balance += amount;
                self.lifetime_earnings += amount;
                self.updated_at = chrono::Utc::now();
                
                tracing::info!(
                    wallet_id = %self.id,
                    amount = %amount,
                    balance = %self.balance,
                    pending_balance = %self.pending_balance,
                    description = %description,
                    "Pending balance cleared to available"
                );
                
                Ok(())
            }
            
            /// Cancel pending balance
            pub fn cancel_pending(&mut self, amount: rust_decimal::Decimal, description: &str) -> Result<(), PaymentError> {
                if amount <= rust_decimal::Decimal::ZERO {
                    return Err(PaymentError::InvalidAmount);
                }
                
                if self.pending_balance < amount {
                    return Err(PaymentError::InvalidAmount);
                }
                
                self.pending_balance -= amount;
                self.updated_at = chrono::Utc::now();
                
                tracing::info!(
                    wallet_id = %self.id,
                    amount = %amount,
                    pending_balance = %self.pending_balance,
                    description = %description,
                    "Pending balance cancelled"
                );
                
                Ok(())
            }
            
            /// Calculate payout amount after fees
            pub fn calculate_payout(
                &self, 
                amount: rust_decimal::Decimal, 
                fee_percentage: rust_decimal::Decimal
            ) -> Result<PayoutCalculation, PaymentError> {
                if amount > self.balance {
                    return Err(PaymentError::InsufficientFunds);
                }
                
                let fee = amount * (fee_percentage / rust_decimal::Decimal::from(100));
                let net_amount = amount - fee;
                
                Ok(PayoutCalculation {
                    gross_amount: amount,
                    fee_percentage,
                    fee_amount: fee,
                    net_amount,
                })
            }
            
            /// Check wallet health metrics
            pub fn health_metrics(&self) -> WalletHealthMetrics {
                let total_balance = self.total_balance();
                let pending_ratio = if total_balance > rust_decimal::Decimal::ZERO {
                    self.pending_balance / total_balance
                } else {
                    rust_decimal::Decimal::ZERO
                };
                
                WalletHealthMetrics {
                    balance: self.balance,
                    pending_balance: self.pending_balance,
                    total_balance,
                    tokens: self.tokens,
                    lifetime_earnings: self.lifetime_earnings,
                    lifetime_spending: self.lifetime_spending,
                    net_earnings: self.lifetime_earnings - self.lifetime_spending,
                    pending_ratio: {
                        use std::str::FromStr;
                        f64::from_str(&pending_ratio.to_string()).unwrap_or(0.0)
                    },
                    last_activity: self.updated_at,
                }
            }
            
            /// Validate minimum balance for operations
            pub fn validate_minimum_balance(&self, minimum: rust_decimal::Decimal) -> Result<(), PaymentError> {
                if self.balance < minimum {
                    return Err(PaymentError::InsufficientFunds);
                }
                Ok(())
            }
            
            /// Lock wallet for maintenance or security
            pub fn lock(&mut self, reason: &str) -> Result<(), PaymentError> {
                if self.locked {
                    return Err(PaymentError::InvalidAmount); // Using available error type
                }
                
                self.locked = true;
                self.locked_at = Some(chrono::Utc::now());
                self.lock_reason = Some(reason.to_string());
                self.updated_at = chrono::Utc::now();
                
                tracing::warn!(
                    wallet_id = %self.id,
                    user_id = %self.user_id,
                    reason = %reason,
                    "Wallet locked"
                );
                
                Ok(())
            }
            
            /// Unlock wallet
            pub fn unlock(&mut self) -> Result<(), PaymentError> {
                if !self.locked {
                    return Err(PaymentError::InvalidAmount); // Using available error type
                }
                
                self.locked = false;
                self.locked_at = None;
                self.lock_reason = None;
                self.updated_at = chrono::Utc::now();
                
                tracing::info!(
                    wallet_id = %self.id,
                    user_id = %self.user_id,
                    "Wallet unlocked"
                );
                
                Ok(())
            }
            
            /// Check if wallet is active
            pub fn is_active(&self) -> bool {
                !self.locked
            }
        }
        
        /// Payout calculation result
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        pub struct PayoutCalculation {
            pub gross_amount: rust_decimal::Decimal,
            pub fee_percentage: rust_decimal::Decimal,
            pub fee_amount: rust_decimal::Decimal,
            pub net_amount: rust_decimal::Decimal,
        }
        
        /// Wallet health metrics
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        pub struct WalletHealthMetrics {
            pub balance: rust_decimal::Decimal,
            pub pending_balance: rust_decimal::Decimal,
            pub total_balance: rust_decimal::Decimal,
            pub tokens: i64,
            pub lifetime_earnings: rust_decimal::Decimal,
            pub lifetime_spending: rust_decimal::Decimal,
            pub net_earnings: rust_decimal::Decimal,
            pub pending_ratio: f64,
            pub last_activity: chrono::DateTime<chrono::Utc>,
        }
    };
    
    TokenStream::from(expanded)
}