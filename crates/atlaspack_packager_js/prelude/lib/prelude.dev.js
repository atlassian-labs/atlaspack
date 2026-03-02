(function() {


//#region src/prelude.ts
	var _ref, _ref2, _ref3, _globalThis;
	const globalObject = (_ref = (_ref2 = (_ref3 = (_globalThis = globalThis) !== null && _globalThis !== void 0 ? _globalThis : global) !== null && _ref3 !== void 0 ? _ref3 : window) !== null && _ref2 !== void 0 ? _ref2 : void 0) !== null && _ref !== void 0 ? _ref : {};
	let registry = {};
	let modules = {};
	let logEntries = void 0;
	const metrics = {
		registered: 0,
		executed: 0,
		timings: {}
	};
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
		metrics.executed += 1;
		return module.exports;
	};
	const define = (id, factory) => {
		registry[id] = factory;
		metrics.registered += 1;
	};
	const __reset = () => {
		registry = {};
		modules = {};
	};
	var prelude_default = {
		require,
		define,
		__reset,
		metrics,
		logEntries
	};

//#endregion
return prelude_default;
})();