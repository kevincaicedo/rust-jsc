const { config } = require("@swc/core/spack");

const options = config({
    entry: {
        jsc: __dirname + "/test.js",
    },
    output: {
        path: __dirname + "/output",
    },
    target: "node",
    externalModules: ["@rust-jsc"],
});

module.exports = options;