// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import App from './App'

const config = {
  arrivalWindowMinutes: 60,
  modes: {
    train: { name: 'Test Station', serviceLabel: 'Train', plural: 'trains' },
    bus: { name: 'Test Stop', serviceLabel: 'Bus', plural: 'buses' },
  },
  display: {
    timezone: 'UTC', weekdaySchedule: { days: ['sun'], start: '00:00', end: '23:59' },
    idleTimeoutSeconds: 30, defaultRefreshSeconds: 60,
  },
}
const response = (data: unknown, ok = true) => Promise.resolve({ ok, json: async () => data })
const arrivals = (destination = 'Downtown') => ({
  arrivals: [{ id: '1', route: '1 Line', destination, direction: 'North', minutes: 5,
    predicted: true, timestamp: Date.now() + 300000 }], updatedAt: Date.now(),
})
const flush = async () => { await act(async () => { await Promise.resolve() }) }
const advance = async (ms: number) => { await act(async () => { await vi.advanceTimersByTimeAsync(ms) }) }
let fetchMock: ReturnType<typeof vi.fn>
beforeEach(() => {
  vi.useFakeTimers()
  vi.setSystemTime(new Date('2026-10-11T12:00:00Z'))
  localStorage.clear()
  fetchMock = vi.fn((url: string) => url === '/api/config' ? response(config) : response(arrivals()))
  vi.stubGlobal('fetch', fetchMock)
})
afterEach(() => { cleanup(); vi.useRealTimers(); vi.unstubAllGlobals() })
const arrivalCalls = () => fetchMock.mock.calls.filter(([url]) => url.startsWith('/api/arrivals'))
const start = async () => { render(<App />); await flush() }

describe('arrival refresh and startup recovery', () => {
  it('loads arrivals immediately when scheduled active', async () => {
    await start(); expect(arrivalCalls()).toHaveLength(1)
    expect(screen.getByText('Downtown')).toBeTruthy()
  })
  it('refreshes immediately on an active refresh click', async () => {
    await start(); fireEvent.click(screen.getByRole('button', { name: 'Refresh arrivals' })); await flush()
    expect(arrivalCalls()).toHaveLength(2)
  })
  it('aborts each superseded request on repeated refresh clicks', async () => {
    await start()
    fetchMock.mockImplementation(() => new Promise(() => {}))
    const button = screen.getByRole('button', { name: 'Refresh arrivals' })
    fireEvent.click(button); const first = arrivalCalls().at(-1)![1].signal
    fireEvent.click(button); expect(first.aborted).toBe(true)
    expect(arrivalCalls().at(-1)![1].signal.aborted).toBe(false)
  })
  it('ignores a stale response even if fetch ignores cancellation', async () => {
    await start()
    let resolveOld!: (value: unknown) => void
    fetchMock.mockImplementationOnce(() => new Promise((resolve) => { resolveOld = resolve }))
    fireEvent.click(screen.getByRole('button', { name: 'Refresh arrivals' }))
    fetchMock.mockImplementationOnce(() => response(arrivals('Newest')))
    fireEvent.click(screen.getByRole('button', { name: 'Refresh arrivals' })); await flush()
    resolveOld({ ok: true, json: async () => arrivals('Stale') }); await flush()
    expect(screen.getByText('Newest')).toBeTruthy(); expect(screen.queryByText('Stale')).toBeNull()
  })
  it('automatically refreshes and cancels the previous request', async () => {
    await start(); const initial = arrivalCalls()[0][1].signal
    await advance(60000); expect(arrivalCalls()).toHaveLength(2); expect(initial.aborted).toBe(true)
  })
  it('cancels requests when changing mode', async () => {
    await start(); const initial = arrivalCalls()[0][1].signal
    fireEvent.click(screen.getByRole('button', { name: 'Bus' })); await flush()
    expect(initial.aborted).toBe(true); expect(arrivalCalls().at(-1)![0]).toContain('mode=bus')
  })
  it('wakes an inactive display with one immediate request', async () => {
    fetchMock.mockImplementation((url: string) => url === '/api/config'
      ? response({ ...config, display: { ...config.display, weekdaySchedule: { ...config.display.weekdaySchedule, days: [] } } })
      : response(arrivals()))
    await start(); expect(arrivalCalls()).toHaveLength(0)
    fireEvent.click(screen.getByRole('button', { name: 'Refresh arrivals' })); await flush()
    expect(arrivalCalls()).toHaveLength(1)
  })
  it('retries HTTP config failures after ten seconds and recovers', async () => {
    fetchMock.mockImplementationOnce(() => response({ error: 'Settings unavailable' }, false))
    await start(); expect(screen.getByText('Settings unavailable')).toBeTruthy()
    await advance(9999); expect(fetchMock).toHaveBeenCalledTimes(1)
    await advance(1); expect(screen.getByText('Downtown')).toBeTruthy()
  })
  it('retries repeated network config failures at ten-second intervals', async () => {
    fetchMock.mockRejectedValue(new Error('Offline'))
    await start(); await advance(20000); expect(fetchMock).toHaveBeenCalledTimes(3)
  })
  it('cancels config retry on unmount', async () => {
    fetchMock.mockRejectedValue(new Error('Offline'))
    const view = render(<App />); await flush(); view.unmount()
    await advance(20000); expect(fetchMock).toHaveBeenCalledTimes(1)
    expect(fetchMock.mock.calls[0][1].signal.aborted).toBe(true)
  })
  it('cancels arrival requests and timers on unmount', async () => {
    const view = render(<App />); await flush(); const signal = arrivalCalls()[0][1].signal
    view.unmount(); expect(signal.aborted).toBe(true)
    await advance(120000); expect(arrivalCalls()).toHaveLength(1)
  })
})
