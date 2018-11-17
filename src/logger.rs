use crate::writer::{Writer};
use crate::event::Event;
use crate::filter::{Filters};
use crate::config::Config;
use crate::appender::{FileAppender, ConsoleAppender};

use std::path::PathBuf;
use std::thread;
use std::sync::{mpsc, Mutex};
use std::time::Duration;

pub enum EventType {
    Log(Event),
}

lazy_static! {
    pub static ref LOGGER_OBJ : Logger = Logger::init();
}

#[allow(unused)]
pub struct Logger {
    wr_th: thread::JoinHandle<()>,
    fmt_th: thread::JoinHandle<()>,
    poster: Mutex<mpsc::Sender<EventType>>,
}

impl Logger {
    pub fn load_config<T: Into<Option<PathBuf>>>( file_path: T) {
        Config::create_instance(file_path.into());
    }

    pub fn init() -> Self {
        let mut w = Writer::new();
        let poster = w.get_poster();

        if Config::instance().console_log() {
            w.add_appender(Box::new(ConsoleAppender{}));
        }

        if Config::instance().file_log() {
            w.add_appender(Box::new(FileAppender::new(Config::instance().file_temp_buf(), Config::instance().file_log_dir())));
        }

        // init writer thread
        let wr_th = thread::spawn(move ||{
            let mut w = w;
            loop {
                w.fetch_logs();

                // release cpu every frame 
                // todo : to be more intelligent
                thread::sleep(Duration::from_micros(1u64));
            }
        });

        let (tx, rx) = mpsc::channel();
        let fmt_th = thread::spawn(move || {
            loop {
                for msg in rx.iter() {
                    // todo : handle msg 
                    match msg {
                        EventType::Log(log) => {
                            poster.insert_log(log.format_by_default());
                        }
                    }
                }

                // release cpu every frame 
                // todo : to be more intelligent
                thread::sleep(Duration::from_micros(1u64));
            }
        });

        Self {
            wr_th,
            fmt_th,
            poster: Mutex::new(tx),
        }
    }    

    pub fn get_poster(&self) -> mpsc::Sender<EventType> {
        self.poster.lock().unwrap().clone()
    }
}

impl Drop for Logger {
    fn drop(&mut self) {
        //self.fmt_th.join();
        //self.wr_th.join();
    }
}

pub trait SendEvent {
    fn send_event(&self, e: Event);
}

#[allow(unused)]
impl SendEvent for mpsc::Sender<EventType> {
    fn send_event(&self, e: Event) {
        self.send(EventType::Log(e));
    }
}

pub struct ThreadLocalLogger {
    filter: Filters,
    sender: mpsc::Sender<EventType>,
    thread_tag: String,
} 

impl ThreadLocalLogger {
    pub fn new() -> Self {
        let tid: u64 = unsafe { std::mem::transmute(thread::current().id()) };
        let thread_tag = match thread::current().name() {
            Some(ref name) => format!("{}:[{}]", name.to_string(), tid),
            None => format!("thread:[{}]", tid),
        };

        Self {
            filter: Filters::generate_by_config(),
            sender: LOGGER_OBJ.get_poster(),
            thread_tag, 
        }
    }

    pub fn get_thread_tag(&self) -> String {
        self.thread_tag.clone()
    }

    pub fn get_filter(&self) -> &Filters {
        &self.filter
    }

    pub fn get_sender(&self) -> &mpsc::Sender<EventType> {
        &self.sender
    }
}

