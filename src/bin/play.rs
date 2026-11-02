//! Local server for the play page (play/index.html). The same page also runs in
//! the browser on its own (see web/).
//!
//! Usage: cargo run --release --bin play [--pytorch]   then open http://localhost:8766
//! (set PORT to use another port)
//!
//! LooNN uses the network exported to web/model.bin (run
//! neural_network/export_weights.py after training), as the browser and terminal
//! versions do. --pytorch uses the PyTorch checkpoint directly instead, which needs
//! the `python` feature, and starts untrained if there is no checkpoint.

use std::fs;
use std::io::{ self, BufRead, BufReader, Read, Write };
use std::net::{ TcpListener, TcpStream };
use std::panic::{ self, AssertUnwindSafe };
use std::sync::mpsc::{ channel, Sender };
use std::sync::{ Arc, Mutex };
use std::thread;
use rand::random;
use loonn::describe::esc;
use loonn::core::Evaluator;
use loonn::evaluators::{ NativeLooNN, Network };
use loonn::session::{ catalog, Session };

const PORT: u16 = 8766;
/// Read on every request so the page can be edited without rebuilding
const PAGE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/play/index.html");
const FAVICON: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/play/favicon.png");
const WEIGHTS: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/web/model.bin"));

enum Command {
    Answer(u64, Vec<String>),
    Restart,
}

fn panic_text(e: &Box<dyn std::any::Any + Send>) -> String {
    e.downcast_ref::<&str>().map(|s| s.to_string())
        .or_else(|| e.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown error".into())
}

/// Runs the game on its own thread, since the network lives there, publishing
/// the state JSON after every command
fn run_game<E: Evaluator>(state: Arc<Mutex<String>>, load: impl FnOnce() -> (E, E) + Send + 'static) -> Sender<Command> {
    let (sender, commands) = channel();
    thread::spawn(move || {
        let publish = |json: String| *state.lock().unwrap() = json;
        publish("{\"seq\":0,\"prompt\":null,\"view\":{\"status\":\"Loading LooNN's network…\",\"player\":null,\"predicted\":null,\"log\":[]}}".into());
        let (chooser, predictor) = load();
        let mut session = Session::new(chooser, predictor, random());
        publish(session.state_json());
        // Error prompts get their own seqs, past any the session will reach soon
        let mut error_seq = 1_000_000;
        for command in commands {
            let result = panic::catch_unwind(AssertUnwindSafe(|| {
                match command {
                    Command::Answer(seq, answer) => { session.answer(seq, answer); }
                    Command::Restart => session.restart(random()),
                }
                session.state_json()
            }));
            publish(result.unwrap_or_else(|e| {
                error_seq += 1;
                let title = esc(&format!("Something went wrong: {}", panic_text(&e)));
                format!("{{\"seq\":{error_seq},\"prompt\":{{\"kind\":\"error\",\"title\":{title},\"error\":null}},\"view\":null}}")
            }));
        }
    });
    sender
}

fn handle(stream: TcpStream, state: &Mutex<String>, commands: &Mutex<Sender<Command>>, catalog: &str) -> io::Result<()> {
    let mut reader = BufReader::new(&stream);
    let mut request = String::new();
    reader.read_line(&mut request)?;
    let mut parts = request.split_whitespace();
    let (method, path) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
    let mut length = 0;
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header)? == 0 || header.trim().is_empty() { break; }
        if let Some((name, value)) = header.split_once(':') && name.eq_ignore_ascii_case("content-length") {
            length = value.trim().parse().unwrap_or(0);
        }
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body)?;
    let body = String::from_utf8_lossy(&body);

    // An image isn't text, so it skips the responses below
    if method == "GET" && path == "/favicon.png" && let Ok(image) = fs::read(FAVICON) {
        write!(
            &stream,
            "HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
            image.len(),
        )?;
        return (&stream).write_all(&image);
    }

    const JSON: &str = "application/json";
    let send = |command| { let _ = commands.lock().unwrap().send(command); };
    let (status, kind, payload) = match (method, path) {
        ("GET", "/") => match fs::read_to_string(PAGE) {
            Ok(page) => ("200 OK", "text/html; charset=utf-8", page),
            Err(e) => ("500 Internal Server Error", "text/plain", format!("Could not read {PAGE}: {e}")),
        },
        ("GET", "/api/catalog") => ("200 OK", JSON, catalog.to_string()),
        ("GET", "/api/state") => ("200 OK", JSON, state.lock().unwrap().clone()),
        // The body is the prompt's seq, then one answer value per line
        ("POST", "/api/answer") => {
            let mut lines = body.lines();
            match lines.next().and_then(|s| s.trim().parse().ok()) {
                Some(seq) => {
                    send(Command::Answer(seq, lines.map(str::trim).filter(|l| !l.is_empty()).map(String::from).collect()));
                    ("200 OK", JSON, "{}".into())
                }
                None => ("400 Bad Request", JSON, "{\"error\":\"The body must start with the prompt's seq\"}".into()),
            }
        }
        ("POST", "/api/restart") => {
            send(Command::Restart);
            ("200 OK", JSON, "{}".into())
        }
        _ => ("404 Not Found", "text/plain", "Not found".into()),
    };
    write!(
        &stream,
        "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{payload}",
        payload.len(),
    )
}

fn start_game(state: Arc<Mutex<String>>) -> Sender<Command> {
    if std::env::args().any(|a| a == "--pytorch") {
        #[cfg(feature = "python")]
        {
            use loonn::evaluators::LooNN;
            return run_game(state, || (
                LooNN::new().expect("Failed to construct LooNN"),
                // Predicts LooNN's final score without the bonus it adds per bird when choosing
                LooNN::with_bird_bonus(0.0).expect("Failed to construct LooNN"),
            ));
        }
        #[cfg(not(feature = "python"))]
        {
            eprintln!("This build doesn't include PyTorch support; run from source to use --pytorch.");
            std::process::exit(1);
        }
    }
    run_game(state, || {
        let network = Arc::new(Network::from_bytes(WEIGHTS).expect("Bad weights file"));
        (NativeLooNN::new(network.clone()), NativeLooNN::predictor(network))
    })
}

fn main() {
    let state = Arc::new(Mutex::new(String::new()));
    let commands = Arc::new(Mutex::new(start_game(state.clone())));
    let catalog = Arc::new(catalog());
    let port = std::env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(PORT);
    let listener = TcpListener::bind(("127.0.0.1", port)).expect("Could not open the port");
    println!("Play with LooNN at http://localhost:{port}  (Ctrl+C to stop)");
    for stream in listener.incoming().flatten() {
        let (state, commands, catalog) = (state.clone(), commands.clone(), catalog.clone());
        thread::spawn(move || { let _ = handle(stream, &state, &commands, &catalog); });
    }
}
