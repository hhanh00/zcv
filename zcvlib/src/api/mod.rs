pub mod init;
pub mod simple;

pub trait ProgressReporter {
    fn report(&self, p: u32);
}

impl ProgressReporter for () {
    fn report(&self, _p: u32) {}
}
