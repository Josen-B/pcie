#![no_std]

extern crate alloc;

mod bar_alloc;
mod chip;
pub mod err;
pub mod mac;
pub mod osal;
pub mod phy;
mod root;
mod types;
use core::{cell::RefCell, ptr::NonNull};
use log::debug;
pub use mac::{MacAddr6, MacStatus};
pub use osal::*;

pub use chip::{
    generic::{Generic, RootComplexGeneric},
    Chip,
};

pub use bar_alloc::*;
pub use root::{EnumElem, RootComplex};
pub use types::*;

pub trait BarAllocator {
    fn alloc_memory32(&mut self, size: u32) -> Option<u32>;
    fn alloc_memory64(&mut self, size: u64) -> Option<u64>;
}

pub struct Igb {
    mac: RefCell<mac::Mac>,
    phy: phy::Phy,
}

impl Igb {
    pub fn new(iobase: NonNull<u8>) -> Result<Self, DError> {
        let mac = RefCell::new(mac::Mac::new(iobase));
        let phy = phy::Phy::new(mac.clone());

        Ok(Self { mac, phy })
    }

    pub fn open(&mut self) -> Result<(), DError> {
        // disable interrupts
        self.mac.borrow_mut().disable_interrupts();
        // reset the device
        debug!("Resetting the device");
        self.mac.borrow_mut().reset()?;
        // disable interrupts
        self.mac.borrow_mut().disable_interrupts();
        // setup the phy and the link
        debug!("setting up PHY and link");
        self.phy.power_up()?;
        self.setup_phy_and_the_link()?;
        // wait for auto-negotiation to complete
        debug!("wait Auto-negotiation to complete");
        self.phy.wait_for_auto_negotiation_complete()?;
        debug!("initialization complete");
        Ok(())
    }

    fn setup_phy_and_the_link(&mut self) -> Result<(), DError> {
        self.phy.power_up()?;
        self.phy.enable_auto_negotiation()?;

        Ok(())
    }

    pub fn check_vid_did(vid: u16, did: u16) -> bool {
        // This is a placeholder for actual VID/DID checking logic.
        // In a real implementation, this would check the device's
        // vendor ID and device ID against the provided values.
        vid == 0x8086 && [0x10C9, 0x1533].contains(&did)
    }

    pub fn status(&self) -> MacStatus {
        self.mac.borrow().status()
    }
}
