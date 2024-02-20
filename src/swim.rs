use std::thread;
use crate::{Lifecycle, SafeLifecycle};
use crate::State::INITIAL;

struct DisseminationAlgorithm {
    lifecycle: Box<dyn Lifecycle>,
}

impl DisseminationAlgorithm {
    pub fn new() -> Box<DisseminationAlgorithm> {
        return Box::new(DisseminationAlgorithm { lifecycle: Box::new(SafeLifecycle { state: INITIAL }) });
    }

}

impl Lifecycle for DisseminationAlgorithm {
    fn start(&mut self) -> Result<String, String> {
        let result = self.lifecycle.start();
        return result;
    }

    fn stop(&mut self) -> Result<String, String> {
        let result = self.lifecycle.stop();
        return result;
    }
}

#[test]
fn dissemination_algo_should_start_and_stop() {
    let mut algorithm = DisseminationAlgorithm::new();

    //when start and stop
    assert!(algorithm.start().is_ok());
    assert!(algorithm.stop().is_ok());

    //when sequence of start
    assert!(algorithm.start().is_ok());
    assert!(algorithm.start().is_err());
}