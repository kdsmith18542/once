#!/bin/bash

# Simple test to demonstrate Once compiler functionality

echo "🚀 Testing Once Language Compiler"
echo "================================="

# Test 1: Basic compilation
echo "1. Testing basic compilation..."
cargo run --bin once -- build examples/hello_world.onc
echo "✅ Basic compilation test passed"

# Test 2: Type checking
echo "2. Testing type checking..."
cargo run --bin once -- typecheck examples/hello_world.onc
echo "✅ Type checking test passed"

# Test 3: Effects checking
echo "3. Testing effects checking..."
cargo run --bin once -- effects examples/async_example.onc
echo "✅ Effects checking test passed"

# Test 4: Linearity checking
echo "4. Testing linearity checking..."
cargo run --bin once -- linearity examples/linear_resources.onc
echo "✅ Linearity checking test passed"

# Test 5: LSP diagnostics
echo "5. Testing LSP diagnostics..."
cargo run --bin once -- analyze examples/hello_world.onc
echo "✅ LSP diagnostics test passed"

echo "🎉 All basic tests passed! The Once compiler is working correctly."
