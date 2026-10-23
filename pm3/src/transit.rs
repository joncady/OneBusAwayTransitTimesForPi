//! OneBusAway adapter. Network/process work stays off the NFC/render thread.
//! curl is part of the audited OS; no Python, Node, or TLS bypass is required.
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Read, Write},
    process::{Command, Stdio},
    sync::{
        atomic::Ordering,
        mpsc::{self, Receiver, TryRecvError},
    },
    thread,
    time::{Duration, Instant},
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Settings {
    pub mode: String,
    pub timezone: String,
    pub refresh_seconds: u64,
    #[serde(default = "default_startup_retry_seconds")]
    pub startup_retry_seconds: u64,
    pub page_seconds: u64,
    pub stale_after_seconds: u64,
    pub max_cache_age_seconds: u64,
    pub max_backoff_seconds: u64,
    base_url: String,
}

fn default_startup_retry_seconds() -> u64 {
    5
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Stop {
    id: String,
    #[serde(default)]
    direction: String,
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Mode {
    name: String,
    service_label: String,
    plural: String,
    stops: Vec<Stop>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Config {
    arrival_window_minutes: u64,
    max_arrivals: usize,
    modes: BTreeMap<String, Mode>,
}

fn load_json<T: for<'a> Deserialize<'a>>(path: &str) -> Result<T, Box<dyn std::error::Error>> {
    if fs::metadata(path)?.len() > 64 * 1024 {
        return Err("configuration exceeds 64 KiB".into());
    }
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

fn api_key(path: &str) -> Result<String, Box<dyn std::error::Error>> {
    if fs::metadata(path)?.len() > 16 * 1024 {
        return Err("environment file too large".into());
    }
    let text = fs::read_to_string(path)?;
    let key = text
        .lines()
        .find_map(|line| {
            let line = line.trim().strip_prefix("export ").unwrap_or(line.trim());
            let (name, value) = line.split_once('=')?;
            (name.trim() == "API_KEY").then(|| value.trim().trim_matches(['\'', '"']).to_string())
        })
        .ok_or("API_KEY missing from environment file")?;
    if key.is_empty() || key.len() > 512 || !key.bytes().all(|b| b.is_ascii_graphic()) {
        return Err("invalid API_KEY".into());
    }
    Ok(key)
}

pub struct Prepared {
    settings: Settings,
    config: Config,
    key: String,
    zone: Tz,
}
impl Prepared {
    pub fn load(
        transit: &str,
        display: &str,
        env: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let settings: Settings = load_json(display)?;
        let config: Config = load_json(transit)?;
        let zone = settings
            .timezone
            .parse::<Tz>()
            .map_err(|_| "unknown display timezone")?;
        if !(10..=3600).contains(&settings.refresh_seconds)
            || !(1..=60).contains(&settings.startup_retry_seconds)
            || !(3..=120).contains(&settings.page_seconds)
            || settings.stale_after_seconds < settings.refresh_seconds
            || settings.stale_after_seconds > 7200
            || settings.max_cache_age_seconds < settings.stale_after_seconds
            || settings.max_cache_age_seconds > 86400
            || settings.max_backoff_seconds < settings.refresh_seconds
            || settings.max_backoff_seconds > 7200
        {
            return Err("invalid polling, paging, or cache limits".into());
        }
        if !settings.base_url.starts_with("https://")
            || settings.base_url.len() > 256
            || !settings
                .base_url
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b":/.-_".contains(&b))
        {
            return Err("baseUrl must be an HTTPS origin/path without query parameters".into());
        }
        if !(1..=360).contains(&config.arrival_window_minutes)
            || !(1..=50).contains(&config.max_arrivals)
            || config.modes.is_empty()
            || config.modes.len() > 8
            || !config.modes.contains_key(&settings.mode)
        {
            return Err("invalid transit configuration or selected mode".into());
        }
        for mode in config.modes.values() {
            if [&mode.name, &mode.service_label, &mode.plural]
                .iter()
                .any(|s| s.trim().is_empty() || s.len() > 160 || s.chars().any(char::is_control))
                || mode.stops.is_empty()
                || mode.stops.len() > 8
            {
                return Err("invalid mode label or stop count".into());
            }
            for stop in &mode.stops {
                if stop.id.is_empty()
                    || stop.id.len() > 100
                    || !stop
                        .id
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
                    || stop.direction.len() > 80
                    || stop.direction.chars().any(char::is_control)
                {
                    return Err("invalid stop configuration; replace any placeholders".into());
                }
            }
        }
        Ok(Self {
            settings,
            config,
            key: api_key(env)?,
            zone,
        })
    }

    pub fn start(self) -> Transit {
        let mode = self.config.modes[&self.settings.mode].clone();
        let (sender, receiver) = mpsc::sync_channel(1);
        let refresh = self.settings.refresh_seconds;
        let startup_retry = self.settings.startup_retry_seconds;
        let max_backoff = self.settings.max_backoff_seconds;
        let base = self.settings.base_url.clone();
        let worker_mode = mode.clone();
        let worker = thread::spawn(move || {
            let mut failures = 0u32;
            let mut has_succeeded = false;
            loop {
                if crate::STOP.load(Ordering::Relaxed) {
                    break;
                }
                let result = fetch(
                    &base,
                    &self.key,
                    &worker_mode,
                    self.config.arrival_window_minutes,
                    self.config.max_arrivals,
                );
                failures = if result.is_ok() {
                    has_succeeded = true;
                    0
                } else {
                    failures.saturating_add(1)
                };
                // Render immediately at boot; recover quickly when DHCP/DNS is
                // still settling. After first success retain the usual API backoff.
                let delay = if has_succeeded {
                    refresh
                        .saturating_mul(1u64 << failures.min(6))
                        .min(max_backoff)
                } else {
                    startup_retry
                        .saturating_mul(1u64 << failures.saturating_sub(1).min(6))
                        .min(60)
                        .min(max_backoff)
                };
                if sender.try_send(result).is_err() && crate::STOP.load(Ordering::Relaxed) {
                    break;
                }
                let until = Instant::now() + Duration::from_secs(delay);
                while Instant::now() < until && !crate::STOP.load(Ordering::Relaxed) {
                    thread::sleep(Duration::from_millis(100));
                }
            }
        });
        Transit {
            settings: self.settings,
            zone: self.zone,
            mode,
            receiver,
            worker: Some(worker),
            snapshot: None,
            failed: false,
            window: self.config.arrival_window_minutes,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Response {
    code: u16,
    current_time: i64,
    data: Option<Data>,
}
#[derive(Deserialize)]
struct Data {
    entry: Entry,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Entry {
    arrivals_and_departures: Vec<ApiArrival>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiArrival {
    #[serde(default)]
    trip_id: String,
    #[serde(default)]
    stop_sequence: u32,
    #[serde(default)]
    route_short_name: String,
    #[serde(default)]
    trip_headsign: String,
    #[serde(default)]
    predicted: bool,
    #[serde(default)]
    predicted_arrival_time: i64,
    #[serde(default)]
    scheduled_arrival_time: i64,
}
struct Arrival {
    route: String,
    destination: String,
    direction: String,
    timestamp: i64,
    predicted: bool,
}
struct Snapshot {
    arrivals: Vec<Arrival>,
    server_time: i64,
    received: Instant,
}

fn encode(value: &str) -> String {
    value
        .bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

fn request(url: &str) -> Result<Response, &'static str> {
    // Credentials travel over stdin, never argv or logs. No redirects or -k.
    let mut child = Command::new("/usr/bin/curl")
        .args([
            "--config",
            "-",
            "--silent",
            "--fail",
            "--proto",
            "=https",
            "--connect-timeout",
            "5",
            "--max-time",
            "12",
            "--max-filesize",
            "2097152",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "HTTPS client unavailable")?;
    let result = (|| {
        child
            .stdin
            .take()
            .ok_or("HTTPS input unavailable")?
            .write_all(format!("url = \"{url}\"\n").as_bytes())
            .map_err(|_| "HTTPS setup failed")?;
        let mut body = Vec::new();
        child
            .stdout
            .take()
            .ok_or("HTTPS output unavailable")?
            .take(2097153)
            .read_to_end(&mut body)
            .map_err(|_| "HTTPS response failed")?;
        if body.len() > 2097152 {
            return Err("API response exceeds size limit");
        }
        if !child.wait().map_err(|_| "HTTPS request failed")?.success() {
            return Err("Network/API request failed");
        }
        let response: Response =
            serde_json::from_slice(&body).map_err(|_| "Invalid API response")?;
        if response.code != 200 {
            return Err("OneBusAway rejected request");
        }
        if !(1577836800000..4102444800000).contains(&response.current_time) {
            return Err("Invalid API clock");
        }
        Ok(response)
    })();
    if result.is_err() {
        let _ = child.kill();
        let _ = child.wait();
    }
    result
}

fn clean(value: String, max: usize) -> String {
    value
        .chars()
        .filter(|c| !c.is_control())
        .take(max)
        .collect()
}

fn fetch(
    base: &str,
    key: &str,
    mode: &Mode,
    window: u64,
    max: usize,
) -> Result<Snapshot, &'static str> {
    let mut arrivals = Vec::new();
    let mut seen = BTreeSet::new();
    let mut server_time = 0;
    let mut received = Instant::now();
    for stop in &mode.stops {
        if crate::STOP.load(Ordering::Relaxed) {
            return Err("Stopping");
        }
        let url = format!(
            "{}/arrivals-and-departures-for-stop/{}.json?key={}&minutesBefore=0&minutesAfter={window}",
            base.trim_end_matches('/'),
            encode(&stop.id),
            encode(key)
        );
        let response = request(&url)?;
        server_time = response.current_time;
        received = Instant::now();
        for item in response
            .data
            .ok_or("Missing API data")?
            .entry
            .arrivals_and_departures
        {
            let timestamp = if item.predicted_arrival_time > 0 {
                item.predicted_arrival_time
            } else {
                item.scheduled_arrival_time
            };
            if timestamp < server_time - 60000 || timestamp > server_time + window as i64 * 60000 {
                continue;
            }
            let identity = format!(
                "{}:{}:{}:{timestamp}",
                item.trip_id, stop.id, item.stop_sequence
            );
            if !seen.insert(identity) {
                continue;
            }
            arrivals.push(Arrival {
                route: clean(item.route_short_name, 16),
                destination: clean(item.trip_headsign, 100),
                direction: stop.direction.clone(),
                timestamp,
                predicted: item.predicted && item.predicted_arrival_time > 0,
            });
        }
    }
    arrivals.sort_by_key(|arrival| arrival.timestamp);
    arrivals.truncate(max);
    println!("Arrivals refreshed: {} departures", arrivals.len());
    Ok(Snapshot {
        arrivals,
        server_time,
        received,
    })
}

pub struct Board {
    pub title: String,
    pub service: String,
    pub clock: String,
    pub rows: Vec<crate::ArrivalRow>,
    pub status: String,
    pub empty: String,
    pub stale: bool,
    pub bus: bool,
}
pub struct Transit {
    settings: Settings,
    zone: Tz,
    mode: Mode,
    receiver: Receiver<Result<Snapshot, &'static str>>,
    worker: Option<thread::JoinHandle<()>>,
    snapshot: Option<Snapshot>,
    failed: bool,
    window: u64,
}
impl Transit {
    pub fn board(&mut self, elapsed: u64) -> Board {
        loop {
            match self.receiver.try_recv() {
                Ok(Ok(snapshot)) => {
                    self.snapshot = Some(snapshot);
                    self.failed = false;
                }
                Ok(Err(error)) => {
                    eprintln!("Arrivals refresh: {error}");
                    self.failed = true;
                }
                Err(TryRecvError::Disconnected) => {
                    self.failed = true;
                    break;
                }
                Err(TryRecvError::Empty) => break,
            }
        }
        let mut board = Board {
            title: self.mode.name.clone(),
            service: self.mode.service_label.clone(),
            clock: "--:--".into(),
            rows: vec![],
            status: if self.failed {
                "Unable to load arrivals; retrying"
            } else {
                "Connecting to OneBusAway..."
            }
            .into(),
            empty: "Loading live arrivals...".into(),
            stale: self.failed,
            bus: self.settings.mode == "bus",
        };
        let Some(snapshot) = &self.snapshot else {
            if self.failed {
                board.empty = "Arrivals unavailable / retrying automatically".into();
            }
            return board;
        };
        let age = snapshot.received.elapsed().as_secs();
        let now = snapshot.server_time + snapshot.received.elapsed().as_millis() as i64;
        board.clock = DateTime::<Utc>::from_timestamp_millis(now)
            .map(|date| {
                date.with_timezone(&self.zone)
                    .format("%-I:%M %p")
                    .to_string()
            })
            .unwrap_or_else(|| "--:--".into());
        board.stale = self.failed || age >= self.settings.stale_after_seconds;
        if age >= self.settings.max_cache_age_seconds {
            board.clock = "--:--".into();
            board.empty = "Cached arrivals expired / waiting for network".into();
            board.status = "Offline / retrying automatically".into();
            return board;
        }
        let visible: Vec<_> = snapshot
            .arrivals
            .iter()
            .filter(|item| item.timestamp >= now - 30000)
            .collect();
        let pages = visible.len().div_ceil(3).max(1);
        let page = (elapsed / self.settings.page_seconds) as usize % pages;
        for item in visible.iter().skip(page * 3).take(3) {
            let minutes = ((item.timestamp - now + 59999) / 60000).max(0);
            let prediction = if board.stale {
                "Cached"
            } else if item.predicted {
                "Predicted"
            } else {
                "Scheduled"
            };
            board.rows.push(crate::ArrivalRow {
                route: item.route.trim_end_matches(" Line").into(),
                destination: if item.destination.is_empty() {
                    "Transit arrival".into()
                } else {
                    item.destination.clone().into()
                },
                detail: format!(
                    "{}{}{}",
                    item.direction,
                    if item.direction.is_empty() {
                        ""
                    } else {
                        " · "
                    },
                    prediction
                )
                .into(),
                minutes: if minutes == 0 {
                    "Due".into()
                } else {
                    minutes.to_string().into()
                },
            });
        }
        board.empty = format!(
            "No {} expected in the next {} minutes",
            self.mode.plural, self.window
        );
        board.status = format!(
            "{} · updated {age}s ago · {}/{}",
            if board.stale {
                "Refresh delayed / cached"
            } else {
                "Live arrivals"
            },
            page + 1,
            pages
        );
        board
    }
}

impl Drop for Transit {
    fn drop(&mut self) {
        crate::STOP.store(true, Ordering::Relaxed);
        // The active HTTPS child has a 12-second timeout; no orphan polling.
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
