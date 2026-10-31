export type Loaded<T> = { ok: true; value: T } | { ok: false; error: string }
