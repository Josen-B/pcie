use crate::osal::*;
use core::fmt::Debug;
use core::{ptr::NonNull, time::Duration};
use log::error;
use mbarrier::mb;
use tock_registers::registers::{ReadOnly, ReadWrite};
use tock_registers::{interfaces::*, register_bitfields, register_structs, registers::*};

#[derive(Clone, Copy)]
pub struct Mac {
    reg: NonNull<MacRegisters>,
}

impl Mac {
    pub fn new(iobase: NonNull<u8>) -> Self {
        Self { reg: iobase.cast() }
    }

    pub fn iobase(&self) -> NonNull<u8> {
        self.reg.cast()
    }

    pub fn reg(&self) -> &MacRegisters {
        unsafe { self.reg.as_ref() }
    }

    pub fn reg_mut(&mut self) -> &mut MacRegisters {
        unsafe { self.reg.as_mut() }
    }

    pub fn status(&self) -> MacStatus {
        let status = self.reg().status.extract();
        let speed = match status.read_as_enum(STATUS::SPEED) {
            Some(STATUS::SPEED::Value::Speed1000) => Speed::Mb1000,
            Some(STATUS::SPEED::Value::Speed100) => Speed::Mb100,
            _ => Speed::Mb10,
        };
        let full_duplex = status.is_set(STATUS::FD);
        let link_up = status.is_set(STATUS::LU);
        let phy_reset_asserted = status.is_set(STATUS::PHYRA);

        MacStatus {
            full_duplex,
            link_up,
            speed,
            phy_reset_asserted,
        }
    }

    pub fn write_mdic(&self, phys_addr: u32, offset: u32, data: u16) -> Result<(), DError> {
        self.reg().mdic.write(
            MDIC::REGADDR.val(offset)
                + MDIC::PHY_ADDR.val(phys_addr)
                + MDIC::DATA.val(data as _)
                + MDIC::OP::Write,
        );
        mb();

        loop {
            let mdic = self.reg().mdic.extract();

            if mdic.is_set(MDIC::READY) {
                break;
            }
            if mdic.is_set(MDIC::E) {
                error!("MDIC read error");
                return Err(DError::Unknown("MDIC read error"));
            }
        }

        Ok(())
    }

    pub fn read_mdic(&self, phys_addr: u32, offset: u32) -> Result<u16, DError> {
        self.reg()
            .mdic
            .write(MDIC::REGADDR.val(offset) + MDIC::PHY_ADDR.val(phys_addr) + MDIC::OP::Read);
        mb();
        loop {
            let mdic = self.reg().mdic.extract();
            if mdic.is_set(MDIC::READY) {
                return Ok(mdic.read(MDIC::DATA) as _);
            }
            if mdic.is_set(MDIC::E) {
                error!("MDIC read error");
                return Err(DError::Unknown("MDIC read error"));
            }
        }
    }

    pub fn disable_interrupts(&mut self) {
        self.reg_mut().eimc.set(u32::MAX);
        self.clear_interrupts();
    }

    pub fn enable_interrupts(&mut self) {
        self.reg_mut().eims.set(u32::MAX);
    }
    pub fn clear_interrupts(&mut self) {
        // Clear interrupt mask
        self.reg_mut().eimc.set(u32::MAX);
        self.reg().eicr.get();
    }

    pub fn link_mode(&self) -> Option<LinkMode> {
        Some(
            match self.reg().ctrl_ext.read_as_enum(CTRL_EXT::LINK_MODE) {
                Some(CTRL_EXT::LINK_MODE::Value::DircetCooper) => LinkMode::DirectCooper,
                Some(CTRL_EXT::LINK_MODE::Value::SGMII) => LinkMode::Sgmii,
                Some(CTRL_EXT::LINK_MODE::Value::InternalSerdes) => LinkMode::InternalSerdes,
                None => return None,
            },
        )
    }

    pub fn reset(&mut self) -> Result<(), DError> {
        self.reg_mut()
            .ctrl
            .modify(CTRL::RST::Reset + CTRL::PHY_RST::SET);
        wait_for(
            || self.reg().ctrl.matches_any(&[CTRL::RST::Normal]),
            Duration::from_millis(1),
            Some(1000),
        )
    }

    pub fn set_link_up(&mut self) {
        self.reg_mut().ctrl.modify(CTRL::SLU::SET + CTRL::FD::SET);
    }
}

// 定义 MAC 寄存器组
register_structs! {
    pub MacRegisters {
        (0x0 => ctrl: ReadWrite<u32, CTRL::Register>),
        (0x4 => _rsv1),
        (0x8 => status: ReadOnly<u32, STATUS::Register>),
        (0xC => _rsv2),
        (0x18 => ctrl_ext: ReadWrite<u32, CTRL_EXT::Register>),
        (0x1c => _rsv3),
        (0x20 => mdic: ReadWrite<u32, MDIC::Register>),
        (0x24 => _rsv4),
        (0xc0 => icr: ReadWrite<u32, ICR::Register>),
        (0xc4 => _rsv13),
        (0xd0 => ims: ReadWrite<u32, IMS::Register>),
        (0xd4 => _rsv14),
        (0xd8 => imc: ReadWrite<u32, IMC::Register>),
        (0xdc => _rsv15),
        (0x100 => pub rctl: ReadWrite<u32, RCTL::Register>),
        (0x104 => _rsv7),
        (0x400 => tctl: ReadWrite<u32, TCTL::Register>),
        (0x404 => _rsv12),
        (0x1514 => gpie: ReadWrite<u32, GPIE::Register>),
        (0x1518 => _rsv16),
        (0x1524 => eims: ReadWrite<u32>),
        (0x1528 => eimc: ReadWrite<u32>),
        (0x152c => eiac: ReadWrite<u32>),
        (0x1530 => eiam: ReadWrite<u32>),
        (0x1534 => _rsv5),
        (0x1580 => eicr: ReadWrite<u32>),
        (0x1584 => _rsv6),
        (0x5400 => ralh_0_15: [ReadWrite<u32>; 32]),
        (0x5480 => _rsv8),
        (0x54e0 => ralh_16_23: [ReadWrite<u32>;32]),
        (0x5560 => _rsv9),
        (0x5B50 => swsm: ReadWrite<u32, SWSM::Register>),
        (0x5B54 => fwsm: ReadWrite<u32>),
        (0x5B58 => _rsv10),
        (0x5B5C => sw_fw_sync: ReadWrite<u32>),
        (0x5B60 => _rsv11),

        // The end of the struct is marked as follows.
        (0xEFFF => @END),
    }
}

register_bitfields! [
    // First parameter is the register width. Can be u8, u16, u32, or u64.
    u32,

    CTRL [
        FD OFFSET(0) NUMBITS(1)[
            HalfDuplex = 0,
            FullDuplex = 1,
        ],
        SLU OFFSET(6) NUMBITS(1)[],
        SPEED OFFSET(8) NUMBITS(2)[
            Speed10 = 0,
            Speed100 = 1,
            Speed1000 = 0b10,
        ],
        FRCSPD OFFSET(11) NUMBITS(1)[],
        FRCDPLX OFFSET(12) NUMBITS(1)[],
        RST OFFSET(26) NUMBITS(1)[
            Normal = 0,
            Reset = 1,
        ],
        PHY_RST OFFSET(31) NUMBITS(1)[],
    ],
    STATUS [
        FD OFFSET(0) NUMBITS(1)[
            HalfDuplex = 0,
            FullDuplex = 1,
        ],
        LU OFFSET(1) NUMBITS(1)[],
        SPEED OFFSET(6) NUMBITS(2)[
            Speed10 = 0,
            Speed100 = 1,
            Speed1000 = 0b10,
        ],
         PHYRA OFFSET(10) NUMBITS(1)[],
    ],
    pub CTRL_EXT [
        LINK_MODE OFFSET(22) NUMBITS(2)[
            DircetCooper = 0,
            SGMII = 0b10,
            InternalSerdes = 0b11,
        ],
    ],
    MDIC [
        DATA OFFSET(0) NUMBITS(16)[],
        REGADDR OFFSET(16) NUMBITS(5)[],
        PHY_ADDR OFFSET(21) NUMBITS(5)[],
        OP OFFSET(26) NUMBITS(2)[
            Write = 0b1,
            Read = 0b10,
        ],
        READY OFFSET(28) NUMBITS(1)[],
        I OFFSET(29) NUMBITS(1)[],
        E OFFSET(30) NUMBITS(1)[
            NoError = 0,
            Error = 1,
        ],
        Destination OFFSET(31) NUMBITS(1)[
            Internal = 0,
            External = 1,
        ]
    ],

    SWSM [
        SMBI OFFSET(0) NUMBITS(1)[],
        SWESMBI OFFSET(1) NUMBITS(1)[],
        WMNG OFFSET(2) NUMBITS(1)[],
        EEUR OFFSET(3) NUMBITS(1)[],
    ],

    SW_FW_SYNC [
        SW_EEP_SM OFFSET(0) NUMBITS(1)[],
        SW_PHY_SM0 OFFSET(1) NUMBITS(1)[],
        SW_PHY_SM1 OFFSET(2) NUMBITS(1)[],
        SW_MAC_CSR_SM OFFSET(3) NUMBITS(1)[],
        SW_FLASH_SM OFFSET(4) NUMBITS(1)[],

        FW_EEP_SM OFFSET(16) NUMBITS(1)[],
        FW_PHY_SM0 OFFSET(17) NUMBITS(1)[],
        FW_PHY_SM1 OFFSET(18) NUMBITS(1)[],
        FW_MAC_CSR_SM OFFSET(19) NUMBITS(1)[],
        FW_FLASH_SM OFFSET(20) NUMBITS(1)[],
    ],

    pub RCTL [
        RXEN OFFSET(1) NUMBITS(1)[
            Disabled = 0,
            Enabled = 1,
        ],
        SBP OFFSET(2) NUMBITS(1)[
            DoNotStore = 0,
            Store = 1,
        ],
        UPE OFFSET(3) NUMBITS(1)[
            Disabled = 0,
            Enabled = 1,
        ],
        MPE OFFSET(4) NUMBITS(1)[
            Disabled = 0,
            Enabled = 1,
        ],
        LPE OFFSET(5) NUMBITS(1)[
            Disabled = 0,
            Enabled = 1,
        ],
        LBM OFFSET(6) NUMBITS(2)[
            Normal = 0b00,
            MacLoopback = 0b01,
            Reserved = 0b11,
        ],
        MO OFFSET(12) NUMBITS(2)[
            Bits47_36 = 0b00,
            Bits46_35 = 0b01,
            Bits45_34 = 0b10,
            Bits43_32 = 0b11,
        ],
        BAM OFFSET(15) NUMBITS(1)[
            Ignore = 0,
            Accept = 1,
        ],
        BSIZE OFFSET(16) NUMBITS(2)[
            Bytes2048 = 0b00,
            Bytes1024 = 0b01,
            Bytes512 = 0b10,
            Bytes256 = 0b11,
        ],
        VFE OFFSET(18) NUMBITS(1)[
            Disabled = 0,
            Enabled = 1,
        ],
        CFIEN OFFSET(19) NUMBITS(1)[
            Disabled = 0,
            Enabled = 1,
        ],
        CFI OFFSET(20) NUMBITS(1)[
            Accept = 0,
            Discard = 1,
        ],
        PSP OFFSET(21) NUMBITS(1)[],
        DPF OFFSET(22) NUMBITS(1)[
            Forward = 0,
            Discard = 1,
        ],
        PMCF OFFSET(23) NUMBITS(1)[
            Pass = 0,
            Filter = 1,
        ],
        SECRC OFFSET(26) NUMBITS(1)[
            DoNotStrip = 0,
            Strip = 1,
        ],
    ],

    // Transmit Control Register - TCTL (0x400)
    TCTL [
        EN OFFSET(1) NUMBITS(1)[
            Disabled = 0,
            Enabled = 1,
        ],
        PSP OFFSET(3) NUMBITS(1)[
            Disabled = 0,
            Enabled = 1,
        ],
        CT OFFSET(4) NUMBITS(8)[],
        COLD OFFSET(12) NUMBITS(10)[],
        SWXOFF OFFSET(22) NUMBITS(1)[],
        RTLC OFFSET(24) NUMBITS(1)[],
        NRTU OFFSET(25) NUMBITS(1)[],
        MULR OFFSET(28) NUMBITS(1)[],
    ],

    // Extended Interrupt Cause Register - EICR (0x01580)
    EICR [
        // Non MSI-X mode (GPIE.Multiple_MSIX = 0)
        RxTxQ OFFSET(0) NUMBITS(16)[],
        Reserved1 OFFSET(16) NUMBITS(14)[],
        TCP_Timer OFFSET(30) NUMBITS(1)[],
        Other_Cause OFFSET(31) NUMBITS(1)[],
    ],

    // Extended Interrupt Cause Register - EICR MSI-X mode
    EICR_MSIX [
        // MSI-X mode (GPIE.Multiple_MSIX = 1)
        MSIX OFFSET(0) NUMBITS(25)[],
        Reserved OFFSET(25) NUMBITS(7)[],
    ],

    // Extended Interrupt Mask Set/Read - EIMS (0x01524)
    EIMS [
        // Non MSI-X mode (GPIE.Multiple_MSIX = 0)
        RxTxQ OFFSET(0) NUMBITS(16)[],
        Reserved1 OFFSET(16) NUMBITS(14)[],
        TCP_Timer OFFSET(30) NUMBITS(1)[],
        Other_Cause OFFSET(31) NUMBITS(1)[],
    ],

    // Extended Interrupt Mask Set/Read - EIMS MSI-X mode
    EIMS_MSIX [
        // MSI-X mode (GPIE.Multiple_MSIX = 1)
        MSIX OFFSET(0) NUMBITS(25)[],
        Reserved OFFSET(25) NUMBITS(7)[],
    ],

    // Legacy Interrupt Cause Register - ICR (0x000C0)
    ICR [
        TXDW OFFSET(0) NUMBITS(1)[],   // Transmit Descriptor Written Back
        TXQE OFFSET(1) NUMBITS(1)[],   // Transmit Queue Empty
        LSC OFFSET(2) NUMBITS(1)[],    // Link Status Change
        RXSEQ OFFSET(3) NUMBITS(1)[],  // Receive Sequence Error
        RXDMT0 OFFSET(4) NUMBITS(1)[], // Receive Descriptor Minimum Threshold Reached
        RXO OFFSET(6) NUMBITS(1)[],    // Receiver Overrun
        RXT0 OFFSET(7) NUMBITS(1)[],   // Receiver Timer Interrupt
        MDAC OFFSET(9) NUMBITS(1)[],   // MDI/O Access Complete
        RXCFG OFFSET(10) NUMBITS(1)[], // Receiving /C/ ordered sets
        GPI_EN0 OFFSET(11) NUMBITS(1)[], // General Purpose Interrupt Enable 0
        GPI_EN1 OFFSET(12) NUMBITS(1)[], // General Purpose Interrupt Enable 1
        GPI_EN2 OFFSET(13) NUMBITS(1)[], // General Purpose Interrupt Enable 2
        GPI_EN3 OFFSET(14) NUMBITS(1)[], // General Purpose Interrupt Enable 3
        TXD_LOW OFFSET(15) NUMBITS(1)[], // Transmit Descriptor Low Threshold Hit
        SRPD OFFSET(16) NUMBITS(1)[],  // Small Receive Packet Detected
        ACK OFFSET(17) NUMBITS(1)[],   // Receive ACK Frame
        MNG OFFSET(18) NUMBITS(1)[],   // Management Bus Interrupt
        DOCK OFFSET(19) NUMBITS(1)[],  // Dock/Undock Status Change
        INT_ASSERTED OFFSET(31) NUMBITS(1)[], // Interrupt Asserted
    ],

    // Legacy Interrupt Mask Set/Read - IMS (0x000D0)
    IMS [
        TXDW OFFSET(0) NUMBITS(1)[],   // Transmit Descriptor Written Back
        TXQE OFFSET(1) NUMBITS(1)[],   // Transmit Queue Empty
        LSC OFFSET(2) NUMBITS(1)[],    // Link Status Change
        RXSEQ OFFSET(3) NUMBITS(1)[],  // Receive Sequence Error
        RXDMT0 OFFSET(4) NUMBITS(1)[], // Receive Descriptor Minimum Threshold Reached
        RXO OFFSET(6) NUMBITS(1)[],    // Receiver Overrun
        RXT0 OFFSET(7) NUMBITS(1)[],   // Receiver Timer Interrupt
        MDAC OFFSET(9) NUMBITS(1)[],   // MDI/O Access Complete
        RXCFG OFFSET(10) NUMBITS(1)[], // Receiving /C/ ordered sets
        GPI_EN0 OFFSET(11) NUMBITS(1)[], // General Purpose Interrupt Enable 0
        GPI_EN1 OFFSET(12) NUMBITS(1)[], // General Purpose Interrupt Enable 1
        GPI_EN2 OFFSET(13) NUMBITS(1)[], // General Purpose Interrupt Enable 2
        GPI_EN3 OFFSET(14) NUMBITS(1)[], // General Purpose Interrupt Enable 3
        TXD_LOW OFFSET(15) NUMBITS(1)[], // Transmit Descriptor Low Threshold Hit
        SRPD OFFSET(16) NUMBITS(1)[],  // Small Receive Packet Detected
        ACK OFFSET(17) NUMBITS(1)[],   // Receive ACK Frame
        MNG OFFSET(18) NUMBITS(1)[],   // Management Bus Interrupt
        DOCK OFFSET(19) NUMBITS(1)[],  // Dock/Undock Status Change
    ],

    // Legacy Interrupt Mask Clear - IMC (0x000D8)
    IMC [
        TXDW OFFSET(0) NUMBITS(1)[],   // Transmit Descriptor Written Back
        TXQE OFFSET(1) NUMBITS(1)[],   // Transmit Queue Empty
        LSC OFFSET(2) NUMBITS(1)[],    // Link Status Change
        RXSEQ OFFSET(3) NUMBITS(1)[],  // Receive Sequence Error
        RXDMT0 OFFSET(4) NUMBITS(1)[], // Receive Descriptor Minimum Threshold Reached
        RXO OFFSET(6) NUMBITS(1)[],    // Receiver Overrun
        RXT0 OFFSET(7) NUMBITS(1)[],   // Receiver Timer Interrupt
        MDAC OFFSET(9) NUMBITS(1)[],   // MDI/O Access Complete
        RXCFG OFFSET(10) NUMBITS(1)[], // Receiving /C/ ordered sets
        GPI_EN0 OFFSET(11) NUMBITS(1)[], // General Purpose Interrupt Enable 0
        GPI_EN1 OFFSET(12) NUMBITS(1)[], // General Purpose Interrupt Enable 1
        GPI_EN2 OFFSET(13) NUMBITS(1)[], // General Purpose Interrupt Enable 2
        GPI_EN3 OFFSET(14) NUMBITS(1)[], // General Purpose Interrupt Enable 3
        TXD_LOW OFFSET(15) NUMBITS(1)[], // Transmit Descriptor Low Threshold Hit
        SRPD OFFSET(16) NUMBITS(1)[],  // Small Receive Packet Detected
        ACK OFFSET(17) NUMBITS(1)[],   // Receive ACK Frame
        MNG OFFSET(18) NUMBITS(1)[],   // Management Bus Interrupt
        DOCK OFFSET(19) NUMBITS(1)[],  // Dock/Undock Status Change
    ],

    // General Purpose Interrupt Enable - GPIE (0x1514)
    GPIE [
        NSICR OFFSET(0) NUMBITS(1)[
            Normal = 0,
            ClearOnRead = 1,
        ],
        Multiple_MSIX OFFSET(4) NUMBITS(1)[
            SingleVector = 0,
            MultipleVectors = 1,
        ],
        LL_Interval OFFSET(7) NUMBITS(5)[],  // Low Latency Credits Increment Rate (bits 11:7)
        EIAME OFFSET(30) NUMBITS(1)[
            Disabled = 0,
            Enabled = 1,
        ],
        PBA_Support OFFSET(31) NUMBITS(1)[
            Legacy = 0,
            MSIX = 1,
        ],
    ],
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkMode {
    DirectCooper,
    Sgmii,
    InternalSerdes,
}

#[derive(Debug, Clone, Copy)]
pub struct MacStatus {
    pub full_duplex: bool,
    pub link_up: bool,
    pub speed: Speed,
    pub phy_reset_asserted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Speed {
    Mb1000,
    Mb100,
    Mb10,
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct MacAddr6([u8; 6]);

impl MacAddr6 {
    pub fn new(bytes: [u8; 6]) -> Self {
        MacAddr6(bytes)
    }

    pub fn bytes(&self) -> [u8; 6] {
        self.0
    }
}

impl Debug for MacAddr6 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5]
        )
    }
}

impl From<[u8; 6]> for MacAddr6 {
    fn from(addr: [u8; 6]) -> Self {
        MacAddr6(addr)
    }
}

impl From<MacAddr6> for [u8; 6] {
    fn from(addr: MacAddr6) -> Self {
        addr.0
    }
}
