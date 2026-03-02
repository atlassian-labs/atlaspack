(function() {


//#region src/prelude.ts
	var _ref, _ref2, _ref3, _globalThis;
	const globalObject = (_ref = (_ref2 = (_ref3 = (_globalThis = globalThis) !== null && _globalThis !== void 0 ? _globalThis : global) !== null && _ref3 !== void 0 ? _ref3 : window) !== null && _ref2 !== void 0 ? _ref2 : void 0) !== null && _ref !== void 0 ? _ref : {};
	let registry = {};
	let modules = {};
	let logEntries = [];
	const requireStack = [];
	const log = (message) => logEntries.push(message);
	const metrics = {
		registered: 0,
		executed: 0,
		timings: {}
	};
	const require = (id) => {
		if (modules[id]) {
			log(`${" ".repeat(requireStack.length)}require(${id})`);
			return modules[id].exports;
		}
		const module = { exports: {} };
		modules[id] = module;
		if (!registry[id]) {
			const e = /* @__PURE__ */ new Error(`Cannot find module '${id}'`);
			e.code = "MODULE_NOT_FOUND";
			throw e;
		}
		{
			requireStack.push(id);
			const startTime = Date.now();
			try {
				registry[id].call(module.exports, require, module, module.exports, globalObject);
				const endTime = Date.now();
				log(`${" ".repeat(requireStack.length - 1)}require(${id})::factory ${endTime - startTime}ms`);
				metrics.timings[id] = endTime - startTime;
			} catch (e) {
				log(`\nRequire stack trace (module that threw listed first):`);
				for (let i = requireStack.length - 1; i >= 0; i--) log(`  ${i === requireStack.length - 1 ? ">" : " "} ${requireStack[i]}`);
				throw e;
			} finally {
				requireStack.pop();
			}
		}
		metrics.executed += 1;
		return module.exports;
	};
	const define = (id, factory) => {
		registry[id] = factory;
		metrics.registered += 1;
	};
	const __reset = void 0;
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