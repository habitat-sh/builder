'use strict';

const webpack = require('webpack');
const path = require('path');
const isProduction = process.env.NODE_ENV === 'production';

let rules = [
    { test: /\.ts$/, use: [{ loader: 'ts-loader' }] },
    { test: /\.html$/, use: [{ loader: 'raw-loader' }] }
];

let plugins = [];
let devtool = 'source-map';

if (isProduction) {
    devtool = false;
}

module.exports = {
    mode: isProduction ? 'production' : 'development',
    devtool: devtool,
    entry: './app/main.ts',
    output: {
        path: path.resolve(__dirname, 'assets'),
        filename: 'app.js'
    },
    resolve: {
        extensions: ['.webpack.js', '.web.js', '.ts', '.js']
    },
    module: {
        rules: rules
    },
    plugins: plugins,
    stats: {
        chunks: false
    },
    bail: true
};
