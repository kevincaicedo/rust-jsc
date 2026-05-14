import { add, label } from "../math.js";
import config from "../config.json";

globalThis.realFileSummary = `${label}:${config.name}:${add(config.count, 5)}`;
globalThis.realFileMeta = import.meta.url;

import("./dynamic.js").then(
    (module) => {
        globalThis.realFileDynamic = `${module.dynamicValue}:${import.meta.url.includes("/nested/")}`;
    },
    (error) => {
        globalThis.realFileDynamicError = String((error && error.message) || error);
    },
);

export const summary = globalThis.realFileSummary;

