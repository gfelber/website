const CopyPlugin = require("copy-webpack-plugin");
const path = require('path');

module.exports = {
  entry: "/bootstrap.js",
  output: {
    path: path.resolve(__dirname, "dist"),
    filename: "bootstrap.js",
    publicPath: "/",
  },
  mode: "development",
  plugins: [
    new CopyPlugin({
      patterns: [
        { from: "main.css", to: "main.css" },
        { from: "node_modules/xterm/css/xterm.css", to: "xterm.css" },
        { from: "node_modules/@fontsource-variable/source-code-pro/index.css", to: "font.css" },
        { from: "node_modules/@fontsource-variable/source-code-pro/files", to: "files" },
        { from: "index.html", to: "index.html" },
        { from: "index.html", to: "404.html" },
        { from: "img", to: "img" },
      ],
    }),
  ],
  experiments: {
    asyncWebAssembly: true,
    syncWebAssembly: true
  },
  devServer: {
    hot: true,
    historyApiFallback: {
      rewrites: [{ from: /./, to: '/index.html' }], 
    }
  }
};
