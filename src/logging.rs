pub use log::{debug, error, info, trace, warn};

pub fn init() {
    env_logger::init();
}
