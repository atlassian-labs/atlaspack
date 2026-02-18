(function() {


//#region src/prelude.ts
	var _ref, _ref2, _ref3, _globalThis;
	const globalObject = (_ref = (_ref2 = (_ref3 = (_globalThis = globalThis) !== null && _globalThis !== void 0 ? _globalThis : global) !== null && _ref3 !== void 0 ? _ref3 : window) !== null && _ref2 !== void 0 ? _ref2 : void 0) !== null && _ref !== void 0 ? _ref : {};
	let registry = {};
	let modules = {};
	const requireStack = [];
	const require = (id) => {
		if (modules[id]) {
			console.log(`${" ".repeat(requireStack.length)}require(${id})`);
			return modules[id].exports;
		}
		const module = { exports: {} };
		modules[id] = module;
		if (!registry[id]) {
			const e = /* @__PURE__ */ new Error(`Cannot find module '${id}'`);
			e.code = "MODULE_NOT_FOUND";
			throw e;
		}
		console.log(`${" ".repeat(requireStack.length)}require(${id})::factory`);
		requireStack.push(id);
		try {
			registry[id].call(module.exports, require, module, module.exports, globalObject);
		} catch (e) {
			console.error(`\nRequire stack trace (module that threw listed first):`);
			for (let i = requireStack.length - 1; i >= 0; i--) console.error(`  ${i === requireStack.length - 1 ? ">" : " "} ${requireStack[i]}`);
			throw e;
		} finally {
			requireStack.pop();
		}
		return module.exports;
	};
	const define = (id, factory) => {
		registry[id] = factory;
	};
	const __reset = void 0;
	var prelude_default = {
		require,
		define,
		__reset
	};

//#endregion
return prelude_default;
})();