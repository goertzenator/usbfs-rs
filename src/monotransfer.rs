use super::*;

use std::slice;

//////////////////////////////////////////////////////////////////////////////
///
/// ControlTransferMut
///

/// Control Transfer on mutable buffer.  IN and OUT transfers permitted.
/// Will panic if buffer is smaller than 8 bytes when wire_urb() is called.
/// Note there is no immutable version of this struct because setup packet
/// always needs to be written to the given buffer.
pub struct ControlTransferMut<B> {
  urb: Urb,
  setup: Setup<BusEndian>,
  pub buf: B,
}
impl<B> ControlTransferMut<B> {
  pub fn new(
    direction: SetupDirection,
    stype: SetupType,
    recipient: SetupRecipient,
    request: u8,
    value: u16,
    index: u16,
    flags: UrbFlags,
    buf: B,
  ) -> Self {

    let setup = Setup::new(
      direction,
      stype,
      recipient,
      request,
      value,
      index,
      0,  // wLength, set at wire_urb time
    ).into();

    let urb = Urb {
      urbtype: UrbType::Control as u8,
      endpoint: 0, // endpoint
      flags,
      ..Urb::default()
    };

    ControlTransferMut {urb, setup, buf}
  }

  /// Access to portion of buffer after the setup packet (the payload).
  pub fn payload(&self) -> &[u8]
  where B: AsRef<[u8]>
  {
    & self.buf.as_ref()[8..]
  }

  /// Mutable access to portion of buffer after the setup packet (the payload).
  pub fn payload_mut(&mut self) -> &mut [u8]
  where B: AsMut<[u8]>
  {
    &mut self.buf.as_mut()[8..]
  }
}

unsafe impl<B: AsMut<[u8]>> Transfer for ControlTransferMut<B> {
  fn wire_urb(&mut self) -> &mut Urb {

    let mbuf: &mut [u8] = self.buf.as_mut();

    // write setup packet to buffer
    if mbuf.len() < 8 {
      panic!("buffer too short for setup packet, min size is 8 bytes");
    }
    self.setup.wLength = ((mbuf.len() - 8) as u16).to_le();
    unsafe {
      mbuf[0..8].copy_from_slice(slice::from_raw_parts(&self.setup as *const Setup<BusEndian> as *const u8, 8));
      // mbuf[0..8].copy_from_slice(slice::from_raw_parts(&self.setup as *const u8, 8));
    }

    // wire up the urb
    self.urb.buffer = mbuf.as_mut_ptr() as *mut u8;
    self.urb.buffer_length = mbuf.len() as i32;
    &mut self.urb
  }
}

//////////////////////////////////////////////////////////////////////////////
///
/// BulkTransfer
///

/// Bulk Transfer on immutable buffer.  Only OUT transfers permitted.
pub struct BulkTransfer<B> {
  urb: Urb,
  pub buf: B,
}
impl<B> BulkTransfer<B> {
  pub fn new(endpoint: u8, flags: UrbFlags, buf: B) -> Self {
    assert!(0 == endpoint & 0x80, "can't IN xfer onto immutable buffer");
    BulkTransfer {
      urb: Urb {
        urbtype: UrbType::Bulk as u8,
        endpoint,
        flags,
        ..Urb::default()
      },
      buf,
    }
  }
}
unsafe impl<B: AsRef<[u8]>> Transfer for BulkTransfer<B> {
  fn wire_urb(&mut self) -> &mut Urb {
    self.urb.buffer = self.buf.as_ref().as_ptr() as *mut u8;
    self.urb.buffer_length = self.buf.as_ref().len() as i32;
    &mut self.urb
  }
}

/// Bulk Transfer on mutable buffer.  IN and OUT transfers permitted.
pub struct BulkTransferMut<B> {
  urb: Urb,
  pub buf: B,
}
impl<B> BulkTransferMut<B> {
  pub fn new(endpoint: u8, flags: UrbFlags, buf: B) -> Self {
    BulkTransferMut {
      urb: Urb {
        urbtype: UrbType::Bulk as u8,
        endpoint,
        flags,
        ..Urb::default()
      },
      buf,
    }
  }
}
unsafe impl<B: AsMut<[u8]>> Transfer for BulkTransferMut<B> {
  fn wire_urb(&mut self) -> &mut Urb {
    self.urb.buffer = self.buf.as_mut().as_mut_ptr() as *mut u8;
    self.urb.buffer_length = self.buf.as_mut().len() as i32;
    &mut self.urb
  }
}

//////////////////////////////////////////////////////////////////////////////
///
/// InterruptTransfer
///

/// Interrupt Transfer on immutable buffer.  Only OUT transfers permitted.
pub struct InterruptTransfer<B> {
  urb: Urb,
  pub buf: B,
}
impl<B> InterruptTransfer<B> {
  pub fn new(endpoint: u8, flags: UrbFlags, buf: B) -> Self {
    assert!(0 == endpoint & 0x80, "not an OUT endpoint");
    InterruptTransfer {
      urb: Urb {
        urbtype: UrbType::Interrupt as u8,
        endpoint,
        flags,
        ..Urb::default()
      },
      buf,
    }
  }
}
unsafe impl<B: AsRef<[u8]>> Transfer for InterruptTransfer<B> {
  fn wire_urb(&mut self) -> &mut Urb {
    self.urb.buffer = self.buf.as_ref().as_ptr() as *mut u8;
    self.urb.buffer_length = self.buf.as_ref().len() as i32;
    &mut self.urb
  }
}

/// Interrupt Transfer on mutable buffer.  IN and OUT transfers permitted.
pub struct InterruptTransferMut<B> {
  urb: Urb,
  pub buf: B,
}
impl<B> InterruptTransferMut<B> {
  pub fn new(endpoint: u8, flags: UrbFlags, buf: B) -> Self {
    InterruptTransferMut {
      urb: Urb {
        urbtype: UrbType::Interrupt as u8,
        endpoint,
        flags,
        ..Urb::default()
      },
      buf,
    }
  }
}
unsafe impl<B: AsMut<[u8]>> Transfer for InterruptTransferMut<B> {
  fn wire_urb(&mut self) -> &mut Urb {
    self.urb.buffer = self.buf.as_mut().as_mut_ptr() as *mut u8;
    self.urb.buffer_length = self.buf.as_mut().len() as i32;
    &mut self.urb
  }
}

// Thoughts for isochronous impl
// - Const generic on iso packets
// - Buffer should have
//     ExactSizeIterator<item = &[u8]> OR
//     ExactSizeIterator<item = &mut [u8]>
// - For packets == 1, AsRef<[u8]> / AsMut<[u8]>
// - Transfer should provide iterator of results (or single result of N==1)
// - See isobuftransfer for more info.

// #[derive(Debug)]
// #[repr(C)]
// pub struct IsochronousOutTransfer<B, N> {
//     urb: Urb,
//     iso_packets: [IsoPacketDesc; N],
//     pub bufs: B,
// }

// impl<B, N> IsochronousOutTransfer<B, N> {
//   fn new(endpoint: u8, flags: UrbFlags, buf: B) -> Self
//   {
//     assert!(0 == endpoint&0x80, "not an OUT endpoint");
//     IsochronousOutTransfer {
//       urb: Urb {
//         urbtype: UrbType::Iso as u8,
//         endpoint,
//         flags,
//         ..Urb::default()
//       },
//       buf,
//     }
//   }
// }
