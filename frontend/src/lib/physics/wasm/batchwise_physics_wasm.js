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

let WASM_VECTOR_LEN = 0;

const cachedTextEncoder = (typeof TextEncoder !== 'undefined' ? new TextEncoder('utf-8') : { encode: () => { throw Error('TextEncoder not available') } } );

const encodeString = (typeof cachedTextEncoder.encodeInto === 'function'
    ? function (arg, view) {
    return cachedTextEncoder.encodeInto(arg, view);
}
    : function (arg, view) {
    const buf = cachedTextEncoder.encode(arg);
    view.set(buf);
    return {
        read: arg.length,
        written: buf.length
    };
});

function passStringToWasm0(arg, malloc, realloc) {

    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }

    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = encodeString(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
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
 * Computes the treated-water profile + predicted mash pH from a JSON payload.
 * @param {string} input_json
 * @returns {WaterTreatment}
 */
export function computeWaterTreatment(input_json) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passStringToWasm0(input_json, wasm.__wbindgen_export_0, wasm.__wbindgen_export_1);
        const len0 = WASM_VECTOR_LEN;
        wasm.computeWaterTreatment(retptr, ptr0, len0);
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
        if (r2) {
            throw takeObject(r1);
        }
        return WaterTreatment.__wrap(r0);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
    }
}

/**
 * Computes OG/FG/ABV/IBU/colour from a recipe-form JSON payload.
 * @param {string} input_json
 * @returns {RecipeCalcs}
 */
export function computeRecipeCalcs(input_json) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passStringToWasm0(input_json, wasm.__wbindgen_export_0, wasm.__wbindgen_export_1);
        const len0 = WASM_VECTOR_LEN;
        wasm.computeRecipeCalcs(retptr, ptr0, len0);
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
        if (r2) {
            throw takeObject(r1);
        }
        return RecipeCalcs.__wrap(r0);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
    }
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
 * Energy (kcal) per 100 ml from ABV %.
 * @param {number} abv_pct
 * @returns {number}
 */
export function energyKcalPer100ml(abv_pct) {
    const ret = wasm.energyKcalPer100ml(abv_pct);
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
 * UK beer duty in **pence** for a volume (litres) at a given ABV %.
 * @param {number} volume_liters
 * @param {number} abv_pct
 * @returns {number}
 */
export function calculateBeerDutyGbPence(volume_liters, abv_pct) {
    const ret = wasm.calculateBeerDutyGbPence(volume_liters, abv_pct);
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
 * EBC → SRM.
 * @param {number} ebc
 * @returns {number}
 */
export function ebcToSrm(ebc) {
    const ret = wasm.ebcToSrm(ebc);
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

const RecipeCalcsFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_recipecalcs_free(ptr >>> 0, 1));
/**
 * The computed recipe values, surfaced to JS with camelCase getters.
 */
export class RecipeCalcs {

    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(RecipeCalcs.prototype);
        obj.__wbg_ptr = ptr;
        RecipeCalcsFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        RecipeCalcsFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_recipecalcs_free(ptr, 0);
    }
    /**
     * @returns {number}
     */
    get calc_abv_pct() {
        const ret = wasm.recipecalcs_calc_abv_pct(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {number}
     */
    get calc_color_ebc() {
        const ret = wasm.recipecalcs_calc_color_ebc(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {number}
     */
    get calc_fg() {
        const ret = wasm.recipecalcs_calc_fg(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {number}
     */
    get calc_og() {
        const ret = wasm.recipecalcs_calc_og(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {number}
     */
    get calc_ibu() {
        const ret = wasm.recipecalcs_calc_ibu(this.__wbg_ptr);
        return ret;
    }
}

const WaterTreatmentFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_watertreatment_free(ptr >>> 0, 1));
/**
 * The computed water-treatment values, surfaced to JS with snake_case getters.
 */
export class WaterTreatment {

    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(WaterTreatment.prototype);
        obj.__wbg_ptr = ptr;
        WaterTreatmentFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WaterTreatmentFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_watertreatment_free(ptr, 0);
    }
    /**
     * @returns {number}
     */
    get alkalinity() {
        const ret = wasm.watertreatment_alkalinity(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {number}
     */
    get sodium_ppm() {
        const ret = wasm.watertreatment_sodium_ppm(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {number}
     */
    get calcium_ppm() {
        const ret = wasm.watertreatment_calcium_ppm(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {number}
     */
    get sulfate_ppm() {
        const ret = wasm.watertreatment_sulfate_ppm(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {number}
     */
    get chloride_ppm() {
        const ret = wasm.watertreatment_chloride_ppm(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {number}
     */
    get residual_alk() {
        const ret = wasm.watertreatment_residual_alk(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {number}
     */
    get magnesium_ppm() {
        const ret = wasm.watertreatment_magnesium_ppm(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {number}
     */
    get bicarbonate_ppm() {
        const ret = wasm.watertreatment_bicarbonate_ppm(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {number}
     */
    get sulfate_to_chloride() {
        const ret = wasm.watertreatment_sulfate_to_chloride(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {number}
     */
    get mash_ph() {
        const ret = wasm.watertreatment_mash_ph(this.__wbg_ptr);
        return ret;
    }
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
    imports.wbg.__wbindgen_throw = function(arg0, arg1) {
        throw new Error(getStringFromWasm0(arg0, arg1));
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
