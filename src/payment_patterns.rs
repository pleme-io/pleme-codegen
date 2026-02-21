//! Payment Processing Patterns
//!
//! Macros for payment processing with Brazilian market support

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

/// Derive macro for payment entities with automatic state management
pub fn derive_payment_entity(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    
    eprintln!("[pleme-codegen] PaymentEntity pattern applied to {} - saving ~150 lines (FIXED VERSION)", struct_name);
    
    let expanded = quote! {
        impl #struct_name {
            /// Mark payment as processing
            pub fn mark_processing(&mut self) -> Result<(), PaymentError> {
                match self.status {
                    PaymentStatus::Pending => {
                        self.status = PaymentStatus::Processing;
                        self.updated_at = chrono::Utc::now();
                        Ok(())
                    }
                    _ => Err(PaymentError::InvalidStateTransition {
                        from: self.status,
                        to: PaymentStatus::Processing,
                    }),
                }
            }
            
            /// Mark payment as completed
            pub fn mark_completed(&mut self) -> Result<(), PaymentError> {
                if self.status != PaymentStatus::Processing && self.status != PaymentStatus::Pending {
                    return Err(PaymentError::InvalidStateTransition {
                        from: self.status,
                        to: PaymentStatus::Completed,
                    });
                }
                self.status = PaymentStatus::Completed;
                self.completed_at = Some(chrono::Utc::now());
                self.updated_at = chrono::Utc::now();
                
                // Track completion metrics
                tracing::info!(
                    payment_id = %self.id,
                    amount = %self.amount,
                    method = ?self.method,
                    "Payment completed successfully"
                );
                
                Ok(())
            }
            
            /// Mark payment as failed with reason
            pub fn mark_failed(&mut self, reason: String) -> Result<(), PaymentError> {
                if self.status == PaymentStatus::Completed || self.status == PaymentStatus::Refunded {
                    return Err(PaymentError::InvalidStateTransition {
                        from: self.status,
                        to: PaymentStatus::Failed,
                    });
                }
                self.status = PaymentStatus::Failed;
                self.failed_at = Some(chrono::Utc::now());
                self.failure_reason = Some(reason.clone());
                self.updated_at = chrono::Utc::now();
                
                // Track failure metrics
                tracing::error!(
                    payment_id = %self.id,
                    amount = %self.amount,
                    reason = %reason,
                    "Payment failed"
                );
                
                Ok(())
            }
            
            /// Check if payment can be refunded
            pub fn can_refund(&self) -> bool {
                self.status == PaymentStatus::Completed
            }
            
            /// Mark payment as refunded
            pub fn mark_refunded(&mut self) -> Result<(), PaymentError> {
                if !self.can_refund() {
                    return Err(PaymentError::InvalidStateTransition {
                        from: self.status,
                        to: PaymentStatus::Refunded,
                    });
                }
                self.status = PaymentStatus::Refunded;
                self.updated_at = chrono::Utc::now();
                
                tracing::info!(
                    payment_id = %self.id,
                    amount = %self.amount,
                    "Payment refunded"
                );
                
                Ok(())
            }
            
            /// Calculate total amount including tax  
            pub fn total_amount(&self) -> rust_decimal::Decimal {
                self.amount + self.tax
            }
            
            /// Calculate net amount after fees (for payouts)
            pub fn net_amount(&self, fee_percentage: rust_decimal::Decimal) -> rust_decimal::Decimal {
                let fee = self.amount * (fee_percentage / rust_decimal::Decimal::from(100));
                self.amount - fee
            }
            
            /// Generate idempotency key for payment processing
            pub fn idempotency_key(&self) -> String {
                use sha2::{Sha256, Digest};
                let mut hasher = Sha256::new();
                hasher.update(self.id.to_string());
                hasher.update(self.amount.to_string());
                hasher.update(self.created_at.to_rfc3339());
                format!("pay_{:x}", hasher.finalize())
            }
            
            /// Validate payment amount
            pub fn validate_amount(&self) -> Result<(), PaymentError> {
                if self.amount <= rust_decimal::Decimal::ZERO {
                    return Err(PaymentError::InvalidAmount);
                }
                
                // Brazilian minimum transaction amount (PIX)
                let min_amount = rust_decimal::Decimal::from_str("0.01").unwrap();
                if self.amount < min_amount {
                    return Err(PaymentError::AmountTooLow { 
                        min: min_amount, 
                        actual: self.amount 
                    });
                }
                
                // Maximum transaction amount check
                let max_amount = rust_decimal::Decimal::from_str("1000000.00").unwrap();
                if self.amount > max_amount {
                    return Err(PaymentError::AmountTooHigh { 
                        max: max_amount, 
                        actual: self.amount 
                    });
                }
                
                Ok(())
            }
            
            /// Get payment age for monitoring
            pub fn age(&self) -> chrono::Duration {
                chrono::Utc::now() - self.created_at
            }
            
            /// Check if payment is expired (for pending payments)
            pub fn is_expired(&self, expiry_minutes: i64) -> bool {
                self.status == PaymentStatus::Pending && 
                self.age() > chrono::Duration::minutes(expiry_minutes)
            }
        }
    };
    
    TokenStream::from(expanded)
}

/// Derive macro for PIX payment handling
pub fn derive_pix_payment(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    
    eprintln!("[pleme-codegen] PixPayment pattern applied to {} - saving ~100 lines", struct_name);
    
    let expanded = quote! {
        impl #struct_name {
            /// Generate PIX QR code payload
            pub fn generate_qr_payload(&self) -> String {
                // PIX payload format according to BCB specification
                let mut payload = String::new();
                
                // Payload Format Indicator
                payload.push_str("000201");
                
                // Point of Initiation Method (12 = Dynamic)
                payload.push_str("010212");
                
                // Merchant Account Information
                payload.push_str("26");
                let merchant_info = format!("0014BR.GOV.BCB.PIX01{:02}{}", 
                    self.pix_key.len(), self.pix_key);
                payload.push_str(&format!("{:02}{}", merchant_info.len(), merchant_info));
                
                // Merchant Category Code (0000 = not informed)
                payload.push_str("52040000");
                
                // Transaction Currency (986 = BRL)
                payload.push_str("5303986");
                
                // Transaction Amount 
                let amount_str = format!("{:.2}", self.amount);
                payload.push_str(&format!("54{:02}{}", amount_str.len(), amount_str));
                
                // Country Code (BR)
                payload.push_str("5802BR");
                
                // Merchant Name
                let name_bytes = self.merchant_name.as_bytes();
                let name_len = name_bytes.len().min(25); // Max 25 chars
                payload.push_str(&format!("59{:02}{}", name_len, &self.merchant_name[..name_len]));
                
                // Additional Data Field Template
                let txid = self.end_to_end_id.clone().unwrap_or_else(|| {
                    uuid::Uuid::new_v4().to_string().replace("-", "")[..25].to_string()
                });
                let additional = format!("05{:02}{}", txid.len(), txid);
                payload.push_str(&format!("62{:02}{}", additional.len(), additional));
                
                // CRC16 placeholder
                payload.push_str("6304");
                
                // Calculate and append CRC16
                let crc = Self::calculate_crc16(&payload);
                payload.push_str(&format!("{:04X}", crc));
                
                payload
            }
            
            /// Calculate CRC16 checksum for PIX payload
            fn calculate_crc16(data: &str) -> u16 {
                const POLYNOMIAL: u16 = 0x1021;
                let mut crc: u16 = 0xFFFF;
                
                for byte in data.bytes() {
                    crc ^= (byte as u16) << 8;
                    for _ in 0..8 {
                        if crc & 0x8000 != 0 {
                            crc = (crc << 1) ^ POLYNOMIAL;
                        } else {
                            crc <<= 1;
                        }
                    }
                }
                
                crc
            }
            
            /// Generate QR code image as base64
            pub fn generate_qr_code_image(&self) -> Result<String, PaymentError> {
                let payload = self.generate_qr_payload();
                
                // Using qrcode crate
                let code = qrcode::QrCode::new(&payload)
                    .map_err(|e| PaymentError::QrCodeGenerationFailed { reason: e.to_string() })?;
                
                // Convert to image
                let image = code.render::<image::Luma<u8>>()
                    .min_dimensions(250, 250)
                    .build();
                
                // Convert to PNG and base64
                let mut buffer = Vec::new();
                let mut cursor = std::io::Cursor::new(&mut buffer);
                image.write_to(&mut cursor, image::ImageFormat::Png)
                    .map_err(|e| PaymentError::QrCodeGenerationFailed { reason: e.to_string() })?;
                
                Ok(base64::encode(&buffer))
            }
            
            /// Validate PIX key format
            pub fn validate_pix_key(&self) -> Result<(), PaymentError> {
                match &self.pix_key_type {
                    PixKeyType::Cpf => {
                        if !Self::validate_cpf(&self.pix_key) {
                            return Err(PaymentError::InvalidPixKey { reason: "Invalid CPF".to_string() });
                        }
                    }
                    PixKeyType::Cnpj => {
                        if !Self::validate_cnpj(&self.pix_key) {
                            return Err(PaymentError::InvalidPixKey { reason: "Invalid CNPJ".to_string() });
                        }
                    }
                    PixKeyType::Email => {
                        if !self.pix_key.contains('@') || self.pix_key.len() > 77 {
                            return Err(PaymentError::InvalidPixKey { reason: "Invalid email".to_string() });
                        }
                    }
                    PixKeyType::Phone => {
                        let digits: String = self.pix_key.chars()
                            .filter(|c| c.is_ascii_digit()).collect();
                        if digits.len() != 11 {
                            return Err(PaymentError::InvalidPixKey { reason: "Invalid phone".to_string() });
                        }
                    }
                    PixKeyType::Random => {
                        // EVP key validation (UUID v4)
                        if self.pix_key.len() != 32 {
                            return Err(PaymentError::InvalidPixKey { reason: "Invalid EVP key".to_string() });
                        }
                    }
                }
                Ok(())
            }
            
            /// Check if PIX payment is expired
            pub fn is_expired(&self) -> bool {
                chrono::Utc::now() > self.expires_at
            }
            
            /// Validate CPF format and checksum
            fn validate_cpf(cpf: &str) -> bool {
                let digits: String = cpf.chars().filter(|c| c.is_ascii_digit()).collect();
                
                // Basic length check
                if digits.len() != 11 {
                    return false;
                }
                
                // Check for invalid sequences (all same digit)
                if digits.chars().all(|c| c == digits.chars().next().unwrap()) {
                    return false;
                }
                
                // Convert to digit array for calculation
                let digits: Vec<u32> = digits.chars()
                    .map(|c| c.to_digit(10).unwrap_or(0))
                    .collect();
                
                // Calculate first verification digit
                let sum1: u32 = (0..9).map(|i| digits[i] * (10 - i as u32)).sum();
                let digit1 = match sum1 % 11 {
                    0 | 1 => 0,
                    n => 11 - n,
                };
                
                if digits[9] != digit1 {
                    return false;
                }
                
                // Calculate second verification digit
                let sum2: u32 = (0..10).map(|i| digits[i] * (11 - i as u32)).sum();
                let digit2 = match sum2 % 11 {
                    0 | 1 => 0,
                    n => 11 - n,
                };
                
                digits[10] == digit2
            }
            
            /// Validate CNPJ format and checksum
            fn validate_cnpj(cnpj: &str) -> bool {
                let digits: String = cnpj.chars().filter(|c| c.is_ascii_digit()).collect();
                
                if digits.len() != 14 {
                    return false;
                }
                
                // Check for invalid sequences
                if digits.chars().all(|c| c == digits.chars().next().unwrap()) {
                    return false;
                }
                
                let digits: Vec<u32> = digits.chars()
                    .map(|c| c.to_digit(10).unwrap_or(0))
                    .collect();
                
                // First verification digit
                let weights1 = [5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2];
                let sum1: u32 = (0..12).map(|i| digits[i] * weights1[i]).sum();
                let digit1 = match sum1 % 11 {
                    0 | 1 => 0,
                    n => 11 - n,
                };
                
                if digits[12] != digit1 {
                    return false;
                }
                
                // Second verification digit
                let weights2 = [6, 5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2];
                let sum2: u32 = (0..13).map(|i| digits[i] * weights2[i]).sum();
                let digit2 = match sum2 % 11 {
                    0 | 1 => 0,
                    n => 11 - n,
                };
                
                digits[13] == digit2
            }
        }
    };
    
    TokenStream::from(expanded)
}