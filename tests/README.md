# Once Language Compiler Test Suite

This directory contains comprehensive tests for the Once language compiler, ensuring correctness, performance, and reliability.

## Test Structure

### Integration Tests (`integration_tests.rs`)
- **Purpose**: Test the full compilation pipeline from source to object code
- **Coverage**: End-to-end compilation, error handling, and real-world scenarios
- **Examples**: Basic programs, error cases, complex programs, async examples

### Unit Tests (`unit_tests.rs`)
- **Purpose**: Test individual compiler components in isolation
- **Coverage**: Lexer, parser, HIR, type system, effects, linearity, regions, MIR, codegen
- **Examples**: Token parsing, AST generation, type inference, error handling

### Performance Tests (`performance_tests.rs`)
- **Purpose**: Measure compilation performance and identify bottlenecks
- **Coverage**: Lexing, parsing, type checking, code generation, full pipeline
- **Examples**: Large programs, repeated operations, memory usage

### Regression Tests (`regression_tests.rs`)
- **Purpose**: Ensure previously fixed bugs don't regress
- **Coverage**: Error handling, string literals, type annotations, HIR structure
- **Examples**: Basic compilation, error messages, whitespace handling

### Benchmark Tests (`benchmark_tests.rs`)
- **Purpose**: Measure performance characteristics and identify regressions
- **Coverage**: Individual components, full pipeline, large programs, Cranelift integration
- **Examples**: Performance thresholds, memory usage, compilation speed

## Running Tests

### Run All Tests
```bash
cargo test
```

### Run Specific Test Categories
```bash
# Integration tests
cargo test integration_tests

# Unit tests
cargo test unit_tests

# Performance tests
cargo test performance_tests

# Regression tests
cargo test regression_tests

# Benchmark tests
cargo test benchmark_tests
```

### Run Tests with Output
```bash
# Show test output
cargo test -- --nocapture

# Run specific test
cargo test test_basic_compilation -- --nocapture
```

### Run Tests in Release Mode
```bash
# For performance testing
cargo test --release
```

## Test Categories

### 1. Integration Tests
- **Basic Compilation**: Simple programs that should compile successfully
- **Error Handling**: Programs with syntax, type, or semantic errors
- **Complex Programs**: Multi-function programs with various language features
- **Async Examples**: Programs using async/await and concurrency features

### 2. Unit Tests
- **Lexer Tests**: Token recognition, string literals, number parsing
- **Parser Tests**: AST generation, expression parsing, statement parsing
- **HIR Tests**: High-level IR generation and structure preservation
- **Type System Tests**: Type inference, constraints, error handling
- **Effects Tests**: Effect checking, propagation, error handling
- **Linearity Tests**: Move/consume checking, resource safety
- **Region Tests**: Lifetime inference, escape analysis
- **MIR Tests**: Mid-level IR generation and optimization
- **Codegen Tests**: Machine code generation, object file creation

### 3. Performance Tests
- **Component Performance**: Individual compiler stage performance
- **Pipeline Performance**: Full compilation pipeline timing
- **Memory Usage**: Memory consumption during compilation
- **Large Programs**: Compilation of programs with many functions
- **Repeated Operations**: Performance of repeated compilation tasks

### 4. Regression Tests
- **Basic Functionality**: Core compilation features that must always work
- **Error Handling**: Consistent error messages and behavior
- **String Handling**: String literal parsing and preservation
- **Type Annotations**: Type annotation parsing and validation
- **HIR Structure**: HIR generation and structure preservation
- **Whitespace Handling**: Consistent handling of whitespace and comments

### 5. Benchmark Tests
- **Performance Benchmarks**: Measured performance of compiler components
- **Memory Benchmarks**: Memory usage patterns during compilation
- **Large Program Benchmarks**: Performance with large codebases
- **Cranelift Integration**: Performance of Cranelift backend integration

## Test Data

### Input Programs
- **Simple Programs**: Basic "Hello, World!" and arithmetic
- **Complex Programs**: Multi-function programs with various features
- **Error Programs**: Programs designed to trigger specific errors
- **Large Programs**: Programs with many functions for performance testing

### Expected Outputs
- **Successful Compilation**: Object files and assembly output
- **Error Messages**: Specific, helpful error messages for various error types
- **Performance Metrics**: Timing and memory usage measurements

## Performance Thresholds

### Compilation Speed
- **Lexing**: < 1000ms for 1000 repetitions
- **Parsing**: < 1000ms for 1000 repetitions
- **Type Checking**: < 1000ms for 1000 repetitions
- **Full Pipeline**: < 5000ms for 1000 repetitions
- **Large Programs**: < 10000ms for 1000 functions

### Memory Usage
- **Token Generation**: Reasonable token-to-character ratio
- **AST Generation**: Reasonable AST-to-token ratio
- **HIR Generation**: Reasonable HIR-to-AST ratio

## Continuous Integration

### Automated Testing
- All tests run automatically on every commit
- Performance tests ensure no regressions
- Integration tests verify end-to-end functionality
- Unit tests catch component-level issues

### Test Coverage
- **Lexer**: 100% token type coverage
- **Parser**: 100% AST node coverage
- **HIR**: 100% HIR node coverage
- **Type System**: 100% type constraint coverage
- **Effects**: 100% effect row coverage
- **Linearity**: 100% linearity constraint coverage
- **Regions**: 100% region constraint coverage
- **MIR**: 100% MIR operation coverage
- **Codegen**: 100% code generation coverage

## Debugging Tests

### Test Failures
- Check error messages for specific failure points
- Verify input programs are syntactically correct
- Ensure all dependencies are properly imported
- Check for type mismatches in test code

### Performance Issues
- Profile slow tests to identify bottlenecks
- Check for memory leaks in long-running tests
- Verify performance thresholds are realistic
- Monitor test execution time trends

### Integration Issues
- Verify all compiler components are properly linked
- Check for missing dependencies in test code
- Ensure test data is valid and complete
- Verify test environment setup

## Contributing

### Adding New Tests
1. **Identify Test Category**: Choose appropriate test file
2. **Write Test Function**: Follow existing naming conventions
3. **Add Test Data**: Include input programs and expected outputs
4. **Verify Coverage**: Ensure test covers new functionality
5. **Update Documentation**: Add test description to this README

### Test Naming Conventions
- **Integration Tests**: `test_<feature>_compilation`
- **Unit Tests**: `test_<component>_<functionality>`
- **Performance Tests**: `test_<component>_performance`
- **Regression Tests**: `test_<feature>_regression`
- **Benchmark Tests**: `benchmark_<component>_performance`

### Test Documentation
- **Purpose**: Clearly state what the test verifies
- **Input**: Describe the test input and expected behavior
- **Output**: Explain expected results and success criteria
- **Dependencies**: List any required setup or data

## Maintenance

### Regular Updates
- **Performance Thresholds**: Update based on hardware improvements
- **Test Data**: Refresh with new language features
- **Coverage**: Ensure new features are tested
- **Documentation**: Keep test descriptions current

### Test Optimization
- **Parallel Execution**: Run independent tests in parallel
- **Test Data**: Use efficient test data structures
- **Memory Management**: Avoid memory leaks in long-running tests
- **Performance**: Optimize slow tests without losing coverage

## Future Enhancements

### Planned Improvements
- **Property-Based Testing**: Random program generation for edge cases
- **Fuzzing**: Automated error injection testing
- **Performance Profiling**: Detailed performance analysis tools
- **Test Visualization**: Graphical test result reporting
- **Continuous Benchmarking**: Automated performance regression detection

### Test Infrastructure
- **Test Runners**: Specialized test execution environments
- **Test Data Management**: Centralized test data repository
- **Performance Monitoring**: Real-time performance tracking
- **Test Reporting**: Comprehensive test result analysis
