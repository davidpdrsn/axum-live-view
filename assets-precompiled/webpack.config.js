const path = require('path');

module.exports = {
  entry: './index.js',
  mode: 'production',
  output: {
    filename: 'axum_live_view.min.js',
    path: path.resolve(__dirname, '.'),
  },
};
