use crate::config::Config;
use std::{
    fs::{remove_file, File},
    io::Write,
    path::PathBuf,
};

#[derive(Debug)]
pub struct Logger {
    buffer: Vec<String>,
    file: File,
}

impl Logger {
    pub fn new(cfg: &Config, path: &str) -> Result<Logger, String> {
        let path = PathBuf::from(path);
        if path.exists() {
            let _ = remove_file(&path);
        }
        Ok(Logger {
            buffer: Vec::with_capacity(cfg.log_buffer),
            file: File::create(&path).unwrap(),
        })
    }
    pub fn log(&mut self, mut str: String, cfg: &Config) {
        str.push_str("\n");
        self.buffer.push(str);
        if self.buffer.len() >= cfg.log_buffer {
            for msg in self.buffer.iter() {
                let _ = self.file.write(msg.as_bytes());
            }
            self.buffer.clear();
        }
    }

    pub fn flush(&mut self) {
        for msg in self.buffer.iter() {
            let _ = self.file.write(msg.as_bytes());
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
    fn test_logger() {
        let mut cfg = Config::default();
        cfg.log_buffer = 5;
        let mut logger = Logger::new(&cfg, "test_logger.log").unwrap();
        for i in 0..5 {
            logger.log(format!("Line: {}", i), &cfg);
        }
        logger.log("Line: 6".to_string(), &cfg);
        std::mem::drop(logger);
        for (i, line) in std::fs::read_to_string("test_logger.log")
            .unwrap()
            .lines()
            .enumerate()
        {
            assert!(i < 6);
            assert_eq!(line, format!("Line: {i}").as_str());
        }
        let _ = remove_file("test_logger.log");
    }

    #[test]
    fn test_flush() {
        let cfg = Config::default();
        let mut logger = Logger::new(&cfg, "test_flush.log").unwrap();
        logger.log("Line: 1".to_string(), &cfg);
        logger.flush();
        std::mem::drop(logger);
        let content = std::fs::read_to_string("test_flush.log").unwrap();
        assert_eq!("Line: 1\n".to_string(), content);
        let _ = remove_file("test_flush.log");
    }
}