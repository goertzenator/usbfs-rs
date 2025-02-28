use std::ptr;

//use usbtypes::*;
//use usbtypes::devfs::*;
//use device::Transfer;
//use nix;

use super::*;

//pub trait Buffer: AsRef<[u8]> + AsMut<[u8]> {}
//impl<T> Buffer for T where T: AsRef<[u8]> + AsMut<[u8]> {}

pub trait Buffer: AsMut<[u8]> {}
impl<T> Buffer for T where T: AsMut<[u8]> {}

/// ///////////////////////////////////////////////////////////////////////////
///
/// StdBufTransfer
///

#[derive(Debug)]
#[repr(C)]
pub struct StdBufTransfer<B: Buffer> {
    urb: Urb,
    iso_packets: [IsoPacketDesc; 1],
    pub buf: B,
}

unsafe impl<B: Buffer> Transfer for StdBufTransfer<B> {
    fn wire_urb(&mut self) -> &mut Urb {
        match self.urb.urbtype {
            urbtype if (UrbType::Iso as u8) == urbtype => {
                self.urb.buffer = self.buf.as_mut().as_mut_ptr();
                self.iso_packets[0].length = self.buf.as_mut().len() as i32;
                // buffer_length is written during submiturb
            }
            urbtype => {
                self.urb.buffer = self.buf.as_mut().as_mut_ptr();
                self.urb.buffer_length = self.buf.as_mut().len() as i32;
                if (UrbType::Control as u8) == urbtype {
                    // update setup packet
                }
            }
        }

        &mut self.urb
    }
}

impl<B: Buffer> StdBufTransfer<B> {
    pub fn control(
        direction: SetupDirection,
        stype: SetupType,
        recipient: SetupRecipient,
        request: u8,
        value: u16,
        index: u16,
        flags: UrbFlags,
        buf: B,
    ) -> StdBufTransfer<B> {
        let mut xfer = StdBufTransfer {
            urb: Urb {
                urbtype: UrbType::Control as u8,
                endpoint: direction as u8,
                flags,
                ..Urb::default()
            },
            iso_packets: Default::default(),
            buf,
        };

        if xfer.buf.as_mut().len() < 8 {
            panic!("control transfer buffer must be at least 8 bytes");
        }
        let setup = Setup::new(
            direction,
            stype,
            recipient,
            request,
            value,
            index,
            (xfer.buf.as_mut().len() - 8) as u16,
        );
        write_setup_struct(&setup.into(), xfer.buf.as_mut());
        xfer
    }

    pub fn bulk(endpoint: u8, flags: UrbFlags, buf: B) -> StdBufTransfer<B> {
        StdBufTransfer {
            urb: Urb {
                urbtype: UrbType::Bulk as u8,
                endpoint,
                flags,
                ..Urb::default()
            },
            iso_packets: Default::default(),
            buf,
        }
    }

    pub fn interrupt(endpoint: u8, flags: UrbFlags, buf: B) -> StdBufTransfer<B> {
        StdBufTransfer {
            urb: Urb {
                urbtype: UrbType::Interrupt as u8,
                endpoint,
                flags,
                ..Urb::default()
            },
            iso_packets: Default::default(),
            buf,
        }
    }

    pub fn isochronous(endpoint: u8, flags: UrbFlags, buf: B) -> StdBufTransfer<B> {
        StdBufTransfer {
            urb: Urb {
                urbtype: UrbType::Iso as u8,
                endpoint,
                flags,
                number_of_packets: 1,
                ..Urb::default()
            },
            iso_packets: Default::default(),
            buf,
        }
    }

    //    pub fn data(&self) -> &[u8] {
    //        match self.urb.urbtype {
    //            urbtype if (UrbType::Control as u8) == urbtype => &self.buf.as_ref()[8..],
    //            _ => self.buf.as_ref(),
    //        }
    //    }

    pub fn data_mut(&mut self) -> &mut [u8] {
        match self.urb.urbtype {
            urbtype if (UrbType::Control as u8) == urbtype => &mut self.buf.as_mut()[8..],
            _ => self.buf.as_mut(),
        }
    }

    //    pub fn result_data(&self) -> nix::Result<&[u8]> {
    //        let actual_length = try!(self.result_length());
    //        Ok(&self.data()[0..actual_length])
    //    }

    pub fn result_data_mut(&mut self) -> nix::Result<&mut [u8]> {
        let actual_length = self.result_length()?;
        Ok(&mut self.data_mut()[0..actual_length])
    }

    pub fn result_length(&self) -> nix::Result<usize> {
        let (status, length) = match self.urb.urbtype {
            urbtype if (UrbType::Iso as u8) == urbtype => (
                self.iso_packets[0].status,
                self.iso_packets[0].actual_length,
            ),
            _ => (self.urb.status, self.urb.actual_length),
        };
        status_to_nixresult(status)?;
        Ok(length as usize)
    }
}

fn status_to_nixresult(status: i32) -> nix::Result<()> {
    if status < 0 {
        Err(nix::Error::from_i32(status))
    } else {
        Ok(())
    }
}

/// Copy a setup packet into the given buffer.
///
/// Async control transfers require an 8 byte bus-endian setup packet at the beginning
/// of the data buffer.  This function installs that packet.
///
/// # Panics
/// This function will panic if the buffer is too short for the 8 byte setup packet.
pub fn write_setup_struct(setup: &Setup<BusEndian>, buf: &mut [u8]) {
    // Write setup packet into buffer
    if buf.len() < 8 {
        panic!("buf() too short for setup packet");
    }
    unsafe { ptr::copy_nonoverlapping(setup, buf.as_mut_ptr() as *mut Setup<BusEndian>, 1) };
}
