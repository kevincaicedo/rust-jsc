// Simple debugger target script for rust-jsc.
//
// This is intentionally minimal and deterministic.
// It should trigger a pause when:
//  - the host enables Debugger domain
//  - the host enables pause-on-debugger-statements (Debugger.setPauseOnDebuggerStatements)
//
// Expected flow:
// 1) Script runs a few statements so "pause at next opportunity" can also work.
// 2) Hits `debugger;` so the debugger must pause.
// 3) Host should receive Debugger.paused and then resume.

(function main() {
  // Some work before the pause point.
  const x = 1;
  const y = x + 1;

  // If you see this but never pause, the embed isn't honoring debugger statements.
  // (You won't see this unless you implement console forwarding.)
  // log("before debugger;", { x, y });

  debugger; // <- should pause here

  // Some work after resume.
  const z = y + 1;
  // log("after debugger;", { z });

  // Make the script produce a value (useful if you print the evaluation result).
  return "simple_debugger.js completed";
})();
