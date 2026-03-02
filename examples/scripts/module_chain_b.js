// Module B: imports from C
import { add } from './module_chain_c.js';

export function multiply(a, b) {
    let result = 0;
    for (let i = 0; i < b; i++) {
        result = add(result, a);
    }
    return result;
}

export default { multiply };
