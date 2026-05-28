// Karma configuration
module.exports = function (config) {
    const enableCoverage = process.env.KARMA_COVERAGE === "1";

    config.set({
        frameworks: ["jasmine"],

        files: [
          { pattern: require.resolve("zone.js"), watched: false },
          { pattern: require.resolve("zone.js/testing"), watched: false },
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
            "app/tests-entry.ts": ["webpack", "sourcemap"]
        },

        reporters: enableCoverage ? ["spec", "coverage"] : ["spec"],

        coverageReporter: {
            dir: "coverage",
            reporters: [
                { type: "text-summary" },
                { type: "json-summary" },
                { type: "html", subdir: "html" },
                { type: "lcovonly", subdir: ".", file: "lcov.info" }
            ]
        },

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
                    ...(enableCoverage ? [{
                        test: /\.ts$/,
                        use: [{ loader: "@jsdevtools/coverage-istanbul-loader" }],
                        exclude: [/node_modules/, /\.test\.ts$/, /\.spec\.ts$/, /tests-entry\.ts$/],
                        enforce: "post"
                    }] : []),
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
