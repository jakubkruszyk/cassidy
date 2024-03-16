use crate::config::Config;
use std::{
    fs::{remove_file, File},
    io::Write,
    path::PathBuf,
};

#[derive(Debug)]
pub struct Logger {
    buffer: Vec<String>,
    file: Option<File>,
}

impl Logger {
    pub fn new(enable: bool, cfg: &Config, path: &str) -> Result<Logger, String> {
        if enable {
            let path = PathBuf::from(path);
            if path.exists() {
                let _ = remove_file(&path);
            }
            Ok(Logger {
                buffer: Vec::with_capacity(cfg.log_buffer),
                file: Some(File::create(&path).unwrap()),
            })
        } else {
            Ok(Logger {
                buffer: Vec::new(),
                file: None,
            })
        }
    }
    pub fn log(&mut self, str: String, time: f64, cfg: &Config) {
        if let Some(file) = &mut self.file {
            let msg = format!("{:.3}\t{}\n", time, str);
            self.buffer.push(msg);
            if self.buffer.len() >= cfg.log_buffer {
                for msg in self.buffer.iter() {
                    let _ = file.write(msg.as_bytes());
                }
                self.buffer.clear();
            }
        }
    }

    pub fn flush(&mut self) {
        if let Some(file) = &mut self.file {
            for msg in self.buffer.iter() {
                let _ = file.write(msg.as_bytes());
            }
        }
        self.buffer.clear();
    }
}

#[cfg(test)]
mod test {
    use std::fs::remove_file;

    use super::Logger;
    use crate::config::Config;

    #[test]
    fn logger() {
        let mut cfg = Config::default();
        cfg.log_buffer = 5;
        let mut logger = Logger::new(true, &cfg, "test_logger.log").unwrap();
        for i in 0..5 {
            logger.log(format!("Line: {}", i), 0.0, &cfg);
        }
        logger.log("Line: 6".to_string(), 0.0, &cfg);
        std::mem::drop(logger);
        for (i, line) in std::fs::read_to_string("test_logger.log")
            .unwrap()
            .lines()
            .enumerate()
        {
            assert!(i < 6);
            assert_eq!(line, format!("0.000\tLine: {i}").as_str());
        }
        let _ = remove_file("test_logger.log");
    }

    #[test]
    fn flush() {
        let cfg = Config::default();
        let mut logger = Logger::new(true, &cfg, "test_flush.log").unwrap();
        logger.log("Line: 1".to_string(), 0.0, &cfg);
        logger.flush();
        std::mem::drop(logger);
        let content = std::fs::read_to_string("test_flush.log").unwrap();
        assert_eq!("0.000\tLine: 1\n".to_string(), content);
        let _ = remove_file("test_flush.log");
    }
}
