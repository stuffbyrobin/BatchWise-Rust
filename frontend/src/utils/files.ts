/** Largest logo the backend accepts (brand assets). */
export const MAX_LOGO_BYTES = 2 * 1024 * 1024

/** Largest import file read in the browser (larger files are almost certainly the wrong file). */
export const MAX_IMPORT_BYTES = 10 * 1024 * 1024

/** A readable size, e.g. `2 MiB`, `512 KiB`. */
export function formatBytes(bytes: number): string {
  if (bytes >= 1024 * 1024) return `${Math.round((bytes / (1024 * 1024)) * 10) / 10} MiB`
  if (bytes >= 1024) return `${Math.round(bytes / 1024)} KiB`
  return `${bytes} B`
}

/** An error message when `file` is larger than `maxBytes`, otherwise `null`. */
export function fileTooLarge(file: File, maxBytes: number): string | null {
  return file.size > maxBytes
    ? `${file.name} is ${formatBytes(file.size)}; the limit is ${formatBytes(maxBytes)}.`
    : null
}
