const { config } = require("@swc/core/spack");

module.exports = config({
    entry: {
        jsc: __dirname + "/test.js",
    },
    output: {
        path: __dirname + "/output",
    },
    target: "node",
    externalModules: ["@rust-jsc"],
});