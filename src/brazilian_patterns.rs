//! Brazilian Market Pattern Macros
//! 
//! Tax calculations, shipping zones, and market-specific logic

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

/// BrazilianTaxEntity - Generate Brazilian tax calculations (saves ~30 lines per entity)
pub fn derive_brazilian_tax_entity(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    
    eprintln!("[pleme-codegen] BrazilianTaxEntity pattern applied to {} - saving ~30 lines", struct_name);
    
    let expanded = quote! {
        impl #struct_name {
            /// Calculate ICMS tax by Brazilian state
            pub fn calculate_icms(&self, subtotal: rust_decimal::Decimal, state: &str) -> rust_decimal::Decimal {
                use rust_decimal::Decimal;
                
                let tax_rate = match state.to_uppercase().as_str() {
                    "SP" => Decimal::new(18, 2), // São Paulo - 18%
                    "RJ" => Decimal::new(20, 2), // Rio de Janeiro - 20%
                    "MG" => Decimal::new(18, 2), // Minas Gerais - 18%
                    "RS" => Decimal::new(17, 2), // Rio Grande do Sul - 17%
                    "PR" => Decimal::new(19, 2), // Paraná - 19%
                    "SC" => Decimal::new(17, 2), // Santa Catarina - 17%
                    "BA" => Decimal::new(19, 2), // Bahia - 19%
                    "PE" => Decimal::new(18, 2), // Pernambuco - 18%
                    "CE" => Decimal::new(19, 2), // Ceará - 19%
                    "DF" => Decimal::new(18, 2), // Distrito Federal - 18%
                    "GO" => Decimal::new(17, 2), // Goiás - 17%
                    "MT" => Decimal::new(17, 2), // Mato Grosso - 17%
                    "MS" => Decimal::new(17, 2), // Mato Grosso do Sul - 17%
                    "ES" => Decimal::new(17, 2), // Espírito Santo - 17%
                    "PA" => Decimal::new(19, 2), // Pará - 19%
                    "AM" => Decimal::new(20, 2), // Amazonas - 20%
                    "MA" => Decimal::new(19, 2), // Maranhão - 19%
                    "PI" => Decimal::new(19, 2), // Piauí - 19%
                    "RN" => Decimal::new(18, 2), // Rio Grande do Norte - 18%
                    "PB" => Decimal::new(18, 2), // Paraíba - 18%
                    "AL" => Decimal::new(19, 2), // Alagoas - 19%
                    "SE" => Decimal::new(19, 2), // Sergipe - 19%
                    "TO" => Decimal::new(18, 2), // Tocantins - 18%
                    "RO" => Decimal::new(17, 2), // Rondônia - 17.5%
                    "RR" => Decimal::new(17, 2), // Roraima - 17%
                    "AC" => Decimal::new(17, 2), // Acre - 17%
                    "AP" => Decimal::new(18, 2), // Amapá - 18%
                    _ => Decimal::new(17, 2),    // Default - 17%
                };
                
                let icms = subtotal * tax_rate / Decimal::new(100, 0);
                
                tracing::debug!(
                    entity = %stringify!(#struct_name),
                    subtotal = %subtotal,
                    state = %state,
                    tax_rate = %tax_rate,
                    icms = %icms,
                    "ICMS calculated"
                );
                
                icms
            }
            
            /// Calculate PIS tax (1.65% for standard regime)
            pub fn calculate_pis(&self, subtotal: rust_decimal::Decimal) -> rust_decimal::Decimal {
                let pis_rate = rust_decimal::Decimal::new(165, 4); // 1.65%
                subtotal * pis_rate / rust_decimal::Decimal::new(100, 0)
            }
            
            /// Calculate COFINS tax (7.60% for standard regime)
            pub fn calculate_cofins(&self, subtotal: rust_decimal::Decimal) -> rust_decimal::Decimal {
                let cofins_rate = rust_decimal::Decimal::new(760, 4); // 7.60%
                subtotal * cofins_rate / rust_decimal::Decimal::new(100, 0)
            }
            
            /// Calculate ISS for services (2-5% depending on city)
            pub fn calculate_iss(&self, subtotal: rust_decimal::Decimal, city: &str) -> rust_decimal::Decimal {
                use rust_decimal::Decimal;
                
                let iss_rate = match city.to_uppercase().as_str() {
                    "SAO PAULO" | "SP" => Decimal::new(5, 2),    // 5%
                    "RIO DE JANEIRO" | "RJ" => Decimal::new(5, 2), // 5%
                    "BELO HORIZONTE" | "BH" => Decimal::new(3, 2), // 3%
                    "CURITIBA" => Decimal::new(2, 2),              // 2%
                    _ => Decimal::new(3, 2),                       // 3% default
                };
                
                subtotal * iss_rate / Decimal::new(100, 0)
            }
            
            /// Calculate total Brazilian taxes for goods
            pub fn calculate_total_tax(&self, subtotal: rust_decimal::Decimal, state: &str, is_service: bool) -> rust_decimal::Decimal {
                if is_service {
                    // Services: ISS + PIS + COFINS
                    let iss = self.calculate_iss(subtotal, state);
                    let pis = self.calculate_pis(subtotal);
                    let cofins = self.calculate_cofins(subtotal);
                    iss + pis + cofins
                } else {
                    // Goods: ICMS + PIS + COFINS
                    let icms = self.calculate_icms(subtotal, state);
                    let pis = self.calculate_pis(subtotal);
                    let cofins = self.calculate_cofins(subtotal);
                    icms + pis + cofins
                }
            }
            
            /// Generate NFe (Nota Fiscal Eletrônica) key
            pub fn generate_nfe_key(&self) -> String {
                let timestamp = chrono::Utc::now();
                let random = uuid::Uuid::new_v4().to_string()[..8].to_uppercase();
                format!("NFE-{}-{}", timestamp.format("%Y%m%d%H%M%S"), random)
            }
        }
    };
    
    TokenStream::from(expanded)
}

/// ShippingEntity - Generate shipping calculations with Brazilian zones (saves ~25 lines)
pub fn derive_shipping_entity(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    
    eprintln!("[pleme-codegen] ShippingEntity pattern applied to {} - saving ~25 lines", struct_name);
    
    let expanded = quote! {
        impl #struct_name {
            /// Calculate shipping cost with Brazilian regional zones
            pub fn calculate_shipping_cost(&self, items_count: i32, weight_kg: f64, origin_state: &str, dest_state: &str, country: &str) -> rust_decimal::Decimal {
                use rust_decimal::Decimal;
                
                if country.to_uppercase() != "BR" {
                    // International shipping flat rate
                    return Decimal::new(5000, 2); // R$ 50.00
                }
                
                let base_cost = Decimal::new(1500, 2); // R$ 15.00 base
                let weight_cost = Decimal::from_f64_retain(weight_kg * 5.0).unwrap_or(Decimal::ZERO); // R$ 5.00 per kg
                
                // Calculate zone multiplier based on origin and destination
                let zone_multiplier = self.calculate_zone_multiplier(origin_state, dest_state);
                
                let subtotal = base_cost + weight_cost;
                let total = subtotal * zone_multiplier / Decimal::new(100, 0);
                
                tracing::info!(
                    entity = %stringify!(#struct_name),
                    items = %items_count,
                    weight_kg = %weight_kg,
                    origin = %origin_state,
                    destination = %dest_state,
                    cost = %total,
                    "Shipping cost calculated"
                );
                
                total
            }
            
            fn calculate_zone_multiplier(&self, origin: &str, dest: &str) -> rust_decimal::Decimal {
                use rust_decimal::Decimal;
                
                // Same state = 1.0x
                if origin.to_uppercase() == dest.to_uppercase() {
                    return Decimal::new(100, 2);
                }
                
                // Regional zones
                let southeast = ["SP", "RJ", "MG", "ES"];
                let south = ["PR", "SC", "RS"];
                let northeast = ["BA", "SE", "AL", "PE", "PB", "RN", "CE", "PI", "MA"];
                let north = ["AC", "AP", "AM", "PA", "RO", "RR", "TO"];
                let center = ["GO", "MT", "MS", "DF"];
                
                let origin_upper = origin.to_uppercase();
                let dest_upper = dest.to_uppercase();
                
                // Same region = 1.2x
                if (southeast.contains(&origin_upper.as_str()) && southeast.contains(&dest_upper.as_str())) ||
                   (south.contains(&origin_upper.as_str()) && south.contains(&dest_upper.as_str())) ||
                   (northeast.contains(&origin_upper.as_str()) && northeast.contains(&dest_upper.as_str())) ||
                   (north.contains(&origin_upper.as_str()) && north.contains(&dest_upper.as_str())) ||
                   (center.contains(&origin_upper.as_str()) && center.contains(&dest_upper.as_str())) {
                    return Decimal::new(120, 2);
                }
                
                // Adjacent regions = 1.5x
                if (southeast.contains(&origin_upper.as_str()) && (south.contains(&dest_upper.as_str()) || center.contains(&dest_upper.as_str()))) ||
                   (south.contains(&origin_upper.as_str()) && southeast.contains(&dest_upper.as_str())) {
                    return Decimal::new(150, 2);
                }
                
                // Distant regions = 1.8x
                if (southeast.contains(&origin_upper.as_str()) && north.contains(&dest_upper.as_str())) ||
                   (north.contains(&origin_upper.as_str()) && southeast.contains(&dest_upper.as_str())) ||
                   (south.contains(&origin_upper.as_str()) && north.contains(&dest_upper.as_str())) ||
                   (north.contains(&origin_upper.as_str()) && south.contains(&dest_upper.as_str())) {
                    return Decimal::new(180, 2);
                }
                
                // Default = 1.6x
                Decimal::new(160, 2)
            }
            
            /// Estimate delivery time in business days
            pub fn estimate_delivery_days(&self, origin_state: &str, dest_state: &str, service_type: &str) -> u32 {
                // Same city express
                if origin_state == dest_state && service_type == "express" {
                    return 1;
                }
                
                // Same state
                if origin_state == dest_state {
                    return match service_type {
                        "express" => 1,
                        "standard" => 2,
                        _ => 3,
                    };
                }
                
                // Calculate based on zone distance
                let multiplier = self.calculate_zone_multiplier(origin_state, dest_state);
                let base_days = match service_type {
                    "express" => 2,
                    "standard" => 5,
                    _ => 7,
                };
                
                // Convert multiplier to days factor
                use rust_decimal::prelude::ToPrimitive;
                let factor = multiplier.to_f64().unwrap_or(1.0) / 100.0;
                (base_days as f64 * factor).ceil() as u32
            }
            
            /// Get recommended carrier for route
            pub fn recommend_carrier(&self, origin: &str, dest: &str, weight_kg: f64) -> &'static str {
                if origin == dest {
                    return "Local Courier";
                }
                
                if weight_kg < 1.0 {
                    "Correios PAC Mini"
                } else if weight_kg < 30.0 {
                    "Correios PAC"
                } else if weight_kg < 100.0 {
                    "Transportadora Regional"
                } else {
                    "Transportadora Pesada"
                }
            }
        }
    };
    
    TokenStream::from(expanded)
}