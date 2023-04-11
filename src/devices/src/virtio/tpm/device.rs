// Copyright 2018 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::cmp::min;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;

use logger::error;
use logger::warn;
use virtio_gen::virtio_blk::VIRTIO_F_VERSION_1;
use virtio_gen::virtio_ring::VIRTIO_RING_F_EVENT_IDX;
use vm_memory::Bytes;
use super::TPM_DEV_ID;
use super::TpmError as Error;
use utils::eventfd::EventFd;
use vm_memory::GuestMemoryMmap;

use crate::virtio::ActivateResult;
use crate::virtio::DescriptorChain;
use crate::virtio::DeviceState;
use crate::virtio::IrqTrigger;
use crate::virtio::Queue;
use crate::virtio::TYPE_TPM;
use crate::virtio::VirtioDevice;
use crate::virtio::tpm::TPM_BUFSIZE;


// A single queue of size 2. The guest kernel driver will enqueue a single
// descriptor chain containing one command buffer and one response buffer at a
// time.
const QUEUE_SIZE: u16 = 2;
const QUEUE_SIZES: &[u16] = &[QUEUE_SIZE];

pub trait TpmBackend: Send {
    fn execute_command(&mut self, command: &[u8]) -> Vec<u8>;
}
/*
struct Worker {
    interrupt: Interrupt,
    queue: Queue,
    mem: GuestMemory,
    queue_evt: Event,
    backend: Box<dyn TpmBackend>,
}



impl Worker {
    fn perform_work(&mut self, desc: DescriptorChain) -> Result<u32> {
        let mut reader = Reader::new(self.mem.clone(), desc.clone()).map_err(Error::Descriptor)?;
        let mut writer = Writer::new(self.mem.clone(), desc).map_err(Error::Descriptor)?;

        let available_bytes = reader.available_bytes();
        if available_bytes > TPM_BUFSIZE {
            return Err(Error::CommandTooLong {
                size: available_bytes,
            });
        }

        let mut command = vec![0u8; available_bytes];
        reader.read_exact(&mut command).map_err(Error::Read)?;

        let response = self.backend.execute_command(&command);

        if response.len() > TPM_BUFSIZE {
            return Err(Error::ResponseTooLong {
                size: response.len(),
            });
        }

        let writer_len = writer.available_bytes();
        if response.len() > writer_len {
            return Err(Error::BufferTooSmall {
                size: writer_len,
                required: response.len(),
            });
        }

        writer.write_all(response).map_err(Error::Write)?;

        Ok(writer.bytes_written() as u32)
    }

    fn process_queue(&mut self) -> NeedsInterrupt {
        let mut needs_interrupt = NeedsInterrupt::No;
        while let Some(avail_desc) = self.queue.pop(&self.mem) {
            let index = avail_desc.index;

            let len = match self.perform_work(avail_desc) {
                Ok(len) => len,
                Err(err) => {
                    error!("{}", err);
                    0
                }
            };

            self.queue.add_used(&self.mem, index, len);
            needs_interrupt = NeedsInterrupt::Yes;
        }

        needs_interrupt
    }

    fn run(mut self, kill_evt: Event) {
        #[derive(EventToken, Debug)]
        enum Token {
            // A request is ready on the queue.
            QueueAvailable,
            // Check if any interrupts need to be re-asserted.
            InterruptResample,
            // The parent thread requested an exit.
            Kill,
        }

        let wait_ctx = match WaitContext::build_with(&[
            (&self.queue_evt, Token::QueueAvailable),
            (&kill_evt, Token::Kill),
        ])
        .and_then(|wc| {
            if let Some(resample_evt) = self.interrupt.get_resample_evt() {
                wc.add(resample_evt, Token::InterruptResample)?;
            }
            Ok(wc)
        }) {
            Ok(pc) => pc,
            Err(e) => {
                error!("vtpm failed creating WaitContext: {}", e);
                return;
            }
        };

        'wait: loop {
            let events = match wait_ctx.wait() {
                Ok(v) => v,
                Err(e) => {
                    error!("vtpm failed waiting for events: {}", e);
                    break;
                }
            };

            let mut needs_interrupt = NeedsInterrupt::No;
            for event in events.iter().filter(|e| e.is_readable) {
                match event.token {
                    Token::QueueAvailable => {
                        if let Err(e) = self.queue_evt.wait() {
                            error!("vtpm failed reading queue Event: {}", e);
                            break 'wait;
                        }
                        needs_interrupt |= self.process_queue();
                    }
                    Token::InterruptResample => {
                        self.interrupt.interrupt_resample();
                    }
                    Token::Kill => break 'wait,
                }
            }
            if needs_interrupt == NeedsInterrupt::Yes {
                self.queue.trigger_interrupt(&self.mem, &self.interrupt);
            }
        }
    }
}

#[derive(PartialEq, Eq)]
enum NeedsInterrupt {
    Yes,
    No,
}

impl BitOrAssign for NeedsInterrupt {
    fn bitor_assign(&mut self, rhs: NeedsInterrupt) {
        if rhs == NeedsInterrupt::Yes {
            *self = NeedsInterrupt::Yes;
        }
    }
}

*/
/// Virtio vTPM device.
pub struct Tpm {
    backend: Box<dyn TpmBackend>,

    // Virtio fields
    pub(crate) avail_features: u64,
    pub(crate) acked_features: u64,

    pub(crate) activate_evt: EventFd,

    // Transport related fields.
    pub(crate) queues: Vec<Queue>,
    pub(crate) queue_evts: [EventFd; 1],
    pub(crate) device_state: DeviceState,
    pub(crate) irq_trigger: IrqTrigger,
}

fn write_to_descriptor_chain(mem: &GuestMemoryMmap, data: &[u8], head: DescriptorChain) -> Result<()>{
    let mut chunk = data;
    let mut next_descriptor = Some(head);
    while let Some(descriptor) = &next_descriptor {
        if !descriptor.is_write_only() {
            // skip read-only descriptors
            next_descriptor = descriptor.next_descriptor();
            continue;
        }
        let len = min(chunk.len(), descriptor.len as usize);
        match mem.write_slice(&chunk[..len], descriptor.addr) {
            Ok(()) => {
                chunk = &chunk[len..];
            }
            Err(err) => {
                error!("Failed to write slice: {:?}", err);
                return Err(Error::GuestMemory(err));
            }
        }
        if chunk.is_empty() {
            return Ok(());
        }
        next_descriptor = descriptor.next_descriptor();
    }
    Err(Error::ResponseTooLong { size: chunk.len() })
}

fn read_from_descriptor_chain(mem: &GuestMemoryMmap, head: DescriptorChain) -> Result<Vec<u8>> {
    let mut read_bytes = 0 as usize;
    let mut buf = vec![0u8; TPM_BUFSIZE];
    let mut next_descriptor = Some(head);
    while let Some(descriptor) = &next_descriptor {
        if descriptor.is_write_only() {
            // skip write-only descriptors
            next_descriptor = descriptor.next_descriptor();
            continue;
        }
        let len = min(buf.len(), descriptor.len as usize);
        if len < descriptor.len as usize {
            // descriptor contains too much data
            error!("Descriptor contains too much data for the TPM buffer");
            return Err(Error::CommandTooLong { size: read_bytes + descriptor.len as usize });
        }
        let chunk = &mut buf[read_bytes..len];
        match mem.read_slice(chunk, descriptor.addr) {
            Ok(()) => {
                read_bytes += len;
            }
            Err(err) => {
                error!("Failed to read slice: {:?}", err);
                return Err(Error::GuestMemory(err));
            }
        }
        next_descriptor = descriptor.next_descriptor();
    }
    buf.truncate(read_bytes);
    Ok(buf)

}

impl Tpm {
    pub fn new(backend: Box<dyn TpmBackend>) -> Result<Tpm> {
        let avail_features: u64 = (1u64 << VIRTIO_F_VERSION_1) | (1u64 << VIRTIO_RING_F_EVENT_IDX);

        let queue_evts = [EventFd::new(libc::EFD_NONBLOCK).map_err(Error::EventFd)?];

        let queues = QUEUE_SIZES.iter().map(|&s| Queue::new(s)).collect();
        Ok(Tpm {
            backend: backend,
            avail_features: avail_features,
            acked_features: 0u64,
            queues,
            queue_evts,
            device_state: DeviceState::Inactive,
            irq_trigger: IrqTrigger::new().map_err(Error::IrqTrigger)?,
            activate_evt: EventFd::new(libc::EFD_NONBLOCK).map_err(Error::EventFd)?,
        })
    }

    pub fn process_virtio_queues(&mut self) {
        self.process_queue(0);
    }

    pub fn id(&self) -> &str {
        TPM_DEV_ID
    }

    fn process_queue(&mut self, queue_index: usize) {
        let mem = self.device_state.mem().unwrap();

        let queue = &mut self.queues[queue_index];
        
        while let Some(head) = queue.pop_or_enable_notification(mem) {
            
            
            if !head.has_next() {
                error!("Descriptorchain only contained 1 item, should be 2 as per the driver.");
                continue;
            }
            let head_index = head.index;
            let len = head.len as usize;
            if len > TPM_BUFSIZE {
                error!("{}", Error::CommandTooLong { size: len });
                // skip this descriptorchain
                continue;
            }
            let cmd = match read_from_descriptor_chain(mem, head.clone()) {
                Ok(cmd) => cmd,
                Err(err) => {
                    error!("Failed to read descriptorchain: {}", err);
                    continue;
                }
            };
            let resp = &self.backend.execute_command(&cmd);
            if resp.len() > TPM_BUFSIZE {
                error!("{}", Error::ResponseTooLong { size: resp.len() });
                continue;
            }
            match write_to_descriptor_chain(mem, resp, head) {
                Ok(()) => match queue.add_used(mem, head_index, resp.len() as u32) {
                    Ok(()) => (),
                    Err(err) => {
                        error!("Failed to add available descriptor {}: {}", head_index, err);
                        continue;
                    }
                }
                Err(err) => {
                    error!("Failed to write descriptorchain {}", err);
                    continue;
                }
            }
        }
    }

    

}

impl VirtioDevice for Tpm {
    fn avail_features(&self) -> u64 {
        self.avail_features
    }

    fn acked_features(&self) -> u64 {
        self.acked_features
    }

    fn set_acked_features(&mut self, acked_features: u64) {
        self.acked_features = acked_features;
    }

    fn device_type(&self) -> u32 {
        TYPE_TPM
    }    

    fn activate(
        &mut self,
        mem: GuestMemoryMmap
    ) -> ActivateResult {
        if self.queues.len() != 1 {
            error!("expected 1 queue, got {}", self.queues.len());
            return Err(super::super::ActivateError::BadActivate);
        }
        if self.activate_evt.write(1).is_err() {
            error!("Tpm: Cannot write to activate_evt");
            return Err(super::super::ActivateError::BadActivate);
        }
        self.device_state = DeviceState::Activated(mem);

        Ok(())
    }

    fn queues(&self) -> &[Queue] {
        &self.queues
    }

    fn queues_mut(&mut self) -> &mut [Queue] {
        &mut self.queues
    }

    fn queue_events(&self) -> &[EventFd] {
        &self.queue_evts
    }

    fn interrupt_evt(&self) -> &EventFd {
        &self.irq_trigger.irq_evt
    }

    fn interrupt_status(&self) -> Arc<AtomicUsize> {
        self.irq_trigger.irq_status.clone()
    }

    fn read_config(&self, offset: u64, data: &mut [u8]) {
        warn!(
            "vtpm: guest driver attempted to read device config (offset={:x}, len={:x})",
            offset,
            data.len()
        );
    }

    fn write_config(&mut self, offset: u64, data: &[u8]) {
        // TODO add metric here
        warn!(
            "vtpm: guest driver attempted to write device config (offset={:x}, len={:x})",
            offset,
            data.len()
        );
    }

    fn is_activated(&self) -> bool {
        self.device_state.is_activated()
    }
}



type Result<T> = std::result::Result<T, Error>;
