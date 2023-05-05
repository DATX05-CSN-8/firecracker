use std::os::fd::AsRawFd;

use event_manager::{MutEventSubscriber, EventOps, Events, EventSet};
use logger::{error};

use crate::virtio::VirtioDevice;

use super::Tpm;

impl Tpm {
    fn register_activate_event(&self, ops: &mut EventOps) {
        if let Err(err) = ops.add(Events::new(&self.activate_evt, EventSet::IN)) {
            error!("Failed to register activate event: {}", err);
        }
    }

    fn register_runtime_events(&self, ops: &mut EventOps) {
        if let Err(err) = ops.add(Events::new(&self.queue_evts[0], EventSet::IN)) {
            error!("Failed to register queue event: {}", err);
        }
    }

    fn process_activate_event(&self, ops: &mut EventOps) {
        if let Err(err) = self.activate_evt.read() {
            error!("Failed to consume tpm activate event: {:?}", err);
        }
        self.register_runtime_events(ops);
        if let Err(err) = ops.remove(Events::new(&self.activate_evt, EventSet::IN)) {
            error!("Failed to un-register activate event: {:?}", err);
        }
    }
}

impl MutEventSubscriber for Tpm {
    fn process(&mut self, event: Events, ops: &mut EventOps) {
        let source = event.fd();
        let event_set = event.event_set();
        let supported_events = EventSet::IN;

        if !supported_events.contains(event_set) {
            error!("Received unknown event: {:?} from source {:?}", event_set, source);
            return;
        }
        if !self.is_activated() {
            error!("TPM: The device is not yet activated. Spurious event received: {:?}", source);
            return;
        }
        let activate_fd = self.activate_evt.as_raw_fd();
        let virtq_fd = self.queue_evts[0].as_raw_fd();
        match source {
            _ if source == activate_fd => self
                .process_activate_event(ops),
            _ if source == virtq_fd => self
                .process_virtio_queues(),
            _ => {
                error!("TPM: Spurious event received: {:?}", source);
            }
       }
    }

    fn init(&mut self, ops: &mut EventOps) {
        if self.is_activated() {
            self.register_runtime_events(ops);
        } else {
            self.register_activate_event(ops);
        }
    }
}