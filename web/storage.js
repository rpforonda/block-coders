"use strict";
// Block Coders browser saves: a tiny miniquad plugin over window.localStorage.
// Load after gl.js and before load("block-coders.wasm").
//
// Privacy: everything stays in this browser on this device. Nothing is sent
// over the network, and there are no cookies, analytics or third parties.
//
// Security: values are only read and written as plain strings and are never
// evaluated or put into the page. Keys are restricted to the two names the game
// uses and prefixed so they can't collide with other pages on the same origin.
// The Rust side validates everything it loads (src/save.rs, src/program.rs).
(function () {
    const PREFIX = "block-coders/";
    const KEY_RE = /^(users\.txt|user-[0-9]{1,10}\.txt|progress\.txt)$/;
    const MAX_BYTES = 256 * 1024; // matches MAX_SAVE_BYTES in src/save.rs
    const encoder = new TextEncoder();
    const decoder = new TextDecoder("utf-8", { fatal: true });

    function store() {
        // Accessing localStorage throws when storage is blocked (some privacy modes).
        try { return window.localStorage; } catch (e) { return null; }
    }

    function bytes(ptr, len) {
        return new Uint8Array(wasm_memory.buffer, ptr >>> 0, len >>> 0);
    }

    function keyFrom(ptr, len) {
        const name = decoder.decode(bytes(ptr, len));
        if (!KEY_RE.test(name)) throw new Error("block-coders: unexpected storage key");
        return PREFIX + name;
    }

    // Returns the value's length in bytes, copying up to buf_len bytes into buf.
    // -1 means missing or unavailable.
    function get(key_ptr, key_len, buf_ptr, buf_len) {
        try {
            const s = store();
            if (!s) return -1;
            const value = s.getItem(keyFrom(key_ptr, key_len));
            if (value === null) return -1;
            const data = encoder.encode(value);
            if (data.length > MAX_BYTES) return -1;
            const n = Math.min(data.length, buf_len >>> 0);
            if (n > 0) bytes(buf_ptr, n).set(data.subarray(0, n));
            return data.length;
        } catch (e) {
            return -1;
        }
    }

    function set(key_ptr, key_len, val_ptr, val_len) {
        try {
            const s = store();
            if (!s || (val_len >>> 0) > MAX_BYTES) return 0;
            s.setItem(keyFrom(key_ptr, key_len), decoder.decode(bytes(val_ptr, val_len)));
            return 1;
        } catch (e) {
            return 0; // quota exceeded, storage disabled, or a bad key
        }
    }

    function remove(key_ptr, key_len) {
        try {
            const s = store();
            if (s) s.removeItem(keyFrom(key_ptr, key_len));
        } catch (e) { /* nothing to do */ }
    }

    // Whether the main pointer is a finger (phones, tablets), so the game can
    // say "tap" instead of "click" before the first touch. Reads only the
    // browser's pointer type: nothing about the player.
    function touchScreen() {
        try { return window.matchMedia("(pointer: coarse)").matches ? 1 : 0; } catch (e) { return 0; }
    }

    miniquad_add_plugin({
        name: "bc_storage",
        version: 2,
        register_plugin: function (importObject) {
            importObject.env.bc_storage_get = get;
            importObject.env.bc_storage_set = set;
            importObject.env.bc_storage_remove = remove;
            importObject.env.bc_touch_screen = touchScreen;
        },
    });
})();
