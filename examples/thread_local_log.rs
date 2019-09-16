use log::info;
use rbc::logger::{WriteLogger, ThreadLocalLogger};

fn main() {
    log::set_logger(&ThreadLocalLogger).unwrap();
    log::set_max_level(log::LevelFilter::Info);

    info!("hi");
    let logger = std::fs::File::create("logs/example_main_thread.info").unwrap();
    let logger = WriteLogger::new(logger);
    ThreadLocalLogger::with(logger, || {
        info!("in main thread");
        let t = std::thread::spawn(|| {
            let logger = std::fs::File::create("logs/example_child_thread.info").unwrap();
            let logger = Box::new(WriteLogger::new(logger));
            ThreadLocalLogger::replace(logger);
            info!("in child thread");
        });
        info!("still in main thread");
        t.join().unwrap();
        info!("in main thread after join");
    });
}
