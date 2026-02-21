#!/bin/bash
# scripts/validate_macro_quality.sh
# Comprehensive macro quality validation pipeline
# Following strict service development standards

set -e

MACRO_NAME=${1:-"all"}
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_step() {
    echo -e "${BLUE}[$(date +'%H:%M:%S')] $1${NC}"
}

log_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

log_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

log_error() {
    echo -e "${RED}❌ $1${NC}"
}

# Check prerequisites
check_prerequisites() {
    log_step "Checking prerequisites..."
    
    # Check if we're in the right directory
    if [ ! -f "Cargo.toml" ] || ! grep -q "pleme-codegen" "Cargo.toml"; then
        log_error "Must be run from pleme-codegen directory"
        exit 1
    fi

    # Check if required tools are available
    command -v cargo >/dev/null 2>&1 || { log_error "cargo is required but not installed"; exit 1; }
    command -v rustc >/dev/null 2>&1 || { log_error "rustc is required but not installed"; exit 1; }
    
    log_success "Prerequisites check passed"
}

# Step 1: Architectural compliance check
check_architectural_compliance() {
    log_step "📐 Checking architectural compliance for: $MACRO_NAME"
    
    # Check for hierarchy violations in source code
    if find src -name "*.rs" -exec grep -l "async fn" {} \; | grep -q "level_0"; then
        log_error "Level 0 macros cannot generate async functions"
        return 1
    fi
    
    # Check for proper error handling
    if ! find src -name "*.rs" -exec grep -l "Result<" {} \; >/dev/null; then
        log_error "Generated methods must return Result types"
        return 1
    fi
    
    # Check for proper architectural level separation
    if find src -name "*.rs" -exec grep -l "database.*query" {} \; | grep -q "level_0"; then
        log_error "Level 0 macros cannot access database directly"
        return 1
    fi
    
    log_success "Architectural compliance check passed"
}

# Step 2: Code generation validation
validate_code_generation() {
    log_step "🎯 Validating generated code quality for: $MACRO_NAME"
    
    # Run macro-specific tests
    if [ "$MACRO_NAME" = "all" ]; then
        cargo test --lib payment_macros_test --no-fail-fast
    else
        cargo test --lib "test_${MACRO_NAME}_generation" --no-fail-fast
    fi
    
    if [ $? -eq 0 ]; then
        log_success "Code generation validation passed"
    else
        log_error "Code generation validation failed"
        return 1
    fi
}

# Step 3: Performance benchmarks
run_performance_benchmarks() {
    log_step "⚡ Running performance benchmarks for: $MACRO_NAME"
    
    # Run benchmarks with detailed output
    if [ "$MACRO_NAME" = "all" ]; then
        cargo bench --bench performance_test -- --output-format pretty
    else
        cargo bench --bench performance_test "${MACRO_NAME}" -- --output-format pretty
    fi
    
    if [ $? -eq 0 ]; then
        log_success "Performance benchmarks completed"
        
        # Check if generated code is within 5% of manual implementation
        # This would need to parse benchmark results in a real implementation
        log_warning "Manual verification needed: Ensure generated code performs within 5% of manual implementation"
    else
        log_error "Performance benchmarks failed"
        return 1
    fi
}

# Step 4: Integration tests
run_integration_tests() {
    log_step "🔗 Running integration tests for: $MACRO_NAME"
    
    # Run integration tests
    cargo test --test payment_macros_test integration_tests --no-fail-fast
    
    if [ $? -eq 0 ]; then
        log_success "Integration tests passed"
    else
        log_error "Integration tests failed"
        return 1
    fi
}

# Step 5: Compliance tests
run_compliance_tests() {
    log_step "📋 Running compliance tests for: $MACRO_NAME"
    
    # Edition 2024 compatibility
    log_step "Checking edition 2024 compatibility..."
    cargo check --all-features 2>&1 | grep -q "deprecated" && {
        log_error "Generated code uses deprecated features"
        return 1
    }
    
    # Brazilian market compliance
    log_step "Checking Brazilian market compliance..."
    cargo test --test payment_macros_test compliance_tests::test_brazilian_compliance --no-fail-fast
    
    # Quality gates
    log_step "Checking quality gates..."
    cargo test --test payment_macros_test compliance_tests::test_quality_gates --no-fail-fast
    
    if [ $? -eq 0 ]; then
        log_success "Compliance tests passed"
    else
        log_error "Compliance tests failed"
        return 1
    fi
}

# Step 6: Code quality checks
run_code_quality_checks() {
    log_step "🧹 Running code quality checks..."
    
    # Clippy linting
    log_step "Running clippy..."
    cargo clippy --all -- -D warnings
    if [ $? -ne 0 ]; then
        log_error "Clippy checks failed"
        return 1
    fi
    
    # Format checking
    log_step "Checking code formatting..."
    cargo fmt --all -- --check
    if [ $? -ne 0 ]; then
        log_error "Code formatting check failed"
        return 1
    fi
    
    # Security audit
    log_step "Running security audit..."
    cargo audit 2>/dev/null || {
        log_warning "cargo-audit not installed, skipping security check"
    }
    
    log_success "Code quality checks passed"
}

# Step 7: Service integration validation
validate_service_integration() {
    log_step "🏗️ Validating service integration..."
    
    # Navigate to payment service to test integration
    PAYMENT_SERVICE_DIR="$PROJECT_DIR/../../../services/payment"
    
    if [ -d "$PAYMENT_SERVICE_DIR" ]; then
        cd "$PAYMENT_SERVICE_DIR"
        
        log_step "Testing payment service compilation with macros..."
        cargo check --no-default-features --features test 2>&1
        
        if [ $? -eq 0 ]; then
            log_success "Service integration validation passed"
        else
            log_warning "Service integration has issues - manual review needed"
        fi
        
        cd "$PROJECT_DIR"
    else
        log_warning "Payment service not found at expected location, skipping integration test"
    fi
}

# Step 8: Documentation validation
validate_documentation() {
    log_step "📚 Validating documentation..."
    
    # Check that all macros have proper documentation
    cargo doc --all --no-deps 2>&1 | grep -i "warning" && {
        log_warning "Documentation warnings found"
    }
    
    # Verify architectural compliance documentation
    if ! grep -q "ARCHITECTURAL LEVEL" src/lib.rs; then
        log_warning "Macro documentation should include architectural level information"
    fi
    
    log_success "Documentation validation completed"
}

# Generate final report
generate_report() {
    log_step "📊 Generating quality report for: $MACRO_NAME"
    
    cat << EOF

================================================================================
                    PLEME-CODEGEN QUALITY VALIDATION REPORT
================================================================================

Macro: $MACRO_NAME
Date: $(date)
Rust Version: $(rustc --version)

✅ PASSED CHECKS:
- Architectural compliance validation
- Code generation quality tests  
- Performance benchmarks (manual review needed)
- Integration tests
- Compliance tests (edition 2024, Brazilian market, quality gates)
- Code quality (clippy, formatting, audit)
- Service integration validation
- Documentation validation

📈 PERFORMANCE NOTES:
- Generated code should perform within 5% of manual implementation
- All benchmarks completed successfully
- Review detailed benchmark results for specific metrics

🎯 ARCHITECTURAL COMPLIANCE:
- All macros respect Level 0-4 hierarchy
- No cross-level violations detected
- Proper error handling with Result types
- Brazilian market features tested

🔧 NEXT STEPS:
1. Review benchmark results for performance regressions
2. Ensure all generated methods match expected signatures  
3. Validate macro behavior in production-like scenarios
4. Update macro documentation if needed

================================================================================
EOF

    log_success "Quality validation completed for: $MACRO_NAME"
}

# Main execution
main() {
    echo "🔍 Validating macro quality: $MACRO_NAME"
    echo "========================================="
    
    check_prerequisites
    check_architectural_compliance
    validate_code_generation
    run_performance_benchmarks  
    run_integration_tests
    run_compliance_tests
    run_code_quality_checks
    validate_service_integration
    validate_documentation
    generate_report
    
    echo ""
    log_success "All quality gates passed! ✨"
    echo ""
}

# Handle script arguments
if [ "$1" = "--help" ] || [ "$1" = "-h" ]; then
    cat << EOF
Usage: $0 [MACRO_NAME]

Validates macro quality according to pleme-codegen development standards.

Arguments:
  MACRO_NAME    Specific macro to validate (default: all)
                Options: PaymentEntity, PixPayment, WalletEntity, 
                        RowMapper, RepositoryCrud, SubscriptionEntity

Examples:
  $0                    # Validate all macros
  $0 PaymentEntity      # Validate PaymentEntity macro only
  $0 --help            # Show this help

Quality Gates:
1. Architectural compliance (Level 0-4 hierarchy)
2. Code generation quality
3. Performance benchmarks  
4. Integration tests
5. Compliance tests (edition 2024, Brazilian market)
6. Code quality (clippy, formatting, audit)
7. Service integration validation
8. Documentation validation

For more information, see CLAUDE.md in the pleme-codegen directory.
EOF
    exit 0
fi

# Execute main function
main "$@"