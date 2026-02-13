// Runner module for breakpoint_debugger example.
// This module imports breakpoint_debugger.js and calls compute() at top-level,
// ensuring the breakpoint inside compute() is hit in the correct call frame context.

import { compute } from "./breakpoint_debugger.js";

// Call compute() with a test input.
// The breakpoint set on line 11 (0-based line 8) inside compute() will be hit here.
const result = compute(42);

// Export the result so the module evaluation produces a value.
export { result };
