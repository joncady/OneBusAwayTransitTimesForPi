mod cards;
mod felica_trial;
mod framebuffer;
mod ipp;
mod media;
mod qr;
mod reader;
mod transit;

use slint::ComponentHandle;
use slint::platform::{
    Platform, WindowAdapter,
    software_renderer::{MinimalSoftwareWindow, RepaintBufferType},
};
use std::{
    error::Error,
    rc::Rc,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

static STOP: AtomicBool = AtomicBool::new(false);
extern "C" fn stop(_: libc::c_int) {
    STOP.store(true, Ordering::Relaxed);
}

slint::slint! {
    export struct ArrivalRow { route: string, destination: string, detail: string, minutes: string }
    component Departure inherits Rectangle {
        in property <string> route;
        in property <string> detail;
        in property <bool> bus-mode;
        in property <string> destination;
        in property <string> minutes;
        background: #0d2035;
        height: 91px;
        Rectangle { x: 12px; y: 19px; width: 56px; height: 53px; border-radius: 8px;
            background: bus-mode ? #e5a321 : route == "2" ? #00a4dc : #008b5a;
            Text { text: route; color: #ffffff; font-size: 26px; font-weight: 700;
                horizontal-alignment: center; vertical-alignment: center; }
        }
        Text { x: 82px; y: 12px; width: parent.width - 210px; height: 40px;
            text: destination; color: #ffffff; font-size: 29px; overflow: elide; vertical-alignment: center; }
        Text { x: 82px; y: 53px; width: parent.width - 210px; height: 27px;
            text: detail; color: #a9c4d6; font-size: 18px; overflow: elide; }
        Text { x: parent.width - 120px; width: 102px; height: parent.height;
            text: minutes; color: #ffd75e; font-size: 44px; font-weight: 700;
            horizontal-alignment: right; vertical-alignment: center; }
    }
    export component Prototype inherits Window {
        width: 800px; height: 480px; background: #071524;
        default-font-family: "DejaVu Sans";
        in property <string> reader-status: "Connecting to card reader...";
        in property <string> elapsed: "";
        in property <bool> card-active: false;
        in property <bool> slideshow-active: false;
        in property <image> slide-image;
        in property <string> station-name: "TRANSIT / DEVICE PREVIEW";
        in property <string> service-label: "";
        in property <string> board-status: "SAMPLE TIMES ONLY / No live arrivals";
        in property <string> empty-message: "";
        in property <bool> stale: false;
        in property <bool> bus-mode: false;
        in property <[ArrivalRow]> arrivals: [
            {route: "1", destination: "Northbound", detail: "Sample departure", minutes: "3"},
            {route: "2", destination: "Southbound", detail: "Sample departure", minutes: "7"},
            {route: "1", destination: "Northbound", detail: "Sample departure", minutes: "12"}
        ];
        Rectangle { x: 0px; y: 0px; width: 800px; height: 6px; background: #00a7e1; }
        Text { x: 36px; y: 12px; width: 550px; height: 34px;
            text: station-name; color: #7fcaee; font-size: 25px; font-weight: 700; overflow: elide; }
        Text { x: 36px; y: 46px; width: 550px; height: 23px;
            text: service-label; color: #a9c4d6; font-size: 16px; overflow: elide; }
        Text { x: 614px; y: 20px; width: 150px; height: 40px;
            text: elapsed; color: #7fcaee; font-size: 20px; horizontal-alignment: right; }
        for arrival[index] in arrivals: Departure {
            x: 36px; y: 76px + index * 97px; width: 728px;
            route: arrival.route; destination: arrival.destination;
            detail: arrival.detail; minutes: arrival.minutes; bus-mode: root.bus-mode;
        }
        if arrivals.length == 0: Text {
            x: 45px; y: 100px; width: 710px; height: 240px;
            text: empty-message; color: #ffffff; font-size: 27px;
            wrap: word-wrap; horizontal-alignment: center; vertical-alignment: center;
        }
        Rectangle { x: 36px; y: 379px; width: 728px; height: 48px;
            background: card-active ? #165e45 : #12344f; border-radius: 8px;
            Text { text: reader-status; color: #ffffff; font-size: 23px;
                horizontal-alignment: center; vertical-alignment: center; }
        }
        Text { x: 36px; y: 437px; width: 728px; height: 24px;
            text: board-status;
            color: stale ? #ffd75e : #9db1c3; font-size: 17px; horizontal-alignment: center; overflow: elide; }
        if slideshow-active: Rectangle {
            background: #000000;
            Image { x: 23px; width: 754px; height: 480px;
                source: slide-image; image-fit: contain; }
        }
    }
}

struct DisplayPlatform {
    window: Rc<MinimalSoftwareWindow>,
    start: Instant,
}
impl Platform for DisplayPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        Ok(self.window.clone())
    }
    fn duration_since_start(&self) -> Duration {
        self.start.elapsed()
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    let mut seconds = 600u64;
    let mut continuous = false;
    let mut probe = false;
    let mut reader_lights = false;
    let mut reader_buzzer = false;
    let mut buzzer_frequency = 1536u16;
    let mut buzzer_duration_ms = 256u16;
    let mut ready_light_mask = 0x40u32;
    let mut tap_light_mask = 0x80u32;
    let mut tap_light_ms = 1000u64;
    let mut enable_cards = false;
    let mut serial = "/dev/ttymxc3".to_string();
    let mut font = "/usr/lib/fonts/DejaVuSans.ttf".to_string();
    let mut cards_path = None::<String>;
    let mut list_cards = false;
    let mut name_card = None::<(String, String)>;
    let mut media_path = None::<String>;
    let mut qr_path = None::<String>;
    let mut transit_path = None::<String>;
    let mut display_path = None::<String>;
    let mut env_path = None::<String>;
    let mut check_config = false;
    let mut reader_info = None::<String>;
    let mut felica_capture_path = None::<String>;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--continuous" => continuous = true,
            "--seconds" => {
                i += 1;
                seconds = args.get(i).ok_or("missing seconds")?.parse()?;
            }
            "--reader" => {
                i += 1;
                serial = args.get(i).ok_or("missing reader path")?.clone();
            }
            "--font" => {
                i += 1;
                font = args.get(i).ok_or("missing font path")?.clone();
            }
            "--reader-lights" => reader_lights = true,
            "--reader-buzzer" => reader_buzzer = true,
            "--buzzer-frequency" | "--buzzer-duration-ms" => {
                let option = args[i].clone();
                i += 1;
                let value = args.get(i).ok_or("missing buzzer value")?;
                if option == "--buzzer-frequency" {
                    buzzer_frequency = value.parse()?;
                } else {
                    buzzer_duration_ms = value.parse()?;
                }
            }
            "--ready-light-mask" | "--tap-light-mask" | "--tap-light-ms" => {
                let option = args[i].clone();
                i += 1;
                let value = args.get(i).ok_or("missing light value")?;
                match option.as_str() {
                    "--ready-light-mask" => ready_light_mask = value.parse()?,
                    "--tap-light-mask" => tap_light_mask = value.parse()?,
                    _ => tap_light_ms = value.parse()?,
                }
            }
            "--probe-reader" => probe = true,
            "--enable-card-detection" => enable_cards = true,
            "--cards-file" => {
                i += 1;
                cards_path = Some(args.get(i).ok_or("missing card registry path")?.clone());
            }
            "--list-cards" => list_cards = true,
            "--felica-trial-file" => {
                i += 1;
                felica_capture_path = Some(
                    args.get(i)
                        .ok_or("missing FeliCa trial capture path")?
                        .clone(),
                );
            }
            "--reader-info-file" => {
                i += 1;
                reader_info = Some(
                    args.get(i)
                        .ok_or("missing reader information path")?
                        .clone(),
                );
            }
            "--transit-file" | "--display-file" | "--env-file" => {
                let option = args[i].clone();
                i += 1;
                let path = Some(args.get(i).ok_or("missing configuration path")?.clone());
                match option.as_str() {
                    "--transit-file" => transit_path = path,
                    "--display-file" => display_path = path,
                    _ => env_path = path,
                }
            }
            "--check-config" => check_config = true,
            "--qr-file" => {
                i += 1;
                qr_path = Some(args.get(i).ok_or("missing QR configuration path")?.clone());
            }
            "--actions-file" => {
                i += 1;
                media_path = Some(args.get(i).ok_or("missing actions path")?.clone());
            }
            "--name-card" => {
                i += 1;
                let old = args.get(i).ok_or("missing existing card label")?.clone();
                i += 1;
                name_card = Some((old, args.get(i).ok_or("missing new card label")?.clone()));
            }
            "--help" => {
                println!(
                    "pm3-transit-prototype [--seconds 600 | --continuous] [--probe-reader] [--enable-card-detection] [--reader PATH] [--font PATH] [--cards-file PATH] [--actions-file PATH] [--qr-file PATH] [--transit-file PATH --display-file PATH --env-file PATH] [--check-config] [--reader-lights] [--reader-buzzer] [--buzzer-frequency 1536] [--buzzer-duration-ms 256] [--ready-light-mask 64] [--tap-light-mask 128] [--tap-light-ms 1000] [--list-cards | --name-card OLD NEW]\nPrivate diagnostics: [--reader-info-file PATH] [--felica-trial-file NEW_PRIVATE_PATH]"
                );
                return Ok(());
            }
            other => return Err(format!("unknown argument: {other}").into()),
        }
        i += 1;
    }
    if seconds == 0 || seconds > 86400 {
        return Err("seconds must be between 1 and 86400".into());
    }
    if !matches!(ready_light_mask, 0 | 64 | 128 | 192)
        || !matches!(tap_light_mask, 0 | 64 | 128 | 192)
        || !(100..=5000).contains(&tap_light_ms)
    {
        return Err(
            "light masks must be 0, 64, 128 or 192; tap duration must be 100..5000 ms".into(),
        );
    }
    if !(100..=5000).contains(&buzzer_frequency) || !(20..=1000).contains(&buzzer_duration_ms) {
        return Err("buzzer frequency must be 100..5000 Hz and duration 20..1000 ms".into());
    }
    let mut registry = if check_config {
        None
    } else {
        cards_path.map(cards::Registry::open).transpose()?
    };
    if list_cards || name_card.is_some() {
        let registry = registry
            .as_mut()
            .ok_or("--cards-file is required for card commands")?;
        if let Some((old, new)) = name_card {
            registry.rename(&old, &new)?;
        }
        for card in registry.cards() {
            let method = match card.method {
                cards::IdentityMethod::Uid => "UID",
                cards::IdentityMethod::ReaderEvent => "experimental reader event",
            };
            println!("{}: {} taps ({method})", card.label, card.taps);
        }
        return Ok(());
    }
    let prepared = match (transit_path, display_path, env_path) {
        (None, None, None) if !check_config || qr_path.is_some() => None,
        (Some(transit), Some(display), Some(env)) => {
            Some(transit::Prepared::load(&transit, &display, &env)?)
        }
        _ => return Err("supply --transit-file, --display-file, and --env-file together".into()),
    };
    let qr_config = match qr_path.as_deref().map(qr::Config::load).transpose() {
        Ok(config) => config,
        Err(_) if check_config => return Err("QR configuration invalid or unreadable".into()),
        Err(_) => {
            eprintln!("QR input disabled: configuration invalid or unreadable");
            None
        }
    };
    if check_config {
        if let Some(config) = qr_config.as_ref() {
            if let Some(path) = media_path.as_deref() {
                let labels = media::configured_labels(path)
                    .map_err(|_| "Media configuration invalid or unreadable")?;
                if config
                    .album_labels()
                    .any(|label| !labels.iter().any(|l| l == label))
                {
                    return Err("QR mapping references an unconfigured media album".into());
                }
            }
            println!("QR configuration valid");
        }
        if prepared.is_some() {
            println!("Transit, display, and API key configuration valid");
        }
        return Ok(());
    }
    unsafe {
        libc::signal(libc::SIGINT, stop as *const () as libc::sighandler_t);
        libc::signal(libc::SIGTERM, stop as *const () as libc::sighandler_t);
    }
    let mut transit = prepared.map(transit::Prepared::start);
    let mut media = media_path.and_then(|path| match media::Media::load(&path) {
        Ok(media) => Some(media),
        Err(error) => {
            eprintln!("Media disabled: {error}");
            None
        }
    });
    let mut qr = qr_config.and_then(|config| {
        config.start(|label| media.as_ref().is_some_and(|m| m.has_album(label)))
    });
    let start = Instant::now();
    let mut nfc = match reader::Reader::open(&serial) {
        Ok(mut port) => {
            port.send(0x02, &[])?;
            port.send(0x04, &[])?;
            Some(port)
        }
        Err(error) => {
            eprintln!("NFC unavailable: {error}");
            None
        }
    };
    // Function enable commands alter polling configuration only. No APDU, card
    // read/write, payment, file transfer, reset, or security-key commands exist here.
    if enable_cards {
        if let Some(port) = nfc.as_mut() {
            port.enable_detection()?;
        }
    }
    let mut framebuffer = if probe {
        None
    } else {
        Some(framebuffer::Framebuffer::open("/dev/fb0")?)
    };
    let window = MinimalSoftwareWindow::new(RepaintBufferType::ReusedBuffer);
    slint::platform::set_platform(Box::new(DisplayPlatform {
        window: window.clone(),
        start,
    }))?;
    let ui = if probe {
        None
    } else {
        window
            .renderer()
            .register_font_from_path(std::path::Path::new(&font))?;
        let ui = Prototype::new()?;
        window.set_size(slint::PhysicalSize::new(800, 480));
        ui.show()?;
        Some(ui)
    };
    let mut pixels = vec![slint::Rgb8Pixel::default(); 800 * 480];
    let mut connected = false;
    let mut polling_ready = false;
    let mut cards = 0u64;
    let mut messages = 0u64;
    let mut render_count = 0u64;
    let mut render_time = Duration::ZERO;
    let mut last_card = None::<Instant>;
    let mut last_qr = None::<Instant>;
    let mut tap_light_until = None::<Instant>;
    let mut current_light = None::<u32>;
    let mut last_uid = None::<Vec<u8>>;
    let mut last_card_label = "Card".to_string();
    let mut released_sessions = 0u64;
    let mut last_ping = Instant::now();
    let mut last_second = u64::MAX;
    let mut saved_reader_info = false;
    let mut felica_capture = felica_capture_path
        .as_deref()
        .map(felica_trial::Capture::open)
        .transpose()?;
    println!(
        "Prototype started: duration={seconds}s, continuous={continuous}, framebuffer={}, card_detection={enable_cards}",
        !probe
    );
    while (continuous || start.elapsed() < Duration::from_secs(seconds))
        && !STOP.load(Ordering::Relaxed)
    {
        let mut album_triggered = false;
        if let Some(port) = nfc.as_mut() {
            for message in port.poll()? {
                messages += 1;
                match message.kind {
                    0x0f if enable_cards => {
                        connected = true;
                        polling_ready = false;
                        current_light = None;
                        tap_light_until = None;
                        if let Some(capture) = felica_capture.as_mut() {
                            capture.ready = false;
                        }
                        port.enable_detection()?;
                        println!("Reader restarted; restoring card polling configuration");
                    }
                    0xf1 if felica_capture.is_some() => {
                        if let Some(scan) =
                            felica_capture.as_mut().unwrap().event(&message.payload)?
                        {
                            last_card = Some(Instant::now());
                            last_card_label = format!("FeliCa detected / scan {scan}");
                            last_second = u64::MAX;
                        }
                    }
                    0x03 | 0x05 => {
                        if message.kind == 0x03 && !saved_reader_info {
                            if let Some(path) = reader_info.as_ref() {
                                use std::io::Write;
                                use std::os::unix::fs::OpenOptionsExt;
                                // The version response includes device identity. Keep
                                // it in an explicit private file, never stdout/logs.
                                let result = (|| -> std::io::Result<()> {
                                    let mut file = std::fs::OpenOptions::new()
                                        .write(true)
                                        .create(true)
                                        .truncate(true)
                                        .mode(0o600)
                                        .open(path)?;
                                    file.write_all(&message.payload)?;
                                    file.sync_all()
                                })();
                                match result {
                                    Ok(()) => {
                                        saved_reader_info = true;
                                        println!("Private NFC version information saved");
                                    }
                                    Err(_) => {
                                        eprintln!("Could not save private NFC version information")
                                    }
                                }
                            }
                        }
                        if !connected {
                            println!("NFC reader replied to version/status query");
                        }
                        connected = true;
                    }
                    0xe5 => {
                        // Configuration replies contain no card identifiers.
                        let accepted = message.payload.len() == 4
                            && message.payload[0] == 0
                            && message.payload[2..] == [0, 0];
                        if accepted && matches!(message.payload[1], 6 | 7) {
                            polling_ready = true;
                        }
                        println!(
                            "NFC polling-function reply: accepted={accepted}, configuration={:?}",
                            message.payload
                        );
                    }
                    0xb1 | 0xb9 | 0xbe => {
                        // Finish this reader session so removal/re-tap can
                        // produce a fresh event. This is not a card write.
                        port.send(0x32, &[])?;
                        // A trial FeliCa card also causes an empty stock fallback.
                        // Never enroll fallback messages while this diagnostic is active.
                        if message.kind == 0xbe
                            && felica_capture.as_ref().is_some_and(|capture| capture.ready)
                        {
                            continue;
                        }
                        let uid = cards::uid_from_event(message.kind, &message.payload);
                        let different = last_uid.as_deref() != uid;
                        if different
                            || last_card.is_none_or(|previous| {
                                previous.elapsed() > Duration::from_millis(800)
                            })
                        {
                            cards += 1;
                            last_card = Some(Instant::now());
                            last_uid = uid.map(<[u8]>::to_vec);
                            last_card_label = match (registry.as_mut(), uid) {
                                (Some(registry), Some(uid)) => match registry.recognize(uid) {
                                    Ok((label, _private_token, enrolled)) => {
                                        println!("Card label: {label}, newly_enrolled={enrolled}");
                                        tap_light_until = Some(
                                            Instant::now() + Duration::from_millis(tap_light_ms),
                                        );
                                        if let Some(media) = media.as_mut() {
                                            album_triggered |= media.tap(&label);
                                        }
                                        label
                                    }
                                    Err(error) => {
                                        eprintln!("Card label storage: {error}");
                                        "Card (label unavailable)".into()
                                    }
                                },
                                (Some(registry), None) if message.kind == 0xbe => {
                                    match registry
                                        .recognize_unhandled(message.kind, &message.payload)
                                    {
                                        Ok((label, _private_token, enrolled)) => {
                                            println!(
                                                "Experimental card label: {label}, newly_enrolled={enrolled}"
                                            );
                                            tap_light_until = Some(
                                                Instant::now()
                                                    + Duration::from_millis(tap_light_ms),
                                            );
                                            if let Some(media) = media.as_mut() {
                                                album_triggered |= media.tap(&label);
                                            }
                                            format!("{label} (experimental)")
                                        }
                                        Err(error) => {
                                            eprintln!("Experimental card label storage: {error}");
                                            "Card (identifier format unknown)".into()
                                        }
                                    }
                                }
                                (_, Some(_)) => "Card".into(),
                                (_, None) => "Card (identifier format unknown)".into(),
                            };
                            // Log only labels and shape metadata, never UID bytes.
                            println!(
                                "Card detected: event={cards}, message_type=0x{:02x}, payload_bytes={}, uid_bytes={}",
                                message.kind,
                                message.payload.len(),
                                uid.map_or(0, <[u8]>::len)
                            );
                            last_second = u64::MAX;
                        }
                    }
                    0xb3 | 0xbb => {
                        released_sessions += 1;
                    }
                    _ => {}
                }
            }
            if reader_lights && enable_cards {
                let mask = if !connected || !polling_ready {
                    0
                } else if tap_light_until.is_some_and(|until| Instant::now() < until) {
                    tap_light_mask
                } else {
                    ready_light_mask
                };
                if current_light != Some(mask) {
                    if let Err(error) = port.set_leds(mask) {
                        eprintln!("Reader lights disabled after UART error: {error}");
                        reader_lights = false;
                    } else {
                        current_light = Some(mask);
                    }
                }
            }
            if last_ping.elapsed() >= Duration::from_secs(5) {
                port.send(0x04, &[])?;
                last_ping = Instant::now();
            }
        }
        if let Some(label) = qr.as_mut().and_then(qr::Scanner::poll) {
            if media.as_mut().is_some_and(|m| m.scan(&label)) {
                album_triggered = true;
                last_qr = Some(Instant::now());
                tap_light_until = Some(Instant::now() + Duration::from_millis(tap_light_ms));
                last_second = u64::MAX;
                println!("QR album action triggered");
            }
        }
        if album_triggered && reader_buzzer {
            if let Some(port) = nfc.as_mut() {
                if let Err(error) = port.beep(buzzer_frequency, buzzer_duration_ms) {
                    eprintln!("Reader buzzer disabled after UART error: {error}");
                    reader_buzzer = false;
                } else {
                    println!("Reader album confirmation beep sent");
                }
            }
        }
        slint::platform::update_timers_and_animations();
        if let (Some(media), Some(ui)) = (media.as_mut(), ui.as_ref()) {
            if let Some((active, image)) = media.tick() {
                if let Some(image) = image {
                    ui.set_slide_image(image);
                }
                ui.set_slideshow_active(active);
            }
        }
        let second = start.elapsed().as_secs();
        if second != last_second {
            if let Some(ui) = ui.as_ref() {
                if let Some(transit) = transit.as_mut() {
                    let board = transit.board(second);
                    ui.set_station_name(board.title.into());
                    ui.set_service_label(board.service.into());
                    ui.set_elapsed(board.clock.into());
                    ui.set_arrivals(slint::ModelRc::new(slint::VecModel::from(board.rows)));
                    ui.set_board_status(board.status.into());
                    ui.set_empty_message(board.empty.into());
                    ui.set_stale(board.stale);
                    ui.set_bus_mode(board.bus);
                } else {
                    ui.set_elapsed(format!("{:02}:{:02}", second / 60, second % 60).into());
                }
                let active =
                    last_card.is_some_and(|previous| previous.elapsed() < Duration::from_secs(4));
                let qr_active = last_qr.is_some_and(|previous| {
                    previous.elapsed() < Duration::from_secs(4)
                        && last_card.is_none_or(|card| previous > card)
                });
                ui.set_card_active(active || qr_active);
                ui.set_reader_status(if qr_active {
                    "QR album detected".into()
                } else if active {
                    format!("{last_card_label} detected / tap {cards}").into()
                } else if felica_capture.as_ref().is_some_and(|capture| capture.ready) {
                    "FeliCa trial ready / tap a Japanese card".into()
                } else if connected && enable_cards && polling_ready {
                    "Reader ready / tap a transit card".into()
                } else if connected && enable_cards {
                    "Reader connected / waiting for polling setup".into()
                } else if connected {
                    "Reader connected / card detection disabled".into()
                } else if nfc.is_none() {
                    "Reader unavailable / display test only".into()
                } else {
                    "Waiting for NFC reader response...".into()
                });
            }
            last_second = second;
        }
        if let Some(fb) = framebuffer.as_mut() {
            let mut drawing_error = None;
            window.draw_if_needed(|renderer| {
                let before = Instant::now();
                renderer.render(&mut pixels, fb.width);
                if let Err(error) = fb.draw(&pixels) {
                    drawing_error = Some(error);
                }
                render_count += 1;
                render_time += before.elapsed();
            });
            if let Some(error) = drawing_error {
                return Err(error.into());
            }
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    println!(
        "Prototype stopped: elapsed={:.1}s, nfc_messages={messages}, cards={cards}, released_sessions={released_sessions}, rejected_frames={}, draws={render_count}, total_render_ms={:.1}",
        start.elapsed().as_secs_f64(),
        nfc.as_ref().map_or(0, |port| port.decoder.rejected),
        render_time.as_secs_f64() * 1000.0
    );
    // RAII restores original pixels/UART settings and disables the polling
    // functions enabled by this process on graceful exit (including errors).
    Ok(())
}
