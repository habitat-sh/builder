// Karma configuration
module.exports = function (config) {
    config.set({
        frameworks: ["jasmine"],

        files: [
          "node_modules/zone.js/dist/zone.js",
          "node_modules/zone.js/dist/zone-testing.js",
          "app/tests-entry.ts",

          // handle asset requests
          { pattern: 'assets/**/*', watched: false, included: false, served: true },
        ],

        proxies: {
            "/assets": "/base/assets"
        },

        plugins: [
            require("karma-jasmine"),
            require("karma-chrome-launcher"),
            require("karma-webpack"),
            require("karma-sourcemap-loader"),
            require("karma-spec-reporter"),
            require("karma-coverage"),
        ],

        preprocessors: {
            "app/tests-entry.ts": ["webpack", "sourcemap", "coverage"]
        },

        reporters: ["spec"],

        webpack: {
            mode: 'development',
            devtool: "inline-source-map",
            resolve: {
                extensions: [".webpack.js", ".web.js", ".ts", ".js"]
            },
            module: {
                rules: [
                    { test: /\.ts$/, use: [{ loader: "ts-loader" }], exclude: /node_modules/ },
                    { test: /\.html$/, use: [{ loader: "raw-loader" }] },
                ]
            }
        },

        webpackMiddleware: {
            noInfo: true
        },

        port: 9876,
        colors: true,
        logLevel: config.LOG_INFO,
        autoWatch: true,
        browsers: ["ChromeHeadless"],
        singleRun: true,
        concurrency: Infinity
    });
};
