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
        { from: "index.html", to: "index.html" },
      ],
    }),
  ],
  experiments: {
    asyncWebAssembly: true,
    syncWebAssembly: true
  },
  devServer: {
    historyApiFallback: {
      rewrites: [{ from: /./, to: '/index.html' }], 
    }
  }
};
