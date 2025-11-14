let wasmInitialized = false;
let initPromise: Promise<void> | null = null;

/**
 * Ensure WASM module is loaded before calling WASM functions
 * Simple solution for slow connections
 */
export async function ensureWasmLoaded(): Promise<void> {
  if (wasmInitialized) {
    return;
  }

  if (initPromise) {
    await initPromise;
    return;
  }

  initPromise = (async () => {
    try {
      const wasmModule = await import('@/pkg/rustsystem_client.js');
      await wasmModule.default();
      wasmInitialized = true;
    } catch (error) {
      // Reset on error so retry is possible
      initPromise = null;
      throw new Error(`Failed to load WASM module: ${error}`);
    }
  })();

  await initPromise;
}

/**
 * Wrapper to ensure WASM is loaded before executing function
 */
export async function withWasm<T>(fn: () => T | Promise<T>): Promise<T> {
  await ensureWasmLoaded();
  return await fn();
}
