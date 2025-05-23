const { rspack } = require("@rspack/core");

/** @type {import("@rspack/core").Configuration} */
module.exports = {
	target: "web",
	node: false,
	module: {
		generator: {
			"css/auto": {
				exportsOnly: false
			}
		}
	},
	devtool: "source-map",
	optimization: {
		minimize: true,
		minimizer: [new rspack.LightningCssMinimizerRspackPlugin()]
	},
	externals: ["source-map"],
	externalsType: "commonjs"
};
