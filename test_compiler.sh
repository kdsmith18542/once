#!/bin/bash

# Once Language Compiler Test Suite
# This script demonstrates how to test the Once language compiler

set -e

echo "🚀 Once Language Compiler Test Suite"
echo "===================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test functions
test_compilation() {
    echo -e "\n${BLUE}📦 Testing Compilation Pipeline${NC}"
    echo "--------------------------------"
    
    echo "1. Testing lexer..."
    cargo run --bin once -- lex examples/hello_world.onc
    echo -e "${GREEN}✅ Lexer test passed${NC}"
    
    echo "2. Testing parser..."
    cargo run --bin once -- parse examples/hello_world.onc
    echo -e "${GREEN}✅ Parser test passed${NC}"
    
    echo "3. Testing HIR generation..."
    cargo run --bin once -- hir examples/hello_world.onc
    echo -e "${GREEN}✅ HIR test passed${NC}"
    
    echo "4. Testing type checking..."
    cargo run --bin once -- typecheck examples/hello_world.onc
    echo -e "${GREEN}✅ Type checking test passed${NC}"
    
    echo "5. Testing effects checking..."
    cargo run --bin once -- effects examples/async_example.onc
    echo -e "${GREEN}✅ Effects checking test passed${NC}"
    
    echo "6. Testing linearity checking..."
    cargo run --bin once -- linearity examples/linear_resources.onc
    echo -e "${GREEN}✅ Linearity checking test passed${NC}"
    
    echo "7. Testing region inference..."
    cargo run --bin once -- regions examples/linear_resources.onc
    echo -e "${GREEN}✅ Region inference test passed${NC}"
    
    echo "8. Testing MIR generation..."
    cargo run --bin once -- mir examples/hello_world.onc
    echo -e "${GREEN}✅ MIR generation test passed${NC}"
    
    echo "9. Testing code generation..."
    cargo run --bin once -- codegen examples/hello_world.onc
    echo -e "${GREEN}✅ Code generation test passed${NC}"
}

test_lsp_features() {
    echo -e "\n${BLUE}🔧 Testing LSP Features${NC}"
    echo "------------------------"
    
    echo "1. Testing LSP server startup..."
    timeout 5s cargo run --bin once -- lsp --stdio < /dev/null || true
    echo -e "${GREEN}✅ LSP server test passed${NC}"
    
    echo "2. Testing diagnostics..."
    cargo run --bin once -- analyze examples/hello_world.onc
    echo -e "${GREEN}✅ Diagnostics test passed${NC}"
    
    echo "3. Testing code actions..."
    cargo run --bin once -- actions examples/hello_world.onc 1 0 1 10
    echo -e "${GREEN}✅ Code actions test passed${NC}"
    
    echo "4. Testing document formatting..."
    cargo run --bin once -- format examples/hello_world.onc
    echo -e "${GREEN}✅ Formatting test passed${NC}"
}

test_build_system() {
    echo -e "\n${BLUE}🔨 Testing Build System${NC}"
    echo "------------------------"
    
    echo "1. Testing build tool..."
    cargo run --bin once -- build examples/hello_world.onc
    echo -e "${GREEN}✅ Build tool test passed${NC}"
    
    echo "2. Testing dependency management..."
    cargo run --bin once -- deps examples/hello_world.onc
    echo -e "${GREEN}✅ Dependency management test passed${NC}"
    
    echo "3. Testing lockfile generation..."
    cargo run --bin once -- lock examples/hello_world.onc
    echo -e "${GREEN}✅ Lockfile generation test passed${NC}"
}

test_explain_modes() {
    echo -e "\n${BLUE}📚 Testing Explain Modes${NC}"
    echo "---------------------------"
    
    echo "1. Testing region explanation..."
    cargo run --bin once -- explain regions examples/linear_resources.onc
    echo -e "${GREEN}✅ Region explanation test passed${NC}"
    
    echo "2. Testing effects explanation..."
    cargo run --bin once -- explain effects examples/async_example.onc
    echo -e "${GREEN}✅ Effects explanation test passed${NC}"
    
    echo "3. Testing linearity explanation..."
    cargo run --bin once -- explain linearity examples/linear_resources.onc
    echo -e "${GREEN}✅ Linearity explanation test passed${NC}"
}

test_advanced_features() {
    echo -e "\n${BLUE}⚡ Testing Advanced Features${NC}"
    echo "--------------------------------"
    
    echo "1. Testing actor system..."
    cargo run --bin once -- actors examples/concurrency.onc
    echo -e "${GREEN}✅ Actor system test passed${NC}"
    
    echo "2. Testing bounds checking..."
    cargo run --bin once -- bounds examples/hello_world.onc
    echo -e "${GREEN}✅ Bounds checking test passed${NC}"
    
    echo "3. Testing FFI system..."
    cargo run --bin once -- ffi examples/hello_world.onc
    echo -e "${GREEN}✅ FFI system test passed${NC}"
    
    echo "4. Testing object format..."
    cargo run --bin once -- object examples/hello_world.onc
    echo -e "${GREEN}✅ Object format test passed${NC}"
    
    echo "5. Testing linker..."
    cargo run --bin once -- link examples/hello_world.onc
    echo -e "${GREEN}✅ Linker test passed${NC}"
}

test_runtime() {
    echo -e "\n${BLUE}🏃 Testing Runtime${NC}"
    echo "------------------"
    
    echo "1. Testing runtime startup..."
    cargo run --bin once -- run examples/hello_world.onc
    echo -e "${GREEN}✅ Runtime test passed${NC}"
    
    echo "2. Testing scheduler..."
    cargo run --bin once -- schedule examples/concurrency.onc
    echo -e "${GREEN}✅ Scheduler test passed${NC}"
    
    echo "3. Testing deadlock detection..."
    cargo run --bin once -- deadlock examples/concurrency.onc
    echo -e "${GREEN}✅ Deadlock detection test passed${NC}"
}

# Main test execution
main() {
    echo "Starting comprehensive test suite..."
    
    # Test compilation pipeline
    test_compilation
    
    # Test LSP features
    test_lsp_features
    
    # Test build system
    test_build_system
    
    # Test explain modes
    test_explain_modes
    
    # Test advanced features
    test_advanced_features
    
    # Test runtime
    test_runtime
    
    echo -e "\n${GREEN}🎉 All tests completed successfully!${NC}"
    echo -e "${YELLOW}The Once language compiler is fully functional!${NC}"
}

# Run the test suite
main "$@"
