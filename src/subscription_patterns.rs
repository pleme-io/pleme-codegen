//! Subscription Management Patterns
//!
//! Macros for subscription lifecycle, billing, and tier management

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

/// Derive macro for subscription entities with billing logic
pub fn derive_subscription_entity(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    
    eprintln!("[pleme-codegen] SubscriptionEntity pattern applied to {} - saving ~250 lines", struct_name);
    
    let expanded = quote! {
        impl #struct_name {
            /// Check if subscription is currently active
            pub fn is_active(&self) -> bool {
                matches!(self.status, SubscriptionStatus::Active | SubscriptionStatus::Trialing)
                    && self.current_period_end > chrono::Utc::now()
            }
            
            /// Check if subscription is in trial period
            pub fn in_trial(&self) -> bool {
                self.status == SubscriptionStatus::Trialing
                    && self.trial_end.map_or(false, |te| te > chrono::Utc::now())
            }
            
            /// Get days remaining in trial
            pub fn trial_days_remaining(&self) -> Option<i64> {
                self.trial_end.and_then(|te| {
                    let remaining = te - chrono::Utc::now();
                    if remaining.num_days() > 0 {
                        Some(remaining.num_days())
                    } else {
                        None
                    }
                })
            }
            
            /// Check if subscription can be cancelled
            pub fn can_cancel(&self) -> bool {
                matches!(self.status, SubscriptionStatus::Active | SubscriptionStatus::Trialing | SubscriptionStatus::PastDue)
            }
            
            /// Check if subscription can be resumed
            pub fn can_resume(&self) -> bool {
                self.status == SubscriptionStatus::Paused
                    || (self.status == SubscriptionStatus::Cancelled && self.current_period_end > chrono::Utc::now())
            }
            
            /// Calculate next billing date
            pub fn next_billing_date(&self) -> chrono::DateTime<chrono::Utc> {
                match self.interval {
                    BillingInterval::Monthly => self.current_period_end + chrono::Duration::days(30),
                    BillingInterval::Quarterly => self.current_period_end + chrono::Duration::days(90),
                    BillingInterval::Yearly => self.current_period_end + chrono::Duration::days(365),
                }
            }
            
            /// Calculate prorated amount for immediate charge
            pub fn calculate_proration(&self, new_price: rust_decimal::Decimal) -> rust_decimal::Decimal {
                let now = chrono::Utc::now();
                if now >= self.current_period_end {
                    return new_price;
                }
                
                let total_period = (self.current_period_end - self.current_period_start).num_seconds() as f64;
                let remaining_period = (self.current_period_end - now).num_seconds() as f64;
                let proration_ratio = remaining_period / total_period;
                
                let price_diff = new_price - self.price;
                price_diff * rust_decimal::Decimal::from_f64_retain(proration_ratio).unwrap_or(rust_decimal::Decimal::ZERO)
            }
            
            /// Start trial period
            pub fn start_trial(&mut self, trial_days: i64) -> Result<(), PaymentError> {
                if self.status != SubscriptionStatus::Active && self.status != SubscriptionStatus::Trialing {
                    return Err(PaymentError::InvalidSubscriptionStateTransition {
                        from: self.status,
                        to: SubscriptionStatus::Trialing,
                    });
                }
                
                let now = chrono::Utc::now();
                self.status = SubscriptionStatus::Trialing;
                self.trial_start = Some(now);
                self.trial_end = Some(now + chrono::Duration::days(trial_days));
                self.updated_at = now;
                
                tracing::info!(
                    subscription_id = %self.id,
                    trial_days = %trial_days,
                    trial_end = %self.trial_end.unwrap(),
                    "Trial period started"
                );
                
                Ok(())
            }
            
            /// Convert trial to paid subscription
            pub fn convert_trial_to_paid(&mut self) -> Result<(), PaymentError> {
                if self.status != SubscriptionStatus::Trialing {
                    return Err(PaymentError::InvalidSubscriptionStateTransition {
                        from: self.status,
                        to: SubscriptionStatus::Active,
                    });
                }
                
                self.status = SubscriptionStatus::Active;
                self.trial_converted_at = Some(chrono::Utc::now());
                self.updated_at = chrono::Utc::now();
                
                tracing::info!(
                    subscription_id = %self.id,
                    "Trial converted to paid subscription"
                );
                
                Ok(())
            }
            
            /// Pause subscription (keeps access until period end)
            pub fn pause(&mut self, reason: Option<String>) -> Result<(), PaymentError> {
                if !matches!(self.status, SubscriptionStatus::Active | SubscriptionStatus::Trialing) {
                    return Err(PaymentError::InvalidSubscriptionStateTransition {
                        from: self.status,
                        to: SubscriptionStatus::Paused,
                    });
                }
                
                self.status = SubscriptionStatus::Paused;
                self.pause_collection = Some(self.current_period_end);
                self.pause_reason = reason;
                self.updated_at = chrono::Utc::now();
                
                tracing::info!(
                    subscription_id = %self.id,
                    pause_until = %self.current_period_end,
                    "Subscription paused"
                );
                
                Ok(())
            }
            
            /// Resume paused subscription
            pub fn resume(&mut self) -> Result<(), PaymentError> {
                if self.status != SubscriptionStatus::Paused {
                    return Err(PaymentError::InvalidSubscriptionStateTransition {
                        from: self.status,
                        to: SubscriptionStatus::Active,
                    });
                }
                
                self.status = SubscriptionStatus::Active;
                self.pause_collection = None;
                self.pause_reason = None;
                self.updated_at = chrono::Utc::now();
                
                // Calculate new billing period if needed
                let now = chrono::Utc::now();
                if self.current_period_end < now {
                    self.current_period_start = now;
                    self.current_period_end = self.next_billing_date();
                }
                
                tracing::info!(
                    subscription_id = %self.id,
                    new_period_end = %self.current_period_end,
                    "Subscription resumed"
                );
                
                Ok(())
            }
            
            /// Cancel subscription (access until period end)
            pub fn cancel(&mut self, reason: Option<String>) -> Result<(), PaymentError> {
                if !self.can_cancel() {
                    return Err(PaymentError::InvalidSubscriptionStateTransition {
                        from: self.status,
                        to: SubscriptionStatus::Cancelled,
                    });
                }
                
                self.status = SubscriptionStatus::Cancelled;
                self.cancelled_at = Some(chrono::Utc::now());
                self.cancellation_reason = reason;
                self.updated_at = chrono::Utc::now();
                
                tracing::info!(
                    subscription_id = %self.id,
                    access_until = %self.current_period_end,
                    "Subscription cancelled"
                );
                
                Ok(())
            }
            
            /// Mark subscription as past due
            pub fn mark_past_due(&mut self) -> Result<(), PaymentError> {
                if self.status != SubscriptionStatus::Active {
                    return Err(PaymentError::InvalidSubscriptionStateTransition {
                        from: self.status,
                        to: SubscriptionStatus::PastDue,
                    });
                }
                
                self.status = SubscriptionStatus::PastDue;
                self.updated_at = chrono::Utc::now();
                
                tracing::warn!(
                    subscription_id = %self.id,
                    "Subscription marked as past due"
                );
                
                Ok(())
            }
            
            /// Update billing period after successful payment
            pub fn update_billing_period(&mut self) -> Result<(), PaymentError> {
                let now = chrono::Utc::now();
                
                // If past the current period, update to new period
                if self.current_period_end <= now {
                    self.current_period_start = self.current_period_end;
                    self.current_period_end = match self.interval {
                        BillingInterval::Monthly => self.current_period_start + chrono::Duration::days(30),
                        BillingInterval::Quarterly => self.current_period_start + chrono::Duration::days(90),
                        BillingInterval::Yearly => self.current_period_start + chrono::Duration::days(365),
                    };
                }
                
                // Clear past due status if applicable
                if self.status == SubscriptionStatus::PastDue {
                    self.status = SubscriptionStatus::Active;
                }
                
                self.updated_at = now;
                
                tracing::info!(
                    subscription_id = %self.id,
                    period_start = %self.current_period_start,
                    period_end = %self.current_period_end,
                    "Billing period updated"
                );
                
                Ok(())
            }
            
            /// Calculate monthly recurring revenue (MRR)
            pub fn monthly_recurring_revenue(&self) -> rust_decimal::Decimal {
                if !self.is_active() {
                    return rust_decimal::Decimal::ZERO;
                }
                
                match self.interval {
                    BillingInterval::Monthly => self.price,
                    BillingInterval::Quarterly => self.price / rust_decimal::Decimal::from(3),
                    BillingInterval::Yearly => self.price / rust_decimal::Decimal::from(12),
                }
            }
            
            /// Get subscription age
            pub fn age_days(&self) -> i64 {
                (chrono::Utc::now() - self.created_at).num_days()
            }
            
            /// Check if grace period is active
            pub fn in_grace_period(&self, grace_days: i64) -> bool {
                if self.status != SubscriptionStatus::PastDue {
                    return false;
                }
                
                let grace_end = self.current_period_end + chrono::Duration::days(grace_days);
                chrono::Utc::now() <= grace_end
            }
            
            /// Generate subscription metrics
            pub fn metrics(&self) -> SubscriptionMetrics {
                SubscriptionMetrics {
                    is_active: self.is_active(),
                    in_trial: self.in_trial(),
                    mrr: self.monthly_recurring_revenue(),
                    age_days: self.age_days(),
                    lifetime_value: self.price * rust_decimal::Decimal::from(self.age_days() / 30),
                }
            }
        }
        
        /// Subscription metrics
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        pub struct SubscriptionMetrics {
            pub is_active: bool,
            pub in_trial: bool,
            pub mrr: rust_decimal::Decimal,
            pub age_days: i64,
            pub lifetime_value: rust_decimal::Decimal,
        }
    };
    
    TokenStream::from(expanded)
}