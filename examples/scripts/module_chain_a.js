// Module A: imports from B
import { multiply } from './module_chain_b.js';

export function compute(x) {
    return multiply(x, x) + x;
}

export default { compute };
