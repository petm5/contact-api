use std::{
    thread,
    net::{TcpListener, TcpStream},
    sync::{Arc, atomic::{AtomicBool, Ordering}},
    error::Error,
    time::Duration,
};

mod threadpool;
mod http;
mod mail;
mod api;

use threadpool::ThreadPool;
use api::Config;

use signal_hook::{consts::{SIGINT, SIGQUIT, SIGTERM}, iterator::Signals};

const LISTEN_ADDR: &str = "0.0.0.0:8080";

fn main() -> Result<(), Box<dyn Error>> {

    let config = Arc::new(Config::from_env());

    let pool = ThreadPool::new(16);

    let stop_flag: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    let stop_me = stop_flag.clone();

    let mut signals = Signals::new([SIGINT, SIGQUIT, SIGTERM]).unwrap();

    thread::spawn(move || {
        signals.wait();

        stop_flag.store(true, Ordering::Relaxed);
        let _ = TcpStream::connect(LISTEN_ADDR);
    });

    let listener = TcpListener::bind(LISTEN_ADDR).unwrap();

    for stream in listener.incoming() {
        if stop_me.load(Ordering::Relaxed) {
            break
        }
        let mut stream = stream.unwrap();
        stream.set_write_timeout(Some(Duration::from_secs(5))).unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();

        let config = config.clone();

        pool.execute(move || {
            if let Some(request) = http::server::read_request(&mut stream) {
                let response = api::http_handler(config, request);
                let result = http::server::respond(&mut stream, response);
                if !result.is_ok() {
                    println!("HTTP respond failed");
                }
            }
        });
    }

    Ok(())

} 
