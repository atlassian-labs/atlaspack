(function() {


//#region src/prelude.ts
	var _ref, _ref2, _ref3, _globalThis;
	const globalObject = (_ref = (_ref2 = (_ref3 = (_globalThis = globalThis) !== null && _globalThis !== void 0 ? _globalThis : global) !== null && _ref3 !== void 0 ? _ref3 : window) !== null && _ref2 !== void 0 ? _ref2 : void 0) !== null && _ref !== void 0 ? _ref : {};
	let registry = {};
	let modules = {};
	const require = (id) => {
		if (modules[id]) return modules[id].exports;
		const module = { exports: {} };
		modules[id] = module;
		if (!registry[id]) {
			const e = /* @__PURE__ */ new Error(`Cannot find module '${id}'`);
			e.code = "MODULE_NOT_FOUND";
			throw e;
		}
		registry[id].call(module.exports, require, module, module.exports, globalObject);
		return module.exports;
	};
	const define = (id, factory) => {
		registry[id] = factory;
	};
	const __reset = () => {
		registry = {};
		modules = {};
	};
	var prelude_default = {
		require,
		define,
		__reset
	};

//#endregion
return prelude_default;
})();