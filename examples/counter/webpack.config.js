const path = require("path");
const dist = path.resolve(__dirname, "dist");
const WasmPackPlugin = require("@wasm-tool/wasm-pack-plugin");

module.exports = {
  entry: "./js/index.js",
  output: {
    publicPath: "dist/",
    path: dist,
    filename: "bundle.js"
  },
  plugins: [
    new WasmPackPlugin({
      crateDirectory: path.resolve(__dirname, "crate"),
    }),
  ],
  devServer: {
    host: '0.0.0.0',
  },
};
