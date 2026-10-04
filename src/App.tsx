import { useCallback, useEffect, useState } from 'react'
import './App.css'

type Arrival = {
  id: string
  route: string
  destination: string
  direction: string
  minutes: number
  predicted: boolean
  timestamp: number
}

type Mode = 'train' | 'bus'
type TransitConfig = {
  arrivalWindowMinutes: number
  modes: Record<Mode, { name: string; serviceLabel: string; plural: string }>
}

function getSavedRefreshRate() {
  const saved = Number(window.localStorage.getItem('train-times-refresh-seconds'))
  return Number.isFinite(saved) && saved >= 10 && saved <= 3600 ? saved : 60
}

function App() {
  const [mode, setMode] = useState<Mode>(() => window.localStorage.getItem('train-times-mode') === 'bus' ? 'bus' : 'train')
  const [refreshSeconds, setRefreshSeconds] = useState(getSavedRefreshRate)
  const [arrivals, setArrivals] = useState<Arrival[]>([])
  const [transitConfig, setTransitConfig] = useState<TransitConfig | null>(null)
  const [lastUpdated, setLastUpdated] = useState<number | null>(null)
  const [now, setNow] = useState(Date.now())
  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState('')

  const refreshArrivals = useCallback(async (signal?: AbortSignal) => {
    try {
      const response = await fetch(`/api/arrivals?mode=${mode}`, { signal, cache: 'no-store' })
      const data = await response.json()
      if (!response.ok) throw new Error(data.error || 'Unable to load arrivals.')
      setArrivals(data.arrivals)
      setLastUpdated(data.updatedAt)
      setError('')
    } catch (cause) {
      if (cause instanceof Error && cause.name === 'AbortError') return
      setError(cause instanceof Error ? cause.message : 'Unable to load arrivals.')
    } finally {
      if (!signal?.aborted) setIsLoading(false)
    }
  }, [mode])

  useEffect(() => {
    const clockTimer = window.setInterval(() => setNow(Date.now()), 15000)
    return () => window.clearInterval(clockTimer)
  }, [])

  useEffect(() => {
    const controller = new AbortController()
    void fetch('/api/config', { signal: controller.signal, cache: 'no-store' })
      .then(async (response) => {
        const data = await response.json()
        if (!response.ok) throw new Error(data.error || 'Unable to load transit settings.')
        setTransitConfig(data)
      })
      .catch((cause) => {
        if (cause instanceof Error && cause.name !== 'AbortError') {
          setError(cause.message)
          setIsLoading(false)
        }
      })
    return () => controller.abort()
  }, [])

  useEffect(() => {
    const controller = new AbortController()
    void refreshArrivals(controller.signal)
    const timer = window.setInterval(() => void refreshArrivals(controller.signal), refreshSeconds * 1000)
    return () => {
      controller.abort()
      window.clearInterval(timer)
    }
  }, [refreshArrivals, refreshSeconds])

  function updateRefreshRate(value: number) {
    const next = Math.min(3600, Math.max(10, value || 10))
    setRefreshSeconds(next)
    window.localStorage.setItem('train-times-refresh-seconds', String(next))
  }

  function changeMode(next: Mode) {
    setMode(next)
    window.localStorage.setItem('train-times-mode', next)
    setArrivals([])
    setLastUpdated(null)
    setIsLoading(true)
  }

  const clock = lastUpdated
    ? new Date(lastUpdated).toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' })
    : '--:--'
  const activeMode = transitConfig?.modes[mode]
  if (!transitConfig || !activeMode) {
    return <main className="dashboard"><div className="empty-state">{error || 'Loading transit settings…'}</div></main>
  }
  const stationParts = activeMode.name.split(' ')

  return (
    <main className="dashboard">
      <section className="station-heading">
        <div className="station-title-row">
          <div className={`station-name ${mode === 'bus' ? 'bus-name' : ''}`}>
            <h1>{mode === 'train' ? <>{stationParts[0]} <span>{stationParts.slice(1).join(' ')}</span></> : activeMode.name}</h1>
            <p className="eyebrow"><span className={`line-key ${mode === 'bus' ? 'bus-key' : ''}`} /> {activeMode.serviceLabel}</p>
          </div>
          <nav className="mode-switch" aria-label="Transit mode">
            <button className={mode === 'train' ? 'selected' : ''} aria-pressed={mode === 'train'} onClick={() => changeMode('train')}>Train</button>
            <button className={mode === 'bus' ? 'selected' : ''} aria-pressed={mode === 'bus'} onClick={() => changeMode('bus')}>Bus</button>
          </nav>
        </div>
        <div className="clock-block">
          <time>{clock}</time>
          <span>LAST UPDATED</span>
        </div>
      </section>

      <section className="arrivals-section" aria-label={`Upcoming ${mode} arrivals`}>
        <div className="section-title"><span>UPCOMING {mode === 'bus' ? 'BUSES' : 'TRAINS'}</span><span className={`service-pill ${error ? 'service-error' : ''}`}>● &nbsp; {error ? 'NO DATA' : 'LIVE'}</span></div>
        <div className="arrival-scroll" role="region" aria-label={`${mode === 'bus' ? 'Bus' : 'Train'} arrivals, scroll for more`} tabIndex={0}>
          <div className="arrival-grid" aria-live="polite">
            {isLoading && arrivals.length === 0 && <div className="empty-state">Checking upcoming {mode === 'bus' ? 'buses' : 'trains'}…</div>}
            {!isLoading && arrivals.length === 0 && !error && <div className="empty-state">No {activeMode.plural} expected in the next {transitConfig.arrivalWindowMinutes} minutes.</div>}
            {error && arrivals.length === 0 && <div className="empty-state error-message">{error}</div>}
            {arrivals.map((arrival) => {
              const minutesUntil = Math.max(0, Math.ceil((arrival.timestamp - now) / 60000))
              return (
                <article className="arrival-card" key={arrival.id}>
                  <div className={`route-symbol ${mode === 'bus' ? 'route-bus' : arrival.route.includes('2') ? 'route-two' : ''}`}>{arrival.route.replace(' Line', '')}</div>
                  <div className="arrival-info">
                    <strong>{arrival.destination}</strong>
                    <span>{mode === 'bus' ? `Route ${arrival.route}` : `${arrival.route} · ${arrival.direction}`} <i>·</i> {arrival.predicted ? 'Predicted' : 'Scheduled'}</span>
                  </div>
                  <div className="countdown"><strong>{minutesUntil === 0 ? 'NOW' : minutesUntil}</strong>{minutesUntil > 0 && <span>MIN</span>}</div>
                </article>
              )
            })}
          </div>
        </div>
      </section>

      <footer className="bottom-bar">
        <div className="update-info"><span className="update-icon">↻</span><span>{error ? 'Showing last successful update' : `Updated ${clock}`}</span></div>
        <div className="controls">
          {error && arrivals.length > 0 && <span className="error-note" role="status">Refresh failed</span>}
          <label htmlFor="refresh-rate">Refresh every</label>
          <input id="refresh-rate" aria-label="Refresh interval in seconds" type="number" min="10" max="3600" step="5" value={refreshSeconds} onChange={(event) => updateRefreshRate(Number(event.target.value))} />
          <span>sec</span>
          <button className="refresh-button" onClick={() => void refreshArrivals()} aria-label="Refresh arrivals">↻</button>
        </div>
      </footer>
    </main>
  )
}

export default App
