export type Loaded<T> = { ok: true; value: T } | { ok: false; error: string }

// A command that must be refused: `kind` is the AppError kind it was rejected with.
export type Refusal = { rejected: false } | { rejected: true; kind: string | null }
