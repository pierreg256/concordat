//! WASM bindings for Concordat — expose CrdtDoc to JavaScript/TypeScript.

use wasm_bindgen::prelude::*;

use crate::codec;
use crate::doc::CrdtDoc as InnerDoc;
use crate::vv::VersionVector;

/// A CRDT JSON document exposed to JavaScript via WebAssembly.
#[wasm_bindgen]
pub struct WasmCrdtDoc {
    inner: InnerDoc,
}

#[wasm_bindgen]
impl WasmCrdtDoc {
    /// Create a new empty document for the given replica.
    #[wasm_bindgen(constructor)]
    pub fn new(replica_id: &str) -> Self {
        Self {
            inner: InnerDoc::new(replica_id),
        }
    }

    /// Set a value at a JSON path.
    ///
    /// `path` — e.g. "/name" or "/user/age"
    /// `value` — a JSON-compatible JS value (string, number, bool, null, object, array)
    #[wasm_bindgen]
    pub fn set(&mut self, path: &str, value: JsValue) -> Result<(), JsError> {
        let v: serde_json::Value =
            serde_wasm_bindgen::from_value(value).map_err(|e| JsError::new(&e.to_string()))?;
        self.inner.set(path, v);
        Ok(())
    }

    /// Set an empty array at the given path.
    #[wasm_bindgen(js_name = "setArray")]
    pub fn set_array(&mut self, path: &str) {
        self.inner.set_array(path);
    }

    /// Remove a key at the given path.
    #[wasm_bindgen]
    pub fn remove(&mut self, path: &str) {
        self.inner.remove(path);
    }

    /// Insert a value into an array at the given path and index.
    #[wasm_bindgen(js_name = "arrayInsert")]
    pub fn array_insert(
        &mut self,
        path: &str,
        index: usize,
        value: JsValue,
    ) -> Result<(), JsError> {
        let v: serde_json::Value =
            serde_wasm_bindgen::from_value(value).map_err(|e| JsError::new(&e.to_string()))?;
        self.inner.array_insert(path, index, v);
        Ok(())
    }

    /// Delete an element from an array at the given path and index.
    #[wasm_bindgen(js_name = "arrayDelete")]
    pub fn array_delete(&mut self, path: &str, index: usize) {
        self.inner.array_delete(path, index);
    }

    /// Materialize the document as a plain JS object (via JSON string).
    #[wasm_bindgen]
    pub fn materialize(&self) -> Result<JsValue, JsError> {
        let v = self.inner.materialize();
        let json_str = serde_json::to_string(&v).map_err(|e| JsError::new(&e.to_string()))?;
        js_sys::JSON::parse(&json_str)
            .map_err(|e| JsError::new(&format!("JSON parse error: {e:?}")))
    }

    /// Produce a delta as opaque bytes (Uint8Array).
    ///
    /// Currently returns the full state delta. The `since_bytes` parameter
    /// is accepted for API compatibility but not yet used for filtering.
    #[wasm_bindgen(js_name = "deltaSince")]
    pub fn delta_since(&self, _since_bytes: Option<Vec<u8>>) -> Vec<u8> {
        let vv = VersionVector::new();
        let delta = self.inner.delta_since(&vv);
        codec::encode(&delta)
    }

    /// Merge a remote delta (Uint8Array) into this document.
    #[wasm_bindgen(js_name = "mergeDelta")]
    pub fn merge_delta(&mut self, bytes: &[u8]) -> Result<(), JsError> {
        let delta = codec::decode(bytes).map_err(|e| JsError::new(&e.to_string()))?;
        self.inner.merge_delta(&delta);
        Ok(())
    }

    /// Get the version vector as opaque bytes (Uint8Array).
    #[wasm_bindgen(js_name = "versionVector")]
    pub fn version_vector(&self) -> Vec<u8> {
        serde_json::to_vec(self.inner.version_vector()).unwrap_or_default()
    }

    /// Get the replica ID.
    #[wasm_bindgen(js_name = "replicaId")]
    pub fn replica_id(&self) -> String {
        self.inner.replica_id().to_string()
    }
}
