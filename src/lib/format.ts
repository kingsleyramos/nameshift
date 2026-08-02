// Display formatting helpers (history dates, middle truncation).

const relative = new Intl.RelativeTimeFormat('en', { numeric: 'auto' });

/** A relative date for history rows (“2 hours ago”); tooltips carry the
 * absolute date. */
export function relativeDate(iso: string, now: Date = new Date()): string {
  const then = new Date(iso);
  const seconds = Math.round((then.getTime() - now.getTime()) / 1000);
  const table: [Intl.RelativeTimeFormatUnit, number][] = [
    ['year', 31536000],
    ['month', 2592000],
    ['week', 604800],
    ['day', 86400],
    ['hour', 3600],
    ['minute', 60],
  ];
  for (const [unit, span] of table) {
    if (Math.abs(seconds) >= span) {
      return relative.format(Math.round(seconds / span), unit);
    }
  }
  return 'just now';
}

/** Absolute date label for tooltips. */
export function absoluteDate(iso: string): string {
  return new Date(iso).toLocaleString();
}
