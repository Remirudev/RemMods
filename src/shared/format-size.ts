const UNITS = ['B', 'KB', 'MB', 'GB'] as const
const STEP = 1024

const formatters = new Map<string, Intl.NumberFormat>()

// The original's `{0:0.##}`: at most two decimals and no digit grouping, so 1 TiB reads
// "1024 GB" in every language.
function numberFormat(locale: string): Intl.NumberFormat {
  let format = formatters.get(locale)
  if (!format) {
    format = new Intl.NumberFormat(locale, { maximumFractionDigits: 2, useGrouping: false })
    formatters.set(locale, format)
  }
  return format
}

// 1024-based and capped at GB, like the original; unit symbols are not translated.
export function formatSize(bytes: number | null, locale: string): string | null {
  if (bytes === null) return null
  let value = bytes
  let unit = 0
  while (value >= STEP && unit < UNITS.length - 1) {
    value /= STEP
    unit += 1
  }
  return `${numberFormat(locale).format(value)} ${UNITS[unit]}`
}
