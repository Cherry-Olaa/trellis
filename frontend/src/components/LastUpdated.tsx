interface LastUpdatedProps {
  lastUpdated: Date
  label?: string
}

export function formatTimestamp(date: Date, now: Date = new Date()): string {
  const time = date.toLocaleTimeString('en-US', {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false,
  })

  const dateDay = date.toISOString().slice(0, 10)
  const nowDay  = now.toISOString().slice(0, 10)

  if (dateDay !== nowDay) {
    return `${dateDay} ${time}`
  }

  return time
}

export default function LastUpdated({ lastUpdated, label = 'Last updated' }: LastUpdatedProps) {
  const displayTime = formatTimestamp(lastUpdated)
  const isoTime     = lastUpdated.toISOString()

  return (
    <div className="flex items-center gap-2 text-xs text-gray-400">
      <span>{label}:</span>
      <time dateTime={isoTime} title={isoTime} className="font-mono text-gray-300">
        {displayTime}
      </time>
    </div>
  )
}
