use std::sync::Mutex;
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
        eprintln!("{} {}", level_to_char(record.level()), record.args());
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

#[derive(Default)]
pub struct StringLogger(String);

impl StringLogger {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl MutLog for StringLogger {
    fn enabled(&mut self, _: &Metadata) -> bool { true }

    fn log(&mut self, record: &Record) {
        self.0.push_str(&format!("{}  {}\n", level_to_char(record.level()), record.args()));
    }

    fn flush(&mut self) {}
}

trait AnyMutLog : MutLog {
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

#[cfg(test)]
mod tests {
    use super::*;
    use log::info;

    #[test]
    fn test() {
        let logger = init_changeable_logger(StringLogger::new());
        log::set_max_level(log::LevelFilter::Info);

        let (lg, res) = logger.with(StringLogger::new(), || {
            info!("hello");
            let (lg2, _) = logger.with(StringLogger::new(), || {
                info!("inner");
            });
            info!("bye");
            lg2.into_string()
        });
        assert_eq!(lg.into_string(), "I  hello\nI  bye\n");
        assert_eq!(res, "I  inner\n");
    }
}
