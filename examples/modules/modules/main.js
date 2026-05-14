import { add, label } from "./math.js";
import config from "./settings.json";

globalThis.exampleStaticSummary = `${label}:${config.name}:${add(config.count, 5)}`;
globalThis.exampleImportMeta = import.meta.url;

import("./dynamic.js").then(
    (module) => {
        globalThis.exampleDynamicSummary = module.dynamicValue;
    },
    (error) => {
        globalThis.exampleDynamicError = String((error && error.message) || error);
    },
);

export const summary = globalThis.exampleStaticSummary;

