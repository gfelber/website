const CopyPlugin = require("copy-webpack-plugin");
const path = require("path");

module.exports = {
  entry: "/index.js",
  output: {
    path: path.resolve(__dirname, "dist"),
    filename: "index.js",
    webassemblyModuleFilename: "wasm-backend.wasm",
    publicPath: "/",
  },
  mode: "development",
  plugins: [
    new CopyPlugin({
      patterns: [
        { from: "main.css", to: "main.css" },
        { from: "node_modules/@xterm/xterm/css/xterm.css", to: "xterm.css" },
        {
          from: "node_modules/@fontsource-variable/source-code-pro/index.css",
          to: "font.css",
        },
        {
          from: "node_modules/@fontsource-variable/source-code-pro/files",
          to: "files",
        },
        { from: "../root", to: "root" },
        { from: "index.html", to: "index.html" },
        { from: "index.html", to: "404.html" },
        { from: "../dirs", to: "." },
        { from: "img", to: "img" },
      ],
    }),
  ],
  experiments: {
    asyncWebAssembly: true,
    syncWebAssembly: true,
  },
  watchOptions: {
    aggregateTimeout: 600,
  },
  devServer: {
    hot: true,
    historyApiFallback: {
      rewrites: [{ from: /./, to: "/index.html" }],
    },
  },
};
