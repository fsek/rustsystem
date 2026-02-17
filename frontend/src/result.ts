/**
 * A Rust-inspired Result type for representing success (`Ok`) or failure (`Err`) in TypeScript.
 * Feels just like home :)
 *
 * This abstraction is useful for functions that may fail but where throwing exceptions
 * is undesirable. Instead of throwing, you return a `Result<T, E>`:
 *
 * - `Ok<T>` represents success and holds a value of type `T`.
 * - `Err<E>` represents failure and holds an error of type `E`.
 *
 * Example:
 * ```ts
 * function divide(a: number, b: number): Result<number, string> {
 *   if (b === 0) {
 *     return err("Cannot divide by zero");
 *   }
 *   return ok(a / b);
 * }
 *
 * const res = divide(10, 2);
 *
 * if (isOk(res)) {
 *   console.log("Quotient:", res.value);
 * } else {
 *   console.error("Error:", res.error);
 * }
 * ```
 */

/** Represents a successful result holding a value of type `T`. */
export type Ok<T> = { ok: true; value: T };

/** Represents a failed result holding an error of type `E`. */
export type Err<E> = { ok: false; error: E };

/** Result type: either an `Ok<T>` or an `Err<E>`. */
export type Result<T, E> = Ok<T> | Err<E>;

/**
 * Construct a successful `Result` from a value.
 *
 * @param value - The value to wrap in `Ok`.
 * @returns An `Ok<T>` result.
 */
export function ok<T>(value: T): Ok<T> {
  return { ok: true, value };
}

/**
 * Construct a failed `Result` from an error.
 *
 * @param error - The error to wrap in `Err`.
 * @returns An `Err<E>` result.
 */
export function err<E>(error: E): Err<E> {
  return { ok: false, error };
}

/**
 * Type guard for checking if a `Result` is `Ok`.
 *
 * @param res - The result to check.
 * @returns `true` if the result is `Ok`, otherwise `false`.
 *
 * Example:
 * ```ts
 * if (isOk(res)) {
 *   console.log(res.value); // TypeScript knows `res` is `Ok<T>`
 * }
 * ```
 */
export function isOk<T, E>(res: Result<T, E>): res is Ok<T> {
  return res.ok;
}

/**
 * Type guard for checking if a `Result` is `Err`.
 *
 * @param res - The result to check.
 * @returns `true` if the result is `Err`, otherwise `false`.
 *
 * Example:
 * ```ts
 * if (isErr(res)) {
 *   console.error(res.error); // TypeScript knows `res` is `Err<E>`
 * }
 * ```
 */
export function isErr<T, E>(res: Result<T, E>): res is Err<E> {
  return !res.ok;
}

/**
 * Extract the value from an `Ok` result, or throw if it is an `Err`.
 *
 * Use this only when you are certain the result is `Ok`,
 * otherwise the program will throw an exception.
 *
 * @param res - The result to unwrap.
 * @returns The contained value if `Ok`.
 * @throws An `Error` if the result is `Err`.
 *
 * Example:
 * ```ts
 * const value = unwrap(divide(10, 2)); // 5
 * unwrap(divide(10, 0)); // throws Error("Tried to unwrap Err: Cannot divide by zero")
 * ```
 */
export function unwrap<T, E>(res: Result<T, E>): T {
  if (res.ok) return res.value;
  throw new Error(`Tried to unwrap Err: ${String(res.error)}`);
}

/**
 * Extract the value from an `Ok` result, or return a fallback if it is an `Err`.
 *
 * @param res - The result to unwrap.
 * @param fallback - The fallback value if `res` is `Err`.
 * @returns The contained value if `Ok`, otherwise `fallback`.
 *
 * Example:
 * ```ts
 * const safe = unwrapOr(divide(10, 0), 0); // returns 0 instead of throwing
 * ```
 */
export function unwrapOr<T, E>(res: Result<T, E>, fallback: T): T {
  return res.ok ? res.value : fallback;
}

/**
 * Match on a `Result`, running one of two handlers depending on the variant.
 *
 * This is the most idiomatic way to handle results, similar to Rust's `match`.
 *
 * @param res - The result to match on.
 * @param handlers - An object with two callbacks:
 *   - `Ok`: called if the result is `Ok`, receives the value.
 *   - `Err`: called if the result is `Err`, receives the error.
 * @returns The return value of whichever handler was executed.
 *
 * Example:
 * ```ts
 * const message = matchResult(divide(10, 0), {
 *   Ok: (v) => `Quotient is ${v}`,
 *   Err: (e) => `Error: ${e}`,
 * });
 * // => "Error: Cannot divide by zero"
 * ```
 */
export function matchResult<T, E, R>(
  res: Result<T, E>,
  handlers: {
    Ok: (val: T) => R;
    Err: (err: E) => R;
  },
): R {
  return res.ok ? handlers.Ok(res.value) : handlers.Err(res.error);
}
