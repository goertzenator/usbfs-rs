//use usbtypes::*;
//use usbtypes::devfs::*;
//use device::Transfer;
//use nix;

use super::*;

use std::fmt::Debug;

pub trait IsoBuffer: AsMut<[u8]> {
    fn packet_length(&self) -> usize;
    //    const PACKET_LENGTH: usize; // maximum size of each iso packet
    //    fn packet_lengths(&self) -> impl Iterator<Item=usize>;
}

//////////////////////////////////////////////////////////////////////////////
///
/// StdBufTransfer
///

#[derive(Debug)]
#[repr(C)]
pub struct IsoBufTransfer<B, const N: usize> {
    urb: Urb,
    iso_packets: [IsoPacketDesc; N],
    pub buf: B,
}

unsafe impl<B: IsoBuffer + Debug, const N: usize> Transfer for IsoBufTransfer<B, N> {
    fn wire_urb(&mut self) -> &mut Urb {
        // Initialize iso packet descriptors.
        // packet_lengths() indicates the number and length of each packet.
        // The actual number of descriptors initialized is constrained by the following things:
        // - The size of the buffer provided by as_mut<[u8]>
        // - Number of packets indicated by packet_lengths()
        // - MAX_ISO_PACKETS, the number of descriptors actually available.

        let mut tot_length = self.buf.as_mut().len();
        let mut tot_packets = 0;

        // leave this as iterator for now in case IsoBuffer ever gets packet_lengths() back.
        let length = self.buf.packet_length();

        for packet in &mut self.iso_packets {
            if 0 == tot_length {
                break;
            }
            let limited_length = std::cmp::min(tot_length, length);
            packet.length = limited_length as i32;
            packet.actual_length = 0;
            packet.status = -22;
            tot_packets += 1;
            tot_length -= limited_length;
        }

        self.urb.buffer = self.buf.as_mut().as_mut_ptr();
        self.urb.number_of_packets = tot_packets;

        &mut self.urb
    }
}

impl<B, const N: usize> IsoBufTransfer<B, N> {
    pub fn isochronous(endpoint: u8, flags: UrbFlags, buf: B) -> IsoBufTransfer<B, N> {
        IsoBufTransfer {
            urb: Urb {
                urbtype: UrbType::Iso as u8,
                endpoint,
                flags,
                ..Urb::default()
            },
            iso_packets: [IsoPacketDesc::default(); N],
            buf,
        }
    }

    pub fn get_urb(&self) -> &Urb {
        &self.urb
    }

    pub fn status(&self) -> &[IsoPacketDesc] {
        &self.iso_packets[..(self.urb.number_of_packets as usize)]
    }
}
