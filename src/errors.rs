use cherrydoor_command::Heartbeat;

use crate::access::AccessStatus;

#[derive(Clone)]
pub struct ErrorReporter;

impl ErrorReporter {
    pub fn new() -> Self {
        Self
    }

    pub fn report_error(&self, e :&Heartbeat) {
        // Write an error to the database
    }

    pub fn report_access(&self, code :String, status :AccessStatus) {

    }

    pub fn clear_error(&self) {
        // Mark the last error as "resolved"
    }
}