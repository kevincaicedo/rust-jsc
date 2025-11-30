# Inspector Testing Documentation

This document describes the comprehensive testing suite for the JavaScriptCore Inspector API integration in rust-jsc.

## Overview

The inspector functionality allows you to debug JavaScript code running in JavaScriptCore contexts using the Chrome DevTools Protocol. Our test suite validates core inspector features including:

- Runtime evaluation
- Debugger functionality
- Breakpoint management
- Script parsing events
- Error handling
- Multiple evaluation scenarios

## Test Architecture

### Test Helper Functions

The tests use a global message storage system to capture and validate inspector responses:

```rust
static mut INSPECTOR_TEST_MESSAGES: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

fn clear_test_messages() // Clears message storage
fn add_test_message(message: &str) // Adds message to storage
fn get_test_messages() -> Vec<String> // Retrieves all messages
fn validate_response(messages: &[String], expected_id: i32, expected_content: &str) -> bool // Validates responses
```

### Inspector Callback Macro

Tests use the `#[inspector_callback]` macro to create C-compatible callback functions:

```rust
#[inspector_callback]
fn my_callback(message: &str) {
    println!("Inspector message: {}", message);
    add_test_message(message);
}
```

## Test Categories

### 1. Basic Inspector Functionality (`test_inspectable_basic`)

**Purpose**: Validates basic inspector setup and simple evaluation.

**Test Flow**:
- Set context as inspectable
- Set up callback
- Send simple arithmetic evaluation
- Verify response

**Key Validations**:
- Context inspector state can be toggled
- Basic message sending/receiving works

### 2. Comprehensive Inspector Testing (`test_inspector_comprehensive`)

**Purpose**: Tests multiple inspector operations in sequence.

**Test Flow**:
1. Runtime.evaluate for arithmetic (`3 * 7`)
2. Debugger.enable
3. Runtime.evaluate for string concatenation (`'Hello' + ' World'`)

**Expected Results**:
- Arithmetic result: `"value":21`
- String result: `"Hello World"`
- Debugger.scriptParsed events
- Multiple messages received

### 3. Error Handling (`test_inspector_error_handling`)

**Purpose**: Validates how inspector handles invalid JavaScript.

**Test Flow**:
- Send invalid JavaScript syntax
- Verify error response format

**Expected Results**:
- `"wasThrown":true` in response
- Proper SyntaxError with description
- Error object with className

### 4. Debugger Workflow (`test_inspector_debugger_workflow`)

**Purpose**: Tests complete debugger setup workflow.

**Test Flow**:
1. Enable debugger
2. Set breakpoint by URL
3. Enable runtime

**Expected Results**:
- Debugger enable success
- Breakpoint ID returned: `"breakpointId":"test.js:1:0"`
- Runtime enable success

### 5. Multiple Evaluations (`test_inspector_multiple_evaluations`)

**Purpose**: Tests robustness with multiple consecutive evaluations.

**Test Cases**:
- `1 + 1` → `2`
- `Math.PI` → `3.141592653589793`
- `typeof 'hello'` → `"string"`
- `[1,2,3].length` → `3`

**Validations**:
- All evaluations return correct results
- Proper ID matching in responses
- No interference between evaluations

### 6. Advanced Debugging (`test_inspector_advanced_debugging`)

**Purpose**: Tests complex debugging scenarios with function creation and execution.

**Test Flow**:
1. Enable debugger and runtime
2. Create function: `function debugTest(x) { var result = x * 2; return result + 1; }`
3. Execute function: `debugTest(5)`
4. Test console API
5. Inspect global object properties

**Expected Results**:
- Function creation succeeds
- Function execution returns `11` (5*2+1)
- Console.log works and returns expected value
- Global object inspection returns array
- Multiple scriptParsed events

### 7. Breakpoint Execution (`test_inspector_breakpoint_with_execution`)

**Purpose**: Tests breakpoint functionality with stateful script execution.

**Test Flow**:
1. Enable debugger
2. Create counter function
3. Set breakpoint
4. Execute function multiple times
5. Verify state changes

**Expected Results**:
- Breakpoint successfully set
- Counter increments correctly (1, 2, 3)
- Each execution generates scriptParsed event
- Final counter value is accurate

## Protocol Messages

### Common Message Format

All inspector messages follow the Chrome DevTools Protocol format:

```json
{
  "id": <number>,
  "method": "<Domain>.<method>",
  "params": { ... }
}
```

### Supported Methods

#### Runtime Domain
- `Runtime.enable` - Enable runtime events
- `Runtime.evaluate` - Evaluate JavaScript expression

#### Debugger Domain  
- `Debugger.enable` - Enable debugger
- `Debugger.setBreakpointByUrl` - Set breakpoint by URL
- `Debugger.setBreakpoint` - Set breakpoint by location

### Events

#### Debugger Events
- `Debugger.scriptParsed` - Fired when script is parsed
  - Contains `scriptId`, `url`, line/column info
  - Indicates script boundaries for debugging

## Response Validation

### Success Response Format
```json
{
  "result": {
    "result": {
      "type": "number|string|object|boolean|undefined",
      "value": <actual_value>,
      "description": "<string_representation>"
    },
    "wasThrown": false
  },
  "id": <matching_request_id>
}
```

### Error Response Format
```json
{
  "result": {
    "result": {
      "type": "object",
      "subtype": "error",
      "className": "SyntaxError",
      "description": "SyntaxError: ..."
    },
    "wasThrown": true
  },
  "id": <matching_request_id>
}
```

## Best Practices

### Test Isolation
- Call `clear_test_messages()` before each test
- Call `JSContext::inspector_cleanup()` after each test
- Use unique IDs for each request to avoid conflicts

### Timing Considerations
- Use `std::thread::sleep()` to allow message processing
- Typical delays: 100-200ms for simple operations
- Longer delays may be needed for complex operations

### Message Validation
- Always validate response IDs match request IDs
- Check for expected content in responses
- Verify error conditions are properly reported

## Debugging Tips

### Enable Debug Output
All callbacks include `println!` statements for debugging:

```rust
#[inspector_callback]
fn debug_callback(message: &str) {
    println!("Debug: {}", message);  // Always visible in test output
    add_test_message(message);
}
```

### Common Issues
- **No response**: Check if inspector is enabled and callback is set
- **Wrong response**: Verify request ID and message format
- **Segmentation fault**: Ensure proper cleanup and avoid running multiple inspector tests simultaneously

## Future Enhancements

Potential areas for expansion:
- Pause/resume functionality testing
- Step debugging (stepInto, stepOver, stepOut)
- Variable inspection during pause
- Call stack analysis
- Source map support testing
- Performance profiling integration

## Integration with Chrome DevTools

These tests validate the same protocol used by Chrome DevTools, meaning:
- Responses should be compatible with Chrome DevTools Inspector
- Message formats follow official Chrome DevTools Protocol specification
- Debugging workflows mirror those available in browser environments