
use std::mem::size_of;
pub use nix::libc::{c_uint, c_int};
use std::io;
use nix;

#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct CtrlTransfer {
    pub bmRequestType: u8,
    pub bRequest: u8,
    pub wValue: u16,
    pub wIndex: u16,
    pub wLength: u16,
    pub timeout: u32, // in milliseconds
    pub data: *mut u8,
}

bitflags! {
    #[repr(C)]
    pub struct UrbFlags: u32 {
        const URB_SHORT_NOT_OK      = 0x01;
        const URB_ISO_ASAP          = 0x02;
        const URB_BULK_CONTINUATION = 0x04;
        const URB_NO_FSBR           = 0x20;
        const URB_ZERO_PACKET       = 0x40;
        const URB_NO_INTERRUPT      = 0x80;
    }
}

/// The [type of transfer](http://www.beyondlogic.org/usbnutshell/usb4.shtml).
///
/// Isochronous transfers not implemented (yet),
#[derive(Debug, Copy, Clone)]
pub enum UrbType {
    Iso = 0,
    Interrupt = 1,
    Control = 2,
    Bulk = 3,
}

#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct Urb {
    pub urbtype: u8, // "type" is Rust keyword
    pub endpoint: u8,
    pub status: i32, // reap result
    pub flags: UrbFlags,
    pub buffer: *mut u8, // assigned upon submit
    pub buffer_length: i32, // assigned upon submit
    pub actual_length: i32, // reap result
    pub start_frame: i32,
    pub number_of_packets: i32,
    pub error_count: i32, // reap result
    pub signr: u32, // signal to be sent on completion, or 0 if none should be sent.
    pub usercontext: usize, /* assigned upon submit
                             * struct usbdevfs_iso_packet_desc iso_frame_desc[0];
                             *
                             * union {
                             *  int number_of_packets;  /* Only used for isoc urbs */
                             *  unsigned int stream_id; /* Only used with bulk streams */
                             * }; */
}

impl Urb {
    pub fn new(urbtype: UrbType, endpoint: u8, flags: UrbFlags) -> Urb {
        Urb {
            urbtype: urbtype as u8,
            endpoint,
            status: -22, // -EINVAL, in case status is read before urb is used.
            flags,
            buffer: 0 as *mut u8,
            buffer_length: 0,
            actual_length: 0,
            start_frame: 0,
            number_of_packets: 0,
            error_count: 0,
            signr: 0,
            usercontext: 0,
        }
    }
}

impl Default for Urb {
    fn default() -> Urb {
        Urb {
            urbtype: UrbType::Control as u8,
            endpoint: 0,
            status: -22, // -EINVAL, in case status is read before urb is used.
            flags: UrbFlags::empty(),
            buffer: 0 as *mut u8,
            buffer_length: 0,
            actual_length: 0,
            start_frame: 0,
            number_of_packets: 0,
            error_count: 0,
            signr: 0,
            usercontext: 0,
        }
    }
}

// Remaining elements of linux usbfs that have not been implemented in this crate.
// These are left here as a reminder of things that can yet be implemented.

// FIXME: shouldn't be pub
// #[allow(non_snake_case)]
// #[derive(Debug)]
// #[repr(C)]
// pub struct bulktransfer {
//     pub ep      :u32,
//     pub len     :u32,
//     pub timeout :u32, // in milliseconds
//     pub data    :*mut u8,
// }

// struct usbdevfs_setinterface {
//  unsigned int interface;
//  unsigned int altsetting;
// };

#[derive(Debug)]
#[repr(C)]
pub struct SetInterface {
    pub interface: c_uint,
    pub altsetting: c_uint,
}

// struct usbdevfs_disconnectsignal {
//  unsigned int signr;
//  void __user *context;
// };

// #define USBDEVFS_MAXDRIVERNAME 255

// struct usbdevfs_getdriver {
//  unsigned int interface;
//  char driver[USBDEVFS_MAXDRIVERNAME + 1];
// };

// struct usbdevfs_connectinfo {
//  unsigned int devnum;
//  unsigned char slow;
// };

#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct IsoPacketDesc {
    pub length: i32, // kernel header uses unsigned int, but use i32 instead for consistency with urb.
    pub actual_length: i32, /* kernel header uses unsigned int, but use i32 instead for consistency with urb. */
    pub status: i32, // kernel header uses unsigned int, but use i32 instead for consistency with urb.
}


impl Default for IsoPacketDesc {
    fn default() -> IsoPacketDesc {
        IsoPacketDesc {
            length: 0,
            actual_length: 0,
            status: -22, // -EINVAL, in case status is read before urb is used.
        }
    }
}

// struct usbdevfs_urb {
//  unsigned char type;
//  unsigned char endpoint;
//  int status;
//  unsigned int flags;
//  void __user *buffer;
//  int buffer_length;
//  int actual_length;
//  int start_frame;
//  union {
//      int number_of_packets;  /* Only used for isoc urbs */
//      unsigned int stream_id; /* Only used with bulk streams */
//  };
//  int error_count;
//  unsigned int signr; /* signal to be sent on completion,
//                or 0 if none should be sent. */
//  void __user *usercontext;
//  struct usbdevfs_iso_packet_desc iso_frame_desc[0];
// };

// /* System and bus capability flags */
// #define USBDEVFS_CAP_ZERO_PACKET     0x01
// #define USBDEVFS_CAP_BULK_CONTINUATION       0x02
// #define USBDEVFS_CAP_NO_PACKET_SIZE_LIM      0x04
// #define USBDEVFS_CAP_BULK_SCATTER_GATHER 0x08
// #define USBDEVFS_CAP_REAP_AFTER_DISCONNECT   0x10
// #define USBDEVFS_CAP_MMAP            0x20
// #define USBDEVFS_CAP_DROP_PRIVILEGES     0x40

// /* USBDEVFS_DISCONNECT_CLAIM flags & struct */

// /* disconnect-and-claim if the driver matches the driver field */
// #define USBDEVFS_DISCONNECT_CLAIM_IF_DRIVER  0x01
// /* disconnect-and-claim except when the driver matches the driver field */
// #define USBDEVFS_DISCONNECT_CLAIM_EXCEPT_DRIVER  0x02

// struct usbdevfs_disconnect_claim {
//  unsigned int interface;
//  unsigned int flags;
//  char driver[USBDEVFS_MAXDRIVERNAME + 1];
// };

// struct usbdevfs_streams {
//  unsigned int num_streams; /* Not used by USBDEVFS_FREE_STREAMS */
//  unsigned int num_eps;
//  unsigned char eps[0];
// };


// Sigh, usbfs ioctls have incorrect inversion of read and write.
// This doesn't matter at all from C, but nix crate applies const/mut to
// wrappers.


// #define USBDEVFS_CONTROL           _IOWR('U', 0, struct usbdevfs_ctrltransfer)
ioctl_readwrite!(control, b'U', 0, CtrlTransfer);

// #define USBDEVFS_CONTROL32           _IOWR('U', 0, struct usbdevfs_ctrltransfer32)
// #define USBDEVFS_BULK              _IOWR('U', 2, struct usbdevfs_bulktransfer)
// #define USBDEVFS_BULK32              _IOWR('U', 2, struct usbdevfs_bulktransfer32)
// #define USBDEVFS_RESETEP           _IOR('U', 3, unsigned int)

// #define USBDEVFS_SETINTERFACE      _IOR('U', 4, struct usbdevfs_setinterface)
ioctl_write_ptr_bad!(setinterface, request_code_read!('U', 4, size_of::<SetInterface>()), SetInterface);

// #define USBDEVFS_SETCONFIGURATION  _IOR('U', 5, unsigned int)
// #define USBDEVFS_GETDRIVER         _IOW('U', 8, struct usbdevfs_getdriver)

// #define USBDEVFS_SUBMITURB         _IOR('U', 10, struct usbdevfs_urb)
ioctl_write_ptr_bad!(submiturb, request_code_read!(b'U', 10, size_of::<Urb>()), Urb);

// #define USBDEVFS_SUBMITURB32       _IOR('U', 10, struct usbdevfs_urb32)
// #define USBDEVFS_DISCARDURB        _IO('U', 11)
//pub const DISCARDURB_IOCTL: libc::c_ulong = io!(b'U', 11) as libc::c_ulong;
// ioctl!(none discardurb with b'U', 11; Urb);  // doesn't work due to defective ioctl def (discardurb actually does take a param)


// #define USBDEVFS_REAPURB           _IOW('U', 12, void *)
ioctl_read_bad!(reapurb, request_code_write!(b'U', 12, size_of::<*mut Urb>()), *mut Urb);

// #define USBDEVFS_REAPURB32         _IOW('U', 12, __u32)

// #define USBDEVFS_REAPURBNDELAY     _IOW('U', 13, void *)
ioctl_read_bad!(reapurbndelay, request_code_write!(b'U', 13, size_of::<*mut Urb>()), *mut Urb);

// #define USBDEVFS_REAPURBNDELAY32   _IOW('U', 13, __u32)
// #define USBDEVFS_DISCSIGNAL        _IOR('U', 14, struct usbdevfs_disconnectsignal)
// #define USBDEVFS_DISCSIGNAL32      _IOR('U', 14, struct usbdevfs_disconnectsignal32)

// #define USBDEVFS_CLAIMINTERFACE    _IOR('U', 15, unsigned int)
//ioctl_write_bad!(claiminterface, request_code_read!('U', 15, sizeof::<c_uint>()), c_uint);
ioctl_write_ptr_bad!(claiminterface, request_code_read!('U', 15, size_of::<c_uint>()), c_uint);

// #define USBDEVFS_RELEASEINTERFACE  _IOR('U', 16, unsigned int)
// #define USBDEVFS_CONNECTINFO       _IOW('U', 17, struct usbdevfs_connectinfo)
// #define USBDEVFS_IOCTL             _IOWR('U', 18, struct usbdevfs_ioctl)
// #define USBDEVFS_IOCTL32           _IOWR('U', 18, struct usbdevfs_ioctl32)
// #define USBDEVFS_HUB_PORTINFO      _IOR('U', 19, struct usbdevfs_hub_portinfo)
// #define USBDEVFS_RESET             _IO('U', 20)
// #define USBDEVFS_CLEAR_HALT        _IOR('U', 21, unsigned int)
// #define USBDEVFS_DISCONNECT        _IO('U', 22)
// #define USBDEVFS_CONNECT           _IO('U', 23)
// #define USBDEVFS_CLAIM_PORT        _IOR('U', 24, unsigned int)
// #define USBDEVFS_RELEASE_PORT      _IOR('U', 25, unsigned int)
// #define USBDEVFS_GET_CAPABILITIES  _IOR('U', 26, __u32)
// #define USBDEVFS_DISCONNECT_CLAIM  _IOR('U', 27, struct usbdevfs_disconnect_claim)
// #define USBDEVFS_ALLOC_STREAMS     _IOR('U', 28, struct usbdevfs_streams)
// #define USBDEVFS_FREE_STREAMS      _IOR('U', 29, struct usbdevfs_streams)
// #define USBDEVFS_DROP_PRIVILEGES   _IOW('U', 30, __u32)

fn nix_err_to_io_err(err: nix::Error) -> io::Error {
    io::Error::from(err)
}

pub fn nix_result_to_io_result<T>(res: nix::Result<T>) -> io::Result<T> {
    res.map_err(nix_err_to_io_err)
}
