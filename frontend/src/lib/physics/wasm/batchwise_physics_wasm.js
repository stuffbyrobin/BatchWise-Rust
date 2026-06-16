let wasm;

const cachedTextDecoder = (typeof TextDecoder !== 'undefined' ? new TextDecoder('utf-8', { ignoreBOM: true, fatal: true }) : { decode: () => { throw Error('TextDecoder not available') } } );

if (typeof TextDecoder !== 'undefined') { cachedTextDecoder.decode(); };

let cachedUint8ArrayMemory0 = null;

function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function getStringFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

const heap = new Array(128).fill(undefined);

heap.push(undefined, null, true, false);

let heap_next = heap.length;

function addHeapObject(obj) {
    if (heap_next === heap.length) heap.push(heap.length + 1);
    const idx = heap_next;
    heap_next = heap[idx];

    heap[idx] = obj;
    return idx;
}
/**
 * Energy (kcal) per 100 ml from ABV %.
 * @param {number} abv_pct
 * @returns {number}
 */
export function energyKcalPer100ml(abv_pct) {
    const ret = wasm.energyKcalPer100ml(abv_pct);
    return ret;
}

/**
 * UK beer duty in **pence** for a volume (litres) at a given ABV %.
 * @param {number} volume_liters
 * @param {number} abv_pct
 * @returns {number}
 */
export function calculateBeerDutyGbPence(volume_liters, abv_pct) {
    const ret = wasm.calculateBeerDutyGbPence(volume_liters, abv_pct);
    return ret;
}

let cachedDataViewMemory0 = null;

function getDataViewMemory0() {
    if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer.detached === true || (cachedDataViewMemory0.buffer.detached === undefined && cachedDataViewMemory0.buffer !== wasm.memory.buffer)) {
        cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
    }
    return cachedDataViewMemory0;
}

function getObject(idx) { return heap[idx]; }

function dropObject(idx) {
    if (idx < 132) return;
    heap[idx] = heap_next;
    heap_next = idx;
}

function takeObject(idx) {
    const ret = getObject(idx);
    dropObject(idx);
    return ret;
}
/**
 * Apparent attenuation % from original and final gravity.
 * @param {number} og
 * @param {number} fg
 * @returns {number}
 */
export function calculateAttenuation(og, fg) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        wasm.calculateAttenuation(retptr, og, fg);
        var r0 = getDataViewMemory0().getFloat64(retptr + 8 * 0, true);
        var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
        var r3 = getDataViewMemory0().getInt32(retptr + 4 * 3, true);
        if (r3) {
            throw takeObject(r2);
        }
        return r0;
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
    }
}

/**
 * UK alcohol units for a serving (ABV % and volume in ml).
 * @param {number} abv_pct
 * @param {number} volume_ml
 * @returns {number}
 */
export function alcoholUnits(abv_pct, volume_ml) {
    const ret = wasm.alcoholUnits(abv_pct, volume_ml);
    return ret;
}

/**
 * Estimated calories per 12 oz from original and final gravity.
 * @param {number} og
 * @param {number} fg
 * @returns {number}
 */
export function calculateCalories(og, fg) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        wasm.calculateCalories(retptr, og, fg);
        var r0 = getDataViewMemory0().getFloat64(retptr + 8 * 0, true);
        var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
        var r3 = getDataViewMemory0().getInt32(retptr + 4 * 3, true);
        if (r3) {
            throw takeObject(r2);
        }
        return r0;
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
    }
}

/**
 * EBC → SRM.
 * @param {number} ebc
 * @returns {number}
 */
export function ebcToSrm(ebc) {
    const ret = wasm.ebcToSrm(ebc);
    return ret;
}

/**
 * Specific gravity → degrees Plato.
 * @param {number} sg
 * @returns {number}
 */
export function sgToPlato(sg) {
    const ret = wasm.sgToPlato(sg);
    return ret;
}

/**
 * Degrees Plato → specific gravity.
 * @param {number} plato
 * @returns {number}
 */
export function platoToSg(plato) {
    const ret = wasm.platoToSg(plato);
    return ret;
}

/**
 * Small Producer Relief rate (0.0–1.0) for an annual production in hl/year.
 * @param {number} annual_production_hl_pa
 * @returns {number}
 */
export function sprReliefRate(annual_production_hl_pa) {
    const ret = wasm.sprReliefRate(annual_production_hl_pa);
    return ret;
}

/**
 * Energy (kJ) per 100 ml from ABV %.
 * @param {number} abv_pct
 * @returns {number}
 */
export function energyKjPer100ml(abv_pct) {
    const ret = wasm.energyKjPer100ml(abv_pct);
    return ret;
}

/**
 * ABV % from original and final gravity (e.g. `1.050, 1.010` → `5.25`).
 * @param {number} og
 * @param {number} fg
 * @returns {number}
 */
export function calculateAbv(og, fg) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        wasm.calculateAbv(retptr, og, fg);
        var r0 = getDataViewMemory0().getFloat64(retptr + 8 * 0, true);
        var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
        var r3 = getDataViewMemory0().getInt32(retptr + 4 * 3, true);
        if (r3) {
            throw takeObject(r2);
        }
        return r0;
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
    }
}

/**
 * SRM → EBC.
 * @param {number} srm
 * @returns {number}
 */
export function srmToEbc(srm) {
    const ret = wasm.srmToEbc(srm);
    return ret;
}

/**
 * Degrees Lovibond → EBC.
 * @param {number} lovibond
 * @returns {number}
 */
export function lovibondToEbc(lovibond) {
    const ret = wasm.lovibondToEbc(lovibond);
    return ret;
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);

            } catch (e) {
                if (module.headers.get('Content-Type') != 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else {
                    throw e;
                }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);

    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };

        } else {
            return instance;
        }
    }
}

function __wbg_get_imports() {
    const imports = {};
    imports.wbg = {};
    imports.wbg.__wbindgen_error_new = function(arg0, arg1) {
        const ret = new Error(getStringFromWasm0(arg0, arg1));
        return addHeapObject(ret);
    };

    return imports;
}

function __wbg_init_memory(imports, memory) {

}

function __wbg_finalize_init(instance, module) {
    wasm = instance.exports;
    __wbg_init.__wbindgen_wasm_module = module;
    cachedDataViewMemory0 = null;
    cachedUint8ArrayMemory0 = null;



    return wasm;
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (typeof module !== 'undefined') {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();

    __wbg_init_memory(imports);

    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }

    const instance = new WebAssembly.Instance(module, imports);

    return __wbg_finalize_init(instance, module);
}

async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;


    if (typeof module_or_path !== 'undefined') {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (typeof module_or_path === 'undefined') {
        module_or_path = new URL('batchwise_physics_wasm_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    __wbg_init_memory(imports);

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync };
export default __wbg_init;
