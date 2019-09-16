use std::sync::Mutex;
use std::cell::RefCell;
use std::any::Any;
use log::{Metadata, Level, Record, Log};

fn level_to_char(level: Level) -> char {
    match level {
        Level::Trace => 'T',
        Level::Debug => 'D',
        Level::Info => 'I',
        Level::Warn => 'W',
        Level::Error => 'E',
    }
}

pub struct SimpleLogger;

impl Log for SimpleLogger {
    fn enabled(&self, _: &Metadata) -> bool { true }

    fn log(&self, record: &Record) {
        eprintln!("{}  {}", level_to_char(record.level()), record.args());
    }

    fn flush(&self) {}
}

pub trait MutLog: Send {
    fn enabled(&mut self, metadata: &Metadata) -> bool;
    fn log(&mut self, record: &Record);
    fn flush(&mut self);
}

impl<T: Log> MutLog for T {
    fn enabled(&mut self, metadata: &Metadata) -> bool {
        <Self as Log>::enabled(self, metadata)
    }
    fn log(&mut self, record: &Record) {
        <Self as Log>::log(self, record);
    }
    fn flush(&mut self) {
        <Self as Log>::flush(self);
    }
}

pub struct WriteLogger<W: std::io::Write>(W);

impl<W: std::io::Write> WriteLogger<W> {
    pub fn new(w: W) -> Self {
        WriteLogger(w)
    }

    pub fn into_inner(self) -> W {
        self.0
    }
}

impl<W: std::io::Write + Send> MutLog for WriteLogger<W> {
    fn enabled(&mut self, _: &Metadata) -> bool { true }

    fn log(&mut self, record: &Record) {
        writeln!(self.0, "{}  {}  {}",
            level_to_char(record.level()),
            chrono::offset::Utc::now().format("%m-%d %H:%M:%S%.3f"),
            record.args()).unwrap();
    }

    fn flush(&mut self) {
        self.0.flush().unwrap();
    }
}

pub trait AnyMutLog : MutLog {
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
}

impl<T: MutLog + 'static> AnyMutLog for T {
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

pub struct ChangeableLogger(Mutex<Box<dyn AnyMutLog>>);

impl ChangeableLogger {
    pub fn with<L: MutLog + 'static, R>(&self, logger: L, f: impl FnOnce() -> R) -> (L, R) {
        let old = std::mem::replace(&mut *self.0.lock().unwrap(), Box::new(logger));
        let result = f();
        let l = std::mem::replace(&mut *self.0.lock().unwrap(), old);
        (*l.into_any().downcast().unwrap(), result)
    }

    pub fn capture_log<R>(&self, f: impl FnOnce() -> R) -> (String, R) {
        let lg = WriteLogger::new(Vec::<u8>::new());
        let (lg, result) = self.with(lg, f);
        let buf = lg.into_inner();
        (String::from_utf8(buf).unwrap(), result)
    }
}

impl Log for ChangeableLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        self.0.lock().unwrap().enabled(metadata)
    }

    fn log(&self, record: &Record) {
        self.0.lock().unwrap().log(record);
    }

    fn flush(&self) {
        self.0.lock().unwrap().flush();
    }
}

pub fn init_changeable_logger<L: MutLog + 'static>(logger: L) -> &'static ChangeableLogger {
    let c = ChangeableLogger(Mutex::new(Box::new(logger)));
    let c = Box::new(c);
    let c = Box::leak(c);
    log::set_logger(c).unwrap();
    c
}

pub struct ThreadLocalLogger;

impl ThreadLocalLogger {
    thread_local!(static LOGGER: RefCell<Box<dyn AnyMutLog>> = RefCell::new(Box::new(SimpleLogger)));

    pub fn replace(new_logger: Box<dyn AnyMutLog>) -> Box<dyn AnyMutLog> {
        ThreadLocalLogger::LOGGER.with(|logger| {
            std::mem::replace(&mut *logger.borrow_mut(), new_logger)
        })
    }

    pub fn with<L: MutLog + 'static, R>(logger: L, f: impl FnOnce() -> R) -> (L, R) {
        let old = Self::replace(Box::new(logger));
        let result = f();
        let l = Self::replace(old);
        (*l.into_any().downcast().unwrap(), result)
    }
}

impl Log for ThreadLocalLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        ThreadLocalLogger::LOGGER.with(|logger| {
            logger.borrow_mut().enabled(metadata)
        })
    }

    fn log(&self, record: &Record) {
        ThreadLocalLogger::LOGGER.with(|logger| {
            logger.borrow_mut().log(record);
        });
    }

    fn flush(&self) {
        ThreadLocalLogger::LOGGER.with(|logger| {
            logger.borrow_mut().flush();
        });
    }
}