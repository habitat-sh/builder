'use strict';

const webpack = require('webpack');
const path = require('path');
const { AngularWebpackPlugin } = require('@ngtools/webpack');
const isProduction = process.env.NODE_ENV === 'production';

let rules = [
    // Run the Angular Linker on Angular library packages (partially compiled Ivy)
    {
        test: /\.m?js$/,
        include: /node_modules[\\/]@angular/,
        use: {
            loader: 'babel-loader',
            options: {
                plugins: ['@angular/compiler-cli/linker/babel'],
                compact: false,
                cacheDirectory: true
            }
        }
    },
    { test: /\.[cm]?[jt]sx?$/, use: [{ loader: '@ngtools/webpack' }], exclude: /node_modules/ },
    { test: /\.html$/, type: 'asset/source' }
];

let plugins = [
    new AngularWebpackPlugin({
        tsconfig: path.resolve(__dirname, 'tsconfig.json'),
        jitMode: false
    })
];
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
