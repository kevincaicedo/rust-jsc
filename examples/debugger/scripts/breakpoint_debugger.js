// Deterministic module script for rust-jsc debugger demo.
//
// Goal:
// - Provide stable, line-addressable code for Debugger.setBreakpointByUrl.
// - Provide locals/objects suitable for Debugger.evaluateOnCallFrame.
// - Provide predictable stepping points for Debugger.stepNext / stepOver / stepInto.
//
// Notes:
// - DevTools / WebKit Inspector uses 0-based lineNumber in breakpoint locations.
// - Keep this file mostly static; if you change lines, adjust breakpoint line numbers accordingly.

export function compute(input) {
  // A small object graph to inspect in paused state.
  const obj = {
    input,
    nested: { a: 1, b: 2 },
    list: [10, 20, 30],
  };

  // Some locals for evaluateOnCallFrame.
  let x = input + 1;
  let y = x * 2;

  // A simple function call to step into/over.
  const sum = add(x, y);

  // --- BREAKPOINT TARGET ---
  // Set a breakpoint on the next line (the assignment to `result`).
  const result = {
    x,
    y,
    sum,
    obj,
    tag: "breakpoint_debugger",
  };
  // --- END BREAKPOINT TARGET ---

  // Another deterministic statement for additional stepping.
  result.sumPlusFirst = result.sum + result.obj.list[0];

  return result;
}

export function add(a, b) {
  // Small function for stepInto / stepOver demonstration.
  const tmp = a + b;
  return tmp;
}

// Side-effect free initialization (kept minimal).
export const VERSION = "1.0.0";
